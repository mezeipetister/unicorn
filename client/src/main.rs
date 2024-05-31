use chrono::Utc;
use corelib::Event;
use docker_api::{
    conn::TtyChunk,
    opts::{ContainerFilter, ContainerListOpts, ContainerStatus, EventsOpts, LogsOpts},
    Docker, Result,
};
use futures_util::StreamExt;
use log::info;
use std::collections::HashMap;
use tokio::sync::broadcast::Receiver;
pub use unicorn::unicorn_service_client::UnicornServiceClient;

pub mod unicorn {
    tonic::include_proto!("unicorn");
}

mod prelude;

#[cfg(unix)]
pub fn new_docker() -> Result<Docker> {
    Ok(Docker::unix("/var/run/docker.sock"))
}

async fn start_watcher(
    docker: Docker,
    name: String,
    mut stop_signal: Receiver<()>,
    tx: tokio::sync::broadcast::Sender<Event>,
) {
    // let container = find_container(&docker, name).await;
    println!("Watch logs for: {}", name);

    // Start background process
    tokio::spawn(async move {
        if let mut stream = docker
            .containers()
            .get(&name)
            .logs(
                &LogsOpts::builder()
                    .stderr(true)
                    .stdout(true)
                    .all()
                    .follow(true)
                    .since(&Utc::now())
                    .build(),
            )
            .fuse()
        {
            loop {
                tokio::select! {
                    _ = stop_signal.recv() => {
                        println!("Stop watching logs for: {}", name);
                        break;
                    }
                    log = stream.next() => {
                            if let Some(log) = log {
                                if let Ok(TtyChunk::StdOut(data)) = log {
                                    // Send log event
                                    tx.send(Event::Log {
                                        container_name: name.clone(),
                                        message: String::from_utf8_lossy(&data).to_string(),
                                        timestamp: Utc::now(),
                                    }).unwrap();
                                } else if let Ok(TtyChunk::StdErr(data)) = log {
                                    // Send log event
                                    tx.send(Event::Log {
                                        container_name: name.clone(),
                                        message: String::from_utf8_lossy(&data).to_string(),
                                        timestamp: Utc::now(),
                                    }).unwrap();
                                }
                            }
                    }
                    else => break
                }
            }
        }
    });
}

async fn start(logger: impl LoggerExt) {
    let docker = new_docker().unwrap();

    let mut watchers = HashMap::new();

    let container_names: Vec<String> = docker
        .containers()
        .list(
            &ContainerListOpts::builder()
                .filter([ContainerFilter::Status(ContainerStatus::Running)])
                .build(),
        )
        .await
        .unwrap()
        .iter()
        .map(|c| c.names.to_owned().unwrap_or_default().join(",")[1..].to_string())
        .collect();

    for name in container_names {
        let (tx, rx) = tokio::sync::broadcast::channel(1);
        watchers.insert(name.clone(), tx);
        let docker = new_docker().unwrap();
        start_watcher(docker, name, rx, logger.clone_tx()).await;
    }

    // Wait for events
    let mut stream = docker.events(&EventsOpts::builder().since(&Utc::now()).build());

    while let Some(event) = stream.next().await {
        let event = event.unwrap();
        if let Some(action) = event.action {
            // Get container name
            let container_name = event
                .actor
                .to_owned()
                .unwrap()
                .attributes
                .unwrap()
                .get("name")
                .unwrap()
                .to_owned();

            // Log docker event
            logger.log(Event::DockerAction {
                container_name: container_name.clone(),
                action: action.clone(),
                timestamp: Utc::now(),
            });

            // Handle docker event when container is started
            if action == "start" {
                let (tx, rx) = tokio::sync::broadcast::channel(1);
                watchers.insert(container_name.clone(), tx);
                let docker = new_docker().unwrap();
                // Start watcher for new container
                start_watcher(docker, container_name.clone(), rx, logger.clone_tx()).await;
            }

            // Handle docker event when container is stopped
            if action == "kill" || action == "die" || action == "stop" {
                if let Some(watcher) = watchers.remove(&container_name) {
                    let _ = watcher.send(()); // Send stop signal
                }
            }
        }
    }
}

trait LoggerExt {
    fn log(&self, event: Event);
    fn clone_tx(&self) -> tokio::sync::broadcast::Sender<Event>;
}

struct Logger {
    tx: tokio::sync::broadcast::Sender<Event>,
}

impl Logger {
    async fn new() -> Self {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<Event>(10000);

        // Create unicorn client
        let mut unicorn_client = UnicornServiceClient::connect("http://[::1]:6000")
            .await
            .expect("Could no connect to unicorn service");

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                unicorn_client
                    .send_log(unicorn::LogMessage {
                        log: event.to_bytes(),
                    })
                    .await
                    .expect("Unable to send log message to unicorn service");
            }
        });

        Self { tx }
    }
}

impl LoggerExt for Logger {
    fn log(&self, event: Event) {
        let _ = self.tx.send(event);
    }
    fn clone_tx(&self) -> tokio::sync::broadcast::Sender<Event> {
        self.tx.clone()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Init logger
    env_logger::init();

    info!("Client started");

    start(Logger::new().await).await;

    Ok(())
}
