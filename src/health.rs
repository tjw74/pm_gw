use axum::{Json, extract::State};
use serde_json::json;
use std::sync::Arc;

use crate::state::AppState;

pub async fn healthz() -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "timestamp": time::OffsetDateTime::now_utc(),
    }))
}

pub async fn readyz(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!({
        "ready": state.is_ready(),
        "active_market": state.active_market_slug().await,
        "timestamp": time::OffsetDateTime::now_utc(),
    }))
}
