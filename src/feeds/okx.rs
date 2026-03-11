use std::sync::Arc;

use serde_json::json;

use crate::{state::AppState, types::PriceSource};

pub async fn spawn(state: Arc<AppState>) {
    super::binance::super_exchange::spawn_public_trade_feed(
        state,
        "okx",
        "wss://ws.okx.com:8443/ws/v5/public",
        Some(json!({
            "op": "subscribe",
            "args": [{"channel": "tickers", "instId": "BTC-USDT"}]
        })),
        PriceSource::Okx,
        "okx",
    )
    .await;
}
