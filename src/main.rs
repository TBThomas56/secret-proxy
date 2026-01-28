use axum::{
    Router,
    extract::{Path, Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Json, Response},
    routing::get,
};
use serde::{Serialize, Deserialize};
use clap::Parser;
use std::sync::Arc;
use reqwest::Client;
use tokio::signal;

#[derive(Serialize)]
struct ApiResponse {
    data: String,
    code: u16,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields, default)] // Catch typos in config and has default values
struct Config {
    backend_url: String,
    secret_token: String,
    port: u16,
    extra_values: Option<String>,
}

// Implementation of the default values - serde(default) ensures that the default values are taken from here
impl Default for Config {
    fn default() -> Self {
        Config {
            backend_url: "0.0.0.0".to_string(),
            secret_token: "my-secret-token".to_string(),
            port: 3000,
            extra_values: None,
        }
    }
}
struct AppState {
    config: Config,
    http_client: reqwest::Client,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, short, default_value="config.yaml", env ="CONFIG_PATH")]
    config: String
}

async fn shutdown_signal() {
    signal::ctrl_c()
        .await
        .expect("Failed to implement Ctrl+C  handler");
    println!("Shutdown signal received, starting graceful shutdown...");
}


#[tokio::main]
async fn main() {
    // matching arguments passed but also allowing for a graceful shutdown
    let args = match Args::try_parse() {
        Ok(args) => args, // must return args to keep it in scope
        Err(e) => {
            println!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // Deserialize config_file - current file name only as it is in header of file
    let server_config_contents = match std::fs::read_to_string(&args.config) {
        Ok(contents) => contents,
        Err(e) => {
            eprintln!("Failed to read config file '{}': {}", args.config, e);
            std::process::exit(1);
        }
    };

    let server_config: Config = match serde_yaml::from_str(&server_config_contents) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to parse file '{}': {}", {args.config}, e);
            std::process::exit(1);
        }
    };

    println!("{}", &server_config_contents);
    // Wrapped server_config which is why no reference used.
    let shared_config = Arc::new(AppState {
        config: server_config,
        http_client: Client::new(),
    });

    // app with routes and fallback
    let app = Router::new()
        .route("/health", get(health))
        .route("/config", get(config))
        .route("/{*path}", get(proxy))
        .layer(from_fn_with_state(shared_config.clone(), my_middleware))
        .with_state(shared_config.clone());

    // run app with hyper, listening on port suggested by the CLI
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", shared_config.config.port)).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    println!("Server shut down gracefully");
}

async fn health() -> impl IntoResponse {
    (
        StatusCode::ACCEPTED,
        Json(ApiResponse {
            data: "OK".to_string(),
            code: 200,
        })
    )
}

async fn config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    (
        StatusCode::ACCEPTED,
        Json(ApiResponse {
            data: format!( "backend_url:{}", state.config.backend_url),
            code: 200,
        })
    )
}

async fn proxy(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    request: Request,
) -> impl IntoResponse {
    // Read the Authorization header that middleware added
    let auth_header = request.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let mut outgoing = state.http_client
        .get(format!("{}/{}", &state.config.backend_url, &path.trim_end_matches('/')));

    if let Some(auth) = auth_header {
        outgoing = outgoing.header("Authorization", auth);
    }

    let response = outgoing.send().await;

    match response {
        Ok(res) => {
            let body = res.text().await.unwrap_or_default();
            (StatusCode::OK, body)
        }
        Err(e) => {
            (StatusCode::BAD_GATEWAY, format!("Proxy error: {}", e))
        }
    }
}

async fn my_middleware(State(state): State<Arc<AppState>>, mut request: Request, next: Next) -> Result<Response, StatusCode> {
    let bearer_token = format!("Bearer {}", state.config.secret_token);
    let header_val = HeaderValue::from_str(&bearer_token)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    request.headers_mut().insert(
        header::AUTHORIZATION,
        header_val,
    );
    Ok(next.run(request).await)
}
