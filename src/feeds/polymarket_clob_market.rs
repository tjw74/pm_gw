use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

use crate::{
    models::normalized::NormalizedEvent,
    state::AppState,
    types::{ConnectionState, EventType, PriceSource},
    util::retry::backoff,
};

pub async fn spawn(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut attempt = 0u32;
        loop {
            if let Err(err) = run_once(state.clone()).await {
                error!(?err, "polymarket market feed disconnected");
                state
                    .record_connection(
                        "polymarket_clob_market",
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

async fn run_once(state: Arc<AppState>) -> anyhow::Result<()> {
    let subscription = wait_for_subscription_target(&state).await?;
    state
        .record_connection("polymarket_clob_market", ConnectionState::Connecting, None)
        .await;
    let (mut ws, _) = connect_async(&state.config.poly_clob_market_ws_url).await?;
    state
        .record_connection("polymarket_clob_market", ConnectionState::Connected, None)
        .await;

    let subscribe = json!({
        "assets_ids": subscription.asset_ids,
        "type": "market",
    });
    ws.send(Message::Text(subscribe.to_string())).await?;
    info!(market = %subscription.slug, "subscribed polymarket market websocket");

    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(20));
    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                let next_market_slug = state.active_market_slug().await;
                if next_market_slug.as_deref() != Some(subscription.slug.as_str()) {
                    info!(from = %subscription.slug, to = ?next_market_slug, "active market changed, resubscribing polymarket market websocket");
                    break;
                }
                ws.send(Message::Ping(Vec::new().into())).await?;
            }
            maybe_message = ws.next() => {
                let Some(message) = maybe_message else { break; };
                match message? {
                    Message::Text(text) => handle_text(&state, &text).await,
                    Message::Binary(bin) => {
                        handle_value(
                            &state,
                            serde_json::from_slice::<Value>(&bin).unwrap_or(Value::Null),
                        )
                        .await
                    }
                    Message::Ping(payload) => ws.send(Message::Pong(payload)).await?,
                    Message::Pong(_) => {}
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

struct SubscriptionTarget {
    slug: String,
    asset_ids: Vec<String>,
}

async fn wait_for_subscription_target(state: &AppState) -> anyhow::Result<SubscriptionTarget> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        if let Some(market) = state.active_market().await {
            let asset_ids = [market.yes_token_id.clone(), market.no_token_id.clone()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            if asset_ids.len() >= 2 {
                return Ok(SubscriptionTarget {
                    slug: market.slug,
                    asset_ids,
                });
            }
        }

        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("timed out waiting for active market token ids");
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

async fn handle_text(state: &AppState, text: &str) {
    let value = serde_json::from_str::<Value>(text).unwrap_or_else(|_| json!({ "raw": text }));
    handle_value(state, value).await;
}

async fn handle_value(state: &AppState, value: Value) {
    if value == Value::Null
        || value.get("event").and_then(Value::as_str) == Some("subscribed")
        || value.get("type").and_then(Value::as_str) == Some("subscribed")
    {
        return;
    }

    if let Some(price) = value
        .get("price")
        .or_else(|| value.get("mid"))
        .and_then(|v| v.as_f64())
    {
        state.record_feed_message("polymarket_clob_market", None).await;
        state.update_price("polymarket_clob", price, true).await;
        state.broadcaster.publish_event(NormalizedEvent {
            event_type: EventType::PriceTick,
            source: PriceSource::PolymarketClob,
            symbol: Some("BTCUSD".to_string()),
            market_slug: state.active_market_slug().await,
            token_id: None,
            timestamp: time::OffsetDateTime::now_utc(),
            payload: value,
        });
        return;
    }

    if value.get("bids").is_some() || value.get("asks").is_some() {
        state.record_feed_message("polymarket_clob_market", None).await;
        state.set_orderbook("polymarket_clob_market", value.clone()).await;
        state.broadcaster.publish_event(NormalizedEvent {
            event_type: EventType::OrderBookSnapshot,
            source: PriceSource::PolymarketClob,
            symbol: Some("BTCUSD".to_string()),
            market_slug: state.active_market_slug().await,
            token_id: None,
            timestamp: time::OffsetDateTime::now_utc(),
            payload: value,
        });
        return;
    }

    warn!(payload = %value, "unclassified polymarket market payload");
}
