use lambda_http::{run, service_fn, tracing, Error, Request, RequestExt, Response};

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

    todo!()
}