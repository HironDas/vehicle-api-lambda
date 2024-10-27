use async_trait::async_trait;
use aws_sdk_dynamodb::Client;
use lambda_http::{
    tracing::{self, subscriber::registry::Data},
    Error,
};
use model::{session::Session, user::User};

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
        todo!()
    }

    async fn delete_session(&self, token: &str) -> Result<(), Error> {
        todo!()
    }
}
