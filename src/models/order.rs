use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderState {
    pub order_id: String,
    pub market_slug: Option<String>,
    pub outcome: Option<String>,
    pub side: String,
    pub price: Option<f64>,
    pub size: f64,
    pub filled_size: Option<f64>,
    pub status: String,
    pub updated_at: OffsetDateTime,
}
