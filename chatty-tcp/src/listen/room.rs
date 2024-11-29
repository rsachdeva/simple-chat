use crate::handler::ChatHandler;
use crate::listen::command::{process_command, RoomError};
use crate::listen::state::RoomState;
use anyhow::Result;
use std::sync::Arc;

pub async fn serve(handler: ChatHandler, room_state: Arc<RoomState>) -> Result<(), RoomError> {
    let ChatHandler {
        writer_half,
        reader_half,
    } = handler;

    process_command(writer_half, reader_half, room_state).await?;

    Ok(())
}
