use anyhow::Result;
use chatty_tcp::config::server_address;
use chatty_tcp::connect::prompt::run;
use chatty_tcp::handler::ChatHandler;
use chatty_types::config::{setup_tracing, Component::Client};
use clap::Parser;
use std::io::stdout;
use std::io::Write;
use tokio::io::AsyncBufReadExt;
use tokio::io::{stdin, BufReader};
use tokio::net::TcpStream;
use tracing::{debug, debug_span, info, Instrument};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    username: Option<String>,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    setup_tracing(Client, "info")?;

    let args = Args::parse();

    let username = if let Some(name) = args.username {
        name
    } else {
        ask_username().await?
    };

    let span = debug_span!("chatty_tcp_client_main");
    span.in_scope(|| debug!("Client connection is being set up for user: {}", username));

    let addr = server_address();

    let stream = TcpStream::connect(&addr).await?;
    span.in_scope(|| info!("Connected to server at {}", addr));

    let handler = ChatHandler::new(stream);
    run(handler, username).instrument(span.clone()).await?;

    Ok(())
}

async fn ask_username() -> Result<String> {
    print!("Username: ");
    stdout().flush()?;
    let mut reader = BufReader::new(stdin()).lines();
    if let Ok(Some(username)) = reader.next_line().await {
        Ok(username)
    } else {
        Err(anyhow::anyhow!("Failed to read username"))
    }
}
