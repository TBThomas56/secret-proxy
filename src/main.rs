use axum::{
    routing::get,
    Router,
    http::StatusCode,
    response::{IntoResponse, Json}
};
use serde::Serialize;
use clap::Parser;

#[derive(Serialize)]
struct Response {
    data: String,
    code: u16,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, short, default_value="3000", env ="SECRET_PROXY_PORT")]
    port: u16
}

#[tokio::main]
async fn main() {
    // matching arguments passed but also allowing for a graceful shutdown
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(e) => {
            println!("Error: {}", e);
            std::process::exit(1);
        }
    };
    // app with routes and fallback
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .fallback(fallback);

    // run app with hyper, listening on port suggested by the CLI
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn fallback() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(Response {
            data: "Unidentified endpoint".to_string(),
            code: 404,
        })
    )
}

async fn root() -> impl IntoResponse {
    (
        StatusCode::ACCEPTED,
        Json(Response {
            data: "Hello, World".to_string(),
            code: 200,
        })
    )
}

async fn health() -> impl IntoResponse {
    (
        StatusCode::ACCEPTED,
        Json(Response {
            data: "OK".to_string(),
            code: 200,
        })
    )
}
