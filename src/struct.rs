use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notes {
    pub id: String,
    pub created_at: i64,
    pub user_id: String,
    pub user_host: Option<String>,
    pub channel_id: Option<String>,
    pub cw: Option<String>,
    pub text: String,
    pub tags: Vec<String>,
}

impl Notes {
    pub fn new(
        id: String,
        created_at: DateTime<Utc>,
        user_id: String,
        user_host: Option<String>,
        channel_id: Option<String>,
        cw: Option<String>,
        text: String,
        tags: Vec<String>,
    ) -> Self {
        let created_at = created_at.timestamp() * 1000 + created_at.timestamp_subsec_millis() as i64;
        Self {
            id,
            created_at,
            user_id,
            user_host,
            channel_id,
            cw,
            text,
            tags,
        }
    }
}