use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ChatResponse {
    Broadcast(ChatMemo),
    Joined(ChatMemo),
    Duplicate(ChatMemo),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMemo {
    pub username: String,
    pub content: String,
}
