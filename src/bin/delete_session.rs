use aws_config::BehaviorVersion;
use lambda_http::{run, service_fn, tracing, Body, Error, Request, RequestExt, Response};
use vehicle_management_lambda::{DBDataAccess, DataAccess};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        // .json()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let table_name: String = std::env::var("TABLE_NAME").unwrap_or("VehicleDB".to_string());
    let sdk_config = aws_config::defaults(BehaviorVersion::v2024_03_28())
        // .endpoint_url("http://localhos:8000")
        .load()
        .await;
    let client = aws_sdk_dynamodb::Client::new(&sdk_config);
    let data_access = DBDataAccess::new(client, table_name);

    run(service_fn(|request| delete_session(&data_access, request))).await?;

    Ok(())
}

#[tracing::instrument( skip(data_access, req), fields(request_id = req.lambda_context().request_id))]
async fn delete_session<T: DataAccess>(
    data_access: &T,
    req: Request,
) -> Result<Response<Body>, Error> {
    let token = req.headers().get("Authorization");

    if token.is_none() {
        return Ok(Response::builder()
            .status(401)
            .body("Unauthorized".into())
            .unwrap());
    }

    let token = token.unwrap().to_str().unwrap();
    tracing::info!({%token}, "this is the Token");

    data_access
        .delete_session(token)
        .await
        .and_then(|_| {
            return Ok(Response::builder()
                .status(200)
                .body("{\"message\": \"All Sessions of the user is deleted\"}".into())
                .unwrap());
        })
        .or_else(|err| {
            tracing::error!("ERROR: {:#?}", err);
            Ok(Response::builder()
                .status(500)
                .body("{\"message\":\"Something Went Wrong!!\"}".into())
                // .body(err.to_string().into())
                .unwrap())
        })
}
