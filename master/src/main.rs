use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use common::{
    BroadcastResult, CreateMessage, LoggedMessage, LogsResponse, MasterLogEntry, SecondaryInfo,
    SecondaryLog,
};
use tower_http::cors::CorsLayer;

mod redis_connection;

use redis_connection::RedisConnection;

#[derive(Clone)]
struct AppState {
    redis: RedisConnection,
    http: reqwest::Client,
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let state = AppState {
        redis: redis_connection::connect_redis().await,
        http: reqwest::Client::new(),
    };

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /post_message` goes to `post_message`
        .route("/post_message", post(post_message))
        // secondaries self-register (and re-register on heartbeat) here
        .route("/secondaries/register", post(register_secondary))
        // `GET /secondaries` lists the currently alive secondaries
        .route("/secondaries", get(list_secondaries))
        // `GET /logs` returns master + every secondary's message history
        .route("/logs", get(get_logs))
        .with_state(state)
        .layer(CorsLayer::permissive());

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}

async fn register_secondary(
    State(mut state): State<AppState>,
    Json(payload): Json<SecondaryInfo>,
) -> StatusCode {
    redis_connection::upsert_secondary(&mut state.redis, &payload).await;
    StatusCode::OK
}

async fn list_secondaries(State(mut state): State<AppState>) -> Json<Vec<SecondaryInfo>> {
    Json(redis_connection::list_secondaries(&mut state.redis).await)
}

async fn post_message(
    State(mut state): State<AppState>,
    // this argument tells axum to parse the request body
    // as JSON into a `CreateMessage` type
    Json(payload): Json<CreateMessage>,
) -> (StatusCode, Json<BroadcastResult>) {
    let timestamp = Utc::now().to_rfc3339();
    let wire_entry = LoggedMessage {
        message: payload.message.clone(),
        timestamp: timestamp.clone(),
    };

    // broadcast the message to every currently registered secondary and
    // wait for each one's ACK (a successful `/receive` response) before
    // counting it as delivered
    let secondaries = redis_connection::list_secondaries(&mut state.redis).await;
    let mut delivered = Vec::new();
    let mut failed = Vec::new();
    for secondary in &secondaries {
        let url = format!("{}/receive", secondary.address);
        match state.http.post(&url).json(&wire_entry).send().await {
            Ok(resp) if resp.status().is_success() => delivered.push(secondary.id.clone()),
            _ => failed.push(secondary.id.clone()),
        }
    }

    let log_entry = MasterLogEntry {
        message: payload.message.clone(),
        timestamp: timestamp.clone(),
        sent: secondaries.len(),
        acked: delivered.len(),
    };
    redis_connection::log_master_message(&mut state.redis, &log_entry).await;

    let result = BroadcastResult {
        message: payload.message,
        timestamp,
        delivered,
        failed,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(result))
}

async fn get_logs(State(mut state): State<AppState>) -> Json<LogsResponse> {
    let master = redis_connection::get_master_messages(&mut state.redis).await;

    let secondaries = redis_connection::list_secondaries(&mut state.redis).await;
    let mut secondary_logs = Vec::new();
    for secondary in &secondaries {
        let messages =
            redis_connection::get_secondary_messages(&mut state.redis, &secondary.id).await;
        secondary_logs.push(SecondaryLog {
            id: secondary.id.clone(),
            messages,
        });
    }

    Json(LogsResponse {
        master,
        secondaries: secondary_logs,
    })
}
