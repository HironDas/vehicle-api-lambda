use std::collections::HashMap;

use aws_sdk_dynamodb::types::AttributeValue;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::vehicle::vehicle_key;

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionHistory {
    pub vehicle_no: String,
    pub date: String,
    pub transaction_type: String,
    pub payer: String,
}

impl TransactionHistory {
    pub fn new(vehicle_no: String, date: String, transaction_type: String, payer: String) -> Self {
        fn date_formatter(date: &str) -> NaiveDate {
            let date: Vec<u32> = date.split("-").map(|d| d.parse::<u32>().unwrap()).collect();
            NaiveDate::from_ymd_opt(date[0] as i32, date[1], date[2]).unwrap()
        }
        let date = date_formatter(&date).format("%Y-%m-%d").to_string();
        Self {
            vehicle_no,
            date,
            transaction_type,
            payer,
        }
    }

    pub fn get_key(&self) -> AttributeValue {
        history_key(&self.date)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn to_item(&self) -> HashMap<String, AttributeValue> {
        HashMap::from([
            ("PK".to_string(), vehicle_key(&self.vehicle_no)),
            (
                "SK".to_string(),
                AttributeValue::S(format!(
                    "TRANSACTION#{}#{}",
                    self.date, self.transaction_type
                )),
            ),
            (
                "payer".to_string(),
                AttributeValue::S(format!("{}", self.payer)),
            ),
            (
                "GSI3PK".to_string(),
                AttributeValue::S(String::from("HISTORY")),
            ),
            ("GSI3SK".to_string(), self.get_key()),
        ])
    }
}

pub fn history_key(transaction_date: &str) -> AttributeValue {
    AttributeValue::S(format!("TRANSACTION#{}", transaction_date))
}

pub fn history_from_item(history_item: &HashMap<String, AttributeValue>) -> TransactionHistory {
    let sk = history_item
        .get("SK")
        .unwrap()
        .as_s()
        .unwrap()
        .split("#")
        .collect::<Vec<&str>>();
    let vehicle_no = &history_item.get("PK").unwrap().as_s().unwrap()[4..];
    let is_number = (&vehicle_no[5..6]).chars().next().unwrap().is_numeric();
    let vehicle_no = format!(
        "{}-{}-{}-{}",
        &vehicle_no[..3],
        if is_number {
            &vehicle_no[3..5]
        } else {
            &vehicle_no[3..6]
        },
        if is_number {
            &vehicle_no[5..7]
        } else {
            &vehicle_no[6..8]
        },
        if is_number {
            &vehicle_no[7..]
        } else {
            &vehicle_no[8..]
        }
    );
    TransactionHistory {
        vehicle_no,
        date: sk[1].to_string(),
        transaction_type: sk[2].to_string(),
        payer: history_item
            .get("payer")
            .unwrap()
            .as_s()
            .unwrap()
            .to_string(),
    }
}

pub fn history_repo(items: Vec<HashMap<String, AttributeValue>>) -> Vec<TransactionHistory> {
    items
        .iter()
        .map(|transaciton_history| history_from_item(transaciton_history))
        .collect()
}
