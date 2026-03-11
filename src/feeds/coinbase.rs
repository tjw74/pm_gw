use std::sync::Arc;

use serde_json::json;

use crate::{state::AppState, types::PriceSource};

pub async fn spawn(state: Arc<AppState>) {
    super::binance::super_exchange::spawn_public_trade_feed(
        state,
        "coinbase",
        "wss://advanced-trade-ws.coinbase.com",
        Some(json!({
            "type": "subscribe",
            "channel": "ticker",
            "product_ids": ["BTC-USD"]
        })),
        PriceSource::Coinbase,
        "coinbase",
    )
    .await;
}
