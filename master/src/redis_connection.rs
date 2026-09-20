use chrono::Utc;
use common::{LoggedMessage, MasterLogEntry, RetryEntry, SecondaryInfo};
use redis::AsyncCommands;

pub type RedisConnection = redis::aio::MultiplexedConnection;

const SECONDARY_TTL_SECONDS: u64 = 20;

/// Sorted set of secondary ids, scored by first-registration time, so the
/// registry can be listed in the order secondaries first appeared instead
/// of the arbitrary order `KEYS`/`HashMap` would give.
const SECONDARY_ORDER_KEY: &str = "secondary:order";

pub async fn connect_redis() -> RedisConnection {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let redis_client = redis::Client::open(redis_url).expect("invalid REDIS_URL");
    redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("failed to connect to redis")
}

/// Keyed by timestamp (RFC3339, so lexical order == chronological order)
/// rather than appended to a list, so a message's entry can be corrected
/// in place — the initial write happens as soon as write concern is
/// satisfied, but a straggler secondary can still succeed afterward (e.g.
/// after several retries), and this lets that be reflected by simply
/// writing the same timestamp again with the updated `acked` count.
pub async fn log_master_message(conn: &mut RedisConnection, entry: &MasterLogEntry) {
    let payload = serde_json::to_string(entry).expect("failed to serialize message");
    let _: () = conn
        .hset("messages:master", &entry.timestamp, payload)
        .await
        .expect("failed to store message in redis");
}

pub async fn get_master_messages(conn: &mut RedisConnection) -> Vec<MasterLogEntry> {
    let map: std::collections::HashMap<String, String> = conn
        .hgetall("messages:master")
        .await
        .expect("failed to read messages from redis");
    let mut entries: Vec<MasterLogEntry> = map
        .values()
        .filter_map(|raw| serde_json::from_str(raw).ok())
        .collect();
    entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    entries
}

/// Secondaries store their received messages in a hash keyed by timestamp
/// (RFC3339, so lexical order == chronological order) rather than an
/// append-only list, so that both the live `/receive` path and their
/// periodic catch-up sync can write idempotently without ever duplicating
/// an entry.
pub async fn get_secondary_messages(conn: &mut RedisConnection, id: &str) -> Vec<LoggedMessage> {
    let key = format!("messages:secondary:{id}");
    let map: std::collections::HashMap<String, String> = conn
        .hgetall(key)
        .await
        .expect("failed to read messages from redis");
    let mut entries: Vec<LoggedMessage> = map
        .into_iter()
        .map(|(timestamp, message)| LoggedMessage { message, timestamp })
        .collect();
    entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    entries
}

/// Records one retry attempt master made against a secondary, so the UI
/// can show it even for a secondary that never actually received the
/// message (in which case it has no entry in its own "received" log at
/// all — this is the only record of the delivery attempt existing).
pub async fn log_retry(conn: &mut RedisConnection, id: &str, entry: &RetryEntry) {
    let key = format!("retries:secondary:{id}");
    let payload = serde_json::to_string(entry).expect("failed to serialize retry entry");
    let _: () = conn
        .rpush(key, payload)
        .await
        .expect("failed to store retry in redis");
}

pub async fn get_retries(conn: &mut RedisConnection, id: &str) -> Vec<RetryEntry> {
    let key = format!("retries:secondary:{id}");
    let raw: Vec<String> = conn
        .lrange(key, 0, -1)
        .await
        .expect("failed to read retries from redis");
    raw.iter()
        .filter_map(|entry| serde_json::from_str(entry).ok())
        .collect()
}

/// Registers (or refreshes) a secondary. The liveness entry expires after
/// `SECONDARY_TTL_SECONDS` so a secondary that stops heartbeating no
/// longer counts as *reachable right now* — but its address is also
/// recorded permanently (see `list_known_secondaries`), and its position
/// in `SECONDARY_ORDER_KEY` (set only the first time it's seen) is never
/// touched here, so stopping a secondary never makes it stop being a
/// *known* one: broadcasts still target it (and correctly record a
/// failure/retry) instead of it silently vanishing from consideration.
pub async fn upsert_secondary(conn: &mut RedisConnection, info: &SecondaryInfo) {
    let key = format!("secondary:registry:{}", info.id);
    let payload = serde_json::to_string(info).expect("failed to serialize secondary info");
    let _: () = conn
        .set_ex(key, payload, SECONDARY_TTL_SECONDS)
        .await
        .expect("failed to register secondary");

    let address_key = format!("secondary:address:{}", info.id);
    let _: () = conn
        .set(address_key, &info.address)
        .await
        .expect("failed to record secondary address");

    let already_ordered: Option<f64> = conn
        .zscore(SECONDARY_ORDER_KEY, &info.id)
        .await
        .expect("failed to check secondary registration order");
    if already_ordered.is_none() {
        let first_seen = Utc::now().timestamp_millis();
        let _: () = conn
            .zadd(SECONDARY_ORDER_KEY, &info.id, first_seen)
            .await
            .expect("failed to record secondary registration order");
    }
}

/// Secondaries currently heartbeating — reachable *right now*. Used only
/// where that specifically matters (proxying the settings UI, which needs
/// an actually-running process to answer). Not used for broadcast
/// targeting: see `list_known_secondaries` for that.
pub async fn list_secondaries(conn: &mut RedisConnection) -> Vec<SecondaryInfo> {
    let ids: Vec<String> = conn
        .zrange(SECONDARY_ORDER_KEY, 0, -1)
        .await
        .expect("failed to read secondary registration order");

    let mut result = Vec::new();
    for id in ids {
        let key = format!("secondary:registry:{id}");
        if let Ok(Some(raw)) = conn.get::<_, Option<String>>(&key).await {
            if let Ok(info) = serde_json::from_str::<SecondaryInfo>(&raw) {
                result.push(info);
            }
        }
    }
    result
}

/// Every secondary this master has ever seen register, with its
/// last-known address, regardless of whether it's currently heartbeating.
/// This is the set that message broadcasts target and that the UI's
/// topology is built from — a stopped secondary stays a known target
/// (delivery to it fails, retries, and gets logged) instead of quietly
/// dropping out once its heartbeat lapses.
pub async fn list_known_secondaries(conn: &mut RedisConnection) -> Vec<SecondaryInfo> {
    let ids: Vec<String> = conn
        .zrange(SECONDARY_ORDER_KEY, 0, -1)
        .await
        .expect("failed to read secondary registration order");

    let mut result = Vec::new();
    for id in ids {
        let key = format!("secondary:address:{id}");
        if let Ok(Some(address)) = conn.get::<_, Option<String>>(&key).await {
            result.push(SecondaryInfo { id, address });
        }
    }
    result
}

/// Permanently forgets a secondary. Only appropriate once Docker confirms
/// the container itself is gone (not merely stopped) — opportunistic
/// cleanup so removed containers don't accumulate here forever.
pub async fn forget_secondary(conn: &mut RedisConnection, id: &str) {
    let _: redis::RedisResult<()> = conn.zrem(SECONDARY_ORDER_KEY, id).await;
    let _: redis::RedisResult<()> = conn.del(format!("secondary:address:{id}")).await;
    let _: redis::RedisResult<()> = conn.del(format!("secondary:registry:{id}")).await;
}
