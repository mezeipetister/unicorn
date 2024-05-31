#[macro_use]
extern crate rocket;

use std::{
    collections::VecDeque,
    error::Error,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};

use corelib::Event;
use futures_util::TryFutureExt;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tonic::{transport::Server, Response};
pub use unicorn::unicorn_service_client::UnicornServiceClient;
use unicorn::{
    unicorn_service_server::{UnicornService, UnicornServiceServer},
    LogMessage,
};

mod web;

pub mod unicorn {
    tonic::include_proto!("unicorn");
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogItem {
    ip: SocketAddr,
    event: Event,
}

#[derive(Clone, Debug)]
struct Unicorn {
    inner: Arc<Mutex<Inner>>,
    tx: tokio::sync::broadcast::Sender<LogItem>,
}

#[derive(Debug)]
struct Inner {
    data: VecDeque<LogItem>,
    capacity: usize,
}

impl Unicorn {
    fn new(capacity: usize) -> Self {
        let (tx, _rx) = tokio::sync::broadcast::channel(1000);
        Self {
            inner: Arc::new(Mutex::new(Inner {
                data: VecDeque::new(),
                capacity,
            })),
            tx,
        }
    }
    async fn push(&self, item: LogItem) {
        let mut inner = self.inner.lock().expect("Error locking inner");
        if (inner.data).len() == inner.capacity {
            inner.data.pop_front();
        }
        // Push logitem
        inner.data.push_back(item.clone());

        println!("{:?}", &item);
    }
}

#[tonic::async_trait]
impl UnicornService for Unicorn {
    async fn send_log(
        &self,
        request: tonic::Request<LogMessage>,
    ) -> Result<Response<()>, tonic::Status> {
        let ip = request.remote_addr().unwrap();

        // Deserialize event from bytes.
        let event = Event::from_bytes(&request.into_inner().log);

        let log_item = LogItem {
            ip,
            event: event.clone(),
        };

        self.push(log_item.clone()).await;

        // Notify listeners about the new item
        if self.tx.receiver_count() > 0 {
            let _ = self.tx.send(log_item);
        }

        let res = Response::new(());
        Ok(res)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Init logger
    env_logger::init();

    // Address is valid?
    // Todo: Implement auto address from environment variable
    let addr = std::env::var("UNICORN_ADDR")
        .unwrap_or("[::1]:6000".into())
        .parse()
        .unwrap();

    // Create shutdown channel
    let (tx, rx) = oneshot::channel();

    let unicorn = Unicorn::new(1000);
    let uni2 = unicorn.clone();

    // Spawn the server into a runtime
    tokio::task::spawn(async move {
        Server::builder()
            .add_service(UnicornServiceServer::new(unicorn))
            .serve_with_shutdown(addr, async {
                let _ = rx.await;
            })
            .await
            .unwrap()
    });

    tokio::task::spawn(async move {
        let _ = web::run(uni2).await;
    });

    tokio::signal::ctrl_c().await?;

    println!("SIGINT");

    // Send shutdown signal after SIGINT received
    let _ = tx.send(());

    Ok(())
}
