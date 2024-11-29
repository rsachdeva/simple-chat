pub fn server_address() -> String {
    format!(
        "{}:{}",
        std::env::var("TCP_SERVER_ADDRESS").unwrap_or_else(|_| "localhost".to_string()),
        std::env::var("TCP_SERVER_PORT").unwrap_or_else(|_| "8081".to_string())
    )
}
