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
                let mut users = room_state.user_set.lock().await;
                let user_already_exist = users.contains(&username);

                let chat_response = if !user_already_exist {
                    users.insert(username.clone());
                    info!("Users in room after addition: {:?}", users);
                    info!("Client {} joined as {}", addr, username);
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
                if let Some(handle) = room_state.task_handles.lock().await.remove(&username) {
                    info!("Aborting background task for user: {}", username);
                    handle.abort();
                }
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
    let mut users = room_state.user_set.lock().await;
    users.remove(&username);
    info!("User {} removed from room", username);
    // list connected users
    let users: Vec<String> = users.iter().cloned().collect();
    info!("Users in room after removal: {:?}", users);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_remove_username() {
        let mut user_set = HashSet::new();
        user_set.insert("test_user".to_string());
        user_set.insert("other_user".to_string());

        let (tx, _) = broadcast::channel(100);
        let room_state = Arc::new(RoomState {
            user_set: Mutex::new(user_set),
            tx,
            task_handles: Mutex::new(HashMap::new()),
        });

        // Execute removal
        remove_username("test_user".to_string(), room_state.clone()).await;

        // Verify user was removed
        let users = room_state.user_set.lock().await;
        assert!(!users.contains("test_user"));
        assert!(users.contains("other_user"));
        assert_eq!(users.len(), 1);
    }
}
