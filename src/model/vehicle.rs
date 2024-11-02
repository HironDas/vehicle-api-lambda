use std::collections::HashMap;

use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{NaiveDate, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Vehicle {
    pub vehicle_no: String,
    pub owner: String,
    pub tax_date: String,
    pub fitness_date: String,
    pub insurance_date: String,
    pub route_date: String,
}

impl Vehicle {
    pub fn new(
        vehicle_no: String,
        owner: String,
        tax_date: String,
        fitness_date: String,
        insurance_date: String,
        route_date: String,
    ) -> Self {
        fn date_formatter(date: &str) -> NaiveDate {
            let date: Vec<u32> = date.split("-").map(|d| d.parse::<u32>().unwrap()).collect();
            NaiveDate::from_ymd_opt(date[0] as i32, date[1], date[2]).unwrap()
        }
        let tax_date = date_formatter(&tax_date).format("%Y-%m-%d").to_string();
        let fitness_date = date_formatter(&fitness_date).format("%Y-%m-%d").to_string();
        let insurance_date = date_formatter(&insurance_date)
            .format("%Y-%m-%d")
            .to_string();
        let route_date = date_formatter(&route_date).format("%Y-%m-%d").to_string();

        Self {
            vehicle_no,
            owner,
            tax_date,
            fitness_date,
            insurance_date,
            route_date,
        }
    }

    pub fn get_key(&self) -> AttributeValue {
        vehicle_key(self.vehicle_no.as_ref())
    }
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn to_search_item(&self) -> HashMap<String, AttributeValue> {
        let last4_digit = self
            .vehicle_no
            .char_indices()
            .rev()
            .nth(3)
            .map(|(i, _)| &self.vehicle_no[i..]);

        HashMap::from([
            ("PK".to_string(), AttributeValue::S("SEARCH".to_owned())),
            ("SK".to_string(), vehicle_search_key(&self.vehicle_no)),
            (
                "LSI1SK".to_string(),
                vehicle_search_key(last4_digit.unwrap()),
            ),
        ])
    }

    pub fn to_item(self) -> HashMap<String, AttributeValue> {
        HashMap::from([
            ("PK".to_string(), vehicle_key(&self.vehicle_no)),
            ("SK".to_string(), vehicle_key(&self.vehicle_no)),
            ("owner".to_string(), AttributeValue::S(self.owner)),
            (
                "fitness_date".to_string(),
                AttributeValue::S(self.fitness_date),
            ),
            ("tax_date".to_string(), AttributeValue::S(self.tax_date)),
            ("route_date".to_string(), AttributeValue::S(self.route_date)),
            (
                "insurance_date".to_string(),
                AttributeValue::S(self.insurance_date),
            ),
            (
                "created_at".to_string(),
                AttributeValue::S(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)),
            ),
            ("updated_at".to_string(), AttributeValue::Null(true)),
            ("Sold".to_string(), AttributeValue::Bool(false)),
            (
                "GSI2PK".to_string(),
                AttributeValue::S(format!("FEE#FITNESS")),
            ),
            ("GSI2SK".to_string(), vehicle_key(&self.vehicle_no)),
            (
                "GSI3PK".to_string(),
                AttributeValue::S(format!("FEE#INSURANCE")),
            ),
            ("GSI3SK".to_string(), vehicle_key(&self.vehicle_no)),
            (
                "GSI4PK".to_string(),
                AttributeValue::S(format!("FEE#ROUTE")),
            ),
            ("GSI4SK".to_string(), vehicle_key(&self.vehicle_no)),
            ("GSI5PK".to_string(), AttributeValue::S(format!("FEE#TAX"))),
            ("GSI5SK".to_string(), vehicle_key(&self.vehicle_no)),
            ("GSI7PK".to_string(), AttributeValue::S(format!("VEHICLE"))),
        ])
    }
}

pub fn vehicle_key(car_id: &str) -> AttributeValue {
    let car_id = &*car_id.split("-").collect::<Vec<&str>>().join("");
    AttributeValue::S(["CAR#", car_id].join(""))
}

pub fn vehicle_search_key(id: &str) -> AttributeValue {
    let id = &*id.split("-").collect::<Vec<&str>>().join("");
    AttributeValue::S(format!("SEARCH#{}", id))
}

pub fn vehicle_from_item(vehicle_itme: &HashMap<String, AttributeValue>) -> Vehicle {
    let vehicle_no = &vehicle_itme.get("SK").unwrap().as_s().unwrap()[4..];
    let vehicle_no = format!(
        "{}-{}-{}-{}",
        &vehicle_no[..3],
        &vehicle_no[3..5],
        &vehicle_no[5..7],
        &vehicle_no[7..]
    );
    let owner = vehicle_itme
        .get("owner")
        .unwrap()
        .as_s()
        .unwrap()
        .to_string();
    let tax_date = vehicle_itme
        .get("tax_date")
        .unwrap_or(&AttributeValue::S(String::from("")))
        .as_s()
        .unwrap()
        .to_string();
    let fitness_date = vehicle_itme
        .get("fitness_date")
        .unwrap_or(&AttributeValue::S(String::from("")))
        .as_s()
        .unwrap()
        .to_string();
    let route_date = vehicle_itme
        .get("route_date")
        .unwrap_or(&AttributeValue::S(String::from("")))
        .as_s()
        .unwrap()
        .to_string();
    let insurance_date = vehicle_itme
        .get("insurance_date")
        .unwrap_or(&AttributeValue::S(String::from("")))
        .as_s()
        .unwrap()
        .to_string();

    // let tax_date = NaiveDate::parse_from_str(&tax_date, "%Y-%m-%d").unwrap();

    Vehicle::new(
        vehicle_no,
        owner,
        tax_date,
        fitness_date,
        insurance_date,
        route_date,
    )
}

pub fn vehicle_repo(items: Vec<HashMap<String, AttributeValue>>) -> Vec<Vehicle> {
    items.iter().map(|item| vehicle_from_item(item)).collect()
}
