use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub user_id: String,
    pub created_at: OffsetDateTime,
    pub last_seen_at: OffsetDateTime,
    pub subscriptions: HashSet<String>,
}

impl Session {
    pub fn new(user_id: String) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: Uuid::new_v4(),
            user_id,
            created_at: now,
            last_seen_at: now,
            subscriptions: HashSet::new(),
        }
    }
}
