use crate::listen::response::{
    send_from_broadcast_channel_task, send_response, send_to_broadcast_channel,
};
use crate::listen::state::RoomState;
use anyhow::Result;
use chatty_types::command::ChatCommand;
use chatty_types::response::{ChatMemo, ChatResponse};
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;
use tracing::{debug, info};

#[derive(Debug, Error)]
pub enum RoomError {
    #[error("IO error is: {0}")]
    Io(#[from] std::io::Error),

    #[error("Json parse error is: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Broadcast receive error is: {0}")]
    BroadcastReceive(String),

    #[error("Broadcast send error: {0}")]
    BroadcastSend(#[from] tokio::sync::broadcast::error::SendError<ChatResponse>),
}
pub async fn process_command(
    writer_half: OwnedWriteHalf,
    reader_half: OwnedReadHalf,
    room_state: Arc<RoomState>,
) -> Result<(), RoomError> {
    let addr = reader_half.peer_addr()?;
    debug!("handling client connection from {}", addr);
    let writer = Arc::new(Mutex::new(writer_half));
    let mut reader = BufReader::new(reader_half).lines();
    while let Some(line) = reader.next_line().await? {
        debug!("Received line for command: {:?}", line);
        let command: ChatCommand = serde_json::from_str(&line)?;
        match command {
            ChatCommand::Join(username) => {
                let user_already_exist =
                    room_state.task_handles.lock().await.contains_key(&username);

                let chat_response = if !user_already_exist {
                    let rx = room_state.tx.subscribe();
                    let send_task_handle = tokio::spawn(send_from_broadcast_channel_task(
                        writer.clone(),
                        rx,
                        username.clone(),
                    ));
                    room_state
                        .task_handles
                        .lock()
                        .await
                        .insert(username.clone(), send_task_handle);
                    info!(
                        "Users in room after addition: {:?}",
                        room_state.task_handles.lock().await.keys()
                    );
                    info!("Client {} joined as {}", addr, username);
                    send_to_broadcast_channel(
                        ChatResponse::Broadcast(ChatMemo {
                            username: username.clone(),
                            content: "Joined".to_string(),
                        }),
                        room_state.clone(),
                    )
                    .await?;
                    ChatResponse::Joined(ChatMemo {
                        username,
                        content: "Warm Welcome".to_string(),
                    })
                } else {
                    ChatResponse::Duplicate(ChatMemo {
                        username,
                        content: "Sorry".to_string(),
                    })
                };
                send_response(chat_response, writer.clone()).await?;
            }
            ChatCommand::Send(message) => {
                debug!(
                    "Received message from {}: {:?}",
                    message.username, message.content
                );
                let chat_response = ChatResponse::Broadcast(ChatMemo {
                    username: message.username.clone(),
                    content: message.content.clone(),
                });
                debug!(
                    "Going to Broadcast for others the Received message {:?}",
                    chat_response
                );
                send_to_broadcast_channel(chat_response, room_state.clone()).await?;
            }
            ChatCommand::Leave(username) => {
                remove_username(username.clone(), room_state.clone()).await;
                debug!("User {} has left", username);

                debug!("User {} has left so sending broadcast message", username);
                send_to_broadcast_channel(
                    ChatResponse::Broadcast(ChatMemo {
                        username: username.clone(),
                        content: "Left".to_string(),
                    }),
                    room_state.clone(),
                )
                .await?;
                debug!("completed User {} leave handling", username);
            }
        }
    }
    Ok(())
}

pub async fn remove_username(username: String, room_state: Arc<RoomState>) {
    let mut lookup = room_state.task_handles.lock().await;
    if let Some(handle) = lookup.remove(&username) {
        info!("Aborting background task for user: {}", username);
        handle.abort();
    }
    info!("User {} removed from room", username);
    // list connected users
    let users: Vec<String> = lookup.keys().cloned().collect();
    info!("Users in room after removal: {:?}", users);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::sync::broadcast;
    use tokio::task::JoinHandle;

    #[tokio::test]
    async fn test_remove_username() {
        let mut lookup_initial = HashMap::new();
        let dummy_task: JoinHandle<Result<(), RoomError>> = tokio::spawn(async { Ok(()) });
        lookup_initial.insert("test_user".to_string(), dummy_task);
        let dummy_task2: JoinHandle<Result<(), RoomError>> = tokio::spawn(async { Ok(()) });
        lookup_initial.insert("other_user".to_string(), dummy_task2);

        let (tx, _) = broadcast::channel(100);
        let room_state = Arc::new(RoomState {
            tx,
            task_handles: Mutex::new(lookup_initial),
        });

        // Execute removal
        remove_username("test_user".to_string(), room_state.clone()).await;

        // Verify user was removed
        let lookup = room_state.task_handles.lock().await;
        assert!(!lookup.contains_key("test_user"));
        assert!(lookup.contains_key("other_user"));
        assert_eq!(lookup.len(), 1);
    }
}
