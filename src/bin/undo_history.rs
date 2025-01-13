use std::env;

use aws_config::{BehaviorVersion, SdkConfig};
use aws_sdk_dynamodb::Client;
use lambda_http::{run, service_fn, tracing, Body, Error, Request, RequestExt, Response};
use vehicle_management_lambda::{DBDataAccess, DataAccess, DeleteHistory};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .json()
        .without_time()
        .with_ansi(false)
        .with_current_span(false)
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .init();

    let sdk_config: SdkConfig = aws_config::defaults(BehaviorVersion::latest()).load().await;
    let client: Client = aws_sdk_dynamodb::Client::new(&sdk_config);
    let table_name = env::var("TABLE_NAME").unwrap_or("VehicleDB".to_string());

    let data_access = DBDataAccess::new(client, table_name);

    run(service_fn(|request: Request| {
        undo_history_handler(&data_access, request)
    }))
    .await
}

#[tracing::instrument( skip(data_access, request), fields(request_id = request.lambda_context().request_id))]
async fn undo_history_handler(
    data_access: &impl DataAccess,
    request: Request,
) -> Result<Response<Body>, Error> {
    if let Some(token) = request.headers().get("Authorization") {
        let token = token.to_str().unwrap();

        if let Body::Text(msg) = request.body() {
            let undo_vehicle_history = match serde_json::from_str::<DeleteHistory>(&msg) {
                Ok(value) => value,
                Err(e) => {
                    return Ok(Response::builder()
                        .status(400)
                        .body(format!("{{\"message\": \"{}\"}}", e).into())
                        .unwrap());
                }
            };

            data_access
                .undo_history(token, undo_vehicle_history)
                .await
                .and_then(|_| {
                    Ok(Response::builder()
                        .status(200)
                        .body("{\"message\": \"The transaction undo successfully!!\"}".into())
                        .unwrap())
                })
                .or_else(|err| {
                    Ok(Response::builder()
                        .status(400)
                        .body(format!("{{\"message\":\"{}\"}}", err).into())
                        .unwrap())
                })
        } else {
            return Ok(Response::builder()
                .status(404)
                .body("{\"message\":\"the body msg format is wrong\"}".into())
                .unwrap());
        }
    } else {
        return Ok(Response::builder()
            .status(401)
            .body("{\"message\": \"Unauthorized\"}".into())
            .unwrap());
    }
}
