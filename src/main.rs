use std::any::Any;
use axum::{extract::ws::{Message, WebSocket, WebSocketUpgrade}, response::Response, routing::get, Json, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use axum::extract::{Query, State};
use axum::http::{header, Method, StatusCode, Request};
use axum::response::{Html, IntoResponse};
use serde::Deserialize;
use tokio::net::TcpListener;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::sensitive_headers::SetSensitiveHeadersLayer;
use tower_http::trace;
use clap::Parser;
use crate::config::ServerConfig;
use crate::room::{Room, RoomManage};
use crate::wshandler::handle_socket;
use axum::middleware::Next;
use axum::body::Body;
use axum_server::tls_rustls::RustlsConfig;
use tracing_subscriber::{fmt, EnvFilter, prelude::*};

pub mod room;
mod wshandler;
mod config;
mod services;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "9000")]
    port: u16,
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

async fn get_id(State(state): State<AppState>) -> Html<String> {
    let client_id = state.room.generate_client_id(None).await;
    Html(client_id)
}

async fn get_peers(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, StatusCode> {
    if state.config.allow_discovery {
        let clients_ids = state.room.get_clients_ids().await;
        Ok(Json(clients_ids))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    
    let config = match ServerConfig::load_from_file(&args.config) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };
    
    init_tracing(&config.log_level);
    
    println!("Server config:");
    println!("  key: {}", config.key);
    println!("  concurrent_limit: {}", config.concurrent_limit);
    println!("  allow_discovery: {}", config.allow_discovery);
    println!("  alive_timeout: {}ms", config.alive_timeout);
    println!("  check_interval: {}s", config.check_interval);
    println!("  TLS enabled: {}", config.tls_enabled);
    if config.tls_enabled {
        println!("  TLS cert: {}", config.tls_cert_path);
        println!("  TLS key: {}", config.tls_key_path);
    }
    println!("  Log level: {}", config.log_level);
    println!("  Debug request headers: {}", config.debug_request_headers);
    
    let state = AppState {
        room: Arc::new(Room::new()),
        config: config.clone(),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/peerjs/id", get(get_id))
        .route("/peerjs/peers", get(get_peers))
        .route("/peerjs", get(ws_handler))
        .with_state(state)
        .layer(axum::middleware::from_fn(ws_upgrade_middleware))
        .layer(
            (
                trace::TraceLayer::new_for_http()
                    .make_span_with(trace::DefaultMakeSpan::new().include_headers(config.debug_request_headers))
                    .on_request(trace::DefaultOnRequest::new().level(tracing::Level::INFO))
                    .on_response(trace::DefaultOnResponse::new().level(tracing::Level::INFO)),
                SetSensitiveHeadersLayer::new(std::iter::once(
                    header::AUTHORIZATION,
                )),
                CompressionLayer::new(),
                CorsLayer::permissive(),
                CatchPanicLayer::custom(|panic_info:Box<dyn Any + Send + 'static>| {
                    let panic_message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_info.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Unknown panic occurred".to_string()
                    };

                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Html(format!(
                            "<h1>500 - Internal Server Error</h1><p>Something went wrong!</p><p>Panic message: {}</p>",
                            panic_message,
                        )),
                    )
                        .into_response()
                }),
            )
        );
    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    
    if config.tls_enabled {
        println!("Server running on https://{}", addr);
        let tls_config = RustlsConfig::from_pem_file(
            config.tls_cert_path.clone(),
            config.tls_key_path.clone(),
        )
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to load TLS certificates: {}", e);
            std::process::exit(1);
        });
        
        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await
            .unwrap();
    } else {
        println!("Server running on http://{}", addr);
        let listener = TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}

async fn index() -> &'static str {
    "PeerJS Server is running"
}

async fn ws_upgrade_middleware(mut req: Request<Body>, next: Next) -> impl IntoResponse {
    let path = req.uri().path();
    
    if path.starts_with("/peerjs") {
        if let Some(upgrade) = req.headers().get(header::UPGRADE) {
            if upgrade.to_str().unwrap_or("").to_lowercase() == "websocket" {
                req.headers_mut().insert(
                    header::CONNECTION,
                    header::HeaderValue::from_static("upgrade")
                );
            }
        }
    }
    
    next.run(req).await
}

#[derive(Debug, Deserialize)]
struct WsQuery {
    id: Option<String>,
    token: Option<String>,
    key: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    room: Arc<Room>,
    config: ServerConfig,
}

async fn ws_handler( ws: WebSocketUpgrade, Query(query): Query<WsQuery>, State(state): State<AppState>) -> Response {
    
    let (id, token, key) = match (query.id, query.token, query.key) {
        (Some(id), Some(token), Some(key)) => (id, token, key),
        _ => {
            return Response::builder()
                .status(400)
                .body("Missing required parameters".into())
                .unwrap();
        }
    };

    if key != state.config.key {
        return Response::builder()
            .status(401)
            .body("Invalid key".into())
            .unwrap();
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state, id, token))
}

fn init_tracing(log_level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(match log_level.to_lowercase().as_str() {
            "trace" => "trace",
            "debug" => "debug",
            "info" => "info",
            "warn" => "warn",
            "error" => "error",
            _ => "info",
        }));
    
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}

