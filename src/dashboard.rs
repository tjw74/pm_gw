use std::sync::Arc;

use axum::{
    Json,
    extract::{
        Query,
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::HeaderMap,
    response::Response,
};
use serde::Deserialize;
use serde_json::json;
use tracing::warn;

use crate::{error::GatewayError, state::AppState};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct KillSwitchRequest {
    pub enabled: bool,
}

pub async fn public_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(state.public_dashboard_snapshot().await)
}

pub async fn public_feeds(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let payload = state.public_dashboard_snapshot().await;
    Json(json!({ "feeds": payload["feeds"], "alerts": payload["alerts"] }))
}

pub async fn public_market(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let payload = state.public_dashboard_snapshot().await;
    Json(payload["market"].clone())
}

pub async fn public_streaming(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let payload = state.public_dashboard_snapshot().await;
    Json(payload["streaming"].clone())
}

pub async fn public_alerts(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let payload = state.public_dashboard_snapshot().await;
    Json(json!({ "alerts": payload["alerts"], "logs": payload["logs"] }))
}

pub async fn admin_login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let token = state.auth.admin_login(&body.username, &body.password)?;
    state
        .push_audit(&body.username, "admin_login", "dashboard login success")
        .await;
    Ok(Json(json!({
        "token": token,
        "user": { "username": body.username, "role": "admin" },
    })))
}

pub async fn admin_me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let admin = extract_admin(&state, &headers)?;
    Ok(Json(json!({
        "username": admin.username,
        "role": "admin",
    })))
}

pub async fn admin_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let _admin = extract_admin(&state, &headers)?;
    Ok(Json(state.admin_dashboard_snapshot().await))
}

pub async fn admin_sessions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let _admin = extract_admin(&state, &headers)?;
    let payload = state.admin_dashboard_snapshot().await;
    Ok(Json(payload["public"]["streaming"]["sessions"].clone()))
}

pub async fn admin_audit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let _admin = extract_admin(&state, &headers)?;
    let payload = state.admin_dashboard_snapshot().await;
    Ok(Json(payload["admin"]["audit"].clone()))
}

pub async fn set_kill_switch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<KillSwitchRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let admin = extract_admin(&state, &headers)?;
    state
        .set_runtime_kill_switch(body.enabled, &admin.username)
        .await;
    Ok(Json(json!({
        "ok": true,
        "enabled": body.enabled,
    })))
}

pub async fn reload_config_stub(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let admin = extract_admin(&state, &headers)?;
    state
        .push_audit(
            &admin.username,
            "reload_config_requested",
            "not implemented in v1",
        )
        .await;
    Err(GatewayError::BadRequest(
        "reload config is not implemented in v1".to_string(),
    ))
}

pub async fn reconnect_feed_stub(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let admin = extract_admin(&state, &headers)?;
    state
        .push_audit(
            &admin.username,
            "reconnect_feed_requested",
            "not implemented in v1",
        )
        .await;
    Err(GatewayError::BadRequest(
        "manual feed reconnect is not implemented in v1".to_string(),
    ))
}

pub async fn dashboard_public_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| stream_dashboard(socket, state, false))
}

pub async fn dashboard_admin_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Response, GatewayError> {
    let token = params.get("token").ok_or(GatewayError::Unauthorized)?;
    let _admin = state.auth.verify_admin_token(token)?;
    Ok(ws.on_upgrade(move |socket| stream_dashboard(socket, state, true)))
}

async fn stream_dashboard(mut socket: WebSocket, state: Arc<AppState>, admin: bool) {
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(1));
    loop {
        ticker.tick().await;
        let payload = if admin {
            state.admin_dashboard_snapshot().await
        } else {
            state.public_dashboard_snapshot().await
        };
        let message = json!({
            "type": "dashboard_snapshot",
            "scope": if admin { "admin" } else { "public" },
            "payload": payload,
        });
        if socket
            .send(Message::Text(message.to_string().into()))
            .await
            .is_err()
        {
            state.record_stream_disconnect().await;
            break;
        }
    }
}

fn extract_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::auth::AuthenticatedAdmin, GatewayError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(GatewayError::Unauthorized)?;
    state.auth.verify_admin_token(token).map_err(|err| {
        warn!(?err, "admin token verification failed");
        GatewayError::Unauthorized
    })
}
