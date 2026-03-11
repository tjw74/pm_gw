use std::sync::Arc;

use serde_json::json;

use crate::{state::AppState, types::PriceSource};

pub async fn spawn(state: Arc<AppState>) {
    super::binance::super_exchange::spawn_public_trade_feed(
        state,
        "bitstamp",
        "wss://ws.bitstamp.net",
        Some(json!({
            "event": "bts:subscribe",
            "data": { "channel": "live_trades_btcusd" }
        })),
        PriceSource::Bitstamp,
        "bitstamp",
    )
    .await;
}
