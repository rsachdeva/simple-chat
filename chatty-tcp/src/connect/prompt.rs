use crate::connect::command::send_command;
use crate::connect::response::process_response;
use crate::handler::ChatHandler;
use anyhow::Result;

pub async fn run(handler: ChatHandler, username: String) -> Result<()> {
    let ChatHandler {
        writer_half,
        reader_half,
    } = handler;

    let response_task = tokio::spawn(process_response(reader_half));
    let command_task = tokio::spawn(send_command(writer_half, username));

    let (command_result, response_result) = tokio::try_join!(command_task, response_task)?;

    command_result?;
    response_result?;

    Ok(())
}
