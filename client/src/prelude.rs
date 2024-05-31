use std::env;

use tonic::transport::Channel;

use crate::UnicornServiceClient; // Import the UnicornServiceClient type

pub async fn create_unicorn_client(
) -> Result<UnicornServiceClient<Channel>, Box<dyn std::error::Error>> {
    // Get the server address from the environment variable
    let server_addr = env::var("UnicornServerAddr")?;

    // Create a gRPC channel to connect to the server
    let channel = Channel::from_shared(server_addr)
        .map_err(|e| format!("Failed to create channel: {}", e))?
        .connect()
        .await?;

    // Create the UnicornService client
    let client = UnicornServiceClient::new(channel);

    Ok(client)
}
