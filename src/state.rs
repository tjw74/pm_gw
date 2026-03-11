use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use serde_json::json;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    auth::AuthService,
    broadcast::EventBroadcaster,
    config::Config,
    error::GatewayError,
    models::{
        account::{AccountSummary, Position, PositionState},
        market::ActiveMarket,
        order::OrderState,
    },
    session::Session,
    types::{ConnectionState, EventType, PriceSource},
};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub broadcaster: EventBroadcaster,
    pub auth: Arc<AuthService>,
    core_started: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    inner: Arc<RwLock<GatewayState>>,
}

#[derive(Default)]
pub struct GatewayState {
    pub active_market: Option<ActiveMarket>,
    pub latest_market_price: Option<f64>,
    pub latest_reference_prices: HashMap<String, f64>,
    pub last_trade_per_source: HashMap<String, f64>,
    pub orderbook_snapshot: Option<serde_json::Value>,
    pub accounts: HashMap<String, AccountSummary>,
    pub active_orders: HashMap<String, Vec<OrderState>>,
    pub positions: HashMap<String, PositionState>,
    pub sessions: HashMap<Uuid, Session>,
    pub connections: HashMap<String, ConnectionState>,
    pub target_price: Option<f64>,
    pub seen_commands: HashMap<Uuid, OffsetDateTime>,
    pub user_rate_window: HashMap<String, Vec<OffsetDateTime>>,
}

impl AppState {
    pub fn new(config: Config, broadcaster: EventBroadcaster, auth: Arc<AuthService>) -> Self {
        let mut accounts = HashMap::new();
        let mut positions = HashMap::new();
        for user in &config.dev_users {
            accounts.insert(
                user.id.clone(),
                AccountSummary {
                    wallet_id: user.id.clone(),
                    ..AccountSummary::default()
                },
            );
            positions.insert(
                user.id.clone(),
                PositionState {
                    wallet_id: user.id.clone(),
                    ..PositionState::default()
                },
            );
        }
        Self {
            config,
            broadcaster,
            auth,
            core_started: Arc::new(AtomicBool::new(false)),
            ready: Arc::new(AtomicBool::new(false)),
            inner: Arc::new(RwLock::new(GatewayState {
                accounts,
                positions,
                ..GatewayState::default()
            })),
        }
    }

    pub fn mark_core_started(&self) {
        self.core_started.store(true, Ordering::Relaxed);
    }

    pub fn is_ready(&self) -> bool {
        self.core_started.load(Ordering::Relaxed) && self.ready.load(Ordering::Relaxed)
    }

    pub async fn record_connection(
        &self,
        adapter: &str,
        state: ConnectionState,
        detail: Option<String>,
    ) {
        {
            let mut inner = self.inner.write().await;
            inner.connections.insert(adapter.to_string(), state);
        }
        self.broadcaster
            .publish_event(crate::models::normalized::NormalizedEvent {
                event_type: EventType::ConnectionStatus,
                source: PriceSource::Gateway,
                symbol: Some("BTCUSD".to_string()),
                market_slug: self.active_market_slug().await,
                token_id: None,
                timestamp: OffsetDateTime::now_utc(),
                payload: json!({
                    "adapter": adapter,
                    "state": state,
                    "detail": detail,
                }),
            });
    }

    pub async fn set_active_market(&self, market: ActiveMarket) {
        {
            let mut inner = self.inner.write().await;
            inner.active_market = Some(market.clone());
        }
        self.ready.store(true, Ordering::Relaxed);
        self.broadcaster
            .publish_event(crate::models::normalized::NormalizedEvent {
                event_type: EventType::MarketRollover,
                source: PriceSource::Gateway,
                symbol: Some("BTCUSD".to_string()),
                market_slug: Some(market.slug.clone()),
                token_id: None,
                timestamp: OffsetDateTime::now_utc(),
                payload: json!(market),
            });
        self.refresh_snapshot().await;
    }

    pub async fn active_market_slug(&self) -> Option<String> {
        self.inner
            .read()
            .await
            .active_market
            .as_ref()
            .map(|m| m.slug.clone())
    }

    pub async fn active_market(&self) -> Option<ActiveMarket> {
        self.inner.read().await.active_market.clone()
    }

    pub async fn update_price(&self, source: &str, price: f64, market_price: bool) {
        {
            let mut inner = self.inner.write().await;
            if market_price {
                inner.latest_market_price = Some(price);
            } else {
                inner
                    .latest_reference_prices
                    .insert(source.to_string(), price);
            }
            inner
                .last_trade_per_source
                .insert(source.to_string(), price);
        }
        self.refresh_snapshot().await;
    }

    pub async fn set_orderbook(&self, snapshot: serde_json::Value) {
        self.inner.write().await.orderbook_snapshot = Some(snapshot);
        self.refresh_snapshot().await;
    }

    pub async fn create_session(&self, user_id: String) -> Session {
        let session = Session::new(user_id);
        self.inner
            .write()
            .await
            .sessions
            .insert(session.id, session.clone());
        session
    }

    pub async fn touch_session(&self, session_id: Uuid) {
        if let Some(session) = self.inner.write().await.sessions.get_mut(&session_id) {
            session.last_seen_at = OffsetDateTime::now_utc();
        }
    }

    pub async fn remove_session(&self, session_id: Uuid) {
        self.inner.write().await.sessions.remove(&session_id);
    }

    pub async fn subscribe_market(&self, session_id: Uuid, market: String) {
        if let Some(session) = self.inner.write().await.sessions.get_mut(&session_id) {
            session.subscriptions.insert(market);
        }
    }

    pub async fn unsubscribe_market(&self, session_id: Uuid, market: &str) {
        if let Some(session) = self.inner.write().await.sessions.get_mut(&session_id) {
            session.subscriptions.remove(market);
        }
    }

    pub async fn validate_command(
        &self,
        user_id: &str,
        command_id: Uuid,
        timestamp: OffsetDateTime,
    ) -> Result<(), GatewayError> {
        let now = OffsetDateTime::now_utc();
        let age = (now - timestamp).whole_seconds().unsigned_abs();
        if age > self.config.command_max_age().as_secs() {
            return Err(GatewayError::bad_request(
                "command timestamp outside allowed age window",
            ));
        }

        let mut inner = self.inner.write().await;
        if inner.seen_commands.contains_key(&command_id) {
            return Err(GatewayError::bad_request("duplicate command_id"));
        }
        inner.seen_commands.insert(command_id, timestamp);

        if let Some(max_rate) = self.config.optional_command_rate_limit_per_sec {
            let entries = inner
                .user_rate_window
                .entry(user_id.to_string())
                .or_default();
            entries.retain(|ts| (now - *ts).whole_seconds() < 1);
            if entries.len() as u32 >= max_rate {
                return Err(GatewayError::Forbidden(
                    "command rate limit exceeded".to_string(),
                ));
            }
            entries.push(now);
        }
        Ok(())
    }

    pub async fn enforce_risk(&self, size: f64, price: Option<f64>) -> Result<(), GatewayError> {
        if self.config.enable_kill_switch {
            return Err(GatewayError::Forbidden("kill switch enabled".to_string()));
        }
        if let Some(max_size) = self.config.optional_max_order_size {
            if size > max_size {
                return Err(GatewayError::Forbidden(
                    "order size exceeds configured limit".to_string(),
                ));
            }
        }
        if let (Some(max_notional), Some(px)) = (self.config.optional_max_notional, price) {
            if size * px > max_notional {
                return Err(GatewayError::Forbidden(
                    "order notional exceeds configured limit".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub async fn set_target_price(&self, price: f64) {
        self.inner.write().await.target_price = Some(price);
        self.refresh_snapshot().await;
    }

    pub async fn snapshot(&self) -> serde_json::Value {
        let inner = self.inner.read().await;
        json!({
            "active_market": inner.active_market,
            "latest_market_price": inner.latest_market_price,
            "latest_reference_prices": inner.latest_reference_prices,
            "last_trade_per_source": inner.last_trade_per_source,
            "orderbook_snapshot": inner.orderbook_snapshot,
            "accounts": inner.accounts,
            "active_orders": inner.active_orders,
            "positions": inner.positions,
            "connections": inner.connections,
            "target_price": inner.target_price,
        })
    }

    pub async fn refresh_snapshot(&self) {
        self.broadcaster.update_snapshot(self.snapshot().await);
    }

    pub async fn set_account_state(&self, user_id: &str, payload: &serde_json::Value) {
        let cash_balance = payload
            .get("balance_allowance")
            .and_then(|v| v.get("balance"))
            .and_then(parse_f64);
        let portfolio_value = payload
            .get("holdings_value")
            .and_then(|v| v.get("total"))
            .or_else(|| payload.get("holdings_value").and_then(|v| v.get("value")))
            .or_else(|| payload.get("holdings_value"))
            .and_then(parse_f64);
        let updated_at = Some(OffsetDateTime::now_utc());
        let orders = payload
            .get("open_orders")
            .map(parse_open_orders)
            .unwrap_or_default();
        let positions = payload
            .get("positions")
            .map(|value| parse_positions(user_id, value))
            .unwrap_or_else(|| PositionState {
                wallet_id: user_id.to_string(),
                positions: HashMap::new(),
            });

        let mut inner = self.inner.write().await;
        inner.accounts.insert(
            user_id.to_string(),
            AccountSummary {
                wallet_id: user_id.to_string(),
                cash_balance,
                portfolio_value,
                updated_at,
            },
        );
        inner.active_orders.insert(user_id.to_string(), orders);
        inner.positions.insert(user_id.to_string(), positions);
    }
}

fn parse_open_orders(value: &serde_json::Value) -> Vec<OrderState> {
    let entries = value
        .as_array()
        .cloned()
        .or_else(|| {
            value
                .get("data")
                .and_then(serde_json::Value::as_array)
                .cloned()
        })
        .unwrap_or_default();

    entries
        .into_iter()
        .filter_map(|entry| {
            let order_id = entry
                .get("id")
                .or_else(|| entry.get("order_id"))
                .and_then(serde_json::Value::as_str)?
                .to_string();
            Some(OrderState {
                order_id,
                market_slug: entry
                    .get("market_slug")
                    .or_else(|| entry.get("market"))
                    .and_then(serde_json::Value::as_str)
                    .map(ToString::to_string),
                outcome: entry
                    .get("outcome")
                    .and_then(serde_json::Value::as_str)
                    .map(ToString::to_string),
                side: entry
                    .get("side")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                price: entry.get("price").and_then(parse_f64),
                size: entry
                    .get("size")
                    .or_else(|| entry.get("original_size"))
                    .and_then(parse_f64)
                    .unwrap_or_default(),
                filled_size: entry
                    .get("filled_size")
                    .or_else(|| entry.get("size_matched"))
                    .and_then(parse_f64),
                status: entry
                    .get("status")
                    .or_else(|| entry.get("type"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                updated_at: OffsetDateTime::now_utc(),
            })
        })
        .collect()
}

fn parse_positions(user_id: &str, value: &serde_json::Value) -> PositionState {
    let entries = value
        .as_array()
        .cloned()
        .or_else(|| {
            value
                .get("data")
                .and_then(serde_json::Value::as_array)
                .cloned()
        })
        .unwrap_or_default();

    let positions = entries
        .into_iter()
        .filter_map(|entry| {
            let asset_key = entry
                .get("asset")
                .or_else(|| entry.get("token_id"))
                .or_else(|| entry.get("market"))
                .and_then(serde_json::Value::as_str)?
                .to_string();
            Some((
                asset_key,
                Position {
                    market_slug: entry
                        .get("market_slug")
                        .or_else(|| entry.get("market"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    outcome: entry
                        .get("outcome")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    size: entry
                        .get("size")
                        .or_else(|| entry.get("amount"))
                        .and_then(parse_f64)
                        .unwrap_or_default(),
                    average_price: entry
                        .get("average_price")
                        .or_else(|| entry.get("avg_price"))
                        .and_then(parse_f64),
                    unrealized_pnl: entry
                        .get("unrealized_pnl")
                        .or_else(|| entry.get("pnl"))
                        .and_then(parse_f64),
                },
            ))
        })
        .collect();

    PositionState {
        wallet_id: user_id.to_string(),
        positions,
    }
}

fn parse_f64(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(text) => text.parse::<f64>().ok(),
        _ => None,
    }
}
