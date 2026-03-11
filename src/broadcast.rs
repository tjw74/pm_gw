use serde_json::json;
use tokio::sync::{broadcast, watch};

use crate::{models::normalized::NormalizedEvent, responses::ServerMessage};

#[derive(Clone)]
pub struct EventBroadcaster {
    events_tx: broadcast::Sender<NormalizedEvent>,
    snapshots_tx: watch::Sender<serde_json::Value>,
}

impl EventBroadcaster {
    pub fn new() -> Self {
        let (events_tx, _) = broadcast::channel(4096);
        let (snapshots_tx, _) = watch::channel(json!({}));
        Self {
            events_tx,
            snapshots_tx,
        }
    }

    pub fn publish_event(&self, event: NormalizedEvent) {
        let _ = self.events_tx.send(event);
    }

    pub fn update_snapshot(&self, snapshot: serde_json::Value) {
        let _ = self.snapshots_tx.send(snapshot);
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<NormalizedEvent> {
        self.events_tx.subscribe()
    }

    pub fn subscribe_snapshots(&self) -> watch::Receiver<serde_json::Value> {
        self.snapshots_tx.subscribe()
    }

    pub fn route_event(event: NormalizedEvent) -> ServerMessage {
        match event.event_type {
            crate::types::EventType::MarketRollover => ServerMessage::MarketRollover { event },
            crate::types::EventType::OrderUpdate => ServerMessage::OrderUpdate {
                payload: event.payload,
            },
            crate::types::EventType::FillUpdate => ServerMessage::FillUpdate {
                payload: event.payload,
            },
            crate::types::EventType::AccountUpdate => ServerMessage::AccountUpdate {
                payload: event.payload,
            },
            crate::types::EventType::PositionUpdate => ServerMessage::PositionUpdate {
                payload: event.payload,
            },
            _ => ServerMessage::MarketUpdate { event },
        }
    }
}
