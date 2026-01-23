use axum::{
    routing::get,
    Router,
    http::StatusCode,
    response::{IntoResponse, Json}
};
use serde::Serialize;

#[derive(Serialize)]
struct Response {
    data: String,
    code: u16,
}

#[tokio::main]
async fn main() {
    // app with routes and fallback
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .fallback(fallback);

    // run app with hyper, listening on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
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
