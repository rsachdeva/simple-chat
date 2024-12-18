use crate::listen::command::RoomError;
use chatty_types::response::ChatResponse;
use std::collections::{HashMap, HashSet};
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

type TaskHandleMap = Mutex<HashMap<String, JoinHandle<Result<(), RoomError>>>>;

pub struct RoomState {
    pub user_set: Mutex<HashSet<String>>,
    pub tx: broadcast::Sender<ChatResponse>,
    pub task_handles: TaskHandleMap,
}
