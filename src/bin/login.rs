use aws_config::BehaviorVersion;
use lambda_http::{run, service_fn, tracing, Body, Error, Request, RequestExt, Response};
use vehicle_management_lambda::{model::user::User, DBDataAccess, DataAccess};

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

    let table_name: String = std::env::var("TABLE_NAME").unwrap_or("VehicleDB".to_string());
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        // .endpoint_url("http://localhos:8000")
        .load()
        .await;
    let client = aws_sdk_dynamodb::Client::new(&sdk_config);

    let data_access = DBDataAccess::new(client, table_name);

    run(service_fn(|request| login(&data_access, request))).await?;

    Ok(())
}

#[tracing::instrument(fields(request_id=req.lambda_context().request_id), skip(data_access))]
async fn login(data_access: &impl DataAccess, req: Request) -> Result<Response<Body>, Error> {
    let user = match req.body() {
        Body::Binary(_) => {
            return Ok(Response::builder()
                .status(400)
                .body("{'message': 'Wrong JSON formate!!'}".into())
                .unwrap());
        }
        Body::Empty => {
            return Ok(Response::builder()
                .status(400)
                .body("{message: 'body is empty'}".into())
                .unwrap());
        }
        Body::Text(text) => match serde_json::from_str::<User>(text.as_str()) {
            Ok(user) => user,
            Err(_) => {
                return Ok(Response::builder()
                    .status(400)
                    .body("{message: 'Wrong JSON formate!!'}".into())
                    .unwrap())
            }
        },
    };

    tracing::info!("USER: {:#?}", user);

    data_access
        .get_session(user)
        .await
        .and_then(|session| {
            Ok(Response::builder()
                .header("Content-Type", "Application/json")
                .status(200)
                .body(format!("{{\"token\": \"{}\"}}", session.session_id).into())
                .unwrap())
        })
        .or_else(|err| {
            tracing::error!(err);
            Ok(Response::builder()
                .header("Content-Type", "Application/json")
                .status(400)
                .body("{\"message\": \"Something went wrong\"}".into())
                .unwrap())
        })
}
