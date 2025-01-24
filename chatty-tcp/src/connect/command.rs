use anyhow::Context;
use anyhow::Result;
use chatty_types::command::{ChatCommand, ChatMessage};
use std::io::stdout;
use std::io::Write;
use std::process;
use tokio::io::{stdin, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::select;
use tokio::signal;
use tracing::debug;

pub async fn send_command(writer_half: OwnedWriteHalf, username: String) -> Result<()> {
    let mut writer = writer_half;
    let command = ChatCommand::Join(username.clone());
    send_request(&mut writer, command).await?;

    debug!("Running client prompt");
    let mut reader = BufReader::new(stdin()).lines();

    print!("> ");
    stdout().flush()?;

    loop {
        select! {
            // Handle input from the user
            line = reader.next_line() => {
                if let Ok(Some(line)) = line {
                    debug!("Read line: {:?}", line);
                    match line.split_whitespace().next() {
                        Some("send") => {
                            let content = line.trim_start_matches("send").trim().to_string();
                            let chat_message = ChatMessage {
                                username: username.clone(),
                                content,
                            };
                            let command = ChatCommand::Send(chat_message);
                            debug!("Sending command for message: {:?}", command);
                            send_request(&mut writer, command).await?;
                        }
                        Some("leave") => {
                            let command = ChatCommand::Leave(username.clone());
                            debug!("Sending command for leave: {:?}", command);
                            send_request(&mut writer, command).await?;
                            process::exit(0);
                        }
                        _ => println!("Unknown command. Use 'send <message>' or 'leave'"),
                    }
                    print!("> ");
                    stdout().flush()?;
                }
            }
            // Handle Ctrl+C as Leave
            _ = signal::ctrl_c() => {
                let command = ChatCommand::Leave(username.clone());
                debug!("Ctrl+C detected. Sending command for leave: {:?}", command);
                send_request(&mut writer, command).await?;
                process::exit(0);
            }
        }
    }
}

pub async fn send_request(writer: &mut OwnedWriteHalf, command: ChatCommand) -> Result<()> {
    let serialized = serde_json::to_string(&command)?;
    writer
        .write_all(serialized.as_bytes())
        .await
        .context(format!("Failed to send command {:?} from user", command))?;
    writer
        .write_all(b"\n")
        .await
        .context("Failed to send newline for framing")?; // Add newline for framing
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn test_send_request() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            line
        });

        let stream = TcpStream::connect(addr).await.unwrap();
        let (_, mut writer) = stream.into_split();

        let test_message = ChatMessage {
            username: "test_user".to_string(),
            content: "Hello world".to_string(),
        };
        let command = ChatCommand::Send(test_message);

        send_request(&mut writer, command).await.unwrap();

        let received = server_handle.await.unwrap();
        assert!(received.contains("test_user"));
        assert!(received.contains("Hello world"));
        assert!(received.ends_with("\n"));
    }
}
