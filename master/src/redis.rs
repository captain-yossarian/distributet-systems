use redis::AsyncCommands;

fn connect_redis() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let redis_client = redis::Client::open(redis_url).expect("invalid REDIS_URL");
    let redis_conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("failed to connect to redis");
    let state = AppState { redis: redis_conn };
}
