use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AccountSummary {
    pub wallet_id: String,
    pub cash_balance: Option<f64>,
    pub portfolio_value: Option<f64>,
    pub updated_at: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PositionState {
    pub wallet_id: String,
    pub positions: HashMap<String, Position>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub market_slug: String,
    pub outcome: String,
    pub size: f64,
    pub average_price: Option<f64>,
    pub unrealized_pnl: Option<f64>,
}
