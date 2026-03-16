use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::HeaderMap,
};
use serde_json::json;

use crate::{error::GatewayError, execution::polymarket::PolymarketExecutionClient, state::AppState};

pub async fn trade_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let user = extract_user(&state, &headers)?;
    let execution = PolymarketExecutionClient::new(state.config.clone(), state.clone())
        .map_err(|err| GatewayError::internal(err.to_string()))?;
    let background_state = state.clone();
    let background_user_id = user.user_id.clone();
    tokio::spawn(async move {
        let _ = execution.sync_user_state_for(&background_user_id).await;
        background_state.refresh_snapshot().await;
    });
    Ok(Json(state.user_trade_snapshot(&user.user_id).await))
}

pub async fn trade_refresh(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let user = extract_user(&state, &headers)?;
    let execution = PolymarketExecutionClient::new(state.config.clone(), state.clone())
        .map_err(|err| GatewayError::internal(err.to_string()))?;
    execution.sync_user_state_for(&user.user_id).await?;
    Ok(Json(json!({
        "ok": true,
        "snapshot": state.user_trade_snapshot(&user.user_id).await,
    })))
}

fn extract_user(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::auth::AuthenticatedUser, GatewayError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(GatewayError::Unauthorized)?;
    state.auth.verify_token(token)
}
