mod auth;
mod broadcast;
mod commands;
mod config;
mod error;
mod execution;
mod feeds;
mod health;
mod market_scheduler;
mod models;
mod responses;
mod session;
mod state;
mod types;
mod util;
mod ws_server;

use std::sync::Arc;

use anyhow::Result;
use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::info;

use crate::{
    auth::AuthService,
    broadcast::EventBroadcaster,
    config::Config,
    execution::polymarket::PolymarketExecutionClient,
    health::{healthz, readyz},
    market_scheduler::spawn_market_scheduler,
    state::AppState,
    ws_server::ws_route,
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;
    util::logging::init_tracing(&config.log_level)?;
    config.log_summary();

    let auth = Arc::new(AuthService::new(&config).await?);
    let broadcaster = EventBroadcaster::new();
    let state = Arc::new(AppState::new(
        config.clone(),
        broadcaster.clone(),
        auth.clone(),
    ));
    let execution = Arc::new(PolymarketExecutionClient::new(
        config.clone(),
        state.clone(),
    )?);

    state.mark_core_started();
    feeds::spawn_all(state.clone(), execution.clone()).await;
    spawn_market_scheduler(state.clone()).await;

    let app = Router::new()
        .route("/ws", get(ws_route))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .with_state(state.clone());

    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = TcpListener::bind(addr).await?;
    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
