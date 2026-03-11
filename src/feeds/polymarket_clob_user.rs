use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

use crate::{
    execution::polymarket::PolymarketExecutionClient,
    models::normalized::NormalizedEvent,
    state::AppState,
    types::{ConnectionState, EventType, PriceSource},
    util::retry::backoff,
};

pub async fn spawn(state: Arc<AppState>, execution: Arc<PolymarketExecutionClient>) {
    tokio::spawn(async move {
        if let Err(err) = execution.sync_user_state().await {
            error!(?err, "initial user sync failed");
        } else {
            info!("initial user sync completed");
        }

        for user in state.config.dev_users.clone() {
            let state = state.clone();
            tokio::spawn(async move {
                let mut attempt = 0u32;
                loop {
                    if let Err(err) = run_user_stream(state.clone(), user.id.clone()).await {
                        error!(user = %user.id, ?err, "polymarket user websocket disconnected");
                        state
                            .record_connection(
                                &format!("polymarket_clob_user_{}", user.id),
                                ConnectionState::Disconnected,
                                Some(err.to_string()),
                            )
                            .await;
                        attempt += 1;
                        tokio::time::sleep(backoff(attempt)).await;
                    } else {
                        attempt = 0;
                    }
                }
            });
        }
    });
}

async fn run_user_stream(state: Arc<AppState>, user_id: String) -> anyhow::Result<()> {
    let context = state
        .auth
        .polymarket_user(&user_id)
        .map_err(anyhow::Error::from)?;
    let connection_key = format!("polymarket_clob_user_{}", user_id);
    state
        .record_connection(&connection_key, ConnectionState::Connecting, None)
        .await;
    let (mut ws, _) = connect_async(&state.config.poly_clob_user_ws_url).await?;
    state
        .record_connection(&connection_key, ConnectionState::Connected, None)
        .await;

    let markets = state
        .active_market()
        .await
        .and_then(|market| market.condition_id.or(Some(market.market_id)))
        .map(|market| vec![market])
        .unwrap_or_default();
    let subscribe = json!({
        "auth": {
            "apiKey": context.api.api_key,
            "secret": context.api.secret,
            "passphrase": context.api.passphrase,
        },
        "markets": markets,
        "type": "user",
    });
    ws.send(Message::Text(subscribe.to_string().into())).await?;

    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(10));
    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                ws.send(Message::Text("PING".into())).await?;
            }
            maybe_message = ws.next() => {
                match maybe_message {
                    Some(Ok(Message::Text(text))) => {
                        if text == "PONG" {
                            continue;
                        }
                        handle_user_message(&state, &user_id, &text).await;
                    }
                    Some(Ok(Message::Binary(bin))) => {
                        if let Ok(text) = String::from_utf8(bin.to_vec()) {
                            handle_user_message(&state, &user_id, &text).await;
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        ws.send(Message::Pong(payload)).await?;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(err)) => return Err(err.into()),
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

async fn handle_user_message(state: &AppState, user_id: &str, text: &str) {
    let value = serde_json::from_str::<Value>(text).unwrap_or_else(|_| json!({ "raw": text }));
    let event_type = match classify_user_event(&value) {
        Some(event_type) => event_type,
        _ => {
            warn!(user = %user_id, payload = text, "unclassified polymarket user payload");
            return;
        }
    };

    state.broadcaster.publish_event(NormalizedEvent {
        event_type,
        source: PriceSource::PolymarketClob,
        symbol: Some("BTCUSD".to_string()),
        market_slug: state.active_market_slug().await,
        token_id: None,
        timestamp: time::OffsetDateTime::now_utc(),
        payload: json!({
            "user_id": user_id,
            "event": value,
        }),
    });
}

fn classify_user_event(value: &Value) -> Option<EventType> {
    let kind = value
        .get("type")
        .or_else(|| value.get("event_type"))
        .and_then(Value::as_str)?
        .to_ascii_lowercase();

    if matches!(kind.as_str(), "trade" | "match" | "matched" | "fill") {
        return Some(EventType::FillUpdate);
    }

    if matches!(
        kind.as_str(),
        "order" | "placement" | "update" | "cancellation" | "cancelled"
    ) {
        return Some(EventType::OrderUpdate);
    }

    None
}
