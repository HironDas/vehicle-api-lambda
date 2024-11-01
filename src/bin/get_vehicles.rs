use aws_config::BehaviorVersion;
use lambda_http::{run, service_fn, tracing, Body, Error, Request, RequestExt, Response};
use vehicle_management_lambda::{DBDataAccess, DataAccess};

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

    let table_name = std::env::var("TABLE_NAME").unwrap_or("VehicleDB".to_string());
    let sdk_config = aws_config::defaults(BehaviorVersion::latest()).load().await;

    let client = aws_sdk_dynamodb::Client::new(&sdk_config);
    let data_access = DBDataAccess::new(client, table_name);

    run(service_fn(|req| get_vehicles_handler(&data_access, req))).await
}

#[tracing::instrument( skip(data_access, request), fields(request_id = request.lambda_context().request_id))]
async fn get_vehicles_handler(
    data_access: &impl DataAccess,
    request: Request,
) -> Result<Response<Body>, Error> {
    let token = request.headers().get("Authorization");
    if token.is_none() {
        return Ok(Response::builder()
            .status(401)
            .body("{\"message\": \"Unauthorized\"}".into())
            .unwrap());
    }

    let token = token.unwrap().to_str().unwrap();

    data_access
        .get_all_vehicle(token)
        .await
        .and_then(|output| {
            let vehicles = serde_json::to_string(&output).unwrap();
            Ok(Response::builder()
                .status(200)
                .body(vehicles.into())
                .unwrap())
        })
        .or_else(|err| {
            Ok(Response::builder()
                .status(400)
                .body(format!("{{\"message\": \"{}\"}}", err).into())
                .unwrap())
        })
}