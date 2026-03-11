use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

use crate::types::{EventType, PriceSource};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedEvent {
    pub event_type: EventType,
    pub source: PriceSource,
    pub symbol: Option<String>,
    pub market_slug: Option<String>,
    pub token_id: Option<String>,
    pub timestamp: OffsetDateTime,
    pub payload: Value,
}
