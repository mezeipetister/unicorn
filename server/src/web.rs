use crate::Unicorn;
use rocket::fs::{relative, FileServer};
use rocket::response::stream::{Event, EventStream};
use rocket::State;
use serde::{Deserialize, Serialize};
use tokio::select;
use tokio::sync::broadcast::error::RecvError;

#[derive(Debug, Serialize, Deserialize)]
struct Item {
    ip: String,
    container_name: String,
    timestamp: String,
    content: String,
}

impl Item {
    fn from_log_item(logitem: &crate::LogItem) -> Self {
        Self {
            ip: logitem.ip.to_string(),
            container_name: match &logitem.event {
                crate::Event::DockerAction { container_name, .. } => container_name.to_owned(),
                crate::Event::Log { container_name, .. } => container_name.to_owned(),
            },
            timestamp: match logitem.event {
                crate::Event::DockerAction { timestamp, .. } => timestamp.to_rfc3339(),
                crate::Event::Log { timestamp, .. } => timestamp.to_rfc3339(),
            },
            content: match &logitem.event {
                crate::Event::DockerAction { .. } => "Started".to_string(),
                crate::Event::Log { message, .. } => message.clone(),
            },
        }
    }
}

#[get("/stream")]
async fn stream(unicorn: &State<Unicorn>) -> EventStream![] {
    let mut rx = unicorn.tx.subscribe();
    let db = unicorn.inner.lock().unwrap().data.clone();
    EventStream! {
        // First send all the data in the db
        for item in db.iter(){
            yield Event::json(&Item::from_log_item(item));
        }
        // Then send new data as it arrives
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
            };

            yield Event::json(&Item::from_log_item(&msg));
        }
    }
}

pub async fn run(unicorn: Unicorn) -> Result<(), rocket::Error> {
    let r = rocket::build()
        .mount("/", routes![stream])
        .manage(unicorn)
        .mount("/", FileServer::from("./static"));

    r.launch().await?;

    Ok(())
}
