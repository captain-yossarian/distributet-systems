use redis::AsyncCommands;

pub type RedisConnection = redis::aio::MultiplexedConnection;

pub async fn connect_redis() -> RedisConnection {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let redis_client = redis::Client::open(redis_url).expect("invalid REDIS_URL");
    redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("failed to connect to redis")
}

pub async fn store_message(conn: &mut RedisConnection, message: &str) {
    let _: () = conn
        .rpush("messages", message)
        .await
        .expect("failed to store message in redis");
}
