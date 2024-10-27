use async_trait::async_trait;
use aws_sdk_dynamodb::Client;
use lambda_http::{
    tracing::{self, subscriber::registry::Data},
    Error,
};
use model::{session::Session, user::{from_item, user_key, User}};

pub mod model;

#[async_trait]
pub trait DataAccess {
    async fn create_user(&self, user: User) -> Result<(), Error>;
    async fn get_session(&self, user: User) -> Result<Session, Error>;
    async fn delete_session(&self, token: &str) -> Result<(), Error>;
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

    async fn delete_session(&self, token: &str) -> Result<(), Error> {
        todo!()
    }
}
