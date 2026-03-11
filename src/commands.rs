use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::types::{Outcome, SizeType, TradeSide};

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Auth {
        token: String,
    },
    Ping {
        command_id: Uuid,
        timestamp: OffsetDateTime,
    },
    SubscribeMarket {
        command_id: Uuid,
        timestamp: OffsetDateTime,
        market: Option<String>,
    },
    UnsubscribeMarket {
        command_id: Uuid,
        timestamp: OffsetDateTime,
        market: Option<String>,
    },
    PlaceLimitOrder {
        command_id: Uuid,
        timestamp: OffsetDateTime,
        side: TradeSide,
        outcome: Outcome,
        size_type: SizeType,
        size: f64,
        price: f64,
    },
    PlaceMarketOrder {
        command_id: Uuid,
        timestamp: OffsetDateTime,
        side: TradeSide,
        outcome: Outcome,
        size_type: SizeType,
        size: f64,
    },
    CancelOrder {
        command_id: Uuid,
        timestamp: OffsetDateTime,
        order_id: String,
    },
    CancelAll {
        command_id: Uuid,
        timestamp: OffsetDateTime,
    },
    GetOpenOrders {
        command_id: Uuid,
        timestamp: OffsetDateTime,
    },
    GetPositions {
        command_id: Uuid,
        timestamp: OffsetDateTime,
    },
    GetAccountState {
        command_id: Uuid,
        timestamp: OffsetDateTime,
    },
    SetTargetPrice {
        command_id: Uuid,
        timestamp: OffsetDateTime,
        price: f64,
    },
}

impl ClientMessage {
    pub fn command_id(&self) -> Option<Uuid> {
        match self {
            ClientMessage::Auth { .. } => None,
            ClientMessage::Ping { command_id, .. }
            | ClientMessage::SubscribeMarket { command_id, .. }
            | ClientMessage::UnsubscribeMarket { command_id, .. }
            | ClientMessage::PlaceLimitOrder { command_id, .. }
            | ClientMessage::PlaceMarketOrder { command_id, .. }
            | ClientMessage::CancelOrder { command_id, .. }
            | ClientMessage::CancelAll { command_id, .. }
            | ClientMessage::GetOpenOrders { command_id, .. }
            | ClientMessage::GetPositions { command_id, .. }
            | ClientMessage::GetAccountState { command_id, .. }
            | ClientMessage::SetTargetPrice { command_id, .. } => Some(*command_id),
        }
    }

    pub fn timestamp(&self) -> Option<OffsetDateTime> {
        match self {
            ClientMessage::Auth { .. } => None,
            ClientMessage::Ping { timestamp, .. }
            | ClientMessage::SubscribeMarket { timestamp, .. }
            | ClientMessage::UnsubscribeMarket { timestamp, .. }
            | ClientMessage::PlaceLimitOrder { timestamp, .. }
            | ClientMessage::PlaceMarketOrder { timestamp, .. }
            | ClientMessage::CancelOrder { timestamp, .. }
            | ClientMessage::CancelAll { timestamp, .. }
            | ClientMessage::GetOpenOrders { timestamp, .. }
            | ClientMessage::GetPositions { timestamp, .. }
            | ClientMessage::GetAccountState { timestamp, .. }
            | ClientMessage::SetTargetPrice { timestamp, .. } => Some(*timestamp),
        }
    }
}
