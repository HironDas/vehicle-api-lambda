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

    run(service_fn(|req| get_route_handler(&data_access, req))).await
}

#[tracing::instrument( skip(data_access, req), fields(request_id = req.lambda_context().request_id))]
async fn get_route_handler(
    data_access: &impl DataAccess,
    req: Request,
) -> Result<Response<Body>, Error> {
    let token = req.headers().get("Authorization");

    if token.is_none() {
        return Ok(Response::builder()
            .status(401)
            .body("{\"message\": \"Unauthorized\"}".into())
            .unwrap());
    }
    let token = token.unwrap().to_str().unwrap();

    let day = req
        .query_string_parameters_ref()
        .and_then(|params| params.all("days"));

    if day.is_none() {
        return Ok(Response::builder()
            .status(404)
            .body("{\"message\": \"Days is not provided\"}".into())
            .unwrap());
    }

    let day = day.unwrap()[0].trim().parse::<u32>().unwrap();

    data_access
        .get_vehicles_by_type(token, "route", day)
        .await
        .and_then(|vehicles: Vec<Vehicle>| {
            let vehicles = serde_json::to_string(&vehicles).unwrap();
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
