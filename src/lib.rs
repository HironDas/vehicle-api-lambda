use std::collections::HashMap;

use async_trait::async_trait;
use aws_sdk_dynamodb::{
    types::{AttributeValue, Put, TransactWriteItem, Update},
    Client,
};
use chrono::{Duration, Local, NaiveDate, SecondsFormat, Utc};
use lambda_http::{
    tracing::{self},
    Error,
};
use model::{
    history::{history_key, history_repo, TransactionHistory},
    session::{session_key, Session},
    user::{from_item, user_key, User},
    vehicle::{vehicle_from_item, vehicle_key, vehicle_repo, vehicle_search_key, Vehicle},
};
use pwhash::bcrypt;
use serde::Deserialize;

pub mod model;

#[async_trait]
pub trait DataAccess {
    async fn create_user(&self, user: User) -> Result<(), Error>;
    async fn get_session(&self, user: User) -> Result<Session, Error>;
    async fn delete_session(&self, token: &str) -> Result<String, Error>;
    async fn change_pass(&self, token: &str, old_pass: &str, new_pass: &str) -> Result<(), Error>;
    async fn add_vehicle(&self, token: &str, car: Vehicle) -> Result<(), Error>;
    async fn get_all_vehicle(&self, token: &str) -> Result<Vec<Vehicle>, Error>;
    async fn get_vehicles_by_type(
        &self,
        token: &str,
        fee_type: &str,
        days: u32,
    ) -> Result<Vec<Vehicle>, Error>;
    async fn pay_fee(
        &self,
        token: &str,
        fee_type: &str,
        update_vehicle: UpdateVehicle,
    ) -> Result<(), Error>;
    async fn update_vehicle(&self, token: &str, update_vheicle: UpdateVehicle)
        -> Result<(), Error>;
    async fn view_history(&self, token: &str, days: u32) -> Result<Vec<TransactionHistory>, Error>;
    async fn undo_history(&self, token: &str, delete_history: DeleteHistory) -> Result<(), Error>;
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateVehicle {
    pub vehicle_no: String,
    pub tax_date: Option<String>,
    pub insurance_date: Option<String>,
    pub route_date: Option<String>,
    pub fitness_date: Option<String>,
}

struct UpdateVehicleIter<'a> {
    unpdate_vehicle: &'a UpdateVehicle,
    index: usize,
}

impl<'a> Iterator for UpdateVehicleIter<'a> {
    type Item = (String, Option<&'a String>);

    fn next(&mut self) -> Option<Self::Item> {
        let resule = match self.index {
            0 => Some((
                ":tax_date".to_string(),
                self.unpdate_vehicle.tax_date.as_ref(),
            )),
            1 => Some((
                ":insurance_date".to_string(),
                self.unpdate_vehicle.insurance_date.as_ref(),
            )),
            2 => Some((
                ":fitness_date".to_string(),
                self.unpdate_vehicle.fitness_date.as_ref(),
            )),
            3 => Some((
                ":route_date".to_string(),
                self.unpdate_vehicle.route_date.as_ref(),
            )),
            _ => None,
        };
        self.index += 1;
        resule
    }
}

impl UpdateVehicle {
    fn iter(&self) -> UpdateVehicleIter {
        UpdateVehicleIter {
            unpdate_vehicle: self,
            index: 0,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DeleteHistory {
    vehicle_id: String,
    fee_type: String,
    date: String,
}

pub struct DBDataAccess {
    client: Client,
    table_name: String,
}

impl DBDataAccess {
    pub fn new(client: Client, table_name: String) -> Self {
        Self { client, table_name }
    }

    fn date_formatter(&self, date: &str) -> NaiveDate {
        let date: Vec<u32> = date.split("-").map(|d| d.parse::<u32>().unwrap()).collect();
        NaiveDate::from_ymd_opt(date[0] as i32, date[1], date[2]).unwrap()
    }

    async fn create_session(&self, user: User) -> Result<Session, Error> {
        tracing::warn!("USER: {:?}", user);
        let session_item = Session::new().to_item(&user.username[..]);
        tracing::info!("SESSION ==> {:#?}", session_item);
        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(session_item.clone()))
            // .return_values(aws_sdk_dynamodb::types::ReturnValue::AllNew)
            .send()
            .await
            .and_then(|output| {
                tracing::info!("OUTPUT: {:#?}", output);
                let item = output.attributes;
                tracing::info!("ITEM: {:?}", item);
                Ok(Session {
                    session_id: session_item.get("SK").unwrap().as_s().unwrap().as_str()[8..]
                        .to_string(),
                    created_at: session_item
                        .get("created_at")
                        .unwrap()
                        .as_s()
                        .unwrap()
                        .to_string(),
                    expired_at: session_item
                        .get("expired_at")
                        .unwrap()
                        .as_s()
                        .unwrap()
                        .to_string(),
                })
            })
            .or_else(|err| Err(err.into()))
    }
    async fn is_login_successful(&self, username: &str, password: &str) -> bool {
        let item = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", user_key(username))
            .key("SK", user_key(username))
            .send()
            .await
            .unwrap()
            .item;

        match item {
            Some(item) => {
                let user: User = from_item(&item);
                tracing::info!("*****User Info****{:?}", user);
                user.varify(password)
            }
            None => false,
        }
    }

    async fn get_fees_info(&self, index_type: &str, days: u32) -> Result<Vec<Vehicle>, Error> {
        let query = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name("GSI2")
            .key_condition_expression("#feesPK = :feesPK")
            .expression_attribute_names("#feesPK", "GSI2PK")
            .expression_attribute_values(":feesPK", AttributeValue::S(format!("VEHICLE")));

        let vehicle_items = match days {
            0 => query
                .filter_expression("#date < :date")
                .expression_attribute_names("#date", format!("{}_date", index_type.to_lowercase()))
                .expression_attribute_values(
                    ":date",
                    AttributeValue::S(Local::now().format("%Y-%m-%d").to_string()),
                )
                .send()
                .await
                .unwrap()
                .items
                .unwrap(),
            _ => query
                .filter_expression("#date between :sdate and :edate")
                .expression_attribute_names("#date", format!("{}_date", index_type.to_lowercase()))
                .expression_attribute_values(
                    ":sdate",
                    AttributeValue::S(Local::now().format("%Y-%m-%d").to_string()),
                )
                .expression_attribute_values(
                    ":edate",
                    AttributeValue::S(
                        (Local::now() + Duration::days(days as i64))
                            .format("%Y-%m-%d")
                            .to_string(),
                    ),
                )
                .send()
                .await
                .unwrap()
                .items
                .unwrap(),
        };

        Ok(vehicle_repo(vehicle_items))
    }

    async fn get_user(&self, token: &str) -> Option<AttributeValue> {
        let user = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name("GSI1")
            .key_condition_expression("#session_id = :token")
            .expression_attribute_names("#session_id", "GSI1PK")
            .expression_attribute_values(":token", session_key(token))
            .send()
            .await
            .unwrap()
            .items?;

        tracing::info!("USER: {:#?}", user);

        let user = user
            .iter()
            .next()
            .map(|user| user.get("GSI1SK").unwrap().to_owned());

        user
    }

    async fn is_session_vaild(&self, token: &str) -> bool {
        self.get_user(token).await.is_some()
    }

    async fn update_vehicle(&self, vehicle: &UpdateVehicle) -> Result<TransactWriteItem, &str> {

        let isVehicle = self.client.get_item().table_name(&self.table_name)
        .key("PK", AttributeValue::S("SEARCH".to_string()))
        .key("SK", vehicle_search_key(&vehicle.vehicle_no)).send().await.unwrap().item.is_some();
    
        let expression: String = vehicle
            .iter()
            .map(|(fee, date)| {
                if date.is_none() {
                    "".to_string()
                } else {
                    format!("{} = {}", fee.replace(":", ""), fee)
                }
            })
            .filter(|value| value != "")
            .collect::<Vec<String>>()
            .join(", ");

        if expression.trim().is_empty() {
            return Err("No updated fee date is provided");
        }
        let expression = format!("SET {}, updated_at = :updated_at", expression);

        let mut expression_attribute_values = vehicle
            .iter()
            .filter(|(_fee, date)| date.is_some())
            .map(|(fee, date)| {
                (
                    fee,
                    AttributeValue::S(
                        self.date_formatter(date.unwrap())
                            .format("%Y-%m-%d")
                            .to_string(),
                    ),
                )
            })
            .collect::<HashMap<String, AttributeValue>>();

        expression_attribute_values.insert(
            String::from(":updated_at"),
            AttributeValue::S(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)),
        );

        let update = Update::builder()
            .table_name(&self.table_name)
            .key("PK", vehicle_key(&vehicle.vehicle_no))
            .key("SK", vehicle_key(&vehicle.vehicle_no))
            .update_expression(expression)
            .set_expression_attribute_values(Some(expression_attribute_values))
            .build()
            .unwrap();

        Ok(TransactWriteItem::builder().update(update).build())
    }
    async fn add_history(&self, transaction_history: TransactionHistory) -> TransactWriteItem {
        let put_transaction = Put::builder()
            .table_name(&self.table_name)
            .set_item(Some(transaction_history.to_item()))
            .build()
            .unwrap();
        TransactWriteItem::builder().put(put_transaction).build()
    }
}

#[async_trait]
impl DataAccess for DBDataAccess {
    async fn create_user(&self, user: User) -> Result<(), Error> {
        tracing::warn!("User:====>{:#?}", user);
        tracing::info!("Table Name: {}", &self.table_name);
        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(user.to_item()))
            .condition_expression("attribute_not_exists(PK) and attribute_not_exists(SK)")
            .send()
            .await
            .and_then(|_output| {
                // tracing::info!("Item Output {:#?}", output);
                Ok(())
            })
            .or_else(|err| {
                tracing::error!("User create Fail Error: {:#?}", err);
                Err(err.into())
            })
    }

    async fn get_session(&self, user: User) -> Result<Session, Error> {
        if self
            .is_login_successful(&user.username, &user.password)
            .await
        {
            self.create_session(user).await
        } else {
            Err("{\"message\": \"Login fail!!\"}".into())
        }
    }

    async fn delete_session(&self, session_id: &str) -> Result<String, Error> {
        let user = self
            .get_user(session_id)
            .await
            .ok_or(0)
            .map_err(|_| "Session Expired!!")?;

        let sessions = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("#username = :username and begins_with(#session_id, :token)")
            .expression_attribute_names("#username", "PK")
            .expression_attribute_names("#session_id", "SK")
            .expression_attribute_values(":username", user.clone())
            .expression_attribute_values(":token", AttributeValue::S("SESSION#".to_string()))
            .send()
            .await
            .unwrap()
            .items
            .unwrap()
            .into_iter()
            .map(|item| item.get("SK").unwrap().to_owned())
            .collect::<Vec<AttributeValue>>();

        for session in sessions {
            self.client
                .delete_item()
                .table_name(&self.table_name)
                .key("PK", user.clone())
                .key("SK", session)
                .send()
                .await
                .and_then(|output| {
                    tracing::info!("Item Output: {:#?}", output);
                    Ok(())
                })
                .or_else(|err| {
                    tracing::error!("{:#?}", err);
                    Err::<(), Error>(err.into())
                })?;
        }
        let user = user.as_s().unwrap()[5..].to_string();
        Ok(user)
    }

    async fn change_pass(&self, token: &str, old_pass: &str, new_pass: &str) -> Result<(), Error> {
        let user = self
            .get_user(token)
            .await
            .ok_or("Session Expired!! login Again.")?;

        let user = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", user.clone())
            .key("SK", user)
            .send()
            .await
            .unwrap()
            .item
            .map(|output| from_item(&output))
            .unwrap();

        if user.varify(old_pass) {
            let pass = bcrypt::hash(new_pass).unwrap();
            self.client
                .update_item()
                .table_name(&self.table_name)
                .key("PK", user.get_key())
                .key("SK", user.get_key())
                .update_expression("SET password = :password")
                .expression_attribute_values(":password", AttributeValue::S(pass))
                // .return_values(aws_sdk_dynamodb::types::ReturnValue::UpdatedNew)
                .send()
                .await
                .and_then(|_output| {
                    // tracing::info!("updated user: {:#?}", output.attributes);
                    Ok(())
                })
                .or_else(|err| Err::<(), Error>(err.into()))
        } else {
            Err("Password is not valid!!!".into())
        }
    }

    async fn add_vehicle(&self, token: &str, car: Vehicle) -> Result<(), Error> {
        if self.is_session_vaild(token).await {
            let put_search = Put::builder()
                .table_name(&self.table_name)
                .set_item(Some(car.to_search_item()))
                .condition_expression("attribute_not_exists(PK) and attribute_not_exists(SK)")
                .build()
                .unwrap();

            let add_search = TransactWriteItem::builder().put(put_search).build();

            let put_vehicle = Put::builder()
                .table_name(&self.table_name)
                .set_item(Some(car.to_item()))
                .condition_expression("attribute_not_exists(PK) and attribute_not_exists(SK)")
                .build()
                .unwrap();

            let add_vehicle = TransactWriteItem::builder().put(put_vehicle).build();

            self.client
                .transact_write_items()
                .transact_items(add_vehicle)
                .transact_items(add_search)
                .send()
                .await
                .and_then(|output| {
                    tracing::info!("New Vehicle Details:  {:#?}", output);
                    Ok(())
                })
                .or_else(|err| {
                    tracing::error!(%err, "Error Message");
                    Err(err.into())
                })
        } else {
            Err("You don't have access!!".into())
        }
    }

    async fn get_all_vehicle(&self, token: &str) -> Result<Vec<Vehicle>, Error> {
        if self.is_session_vaild(token).await {
            let vehicle_items = self
                .client
                .query()
                .table_name(&self.table_name)
                .index_name("GSI2")
                .key_condition_expression("#vehicle = :vehicle_key")
                .expression_attribute_names("#vehicle", "GSI2PK")
                .expression_attribute_values(
                    ":vehicle_key",
                    AttributeValue::S("VEHICLE".to_string()),
                )
                .send()
                .await
                .unwrap()
                .items
                .unwrap();

            Ok(vehicle_repo(vehicle_items))
        } else {
            Err("You don't have access!!".into())
        }
    }

    async fn get_vehicles_by_type(
        &self,
        token: &str,
        fee_type: &str,
        days: u32,
    ) -> Result<Vec<Vehicle>, Error> {
        if self.is_session_vaild(token).await {
            match fee_type {
                "fitness" => self.get_fees_info("fitness", days).await,
                "insurance" => self.get_fees_info("insurance", days).await,
                "route" => self.get_fees_info("route", days).await,
                _ => self.get_fees_info("tax", days).await,
            }
        } else {
            Err("You don't have access!!".into())
        }
    }

    async fn pay_fee(
        &self,
        token: &str,
        fee_type: &str,
        update_vehicle: UpdateVehicle,
    ) -> Result<(), Error> {
        let user = self
            .get_user(token)
            .await
            .ok_or("You don't have valid access!!")?;

        let old_vhicle = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", vehicle_key(&update_vehicle.vehicle_no))
            .key("SK", vehicle_key(&update_vehicle.vehicle_no))
            .send()
            .await
            .unwrap()
            .item
            .unwrap();

        let update_vehicle_write_item = self.update_vehicle(&update_vehicle).await?;

        // let date = update_vehicle
        //     .iter()
        //     .find(|value| value.0 == format!("{}_date", fee_type))
        //     .unwrap()
        //     .1
        //     .unwrap()
        //     .to_string();

        let exp_date = old_vhicle
            .get(&*[fee_type, "_date"].join(""))
            .unwrap()
            .as_s()
            .unwrap()
            .to_string();

        let transaction_history = TransactionHistory::new(
            update_vehicle.vehicle_no,
            exp_date,
            fee_type.to_string(),
            user.as_s().unwrap()[5..].to_string(),
        );

        let transaction_history_write_item = self.add_history(transaction_history).await;

        self.client
            .transact_write_items()
            .transact_items(transaction_history_write_item)
            .transact_items(update_vehicle_write_item)
            .send()
            .await
            .and_then(|output| {
                tracing::info!(
                    "Vehicle {} updated and transaction is added:  {:#?}",
                    fee_type,
                    output
                );
                Ok(())
            })
            .or_else(|err| {
                tracing::error!(%err, "Error Message");
                Err(err.into())
            })
    }

    async fn update_vehicle(
        &self,
        token: &str,
        update_vehicle: UpdateVehicle,
    ) -> Result<(), Error> {
        if self.is_session_vaild(token).await {
            self.client
                .transact_write_items()
                .transact_items(self.update_vehicle(&update_vehicle).await?)
                .send()
                .await
                .and_then(|_output| Ok(()))
                .or_else(|err| {
                    tracing::error!(%err, "Error Message");
                    Err(err.into())
                })
        } else {
            Err("You don't have valid access!!".into())
        }
    }
    async fn view_history(&self, token: &str, days: u32) -> Result<Vec<TransactionHistory>, Error> {
        if self.is_session_vaild(token).await {
            let historys = self
                .client
                .query()
                .table_name(&self.table_name)
                .index_name("GSI3")
                .key_condition_expression("GSI3PK = :pk AND GSI3SK between :sdate and :edate")
                .set_expression_attribute_values(Some(HashMap::from([
                    (":pk".to_string(), AttributeValue::S("HISTORY".to_string())),
                    (
                        ":edate".to_string(),
                        history_key(&Local::now().format("%Y-%m-%d").to_string()),
                    ),
                    (
                        ":sdate".to_string(),
                        history_key(
                            &(Local::now() - Duration::days(days as i64))
                                .format("%Y-%m-%d")
                                .to_string(),
                        ),
                    ),
                ])))
                .send()
                .await
                .unwrap()
                .items
                .unwrap()
                .into_iter()
                .rev()
                .collect();

            Ok(history_repo(historys))
        } else {
            Err("Your Session is invalid!!".into())
        }
        // todo!()
    }
    async fn undo_history(&self, token: &str, delete_history: DeleteHistory) -> Result<(), Error> {
        if self.is_session_vaild(token).await {
            let history = self
                .client
                .get_item()
                .table_name(&self.table_name)
                .set_key(Some(HashMap::from([
                    ("PK".to_string(), vehicle_key(&delete_history.vehicle_id)),
                    (
                        "SK".to_string(),
                        AttributeValue::S(format!(
                            "TRANSACTION#{}#{}",
                            delete_history.fee_type, delete_history.date
                        )),
                    ),
                ]))).send().await;
        } else {
            return Err("Your Session is invalid!!".into());
        }
        todo!()
    }
}
