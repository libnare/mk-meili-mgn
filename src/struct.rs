use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use chrono::serde::ts_milliseconds;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Notes {
    pub(crate) id: String,
    #[serde(with = "ts_milliseconds")]
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) user_id: String,
    pub(crate) user_host: Option<String>,
    pub(crate) channel_id: Option<String>,
    pub(crate) cw: Option<String>,
    pub(crate) text: String,
}