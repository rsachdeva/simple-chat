use crate::listen::command::RoomError;
use crate::listen::state::RoomState;
use anyhow::{Context, Result};
use chatty_types::response::ChatResponse;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::{broadcast, Mutex};
use tracing::debug;

pub async fn send_to_broadcast_channel(
    chat_response: ChatResponse,
    room_state: Arc<RoomState>,
) -> Result<()> {
    // send the chat_response to the broadcast channel
    let _ = room_state.tx.send(chat_response)?;

    Ok(())
}

pub async fn send_from_broadcast_channel_task(
    writer: Arc<Mutex<OwnedWriteHalf>>,
    mut rx: broadcast::Receiver<ChatResponse>,
    username: String,
) -> Result<(), RoomError> {
    while let Ok(recv_chat_response) = rx.recv().await {
        debug!(
            "send_task received from broadcast::Receiver: recv_chat_response  is {:?}",
            recv_chat_response
        );
        let ChatResponse::Broadcast(recv_memo) = recv_chat_response.clone() else {
            return Err(RoomError::Broadcast(
                "Failed to get memo from received chat response".to_string(),
            ));
        };
        let recv_username = recv_memo.username.clone();
        debug!("recv_username in send_task is {:?}", recv_username);
        debug!("username in send_task is {:?}", username);

        if !recv_username.eq(&username) {
            debug!(
                "Sending to -> {} chat response for received username -> {}",
                username, recv_username
            );
            if let Err(e) = send_response(recv_chat_response, writer.clone()).await {
                debug!("Failed to send response: {:?}", e);
                break;
            }
        }
    }
    Ok(())
}

pub async fn send_response(
    chat_response: ChatResponse,
    writer: Arc<Mutex<OwnedWriteHalf>>,
) -> Result<()> {
    let serialized = serde_json::to_string(&chat_response)?;
    let mut writer = writer.lock().await;
    writer
        .write_all(serialized.as_bytes())
        .await
        .context(format!("Failed to send response {:?}", chat_response))?;
    writer
        .write_all(b"\n")
        .await
        .context("Failed to send newline for framing")?; // Add newline for framing
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chatty_types::response::{ChatMemo, ChatResponse};
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;
    use tokio::net::TcpStream;
    use tokio_test::{assert_err, assert_ok};

    #[tokio::test]
    async fn test_send_task_different_username() {
        let (tx, rx) = broadcast::channel(100);

        let listener = assert_ok!(TcpListener::bind("127.0.0.1:0").await);
        let addr = assert_ok!(listener.local_addr());

        let client = assert_ok!(TcpStream::connect(addr).await);
        let (_, writer_half) = client.into_split();

        let writer = Arc::new(Mutex::new(writer_half));
        let _handle = tokio::spawn(async move {
            assert_ok!(send_from_broadcast_channel_task(writer, rx, "alice".to_string()).await);
        });

        let (mut stream, _) = assert_ok!(listener.accept().await);

        assert_ok!(tx.send(ChatResponse::Broadcast(ChatMemo {
            username: "carl".to_string(),
            content: "hello, I love tokio".to_string(),
        })));

        let mut buf = vec![0; 1024];
        let n = assert_ok!(stream.read(&mut buf).await);
        let received = String::from_utf8_lossy(&buf[..n]);

        assert!(received.contains("carl"));
        assert!(received.contains("hello, I love tokio"));
    }

    #[tokio::test]
    async fn test_send_task_same_username() {
        let (tx, rx) = broadcast::channel(100);

        // Create a real TCP connection for testing
        let listener = assert_ok!(TcpListener::bind("127.0.0.1:0").await);
        let addr = assert_ok!(listener.local_addr());

        let client = assert_ok!(TcpStream::connect(addr).await);
        let (_, writer_half) = client.into_split();

        let writer = Arc::new(Mutex::new(writer_half));
        let _handle = tokio::spawn(async move {
            assert_ok!(send_from_broadcast_channel_task(writer, rx, "alice".to_string()).await);
        });

        let (mut stream, _) = assert_ok!(listener.accept().await);
        assert_ok!(tx.send(ChatResponse::Broadcast(ChatMemo {
            username: "alice".to_string(),
            content: "hello, I love tokio".to_string(),
        })));

        // Verify no data was sent
        let mut buf = vec![0; 1024];
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(100), stream.read(&mut buf))
                .await;

        assert_err!(result);
    }
}
