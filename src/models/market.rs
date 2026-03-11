use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketWindow {
    pub window_start: OffsetDateTime,
    pub window_end: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActiveMarket {
    pub market_id: String,
    pub condition_id: Option<String>,
    pub slug: String,
    pub question: Option<String>,
    pub yes_token_id: Option<String>,
    pub no_token_id: Option<String>,
    pub window: MarketWindow,
    pub status: String,
}
