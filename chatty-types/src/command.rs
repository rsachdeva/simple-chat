use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ChatCommand {
    Join(String),
    Send(ChatMessage),
    Leave(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub username: String,
    pub content: String,
}
