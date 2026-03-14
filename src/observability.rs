use std::collections::{HashMap, VecDeque};

use serde::Serialize;
use time::OffsetDateTime;

use crate::types::ConnectionState;

const WINDOW_SECONDS: i64 = 60;
const HISTORY_LIMIT: usize = 240;
const AUDIT_LIMIT: usize = 250;
const MIN_PRICE_SAMPLE_INTERVAL_MS: i128 = 500;
const DUPLICATE_PRICE_INTERVAL_MS: i128 = 2_000;

#[derive(Clone, Debug, Serialize)]
pub struct AuditEntry {
    pub timestamp: OffsetDateTime,
    pub actor: String,
    pub action: String,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PricePoint {
    pub timestamp: OffsetDateTime,
    pub value: f64,
}

#[derive(Clone, Debug)]
pub struct FeedRuntimeStats {
    pub adapter: String,
    pub connection: ConnectionState,
    pub last_message_at: Option<OffsetDateTime>,
    pub last_error: Option<String>,
    pub reconnect_count: u64,
    pub last_latency_ms: Option<i64>,
    pub messages: VecDeque<OffsetDateTime>,
    pub disconnects: VecDeque<OffsetDateTime>,
}

impl FeedRuntimeStats {
    pub fn new(adapter: impl Into<String>) -> Self {
        Self {
            adapter: adapter.into(),
            connection: ConnectionState::Connecting,
            last_message_at: None,
            last_error: None,
            reconnect_count: 0,
            last_latency_ms: None,
            messages: VecDeque::new(),
            disconnects: VecDeque::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct StreamingRuntimeStats {
    pub outbound_messages: u64,
    pub outbound_bytes: u64,
    pub dropped_messages: u64,
    pub auth_failures: u64,
    pub disconnects: u64,
    pub outbound_samples: VecDeque<(OffsetDateTime, usize)>,
    pub command_samples_ms: VecDeque<(OffsetDateTime, u64)>,
}

#[derive(Clone, Debug)]
pub struct ObservabilityState {
    pub started_at: OffsetDateTime,
    pub feed_stats: HashMap<String, FeedRuntimeStats>,
    pub price_history: HashMap<String, VecDeque<PricePoint>>,
    pub streaming: StreamingRuntimeStats,
    pub runtime_kill_switch: bool,
    pub audit_log: VecDeque<AuditEntry>,
}

impl Default for ObservabilityState {
    fn default() -> Self {
        Self {
            started_at: OffsetDateTime::now_utc(),
            feed_stats: HashMap::new(),
            price_history: HashMap::new(),
            streaming: StreamingRuntimeStats::default(),
            runtime_kill_switch: false,
            audit_log: VecDeque::new(),
        }
    }
}

impl ObservabilityState {
    pub fn record_connection(
        &mut self,
        adapter: &str,
        state: ConnectionState,
        detail: Option<String>,
    ) {
        let entry = self
            .feed_stats
            .entry(adapter.to_string())
            .or_insert_with(|| FeedRuntimeStats::new(adapter));
        if matches!(state, ConnectionState::Disconnected) {
            entry.reconnect_count += 1;
            entry.disconnects.push_back(OffsetDateTime::now_utc());
            trim_timestamps(&mut entry.disconnects, WINDOW_SECONDS);
        }
        entry.connection = state;
        entry.last_error = detail;
    }

    pub fn record_feed_message(
        &mut self,
        adapter: &str,
        observed_at: OffsetDateTime,
        latency_ms: Option<i64>,
    ) {
        let entry = self
            .feed_stats
            .entry(adapter.to_string())
            .or_insert_with(|| FeedRuntimeStats::new(adapter));
        entry.last_message_at = Some(observed_at);
        entry.last_latency_ms = latency_ms;
        entry.messages.push_back(observed_at);
        trim_timestamps(&mut entry.messages, WINDOW_SECONDS);
    }

    pub fn record_price(&mut self, source: &str, price: f64, observed_at: OffsetDateTime) {
        let entry = self.price_history.entry(source.to_string()).or_default();
        if let Some(last) = entry.back_mut() {
            let age_ms = (observed_at - last.timestamp).whole_milliseconds();
            if age_ms < MIN_PRICE_SAMPLE_INTERVAL_MS {
                last.timestamp = observed_at;
                last.value = price;
                return;
            }
            if (last.value - price).abs() < f64::EPSILON && age_ms < DUPLICATE_PRICE_INTERVAL_MS {
                last.timestamp = observed_at;
                return;
            }
        }
        entry.push_back(PricePoint {
            timestamp: observed_at,
            value: price,
        });
        while entry.len() > HISTORY_LIMIT {
            let _ = entry.pop_front();
        }
    }

    pub fn record_outbound(&mut self, size: usize) {
        let now = OffsetDateTime::now_utc();
        self.streaming.outbound_messages += 1;
        self.streaming.outbound_bytes += size as u64;
        self.streaming.outbound_samples.push_back((now, size));
        trim_pairs(&mut self.streaming.outbound_samples, WINDOW_SECONDS);
    }

    pub fn record_drop(&mut self) {
        self.streaming.dropped_messages += 1;
    }

    pub fn record_auth_failure(&mut self) {
        self.streaming.auth_failures += 1;
    }

    pub fn record_disconnect(&mut self) {
        self.streaming.disconnects += 1;
    }

    pub fn record_command_latency(&mut self, latency_ms: u64) {
        self.streaming
            .command_samples_ms
            .push_back((OffsetDateTime::now_utc(), latency_ms));
        trim_pairs(&mut self.streaming.command_samples_ms, WINDOW_SECONDS);
    }

    pub fn push_audit(&mut self, actor: impl Into<String>, action: impl Into<String>, detail: impl Into<String>) {
        self.audit_log.push_front(AuditEntry {
            timestamp: OffsetDateTime::now_utc(),
            actor: actor.into(),
            action: action.into(),
            detail: detail.into(),
        });
        while self.audit_log.len() > AUDIT_LIMIT {
            let _ = self.audit_log.pop_back();
        }
    }
}

fn trim_timestamps(queue: &mut VecDeque<OffsetDateTime>, seconds: i64) {
    let now = OffsetDateTime::now_utc();
    while queue
        .front()
        .is_some_and(|ts| (now - *ts).whole_seconds() > seconds)
    {
        let _ = queue.pop_front();
    }
}

fn trim_pairs<T>(queue: &mut VecDeque<(OffsetDateTime, T)>, seconds: i64) {
    let now = OffsetDateTime::now_utc();
    while queue
        .front()
        .is_some_and(|(ts, _)| (now - *ts).whole_seconds() > seconds)
    {
        let _ = queue.pop_front();
    }
}
