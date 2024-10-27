use std::collections::HashMap;

use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use uuid::Uuid;

use super::user::user_key;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Session {
    pub session_id: String,
    pub created_at: String,
    pub expired_at: String,
}

impl Session {
    pub fn new() -> Self {
        Session {
            session_id: Uuid::new_v4().to_string(),
            created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            expired_at: (Utc::now() + Duration::days(7)).to_rfc3339_opts(SecondsFormat::Secs, true),
        }
    }

    pub fn to_item(&self, username: &str) -> HashMap<String, AttributeValue> {
        HashMap::from([
            ("PK".to_string(), user_key(username)),
            ("SK".to_string(), session_key(&self.session_id)),
            (
                "TTL".to_string(),
                AttributeValue::N(
                    DateTime::parse_from_rfc3339(&self.expired_at)
                        .unwrap()
                        .timestamp()
                        .to_string(),
                ),
            ),
            (
                "created_at".to_string(),
                AttributeValue::S(self.created_at.clone().to_string()),
            ),
            (
                "expired_at".to_string(),
                AttributeValue::S(self.expired_at.clone().to_string()),
            ),
            ("GSI1PK".to_string(), session_key(&self.session_id)),
            ("GSI1SK".to_string(), user_key(username)),
        ])
    }
}

pub fn session_key(session_id: &str) -> AttributeValue {
    let key = format!("SESSION#{session_id}");

    AttributeValue::S(key)
}
