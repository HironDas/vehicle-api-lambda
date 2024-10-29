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
}
