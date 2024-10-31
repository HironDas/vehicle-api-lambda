use aws_config::BehaviorVersion;
use lambda_http::{run, service_fn, tracing, Body, Error, Request, RequestExt, Response};
use vehicle_management_lambda::{model::vehicle::Vehicle, DBDataAccess, DataAccess};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        .with_current_span(false)
        .with_ansi(false)
        .with_target(false)
        .without_time()
        .init();

    let table_name = std::env::var("TABLE_NAME").unwrap_or("VehicleDB".to_owned());
    let sdk_config = aws_config::defaults(BehaviorVersion::latest()).load().await;

    let client = aws_sdk_dynamodb::Client::new(&sdk_config);

    let data_access = DBDataAccess::new(client, table_name);

    run(service_fn(|request| {
        add_vehicle_handeler(&data_access, request)
    }))
    .await
}

#[tracing::instrument( skip(data_access, request), fields(request_id = request.lambda_context().request_id))]
async fn add_vehicle_handeler(
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

    if let Body::Text(text) = request.body() {
        let car = match serde_json::from_str::<Vehicle>(&text) {
            Ok(vehicle) => vehicle,
            Err(_) => {
                return Ok(Response::builder()
                    .status(400)
                    .body("{'message':'the body msg format is wrong'}".into())
                    .unwrap())
            }
        };
        data_access
            .add_vehicle(token, car)
            .await
            .and_then(|_| {
                Ok(Response::builder()
                    .status(201)
                    .body("{\"message\": \"new car is added\"}".into())
                    .unwrap())
            })
            .or_else(|err| {
                Ok(Response::builder()
                    .status(400)
                    .body(format!("{{\"message\":\"{}\"}}", err).into())
                    .unwrap())
            })
    } else {
        Ok(Response::builder()
            .status(403)
            .body("{\"message\": \"the message body is empty or in wrong format!!\"}".into())
            .unwrap())
    }
}
