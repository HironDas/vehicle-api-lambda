use aws_sdk_dynamodb::config::BehaviorVersion;
use lambda_http::{run, service_fn, tracing, Body, Error, Request, RequestExt, Response};
use vehicle_management_lambda::{self, model::user::User, DBDataAccess, DataAccess};

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
    let aws_config = aws_config::defaults(BehaviorVersion::v2024_03_28())
        .load()
        .await;
    let client = aws_sdk_dynamodb::Client::new(&aws_config);

    let data_access = DBDataAccess::new(client, table_name);

    run(service_fn(|req| signup(&data_access, req))).await
}

#[tracing::instrument(skip(data_access), fields(request_id = %req.lambda_context().request_id))]
async fn signup<T: DataAccess>(data_access: &T, req: Request) -> Result<Response<Body>, Error> {
    let user: User = match req.body() {
        Body::Empty => {
            return Ok(Response::builder()
                .status(400)
                .header("content-type", "application/json")
                .body("{\"message\":\"The msg body is empty\"}".into())
                .unwrap())
        }
        Body::Text(text) => match serde_json::from_str::<User>(text.as_str()) {
            Ok(user) => user,
            Err(_) => {
                return Ok(Response::builder()
                    .status(402)
                    .header("Content-Type", "Application/json")
                    .body("{'message':'the body format is wrong'}".into())
                    .unwrap())
            }
        },
        Body::Binary(_) => {
            return Ok(Response::builder()
                .status(400)
                .header("Content-Type", "Application/json")
                .body("{\"message\":\"The msg body is binary\"}".into())
                .unwrap())
        }
    };

    data_access
        .create_user(user)
        .await
        .and_then(|_| {
            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "Application/json")
                .body("{\"message\":\"Signup successful!!\"}".into())
                .unwrap())
        })
        .or_else(|_err| {
            Ok(Response::builder()
                .status(400)
                .header("Content-Type", "Application/json")
                .body(format!("{{\"message\": \"Username already exist!!\" }}").into())
                .unwrap())
        })
}
