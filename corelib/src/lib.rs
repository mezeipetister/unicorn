use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Log {
        container_name: String,
        message: String,
        timestamp: DateTime<Utc>,
    },
    DockerAction {
        container_name: String,
        action: String,
        timestamp: DateTime<Utc>,
    },
}

impl Event {
    pub fn to_bytes(&self) -> Vec<u8> {
        // Serialize using bincode
        bincode::serialize(self).unwrap()
    }
    pub fn from_bytes(bytes: &[u8]) -> Self {
        // Deserialize using bincode
        bincode::deserialize(bytes).unwrap()
    }
}
