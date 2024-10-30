use std::collections::HashMap;

use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{SecondsFormat, Utc};
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
        Self {
            vehicle_no,
            owner,
            tax_date,
            fitness_date,
            insurance_date,
            route_date,
        }
    }

    pub fn get_key(&self)->AttributeValue{
        vehicle_key(self.vehicle_no.as_ref())
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
    AttributeValue::S(["CAR#", car_id].join(""))
}

pub fn vehicle_from_item(_vehicle_itme: &HashMap<String, AttributeValue>)->Vehicle{
    todo!()
}

pub fn vehicle_repo(_items: Vec<HashMap<String, AttributeValue>>)-> Vec<Vehicle>{
    todo!()
}