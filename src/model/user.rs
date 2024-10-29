use std::collections::HashMap;

use chrono::{SecondsFormat, Utc};
use pwhash::bcrypt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password: String,
    pub phone: Option<String>,
}

impl User {
    pub fn new(username: String, password: String, phone: Option<String>) -> Self {
        User {
            username,
            password,
            phone,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn varify(&self, password: &str) -> bool {
        bcrypt::verify(password, &self.password)
    }

    pub fn to_item(&self) -> HashMap<String, aws_sdk_dynamodb::types::AttributeValue> {
        let hash_password = bcrypt::hash(&self.password).unwrap();
        let date = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

        let mut user_map = HashMap::from([
            (
                "PK".to_string(),
                aws_sdk_dynamodb::types::AttributeValue::S(["USER#", &self.username].join("")),
            ),
            (
                "SK".to_string(),
                aws_sdk_dynamodb::types::AttributeValue::S(["USER#", &self.username].concat()),
            ),
            (
                "created_at".to_string(),
                aws_sdk_dynamodb::types::AttributeValue::S(date),
            ),
            (
                "password".to_string(),
                aws_sdk_dynamodb::types::AttributeValue::S(hash_password),
            ),
        ]);

        if let Some(number) = &self.phone {
            println!("{}", number);
            user_map.insert(
                "phone".to_string(),
                aws_sdk_dynamodb::types::AttributeValue::S(number.to_owned()),
            );
        }

        user_map
    }
}

pub fn user_key(username: &str) -> aws_sdk_dynamodb::types::AttributeValue {
    aws_sdk_dynamodb::types::AttributeValue::S(format!("USER#{}", username))
}

pub fn from_item(item: &HashMap<String, aws_sdk_dynamodb::types::AttributeValue>) -> User {
    let username = item.get("PK").unwrap().as_s().unwrap().to_string()[4..].to_string();
    let password = item.get("password").unwrap().as_s().unwrap().to_string();
    let phone: Option<String> = item.get("phone").map(|s| s.as_s().unwrap().to_string());
    User::new(username, password, phone)
}
