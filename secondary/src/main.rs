use std::net::UdpSocket;
use std::time::Duration;

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use common::{LoggedMessage, SecondaryInfo};

mod redis_connection;

use redis_connection::RedisConnection;

#[derive(Clone)]
struct AppState {
    redis: RedisConnection,
    id: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let id =
        std::env::var("HOSTNAME").unwrap_or_else(|_| format!("secondary-{}", std::process::id()));
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4000);
    let master_url =
        std::env::var("MASTER_URL").unwrap_or_else(|_| "http://master:3000".to_string());
    let address = format!("http://{}:{port}", local_ip());

    let state = AppState {
        redis: redis_connection::connect_redis().await,
        id: id.clone(),
    };

    let app = Router::new()
        .route("/receive", post(receive_message))
        .with_state(state);

    tokio::spawn(register_loop(master_url, id.clone(), address));

    tracing::info!("secondary {id} listening on port {port}");
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn receive_message(
    State(mut state): State<AppState>,
    Json(entry): Json<LoggedMessage>,
) -> StatusCode {
    redis_connection::log_message(&mut state.redis, &state.id, &entry).await;
    StatusCode::OK
}

/// Repeatedly (re-)registers this secondary with master so master's
/// registry entry (TTL-backed) never expires while this process is alive.
async fn register_loop(master_url: String, id: String, address: String) {
    let client = reqwest::Client::new();
    let info = SecondaryInfo { id, address };
    let url = format!("{master_url}/secondaries/register");
    loop {
        if let Err(err) = client.post(&url).json(&info).send().await {
            tracing::warn!("failed to register with master: {err}");
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// Determines the IP address other containers on this Docker network would
/// use to reach this process. Connecting a UDP socket does not send any
/// packets; it only asks the kernel to resolve the outbound route, which is
/// enough to read back the local interface address.
fn local_ip() -> String {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("failed to bind udp socket");
    socket
        .connect("8.8.8.8:80")
        .expect("failed to resolve local ip via route lookup");
    socket
        .local_addr()
        .expect("failed to read local socket address")
        .ip()
        .to_string()
}
