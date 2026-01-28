use axum::{
    Router, 
    extract::State,
    http::StatusCode, 
    response::{IntoResponse, Json},
    routing::get,
    extract::Path
};
use serde::{Serialize, Deserialize};
use clap::Parser;
use std::sync::Arc;
use reqwest::Client;

#[derive(Serialize)]
struct Response {
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
        .with_state(shared_config.clone());

    // run app with hyper, listening on port suggested by the CLI
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", shared_config.config.port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();
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

async fn config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    (
        StatusCode::ACCEPTED,
        Json(Response {
            data: format!( "backend_url:{}", state.config.backend_url),
            code: 200,
        })
    )
}

async fn proxy(State(state): State<Arc<AppState>>, Path(path): Path<String>,) -> impl IntoResponse {
   // use the shared config that creates a new http_client
   let response = state.http_client
    .get(format!("{}/{}",&state.config.backend_url,&path.trim_end_matches('/')))
    .header("Authorization", format!("Bearer {}", state.config.secret_token))
    .send()
    .await;

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
