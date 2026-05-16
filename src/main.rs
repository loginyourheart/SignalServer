use std::any::Any;
use axum::{extract::ws::{Message, WebSocket, WebSocketUpgrade}, response::Response, routing::get, Json, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
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

pub mod room;
mod wshandler;
mod config;
mod services;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "9000")]
    port: u16,
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
    let config = ServerConfig::default();
    let state = AppState {
        room: Arc::new(Room::new()),
        config,
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/peerjs/id", get(get_id))
        .route("/peerjs/peers", get(get_peers))
        .route("/peerjs", get(ws_handler))
        .with_state(state)
        .layer(
            (
                trace::TraceLayer::new_for_http()
                    .make_span_with(trace::DefaultMakeSpan::new().include_headers(true))
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
    println!("Server running on http://{}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> &'static str {
    "PeerJS Server is running"
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
