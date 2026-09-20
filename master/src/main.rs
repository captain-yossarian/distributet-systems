use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use common::{
    BroadcastResult, ContainerInfo, CreateMessage, LoggedMessage, LogsResponse, MasterLogEntry,
    SecondaryInfo, SecondaryLog, SecondaryNode, SecondarySettings,
};
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;

mod docker;
mod redis_connection;

use redis_connection::RedisConnection;

#[derive(Clone)]
struct AppState {
    redis: RedisConnection,
    http: reqwest::Client,
    docker: bollard::Docker,
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let state = AppState {
        redis: redis_connection::connect_redis().await,
        http: reqwest::Client::new(),
        docker: docker::connect().await,
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
        // launches a new secondary container via the docker socket
        .route("/secondaries/spawn", post(spawn_secondary))
        // real docker stop/start of one secondary's container
        .route("/secondaries/:id/stop", post(stop_secondary))
        .route("/secondaries/:id/start", post(start_secondary))
        // read/update one secondary's test settings (name, delay)
        .route(
            "/secondaries/:id/settings",
            get(get_secondary_settings).post(update_secondary_settings),
        )
        // `GET /logs` returns master + every secondary's message history
        .route("/logs", get(get_logs))
        // canonical broadcast history, pulled by secondaries for catch-up sync
        .route("/messages", get(get_all_messages))
        // `GET /docker/ps` lists currently-running containers on the host
        .route("/docker/ps", get(docker_ps))
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

/// The secondaries the UI should know about: every container Docker has
/// labeled as a secondary (running or stopped), with a reachable address
/// filled in from the live registry wherever one is currently registered.
/// A stopped container has no address (nothing to reach), but it still
/// appears — that's what makes `/secondaries/:id/start` discoverable.
async fn list_secondary_nodes(state: &mut AppState) -> Vec<SecondaryNode> {
    let containers = docker::list_secondary_containers(&state.docker)
        .await
        .unwrap_or_default();
    let registered = redis_connection::list_secondaries(&mut state.redis).await;
    let addresses: std::collections::HashMap<String, String> = registered
        .into_iter()
        .map(|secondary| (secondary.id, secondary.address))
        .collect();

    containers
        .into_iter()
        .map(|(id, running)| {
            let address = addresses.get(&id).cloned();
            SecondaryNode {
                address,
                running,
                id,
            }
        })
        .collect()
}

async fn list_secondaries(State(mut state): State<AppState>) -> Json<Vec<SecondaryNode>> {
    Json(list_secondary_nodes(&mut state).await)
}

async fn spawn_secondary(State(state): State<AppState>) -> StatusCode {
    match docker::spawn_secondary(&state.docker).await {
        Ok(id) => {
            tracing::info!("spawned new secondary container {id}");
            StatusCode::CREATED
        }
        Err(err) => {
            tracing::error!("failed to spawn secondary container: {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn stop_secondary(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    match docker::stop_container(&state.docker, &id).await {
        Ok(()) => StatusCode::OK,
        Err(err) => {
            tracing::error!("failed to stop container {id}: {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn start_secondary(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    match docker::start_container(&state.docker, &id).await {
        Ok(()) => StatusCode::OK,
        Err(err) => {
            tracing::error!("failed to start container {id}: {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn find_secondary(state: &mut AppState, id: &str) -> Result<SecondaryInfo, StatusCode> {
    redis_connection::list_secondaries(&mut state.redis)
        .await
        .into_iter()
        .find(|secondary| secondary.id == id)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn get_secondary_settings(
    State(mut state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SecondarySettings>, StatusCode> {
    let secondary = find_secondary(&mut state, &id).await?;
    let url = format!("{}/settings", secondary.address);
    let resp = state
        .http
        .get(&url)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let settings = resp
        .json::<SecondarySettings>()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(Json(settings))
}

async fn update_secondary_settings(
    State(mut state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SecondarySettings>,
) -> Result<Json<SecondarySettings>, StatusCode> {
    let secondary = find_secondary(&mut state, &id).await?;
    let url = format!("{}/settings", secondary.address);
    let resp = state
        .http
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let settings = resp
        .json::<SecondarySettings>()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(Json(settings))
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

    // Broadcast to every currently registered secondary as an independent,
    // detached task (tokio::spawn) so each one always runs to completion —
    // and therefore still logs the message on its end — even if this
    // handler stops waiting before it finishes. Write concern: `w` counts
    // master's own (always-satisfied) write as 1, so the number of
    // secondary ACKs actually waited on is `w - 1`.
    let secondaries = redis_connection::list_secondaries(&mut state.redis).await;
    let total = secondaries.len();
    let w = payload.write_concern.unwrap_or(1).clamp(1, 1 + total);
    let target = w - 1;

    let (tx, mut rx) = mpsc::channel::<(String, bool)>(total.max(1));
    for secondary in &secondaries {
        let url = format!("{}/receive", secondary.address);
        let client = state.http.clone();
        let entry = wire_entry.clone();
        let id = secondary.id.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            let acked = matches!(
                client.post(&url).json(&entry).send().await,
                Ok(resp) if resp.status().is_success()
            );
            let _ = tx.send((id, acked)).await;
        });
    }
    drop(tx);

    let mut delivered = Vec::new();
    let mut failed = Vec::new();
    while delivered.len() < target {
        match rx.recv().await {
            Some((id, true)) => delivered.push(id),
            Some((id, false)) => failed.push(id),
            // every task has reported in — `target` was never reachable
            // (too many failures), nothing more to wait for
            None => break,
        }
    }

    let log_entry = MasterLogEntry {
        message: payload.message.clone(),
        timestamp: timestamp.clone(),
        sent: total,
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

async fn docker_ps(State(state): State<AppState>) -> Result<Json<Vec<ContainerInfo>>, StatusCode> {
    docker::list_containers(&state.docker)
        .await
        .map(Json)
        .map_err(|err| {
            tracing::error!("failed to list containers: {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// The full canonical history of broadcast messages, in order. Secondaries
/// poll this to catch up on anything they missed (while simulated down, or
/// after a real restart) that live `/receive` calls never redelivered.
async fn get_all_messages(State(mut state): State<AppState>) -> Json<Vec<LoggedMessage>> {
    let master = redis_connection::get_master_messages(&mut state.redis).await;
    Json(
        master
            .into_iter()
            .map(|entry| LoggedMessage {
                message: entry.message,
                timestamp: entry.timestamp,
            })
            .collect(),
    )
}

async fn get_logs(State(mut state): State<AppState>) -> Json<LogsResponse> {
    let master = redis_connection::get_master_messages(&mut state.redis).await;

    // includes stopped secondaries too, so a container's history doesn't
    // vanish from the UI just because it's currently powered off
    let nodes = list_secondary_nodes(&mut state).await;
    let mut secondary_logs = Vec::new();
    for node in &nodes {
        let messages = redis_connection::get_secondary_messages(&mut state.redis, &node.id).await;
        secondary_logs.push(SecondaryLog {
            id: node.id.clone(),
            messages,
        });
    }

    Json(LogsResponse {
        master,
        secondaries: secondary_logs,
    })
}
