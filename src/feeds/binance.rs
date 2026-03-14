use std::sync::Arc;

use serde_json::json;

use crate::{state::AppState, types::PriceSource};

pub async fn spawn(state: Arc<AppState>) {
    super_exchange::spawn_public_trade_feed(
        state,
        "binance",
        "wss://stream.binance.com:9443/ws/btcusdt@bookTicker",
        Some(json!({"method":"SUBSCRIBE","params":["btcusdt@trade"],"id":1})),
        PriceSource::Binance,
        "binance",
    )
    .await;
}

pub(crate) mod super_exchange {
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

    pub async fn spawn_public_trade_feed(
        state: Arc<AppState>,
        name: &'static str,
        url: &'static str,
        subscribe: Option<Value>,
        source: PriceSource,
        state_key: &'static str,
    ) {
        tokio::spawn(async move {
            let mut attempt = 0u32;
            loop {
                if let Err(err) = run_once(
                    state.clone(),
                    name,
                    url,
                    subscribe.clone(),
                    source,
                    state_key,
                )
                .await
                {
                    if err.to_string().contains("Connection reset without closing handshake") {
                        warn!(?err, adapter = name, "feed reset by upstream, reconnecting");
                    } else {
                        error!(?err, adapter = name, "feed disconnected");
                    }
                    state
                        .record_connection(
                            state_key,
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

    async fn run_once(
        state: Arc<AppState>,
        name: &'static str,
        url: &'static str,
        subscribe: Option<Value>,
        source: PriceSource,
        state_key: &'static str,
    ) -> anyhow::Result<()> {
        state
            .record_connection(state_key, ConnectionState::Connecting, None)
            .await;
        let (mut ws, _) = connect_async(url).await?;
        state
            .record_connection(state_key, ConnectionState::Connected, None)
            .await;
        if let Some(subscribe) = subscribe {
            ws.send(Message::Text(subscribe.to_string())).await?;
        }
        let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(20));
        loop {
            tokio::select! {
                _ = heartbeat.tick() => {
                    match name {
                        "coinbase" => ws.send(Message::Text(json!({"type":"heartbeat","on":true}).to_string().into())).await?,
                        _ => ws.send(Message::Ping(Vec::new().into())).await?,
                    }
                }
                maybe_message = ws.next() => {
                    let Some(message) = maybe_message else { break; };
                    match message? {
                        Message::Text(text) => {
                            let value = serde_json::from_str::<Value>(&text).unwrap_or(Value::Null);
                            if let Some(event) = normalize_feed_message(name, source, &state, value).await {
                                state.record_feed_message(state_key, None).await;
                                if let Some(price) = event.payload.get("price").and_then(parse_f64_value) {
                                    state.update_price(name, price, false).await;
                                }
                                state.broadcaster.publish_event(event);
                            }
                        }
                        Message::Ping(payload) => ws.send(Message::Pong(payload)).await?,
                        Message::Pong(_) => {}
                        Message::Close(_) => break,
                        _ => {}
                    }
                }
            }
        }
        info!(adapter = name, "public feed loop ended, reconnecting");
        Ok(())
    }

    async fn normalize_feed_message(
        adapter: &str,
        source: PriceSource,
        state: &Arc<AppState>,
        value: Value,
    ) -> Option<NormalizedEvent> {
        let payload = match adapter {
            "binance" => normalize_binance(value),
            "coinbase" => normalize_coinbase(value),
            "kraken" => normalize_kraken(value),
            "okx" => normalize_okx(value),
            "bitstamp" => normalize_bitstamp(value),
            _ => None,
        }?;

        Some(NormalizedEvent {
            event_type: EventType::PriceTick,
            source,
            symbol: Some("BTCUSD".to_string()),
            market_slug: state.active_market_slug().await,
            token_id: None,
            timestamp: time::OffsetDateTime::now_utc(),
            payload,
        })
    }

    fn normalize_binance(value: Value) -> Option<Value> {
        let bid = parse_f64(value.get("b"))?;
        let ask = parse_f64(value.get("a"))?;
        Some(json!({
            "price": (bid + ask) / 2.0,
            "bid": bid,
            "ask": ask,
            "source_symbol": value.get("s"),
            "raw": value,
        }))
    }

    fn normalize_coinbase(value: Value) -> Option<Value> {
        if value.get("channel").and_then(Value::as_str) == Some("heartbeats") {
            return None;
        }
        let ticker = value
            .get("events")?
            .as_array()?
            .first()?
            .get("tickers")?
            .as_array()?
            .first()?
            .clone();
        let price = parse_f64(ticker.get("price"))?;
        Some(json!({
            "price": price,
            "bid": parse_f64(ticker.get("best_bid")),
            "ask": parse_f64(ticker.get("best_ask")),
            "source_symbol": ticker.get("product_id"),
            "raw": value,
        }))
    }

    fn normalize_kraken(value: Value) -> Option<Value> {
        let ticker = value.get("data")?.as_array()?.first()?.clone();
        let price = parse_f64(ticker.get("last"))?;
        Some(json!({
            "price": price,
            "bid": parse_f64(ticker.get("bid")),
            "ask": parse_f64(ticker.get("ask")),
            "source_symbol": ticker.get("symbol"),
            "raw": value,
        }))
    }

    fn normalize_okx(value: Value) -> Option<Value> {
        let ticker = value.get("data")?.as_array()?.first()?.clone();
        let price = parse_f64(ticker.get("last"))?;
        Some(json!({
            "price": price,
            "bid": parse_f64(ticker.get("bidPx")),
            "ask": parse_f64(ticker.get("askPx")),
            "source_symbol": ticker.get("instId"),
            "raw": value,
        }))
    }

    fn normalize_bitstamp(value: Value) -> Option<Value> {
        if value.get("event")?.as_str()? != "trade" {
            return None;
        }
        let data = value.get("data")?.clone();
        let price = parse_f64(data.get("price").or_else(|| data.get("price_str")))?;
        Some(json!({
            "price": price,
            "source_symbol": "BTCUSD",
            "raw": value,
        }))
    }

    fn parse_f64(value: Option<&Value>) -> Option<f64> {
        parse_f64_value(value?)
    }

    fn parse_f64_value(value: &Value) -> Option<f64> {
        match value {
            Value::Number(number) => number.as_f64(),
            Value::String(text) => text.parse::<f64>().ok(),
            _ => None,
        }
    }
}
