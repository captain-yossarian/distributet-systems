use std::collections::HashSet;

use common::LoggedMessage;
use redis::AsyncCommands;

pub type RedisConnection = redis::aio::MultiplexedConnection;

fn key_for(id: &str) -> String {
    format!("messages:secondary:{id}")
}

pub async fn connect_redis() -> RedisConnection {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let redis_client = redis::Client::open(redis_url).expect("invalid REDIS_URL");
    redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("failed to connect to redis")
}

/// Stores a received message keyed by its (master-assigned, RFC3339, thus
/// lexically sortable) timestamp. Writing the same timestamp twice is a
/// no-op, which makes this safe to call from both the live `/receive` path
/// and the periodic catch-up sync (see `sync_loop` in main.rs) without ever
/// producing duplicate entries.
pub async fn log_message(conn: &mut RedisConnection, id: &str, entry: &LoggedMessage) {
    let _: () = conn
        .hset(key_for(id), &entry.timestamp, &entry.message)
        .await
        .expect("failed to store message in redis");
}

/// Timestamps of messages this secondary already has, used by the catch-up
/// sync to figure out what it's missing from master's full history.
pub async fn known_timestamps(conn: &mut RedisConnection, id: &str) -> HashSet<String> {
    let map: std::collections::HashMap<String, String> = conn
        .hgetall(key_for(id))
        .await
        .expect("failed to read known messages from redis");
    map.into_keys().collect()
}
