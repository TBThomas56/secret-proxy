use axum::{
    Router, extract::State, http::StatusCode, response::{IntoResponse, Json}, routing::get
};
use serde::{Serialize, Deserialize};
use clap::Parser;
use std::sync::Arc;

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
    let shared_config = Arc::new(server_config);

    // app with routes and fallback
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/config", get(config))
        .fallback(fallback)
        .with_state(shared_config.clone());

    // run app with hyper, listening on port suggested by the CLI
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", shared_config.backend_url, shared_config.port)).await.unwrap();
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

async fn config(State(config): State<Arc<Config>>) -> impl IntoResponse {
    (
        StatusCode::ACCEPTED,
        Json(Response {
            data: format!( "backend_url:{}", config.backend_url),
            code: 200,
        })
    )
}