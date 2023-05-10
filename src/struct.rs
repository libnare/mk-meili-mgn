use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Notes {
    pub(crate) id: String,
    pub(crate) created_at: i64,
    pub(crate) user_id: String,
    pub(crate) user_host: Option<String>,
    pub(crate) channel_id: Option<String>,
    pub(crate) cw: Option<String>,
    pub(crate) text: String,
}