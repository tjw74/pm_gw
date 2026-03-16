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
    observability::ObservabilityState,
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
    ops: Arc<RwLock<ObservabilityState>>,
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
    pub portfolio_history: HashMap<String, Vec<PortfolioPoint>>,
    pub position_history: HashMap<String, Vec<PositionHistoryEntry>>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PortfolioPoint {
    pub timestamp: OffsetDateTime,
    pub portfolio_value: f64,
    pub unrealized_pnl: Option<f64>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PositionHistoryEntry {
    pub timestamp: OffsetDateTime,
    pub market_slug: String,
    pub outcome: String,
    pub size: f64,
    pub average_price: Option<f64>,
    pub unrealized_pnl: Option<f64>,
    pub status: String,
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
            ops: Arc::new(RwLock::new(ObservabilityState::default())),
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
        self.ops
            .write()
            .await
            .record_connection(adapter, state, detail.clone());
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
            inner.target_price = reference_midpoint(&inner.latest_reference_prices);
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
        let now = OffsetDateTime::now_utc();
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
        {
            let mut ops = self.ops.write().await;
            ops.record_feed_message(source, now, Some(0));
            ops.record_price(source, price, now);
        }
        self.refresh_snapshot().await;
    }

    pub async fn set_orderbook(&self, source: &str, snapshot: serde_json::Value) {
        self.inner.write().await.orderbook_snapshot = Some(snapshot);
        self.ops
            .write()
            .await
            .record_feed_message(source, OffsetDateTime::now_utc(), Some(0));
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
        if self.config.enable_kill_switch || self.runtime_kill_switch().await {
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
        let ops = self.ops.read().await;
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
            "runtime_kill_switch": ops.runtime_kill_switch,
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
        let current_positions = positions.clone();
        let total_unrealized_pnl = aggregate_unrealized_pnl(&positions);
        let now = OffsetDateTime::now_utc();

        let mut inner = self.inner.write().await;
        let previous_positions = inner
            .positions
            .get(user_id)
            .cloned()
            .unwrap_or_else(|| PositionState {
                wallet_id: user_id.to_string(),
                positions: HashMap::new(),
            });
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
        if let Some(value) = portfolio_value {
            let history = inner.portfolio_history.entry(user_id.to_string()).or_default();
            let should_push = history
                .last()
                .map(|point| {
                    (now - point.timestamp).whole_seconds() >= 10
                        || (point.portfolio_value - value).abs() >= 0.01
                        || point.unrealized_pnl != total_unrealized_pnl
                })
                .unwrap_or(true);
            if should_push {
                history.push(PortfolioPoint {
                    timestamp: now,
                    portfolio_value: value,
                    unrealized_pnl: total_unrealized_pnl,
                });
                trim_vec(history, 180);
            }
        }
        record_position_history(
            inner.position_history.entry(user_id.to_string()).or_default(),
            &previous_positions,
            &current_positions,
            now,
        );
        drop(inner);
        self.refresh_snapshot().await;
    }

    pub async fn record_feed_message(
        &self,
        adapter: &str,
        source_timestamp: Option<OffsetDateTime>,
    ) {
        let observed_at = OffsetDateTime::now_utc();
        let latency_ms = source_timestamp.map(|ts| (observed_at - ts).whole_milliseconds() as i64);
        self.ops
            .write()
            .await
            .record_feed_message(adapter, observed_at, latency_ms);
    }

    pub async fn record_stream_outbound(&self, bytes: usize) {
        self.ops.write().await.record_outbound(bytes);
    }

    pub async fn record_stream_drop(&self) {
        self.ops.write().await.record_drop();
    }

    pub async fn record_auth_failure(&self, reason: &str) {
        let mut ops = self.ops.write().await;
        ops.record_auth_failure();
        ops.push_audit("anonymous", "auth_failure", reason);
    }

    pub async fn record_stream_disconnect(&self) {
        self.ops.write().await.record_disconnect();
    }

    pub async fn record_command_latency(&self, timestamp: OffsetDateTime) {
        let now = OffsetDateTime::now_utc();
        let latency = (now - timestamp).whole_milliseconds().max(0) as u64;
        self.ops.write().await.record_command_latency(latency);
    }

    pub async fn runtime_kill_switch(&self) -> bool {
        self.ops.read().await.runtime_kill_switch
    }

    pub async fn set_runtime_kill_switch(&self, enabled: bool, actor: &str) {
        let mut ops = self.ops.write().await;
        ops.runtime_kill_switch = enabled;
        ops.push_audit(
            actor.to_string(),
            "set_runtime_kill_switch",
            format!("enabled={enabled}"),
        );
    }

    pub async fn push_audit(&self, actor: &str, action: &str, detail: &str) {
        self.ops
            .write()
            .await
            .push_audit(actor.to_string(), action.to_string(), detail.to_string());
    }

    pub async fn public_dashboard_snapshot(&self) -> serde_json::Value {
        let inner = self.inner.read().await;
        let ops = self.ops.read().await;
        let feeds = build_feed_status(&inner, &ops);
        let alerts = build_alerts(&inner, &ops, self.is_ready());
        let connected_feeds = feeds
            .iter()
            .filter(|feed| feed["connection"] == serde_json::Value::String("connected".to_string()))
            .count();
        let client_count = inner.sessions.len();
        let outbound_messages_per_sec = ops.streaming.outbound_samples.len() as u64;
        let outbound_bytes_per_sec: u64 = ops.streaming.outbound_samples.iter().map(|(_, size)| *size as u64).sum();
        let avg_command_latency_ms = average_u64(
            ops.streaming
                .command_samples_ms
                .iter()
                .map(|(_, value)| *value),
        );
        json!({
            "meta": {
                "version": self.config.build_version,
                "commit": self.config.build_commit,
                "uptime_seconds": (OffsetDateTime::now_utc() - ops.started_at).whole_seconds().max(0),
                "generated_at": OffsetDateTime::now_utc(),
                "ready": self.is_ready(),
            },
            "global": {
                "gateway": if self.is_ready() { "up" } else { "degraded" },
                "overall_health": overall_health(&alerts),
                "active_market_family": "BTC 5m",
                "active_market_slug": inner.active_market.as_ref().map(|market| market.slug.clone()),
                "window_countdown_seconds": inner.active_market.as_ref().map(|market| (market.window.window_end - OffsetDateTime::now_utc()).whole_seconds().max(0)),
                "upstreams_connected": connected_feeds,
                "upstreams_total": feeds.len(),
                "downstream_clients": client_count,
                "alert_level": overall_health(&alerts),
            },
            "feeds": feeds,
            "market": {
                "active": inner.active_market,
                "target_price": inner.target_price,
                "latest_market_price": inner.latest_market_price,
                "latest_reference_prices": inner.latest_reference_prices,
                "price_history": ops.price_history,
                "orderbook_snapshot": inner.orderbook_snapshot,
            },
            "streaming": {
                "active_clients": client_count,
                "authenticated_clients": client_count,
                "outbound_messages_per_sec": outbound_messages_per_sec,
                "outbound_bytes_per_sec": outbound_bytes_per_sec,
                "dropped_messages_total": ops.streaming.dropped_messages,
                "auth_failures_total": ops.streaming.auth_failures,
                "disconnects_total": ops.streaming.disconnects,
                "avg_command_latency_ms": avg_command_latency_ms,
                "sessions": inner.sessions.values().cloned().collect::<Vec<_>>(),
            },
            "alerts": alerts,
            "logs": ops.audit_log.iter().take(50).cloned().collect::<Vec<_>>(),
        })
    }

    pub async fn admin_dashboard_snapshot(&self) -> serde_json::Value {
        let public = self.public_dashboard_snapshot().await;
        let inner = self.inner.read().await;
        let ops = self.ops.read().await;
        json!({
            "public": public,
            "admin": {
                "runtime_kill_switch": ops.runtime_kill_switch,
                "accounts": inner.accounts,
                "active_orders": inner.active_orders,
                "positions": inner.positions,
                "audit": ops.audit_log,
            }
        })
    }

    pub async fn user_trade_snapshot(&self, user_id: &str) -> serde_json::Value {
        let inner = self.inner.read().await;
        let active_market = inner.active_market.clone();
        let target_price = inner.target_price;
        let latest_market_price = inner.latest_market_price;
        let latest_reference_prices = inner.latest_reference_prices.clone();
        let orderbook_snapshot = inner.orderbook_snapshot.clone();
        let account = inner.accounts.get(user_id).cloned();
        let positions = inner.positions.get(user_id).cloned();
        let active_orders = inner.active_orders.get(user_id).cloned().unwrap_or_default();
        let portfolio_history = inner
            .portfolio_history
            .get(user_id)
            .cloned()
            .unwrap_or_default();
        let position_history = inner
            .position_history
            .get(user_id)
            .cloned()
            .unwrap_or_default();
        drop(inner);
        let price_history = self.ops.read().await.price_history.clone();
        let unrealized_pnl = positions
            .as_ref()
            .and_then(aggregate_unrealized_pnl);
        let portfolio_value = account.as_ref().and_then(|value| value.portfolio_value);
        let cash_balance = account.as_ref().and_then(|value| value.cash_balance);
        json!({
            "meta": {
                "generated_at": OffsetDateTime::now_utc(),
                "ready": self.is_ready(),
                "user_id": user_id,
            },
            "market": {
                "active": active_market,
                "target_price": target_price,
                "latest_market_price": latest_market_price,
                "latest_reference_prices": latest_reference_prices,
                "orderbook_snapshot": orderbook_snapshot,
                "price_history": price_history,
            },
            "account": {
                "summary": account,
                "profitability": {
                    "portfolio_value": portfolio_value,
                    "cash_balance": cash_balance,
                    "unrealized_pnl": unrealized_pnl,
                },
                "open_orders": active_orders,
                "positions": positions,
                "portfolio_history": portfolio_history,
                "position_history": position_history,
            }
        })
    }
}

fn build_feed_status(inner: &GatewayState, ops: &ObservabilityState) -> Vec<serde_json::Value> {
    let now = OffsetDateTime::now_utc();
    let uptime_ms = (now - ops.started_at).whole_milliseconds().max(0) as i64;
    let mut adapters = ops.feed_stats.keys().cloned().collect::<Vec<_>>();
    adapters.sort();
    adapters
        .into_iter()
        .map(|adapter| {
            let feed = ops.feed_stats.get(&adapter).expect("feed exists");
            let last_message_age_ms = feed
                .last_message_at
                .map(|ts| (now - ts).whole_milliseconds().max(0) as i64);
            json!({
                "adapter": adapter,
                "connection": feed.connection,
                "last_message_at": feed.last_message_at,
                "last_message_age_ms": last_message_age_ms,
                "reconnect_count": feed.reconnect_count,
                "message_rate_per_sec": feed.messages.len(),
                "recent_disconnects_60s": feed.disconnects.len(),
                "last_latency_ms": feed.last_latency_ms,
                "last_error": feed.last_error,
                "stale": is_stale(&adapter, feed.connection, last_message_age_ms, uptime_ms),
                "connection_detail": inner.connections.get(&feed.adapter),
                "freshness_expected": feed_freshness_policy(&adapter).threshold_ms.is_some(),
            })
        })
        .collect()
}

fn build_alerts(
    inner: &GatewayState,
    ops: &ObservabilityState,
    ready: bool,
) -> Vec<serde_json::Value> {
    let now = OffsetDateTime::now_utc();
    let uptime_ms = (now - ops.started_at).whole_milliseconds().max(0) as i64;
    let mut alerts = Vec::new();
    if !ready {
        alerts.push(json!({
            "id": "gateway_not_ready",
            "severity": "critical",
            "title": "Gateway not ready",
            "detail": "Core services have not reached ready state",
            "timestamp": now,
        }));
    }
    if inner.active_market.is_none() {
        alerts.push(json!({
            "id": "missing_active_market",
            "severity": "critical",
            "title": "No active market selected",
            "detail": "Scheduler has not resolved an active BTC market window",
            "timestamp": now,
        }));
    }
    for (adapter, feed) in &ops.feed_stats {
        let age_ms = feed
            .last_message_at
            .map(|ts| (now - ts).whole_milliseconds().max(0) as i64);
        if matches!(feed.connection, ConnectionState::Disconnected) {
            alerts.push(json!({
                "id": format!("{adapter}_disconnected"),
                "severity": "critical",
                "title": format!("{adapter} disconnected"),
                "detail": feed.last_error.clone().unwrap_or_else(|| "Upstream connection is down".to_string()),
                "timestamp": now,
            }));
        } else if matches!(feed.connection, ConnectionState::Degraded) || is_stale(adapter, feed.connection, age_ms, uptime_ms) {
            let detail = if matches!(feed.connection, ConnectionState::Degraded) {
                feed.last_error.clone().unwrap_or_else(|| "Feed is connected but currently degraded".to_string())
            } else if let Some(age_ms) = age_ms {
                format!("Last message age: {age_ms} ms")
            } else {
                let policy = feed_freshness_policy(adapter);
                format!(
                    "No fresh messages observed after {} ms startup grace",
                    policy.startup_grace_ms
                )
            };
            alerts.push(json!({
                "id": format!("{adapter}_stale"),
                "severity": "warning",
                "title": format!("{adapter} stale"),
                "detail": detail,
                "timestamp": now,
            }));
        } else if feed.disconnects.len() >= 3 {
            alerts.push(json!({
                "id": format!("{adapter}_reconnect_storm"),
                "severity": "warning",
                "title": format!("{adapter} reconnect storm"),
                "detail": format!("{} disconnects in the last minute", feed.disconnects.len()),
                "timestamp": now,
            }));
        }
    }
    if ops.streaming.dropped_messages > 0 {
        alerts.push(json!({
            "id": "downstream_drops",
            "severity": "warning",
            "title": "Downstream backpressure",
            "detail": format!("{} outbound messages dropped", ops.streaming.dropped_messages),
            "timestamp": now,
        }));
    }
    if ops.runtime_kill_switch {
        alerts.push(json!({
            "id": "runtime_kill_switch",
            "severity": "warning",
            "title": "Runtime kill switch enabled",
            "detail": "Order placement is blocked by operator control",
            "timestamp": now,
        }));
    }
    alerts
}

#[derive(Clone, Copy)]
struct FeedFreshnessPolicy {
    threshold_ms: Option<i64>,
    startup_grace_ms: i64,
}

fn feed_freshness_policy(adapter: &str) -> FeedFreshnessPolicy {
    if adapter.contains("polymarket_clob_user") || adapter.contains("polymarket_gamma") || adapter.contains("market_scheduler") {
        FeedFreshnessPolicy {
            threshold_ms: None,
            startup_grace_ms: 0,
        }
    } else if adapter.contains("bitstamp") {
        FeedFreshnessPolicy {
            threshold_ms: Some(150_000),
            startup_grace_ms: 90_000,
        }
    } else if adapter.contains("polymarket_clob_market") {
        FeedFreshnessPolicy {
            threshold_ms: Some(60_000),
            startup_grace_ms: 45_000,
        }
    } else if adapter.contains("rtds") {
        FeedFreshnessPolicy {
            threshold_ms: Some(45_000),
            startup_grace_ms: 45_000,
        }
    } else {
        FeedFreshnessPolicy {
            threshold_ms: Some(45_000),
            startup_grace_ms: 30_000,
        }
    }
}

fn is_stale(
    adapter: &str,
    connection: ConnectionState,
    age_ms: Option<i64>,
    uptime_ms: i64,
) -> bool {
    let policy = feed_freshness_policy(adapter);
    let Some(threshold_ms) = policy.threshold_ms else {
        return false;
    };
    if matches!(connection, ConnectionState::Connecting | ConnectionState::Disconnected) {
        return false;
    }
    match age_ms {
        Some(age_ms) => age_ms > threshold_ms,
        None => uptime_ms > policy.startup_grace_ms,
    }
}

fn overall_health(alerts: &[serde_json::Value]) -> &'static str {
    if alerts
        .iter()
        .any(|alert| alert.get("severity").and_then(serde_json::Value::as_str) == Some("critical"))
    {
        "critical"
    } else if !alerts.is_empty() {
        "warning"
    } else {
        "healthy"
    }
}

fn average_u64(values: impl Iterator<Item = u64>) -> Option<u64> {
    let mut total = 0u64;
    let mut count = 0u64;
    for value in values {
        total += value;
        count += 1;
    }
    if count > 0 {
        Some(total / count)
    } else {
        None
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

fn reference_midpoint(values: &HashMap<String, f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut entries = values.values().copied().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let middle = entries.len() / 2;
    if entries.len() % 2 == 1 {
        entries.get(middle).copied()
    } else {
        Some((entries[middle - 1] + entries[middle]) / 2.0)
    }
}

fn aggregate_unrealized_pnl(positions: &PositionState) -> Option<f64> {
    let mut total = 0.0;
    let mut found = false;
    for position in positions.positions.values() {
        if let Some(value) = position.unrealized_pnl {
            total += value;
            found = true;
        }
    }
    found.then_some(total)
}

fn record_position_history(
    history: &mut Vec<PositionHistoryEntry>,
    previous: &PositionState,
    current: &PositionState,
    timestamp: OffsetDateTime,
) {
    for (key, position) in &current.positions {
        let status = match previous.positions.get(key) {
            None => Some("opened"),
            Some(existing) if position_changed(existing, position) => Some("updated"),
            _ => None,
        };
        if let Some(status) = status {
            history.push(PositionHistoryEntry {
                timestamp,
                market_slug: position.market_slug.clone(),
                outcome: position.outcome.clone(),
                size: position.size,
                average_price: position.average_price,
                unrealized_pnl: position.unrealized_pnl,
                status: status.to_string(),
            });
        }
    }

    for (key, position) in &previous.positions {
        if !current.positions.contains_key(key) {
            history.push(PositionHistoryEntry {
                timestamp,
                market_slug: position.market_slug.clone(),
                outcome: position.outcome.clone(),
                size: position.size,
                average_price: position.average_price,
                unrealized_pnl: position.unrealized_pnl,
                status: "closed".to_string(),
            });
        }
    }

    trim_vec(history, 60);
}

fn position_changed(left: &Position, right: &Position) -> bool {
    left.market_slug != right.market_slug
        || left.outcome != right.outcome
        || (left.size - right.size).abs() >= 0.0001
        || option_changed(left.average_price, right.average_price)
        || option_changed(left.unrealized_pnl, right.unrealized_pnl)
}

fn option_changed(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => (left - right).abs() >= 0.0001,
        (None, None) => false,
        _ => true,
    }
}

fn trim_vec<T>(entries: &mut Vec<T>, max_len: usize) {
    if entries.len() > max_len {
        let overflow = entries.len() - max_len;
        entries.drain(0..overflow);
    }
}
