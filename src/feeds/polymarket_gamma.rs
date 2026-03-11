use std::sync::Arc;

use tracing::warn;

use crate::state::AppState;

pub async fn spawn(state: Arc<AppState>) {
    tokio::spawn(async move {
        if let Err(err) = probe_gamma(state.clone()).await {
            warn!(?err, "gamma probe failed");
        }
    });
}

async fn probe_gamma(state: Arc<AppState>) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/markets?limit=1", state.config.poly_gamma_base_url);
    let _ = client.get(url).send().await?;
    state
        .record_connection(
            "polymarket_gamma",
            crate::types::ConnectionState::Connected,
            None,
        )
        .await;
    Ok(())
}
