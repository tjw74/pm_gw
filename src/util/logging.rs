use anyhow::Result;
use tracing_subscriber::{EnvFilter, fmt};

pub fn init_tracing(log_level: &str) -> Result<()> {
    let filter = EnvFilter::try_new(log_level).or_else(|_| EnvFilter::try_new("info"))?;
    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .json()
        .init();
    Ok(())
}
