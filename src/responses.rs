use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::models::normalized::NormalizedEvent;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    AuthOk {
        session_id: Uuid,
        user_id: String,
    },
    AuthError {
        error: String,
    },
    CommandAck {
        command_id: Uuid,
        accepted_at: OffsetDateTime,
    },
    MarketUpdate {
        event: NormalizedEvent,
    },
    MarketRollover {
        event: NormalizedEvent,
    },
    OrderUpdate {
        payload: Value,
    },
    FillUpdate {
        payload: Value,
    },
    AccountUpdate {
        payload: Value,
    },
    PositionUpdate {
        payload: Value,
    },
    Heartbeat {
        timestamp: OffsetDateTime,
    },
    Snapshot {
        payload: Value,
    },
}
