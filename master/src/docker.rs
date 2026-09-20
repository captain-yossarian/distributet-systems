use std::collections::HashMap;

use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, StartContainerOptions,
    StopContainerOptions,
};
use bollard::models::HostConfig;
use bollard::Docker;
use common::ContainerInfo;

/// Every secondary container — compose-managed or api-spawned — carries
/// this label, so both kinds can be found together (including stopped
/// ones, which `docker-compose ps`/the live registry alone would miss).
const SECONDARY_LABEL: &str = "app.role=secondary";

pub async fn connect() -> Docker {
    Docker::connect_with_socket_defaults().expect("failed to connect to the docker socket")
}

/// A `docker ps`-equivalent listing of every currently-running container on
/// the host (not scoped to this compose project — master's docker-socket
/// access is host-wide, so this shows everything the daemon knows about).
pub async fn list_containers(docker: &Docker) -> Result<Vec<ContainerInfo>, bollard::errors::Error> {
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: false,
            ..Default::default()
        }))
        .await?;

    Ok(containers
        .into_iter()
        .map(|container| ContainerInfo {
            id: container.id.unwrap_or_default().chars().take(12).collect(),
            name: container
                .names
                .and_then(|names| names.into_iter().next())
                .map(|name| name.trim_start_matches('/').to_string())
                .unwrap_or_default(),
            image: container.image.unwrap_or_default(),
            state: container.state.unwrap_or_default(),
            status: container.status.unwrap_or_default(),
        })
        .collect())
}

/// A short (12-char, matching the `HOSTNAME` a secondary reports as its own
/// id) container id plus whether it's currently running, for every
/// container labeled as a secondary — running or stopped.
pub async fn list_secondary_containers(
    docker: &Docker,
) -> Result<Vec<(String, bool)>, bollard::errors::Error> {
    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec![SECONDARY_LABEL.to_string()]);

    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            filters,
            ..Default::default()
        }))
        .await?;

    Ok(containers
        .into_iter()
        .map(|container| {
            let id = container.id.unwrap_or_default().chars().take(12).collect();
            let running = container.state.as_deref() == Some("running");
            (id, running)
        })
        .collect())
}

/// Creates and starts a new `secondary` container on the same image and
/// docker network as this (master's) own container, so it comes up ready
/// to register itself with master. Returns the new container's id.
pub async fn spawn_secondary(docker: &Docker) -> Result<String, bollard::errors::Error> {
    let self_id =
        std::env::var("HOSTNAME").expect("HOSTNAME env var not set (not running in docker?)");
    let self_info = docker.inspect_container(&self_id, None).await?;

    let image = self_info
        .config
        .and_then(|config| config.image)
        .expect("master's own container has no image")
        .replace("-master", "-secondary");

    let network_name = self_info
        .network_settings
        .and_then(|settings| settings.networks)
        .and_then(|networks| networks.into_keys().next())
        .expect("master's own container is not attached to any docker network");

    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis:6379".to_string());
    let master_url =
        std::env::var("SELF_URL").unwrap_or_else(|_| "http://master:3000".to_string());

    let config = Config {
        image: Some(image),
        cmd: Some(vec!["secondary".to_string()]),
        env: Some(vec![
            format!("REDIS_URL={redis_url}"),
            format!("MASTER_URL={master_url}"),
        ]),
        labels: Some(HashMap::from([("app.role".to_string(), "secondary".to_string())])),
        host_config: Some(HostConfig {
            network_mode: Some(network_name),
            ..Default::default()
        }),
        ..Default::default()
    };

    let created = docker
        .create_container(None::<CreateContainerOptions<String>>, config)
        .await?;
    docker
        .start_container(&created.id, None::<StartContainerOptions<String>>)
        .await?;

    Ok(created.id)
}

/// Stops a secondary's container without removing it — the container (and
/// therefore its id) still exists afterward, ready for `start_container`.
pub async fn stop_container(docker: &Docker, id: &str) -> Result<(), bollard::errors::Error> {
    docker.stop_container(id, None::<StopContainerOptions>).await
}

/// Starts a previously-stopped container back up under the same id.
pub async fn start_container(docker: &Docker, id: &str) -> Result<(), bollard::errors::Error> {
    docker
        .start_container(id, None::<StartContainerOptions<String>>)
        .await
}
