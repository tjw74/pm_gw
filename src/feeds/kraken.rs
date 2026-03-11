use std::sync::Arc;

use serde_json::json;

use crate::{state::AppState, types::PriceSource};

pub async fn spawn(state: Arc<AppState>) {
    super::binance::super_exchange::spawn_public_trade_feed(
        state,
        "kraken",
        "wss://ws.kraken.com/v2",
        Some(json!({
            "method": "subscribe",
            "params": {
                "channel": "ticker",
                "symbol": ["BTC/USD"]
            }
        })),
        PriceSource::Kraken,
        "kraken",
    )
    .await;
}
