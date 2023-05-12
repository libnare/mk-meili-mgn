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
}
