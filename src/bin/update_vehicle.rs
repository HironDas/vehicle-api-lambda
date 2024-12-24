use aws_config::BehaviorVersion;
use lambda_http::{ext::request, run, service_fn, tracing, Body, Error, Request, RequestExt, Response};
use vehicle_management_lambda::{DBDataAccess, DataAccess, UpdateVehicle};

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
        update_vehicle_handeler(&data_access, request)
    }))
    .await
}

#[tracing::instrument( skip(data_access, request), fields(request_id = request.lambda_context().request_id))]
async fn update_vehicle_handeler(data_access: &impl DataAccess, request: Request)->Result<Response<Body>, Error>{
    let token = request.headers().get("Authorization");
    if token.is_none() {
        return Ok(Response::builder()
        .status(401)
        .body("{\"message\": \"Unauthorized\"}".into())
        .unwrap());
    }

    let token = token.unwrap().to_str().unwrap();

    if let Body::Text(msg) = request.body() {
        let update_vehicle = match serde_json::from_str::<UpdateVehicle>(&msg) {
            Ok(update) => update,
            Err(err) => {
                return Ok(Response::builder()
                    .status(400)
                    .body(format!("{{'message':'{}'}}", err).into())
                    .unwrap());
            }
        };

        data_access
            .update_vehicle(token, update_vehicle)
            .await
            .and_then(|_| {
                Ok(Response::builder()
                    .status(200)
                    .body(
                        format!("{{\"message\": \"the car is updated\"}}").into(),
                    )
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
