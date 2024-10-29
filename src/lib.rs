use async_trait::async_trait;
use aws_sdk_dynamodb::{types::AttributeValue, Client};
use lambda_http::{
    tracing::{self},
    Error,
};
use model::{
    session::{session_key, Session},
    user::{from_item, user_key, User},
    vehicle::Vehicle,
};

pub mod model;

#[async_trait]
pub trait DataAccess {
    async fn create_user(&self, user: User) -> Result<(), Error>;
    async fn get_session(&self, user: User) -> Result<Session, Error>;
    async fn delete_session(&self, token: &str) -> Result<String, Error>;
    async fn add_vehicle(&self, car: Vehicle) -> Result<(), Error>;
}

pub struct DBDataAccess {
    client: Client,
    table_name: String,
}

impl DBDataAccess {
    pub fn new(client: Client, table_name: String) -> Self {
        Self { client, table_name }
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
                user.varify(password)
            }
            None => false,
        }
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
            .items?
            .iter()
            .next()
            .map(|user| user.get("GSI1SK").unwrap().to_owned());

        tracing::info!("USER: {:#?}", user);

        user
    }

    async fn is_session_vaild(&self, token: &str) -> bool {
        self.get_user(token).await.is_some()
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
            .and_then(|output| {
                tracing::info!("Item Output {:#?}", output);
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

    async fn add_vehicle(&self, _car: Vehicle) -> Result<(), Error> {
        todo!()
    }
}
