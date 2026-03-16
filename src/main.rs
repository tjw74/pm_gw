mod auth;
mod broadcast;
mod commands;
mod config;
mod dashboard;
mod error;
mod execution;
mod feeds;
mod health;
mod market_scheduler;
mod models;
mod observability;
mod responses;
mod session;
mod state;
mod trade_api;
mod types;
mod util;
mod ws_server;

use std::sync::Arc;

use anyhow::Result;
use axum::{Router, routing::{get, post}};
use tokio::net::TcpListener;
use tracing::info;

use crate::{
    auth::AuthService,
    broadcast::EventBroadcaster,
    config::Config,
    dashboard::{
        admin_audit, admin_login, admin_me, admin_sessions, admin_status, dashboard_admin_ws,
        dashboard_public_ws, public_alerts, public_feeds, public_market, public_status,
        public_streaming, reconnect_feed_stub, reload_config_stub, set_kill_switch,
    },
    execution::polymarket::PolymarketExecutionClient,
    health::{healthz, readyz},
    market_scheduler::spawn_market_scheduler,
    state::AppState,
    trade_api::{trade_refresh, trade_session},
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
        .route("/ws/trade", get(ws_route))
        .route("/ws/dashboard/public", get(dashboard_public_ws))
        .route("/ws/dashboard/admin", get(dashboard_admin_ws))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/public/status", get(public_status))
        .route("/api/public/feeds", get(public_feeds))
        .route("/api/public/market", get(public_market))
        .route("/api/public/streaming", get(public_streaming))
        .route("/api/public/alerts", get(public_alerts))
        .route("/api/trade/session", get(trade_session))
        .route("/api/trade/refresh", post(trade_refresh))
        .route("/api/admin/login", post(admin_login))
        .route("/api/admin/me", get(admin_me))
        .route("/api/admin/status", get(admin_status))
        .route("/api/admin/sessions", get(admin_sessions))
        .route("/api/admin/audit", get(admin_audit))
        .route("/api/admin/controls/kill-switch", post(set_kill_switch))
        .route("/api/admin/controls/reload-config", post(reload_config_stub))
        .route("/api/admin/controls/reconnect-feed", post(reconnect_feed_stub))
        .with_state(state.clone());

    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = TcpListener::bind(addr).await?;
    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
