use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use common::{
    BroadcastResult, ContainerInfo, CreateMessage, LoggedMessage, LogsResponse, MasterLogEntry,
    RetryEntry, SecondaryInfo, SecondaryLog, SecondaryNode, SecondarySettings,
};
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;

/// Delivery retry policy for `POST /receive`: if a secondary doesn't
/// respond successfully, retry this many more times, waiting this long
/// between each attempt, before finally counting it as failed.
const MAX_DELIVERY_RETRIES: u32 = 3;
const RETRY_INTERVAL: Duration = Duration::from_secs(10);

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
        // Without an explicit timeout, a secondary that's genuinely
        // unreachable (its container stopped, its network endpoint gone)
        // doesn't fail fast with "connection refused" — the connect
        // attempt can hang on OS-level TCP retransmission for a long time
        // (observed: over a minute), which would make the retry policy's
        // 10s interval meaningless. Capped well under that interval so
        // each attempt fails predictably before the next retry is due.
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .expect("failed to build http client"),
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
        // user-initiated removal from the known-secondaries list — the
        // only thing that ever takes a secondary off it now
        .route("/secondaries/:id/unregister", post(unregister_secondary))
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

/// The secondaries the UI should know about: every secondary that has ever
/// registered, still present regardless of what Docker currently thinks of
/// its container (stopped, or even removed outright) — only an explicit
/// `/secondaries/:id/unregister` call takes one off this list. Docker is
/// consulted only to say whether it's *currently running*.
async fn list_secondary_nodes(state: &mut AppState) -> Vec<SecondaryNode> {
    let containers = docker::list_secondary_containers(&state.docker)
        .await
        .unwrap_or_default();
    let running_ids: std::collections::HashSet<String> = containers
        .into_iter()
        .filter(|(_, running)| *running)
        .map(|(id, _)| id)
        .collect();

    redis_connection::list_known_secondaries(&mut state.redis)
        .await
        .into_iter()
        .map(|secondary| SecondaryNode {
            running: running_ids.contains(&secondary.id),
            address: Some(secondary.address),
            id: secondary.id,
        })
        .collect()
}

/// The secondaries a message broadcast should target: every *known*
/// secondary with a recorded address, regardless of whether it's
/// currently heartbeating. Stopping a secondary must not remove it from
/// this list — delivery to it should fail (and retry, and get logged),
/// not silently skip it.
async fn list_broadcast_targets(state: &mut AppState) -> Vec<SecondaryInfo> {
    redis_connection::list_known_secondaries(&mut state.redis).await
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

/// Removes a secondary from the known list entirely. Only allowed while
/// it's stopped — unregistering a still-running one would be pointless
/// (its own heartbeat would just re-register it within a few seconds) —
/// so this is checked server-side too, not just hidden client-side.
async fn unregister_secondary(
    State(mut state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    let nodes = list_secondary_nodes(&mut state).await;
    match nodes.into_iter().find(|node| node.id == id) {
        Some(node) if node.running => StatusCode::CONFLICT,
        _ => {
            redis_connection::forget_secondary(&mut state.redis, &id).await;
            tracing::info!("unregistered secondary {id}");
            StatusCode::OK
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

    // Broadcast to every *known* secondary (not just currently-live ones —
    // a stopped secondary must still be attempted, so it correctly shows
    // up as a failure/retry rather than silently dropping out of `total`)
    // as an independent, detached task (tokio::spawn) so each one always
    // runs to completion — and therefore still logs the message on its
    // end — even if this handler stops waiting before it finishes. Write
    // concern: `w` counts master's own (always-satisfied) write as 1, so
    // the number of secondary ACKs actually waited on is `w - 1`.
    let secondaries = list_broadcast_targets(&mut state).await;
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
        let mut redis = state.redis.clone();
        tokio::spawn(async move {
            let mut acked;
            let mut attempt = 0;
            loop {
                acked = matches!(
                    client.post(&url).json(&entry).send().await,
                    Ok(resp) if resp.status().is_success()
                );
                if acked || attempt >= MAX_DELIVERY_RETRIES {
                    break;
                }
                attempt += 1;
                tracing::warn!(
                    "delivery to {id} failed, retrying ({attempt}/{MAX_DELIVERY_RETRIES}) in {}s",
                    RETRY_INTERVAL.as_secs()
                );
                redis_connection::log_retry(
                    &mut redis,
                    &id,
                    &RetryEntry {
                        message: entry.message.clone(),
                        timestamp: entry.timestamp.clone(),
                        attempt,
                        max_attempts: MAX_DELIVERY_RETRIES,
                    },
                )
                .await;
                tokio::time::sleep(RETRY_INTERVAL).await;
            }
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

    // The response (and the log entry above) reflect the outcome as of the
    // moment write concern was satisfied — but a secondary still mid-retry
    // at that moment can succeed afterward, well after this request has
    // already returned. Keep draining stragglers in the background and
    // correct the logged `acked` count once the true final outcome is
    // known, instead of leaving it permanently frozen at a now-stale
    // snapshot. `log_master_message` upserts by timestamp, so this simply
    // overwrites the entry written above.
    {
        let mut acked_count = delivered.len();
        let message = payload.message.clone();
        let timestamp = timestamp.clone();
        let mut redis = state.redis.clone();
        tokio::spawn(async move {
            while let Some((_id, acked)) = rx.recv().await {
                if acked {
                    acked_count += 1;
                }
            }
            let corrected_entry = MasterLogEntry {
                message,
                timestamp,
                sent: total,
                acked: acked_count,
            };
            redis_connection::log_master_message(&mut redis, &corrected_entry).await;
        });
    }

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
        let retries = redis_connection::get_retries(&mut state.redis, &node.id).await;
        secondary_logs.push(SecondaryLog {
            id: node.id.clone(),
            messages,
            retries,
        });
    }

    Json(LogsResponse {
        master,
        secondaries: secondary_logs,
    })
}
