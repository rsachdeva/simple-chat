use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};

pub enum Component {
    Server,
    Client,
}

pub fn setup_tracing(component: Component, log_level: &str) -> Result<()> {
    let prefix = match component {
        Component::Client => "client",
        Component::Server => "server",
    };
    let directive = format!("{}={}", prefix, log_level);
    registry()
        .with(EnvFilter::try_from_default_env()?.add_directive(directive.parse()?))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // let rust_log = std::env::var("RUST_LOG")?;
    // println!("RUST_LOG is {}", rust_log);
    Ok(())
}
