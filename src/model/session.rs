use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub token: String,
    pub created_at: String,
    pub expired_at: String,
}
