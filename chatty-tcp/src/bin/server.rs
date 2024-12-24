use anyhow::Result;
use chatty_tcp::config::server_address;
use chatty_tcp::handler::ChatHandler;
use chatty_tcp::listen::command::RoomError;
use chatty_tcp::listen::room::serve;
use chatty_tcp::listen::state::RoomState;
use chatty_types::config::{setup_tracing, Component::Server};
use chatty_types::response::ChatResponse;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::{select, signal};
use tracing::{debug, debug_span, info, Instrument};

#[tokio::main]
pub async fn main() -> Result<()> {
    setup_tracing(Server, "info")?;

    let span = debug_span!("chatty_tcp_server_main");
    span.in_scope(|| debug!("Server is being set up"));

    let addr = server_address();
    let listener = TcpListener::bind(addr).await?;
    let listening_on = listener.local_addr()?;
    span.in_scope(|| info!("listening on {}", listening_on));

    // Set up room state for use
    // bounded channel
    let (tx, _rx) = broadcast::channel::<ChatResponse>(100);
    // task handles
    let task_handles = Mutex::new(HashMap::new());

    let room_state = Arc::new(RoomState { tx, task_handles });

    let mut connection_handles = Vec::new();

    loop {
        select! {
            accept_result = listener.accept() => {
                let (stream, addr) = accept_result?;
                span.in_scope(|| info!("accepted connection from {}", addr));
                let state = room_state.clone();

                let handle = tokio::spawn(
                    async move {
                        let handler = ChatHandler::new(stream);
                        serve(handler, state).await?;
                        Ok::<_, RoomError>(())
                    }
                    .instrument(span.clone()),
                );
                connection_handles.push(handle);
            }
            _ = signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down server...");
                info!("Aborting connection handles");
                for  handle in connection_handles.iter() {
                    handle.abort();
                }
                let mut handles = room_state.task_handles.lock().await;
                for (username, handle) in handles.iter() {
                    info!("Aborting background send task for user: {}", username);
                    handle.abort();
                }
                handles.clear();
                info!("All background send tasks aborted");
                return Ok(());
            }
        }
    }
}
