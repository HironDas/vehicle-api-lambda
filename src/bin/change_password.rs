use aws_config::BehaviorVersion;
use lambda_http::{run, service_fn, tracing, Body, Error, Request, RequestExt, Response};
use vehicle_management_lambda::{DBDataAccess, DataAccess};

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

    let table_name = std::env::var("TABLE_NAME").unwrap_or("VehicleDB".to_string());
    let sdk_config = aws_config::defaults(BehaviorVersion::latest()).load().await;
    let client = aws_sdk_dynamodb::Client::new(&sdk_config);
    let db_access = DBDataAccess::new(client, table_name);

    run(service_fn(|req| change_pass_handeler(&db_access, req))).await
}

#[tracing::instrument( skip(data_access, request), fields(request_id = request.lambda_context().request_id))]
async fn change_pass_handeler(
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

    let passmsg = match request.body() {
        Body::Empty => {
            return Ok(Response::builder()
                .status(400)
                .body("{\"message\":\"The msg body is empty\"}".into())
                .unwrap())
        }
        Body::Text(text) => match serde_json::from_str::<ChangePass>(text.as_str()) {
            Ok(user) => user,
            Err(_) => {
                return Ok(Response::builder()
                    .status(403)
                    .body("{\"message\":\"the body format is wrong\"}".into())
                    .unwrap())
            }
        },
        Body::Binary(_) => {
            return Ok(Response::builder()
                .status(400)
                .body("{\"message\":\"The msg body is binary\"}".into())
                .unwrap())
        }
    };

    data_access
        .change_pass(
            token,
            passmsg.old_password.as_ref(),
            passmsg.new_password.as_ref(),
        )
        .await
        .and_then(|_| {
            Ok(Response::builder()
                .status(200)
                .body("{\"message\":\"Password Changed!!\"}".into())
                .unwrap())
        })
        .or_else(|err| {
            Ok(Response::builder()
                .status(400)
                .body(format!("{{\"message\":\"{}\"}}", err).into())
                .unwrap())
        })
}

#[derive(Debug, serde::Deserialize)]
struct ChangePass {
    old_password: String,
    new_password: String,
}
