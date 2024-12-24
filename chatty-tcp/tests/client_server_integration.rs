use chatty_tcp::handler::ChatHandler;
use chatty_tcp::listen::room::serve;
use chatty_tcp::listen::state::RoomState;
use chatty_types::config::setup_tracing;
use chatty_types::config::Component::Server;
use chatty_types::response::ChatResponse;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, Mutex};
use tokio_test::{assert_err, assert_ok};

use std::sync::Once;

static INIT: Once = Once::new();

fn init_tracing_for_tests() {
    INIT.call_once(|| {
        setup_tracing(Server, "debug").unwrap();
    });
}

#[tokio::test]
async fn single_client() {
    init_tracing_for_tests();
    // Set up room state
    let (tx, _rx) = broadcast::channel::<ChatResponse>(100);
    let task_handles = Mutex::new(HashMap::new());

    let room_state = Arc::new(RoomState { tx, task_handles });

    // Start the server in a background task
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    // Clone the Arc before moving it into the async block
    let room_state_for_server = room_state.clone();
    let server_handle = tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let state = room_state_for_server.clone();
            tokio::spawn(async move {
                let handler = ChatHandler::new(stream);
                serve(handler, state).await.unwrap();
            });
        }
    });

    // Connect to the server
    let client = assert_ok!(TcpStream::connect(addr).await);
    let (reader_half, mut writer_half) = client.into_split();
    let mut reader = BufReader::new(reader_half).lines();

    // Joins the room
    let join_command = r#"{"Join":"alone"}"#;
    writer_half
        .write_all(join_command.as_bytes())
        .await
        .unwrap();
    writer_half.write_all(b"\n").await.unwrap();

    // Read the response
    let response = reader.next_line().await.unwrap().unwrap();
    assert!(response.contains("Warm Welcome"));

    // Send a message
    let send_command = r#"{"Send":{"username":"alone","content":"Hello, world!"}}"#;
    writer_half
        .write_all(send_command.as_bytes())
        .await
        .unwrap();
    writer_half.write_all(b"\n").await.unwrap();

    let read_future = reader.next_line();
    let result = tokio::time::timeout(std::time::Duration::from_millis(100), read_future).await;

    assert_err!(result);

    // // Verify user is there
    let room_state_for_removal = room_state.clone();
    let lookup = room_state_for_removal.task_handles.lock().await;
    assert!(lookup.contains_key("alone"));

    // leave command
    let leave_command = r#"{"Leave":"alone"}"#;
    assert_ok!(writer_half.write_all(leave_command.as_bytes()).await);
    assert_ok!(writer_half.write_all(b"\n").await);

    server_handle.abort();
}

#[tokio::test]
async fn multiple_clients() {
    init_tracing_for_tests();
    let (tx, _rx) = broadcast::channel::<ChatResponse>(100);
    // Create a separate subscriber for test verification
    let test_tx = tx.clone();
    let mut test_rx = test_tx.subscribe();
    let task_handles = Mutex::new(HashMap::new());

    // Set up room state
    let room_state = Arc::new(RoomState { tx, task_handles });
    let state = room_state.clone();

    // Start the server in a background task
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_handle = tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let state = room_state.clone();
            tokio::spawn(async move {
                let handler = ChatHandler::new(stream);
                serve(handler, state).await.unwrap();
            });
        }
    });

    // Connect the first client
    let client_stream1 = assert_ok!(TcpStream::connect(addr).await);
    let (reader_half1, mut writer_half1) = client_stream1.into_split();
    let mut reader1 = BufReader::new(reader_half1).lines();

    // Connect the second client
    let client_stream2 = assert_ok!(TcpStream::connect(addr).await);
    let (reader_half2, mut writer_half2) = client_stream2.into_split();
    let mut reader2 = BufReader::new(reader_half2).lines();

    // First client joins the room
    let join_command1 = r#"{"Join":"carl"}"#;
    writer_half1
        .write_all(join_command1.as_bytes())
        .await
        .unwrap();
    writer_half1.write_all(b"\n").await.unwrap();

    // Read the response for the first client
    let response1 = reader1.next_line().await.unwrap().unwrap();
    assert!(response1.contains("Warm Welcome"));

    // Second client joins the room
    let join_command2 = r#"{"Join":"david"}"#;
    writer_half2
        .write_all(join_command2.as_bytes())
        .await
        .unwrap();
    writer_half2.write_all(b"\n").await.unwrap();

    // Read the response for the second client
    let response2 = reader2.next_line().await.unwrap().unwrap();
    assert!(response2.contains("Warm Welcome"));

    // The First client reads the broadcast message
    let broadcast_message = reader1.next_line().await.unwrap().unwrap();
    let expected_message = r#"{"Broadcast":{"username":"david","content":"Joined"}}"#;
    assert_eq!(broadcast_message, expected_message);

    // First client sends a message
    let send_command = r#"{"Send":{"username":"carl","content":"Hello, world!"}}"#;
    writer_half1
        .write_all(send_command.as_bytes())
        .await
        .unwrap();
    writer_half1.write_all(b"\n").await.unwrap();

    // Wait for broadcast confirmation using test subscriber
    assert_ok!(test_rx.recv().await);

    // The Second client reads the broadcast message
    let broadcast_message = reader2.next_line().await.unwrap().unwrap();
    let expected_message1 = r#"{"Broadcast":{"username":"carl","content":"Hello, world!"}}"#;
    assert_eq!(broadcast_message, expected_message1);

    // leave command from the first client
    let leave_command = r#"{"Leave":"carl"}"#;
    writer_half1
        .write_all(leave_command.as_bytes())
        .await
        .unwrap();
    writer_half1.write_all(b"\n").await.unwrap();

    // The Second client reads the next broadcast message
    let broadcast_message = reader2.next_line().await.unwrap().unwrap();
    let expected_message2 = r#"{"Broadcast":{"username":"carl","content":"Left"}}"#;
    assert_eq!(broadcast_message, expected_message2);

    let lookup = state.task_handles.lock().await;
    assert_eq!(lookup.len(), 1);
    assert!(lookup.contains_key("david"));

    // Clean up
    server_handle.abort();
}
