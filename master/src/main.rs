use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

mod redis_connection;

use redis_connection::RedisConnection;

#[derive(Clone)]
struct AppState {
    redis: RedisConnection,
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let state = AppState {
        redis: redis_connection::connect_redis().await,
    };

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /post_message` goes to `post_message`
        .route("/post_message", post(post_message))
        .with_state(state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("localhost:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}

async fn post_message(
    State(mut state): State<AppState>,
    // this argument tells axum to parse the request body
    // as JSON into a `CreateMessage` type
    Json(payload): Json<CreateMessage>,
) -> (StatusCode, Json<Message>) {
    redis_connection::store_message(&mut state.redis, &payload.message).await;

    let message = Message {
        message: payload.message,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(message))
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateMessage {
    message: String,
}

#[derive(Serialize)]
struct Message {
    message: String,
}
