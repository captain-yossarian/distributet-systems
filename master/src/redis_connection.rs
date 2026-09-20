use chrono::Utc;
use common::{LoggedMessage, MasterLogEntry, SecondaryInfo};
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

pub async fn log_master_message(conn: &mut RedisConnection, entry: &MasterLogEntry) {
    let payload = serde_json::to_string(entry).expect("failed to serialize message");
    let _: () = conn
        .rpush("messages:master", payload)
        .await
        .expect("failed to store message in redis");
}

pub async fn get_master_messages(conn: &mut RedisConnection) -> Vec<MasterLogEntry> {
    let raw: Vec<String> = conn
        .lrange("messages:master", 0, -1)
        .await
        .expect("failed to read messages from redis");
    raw.iter()
        .filter_map(|entry| serde_json::from_str(entry).ok())
        .collect()
}

pub async fn get_secondary_messages(conn: &mut RedisConnection, id: &str) -> Vec<LoggedMessage> {
    let key = format!("messages:secondary:{id}");
    let raw: Vec<String> = conn
        .lrange(key, 0, -1)
        .await
        .expect("failed to read messages from redis");
    raw.iter()
        .filter_map(|entry| serde_json::from_str(entry).ok())
        .collect()
}

/// Registers (or refreshes) a secondary. Registry entries expire after
/// `SECONDARY_TTL_SECONDS` so a secondary that stops heartbeating
/// automatically drops out, but its position in `SECONDARY_ORDER_KEY` (set
/// only the first time it's seen) is preserved so re-registering doesn't
/// move it to the back of the list.
pub async fn upsert_secondary(conn: &mut RedisConnection, info: &SecondaryInfo) {
    let key = format!("secondary:registry:{}", info.id);
    let payload = serde_json::to_string(info).expect("failed to serialize secondary info");
    let _: () = conn
        .set_ex(key, payload, SECONDARY_TTL_SECONDS)
        .await
        .expect("failed to register secondary");

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

pub async fn list_secondaries(conn: &mut RedisConnection) -> Vec<SecondaryInfo> {
    let ids: Vec<String> = conn
        .zrange(SECONDARY_ORDER_KEY, 0, -1)
        .await
        .expect("failed to read secondary registration order");

    let mut result = Vec::new();
    for id in ids {
        let key = format!("secondary:registry:{id}");
        match conn.get::<_, Option<String>>(&key).await {
            Ok(Some(raw)) => {
                if let Ok(info) = serde_json::from_str::<SecondaryInfo>(&raw) {
                    result.push(info);
                }
            }
            _ => {
                // registry entry expired: drop it from the ordering set too
                let _: redis::RedisResult<()> = conn.zrem(SECONDARY_ORDER_KEY, &id).await;
            }
        }
    }
    result
}
