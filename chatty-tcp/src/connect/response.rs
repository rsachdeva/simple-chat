use anyhow::Result;
use chatty_types::response::ChatResponse;
use std::io::{stdout, Write};
use std::process;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::tcp::OwnedReadHalf;
use tracing::debug;

pub async fn process_response(reader_half: OwnedReadHalf) -> Result<()> {
    debug!("Running response handler");
    let mut reader = BufReader::new(reader_half).lines();

    while let Some(line) = reader.next_line().await? {
        let response = serde_json::from_str::<ChatResponse>(&line)?;
        match response {
            ChatResponse::Joined(message) => {
                debug!("{} Joined", message.username);
                println!("{}, {}", message.content, message.username);
                print!("> ");
                stdout().flush()?;
            }
            ChatResponse::Duplicate(message) => {
                println!(
                    "{}, Attempted Username {} already taken",
                    message.content, message.username
                );
                println!("Disconnecting from chat server");
                process::exit(0);
            }
            ChatResponse::Broadcast(message) => {
                debug!(
                    "Received message from {}: {:?}",
                    message.username, message.content
                );
                println!("({}): {}", message.username, message.content);
                print!("> ");
                stdout().flush()?;
            }
        }
    }

    println!("Connection closed by chat server");
    process::exit(0);
}
