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
use crate::config::ServerConfig;
use crate::room::{Room, RoomManage};
use crate::wshandler::handle_socket;

pub mod room;
mod wshandler;
mod config;
mod services;

// GET /id 路由处理器
async fn get_id(State(state): State<AppState>) -> Html<String> {
    let client_id = state.room.generate_client_id(None).await;
    Html(client_id)
}

// GET /peers 路由处理器
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
    let config = ServerConfig::default();
    let state = AppState {
        room: Arc::new(Room::new()),
        config,
    };

    // 创建路由
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
                    // 设置request和response的日志级别
                    //on_request,on_failure(请求失败的日志),on_response(请求处理完成的日志)
                    .on_request(trace::DefaultOnRequest::new().level(tracing::Level::INFO))
                    .on_response(trace::DefaultOnResponse::new().level(tracing::Level::INFO)),
                //  标记Authorization 请求头为敏感头，因此在日志中不会记录该头的值
                SetSensitiveHeadersLayer::new(std::iter::once(
                    header::AUTHORIZATION,
                )),
                CompressionLayer::new(),// 压缩响应数据，以减少带宽使用
                CorsLayer::permissive(),// 配置CORS
                CatchPanicLayer::custom(|panic_info:Box<dyn Any + Send + 'static>| {
                    // 提取 panic 的信息
                    let panic_message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_info.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Unknown panic occurred".to_string()
                    };

                    // 返回带有 panic 信息的自定义错误页面
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
    // 绑定地址
    let addr = SocketAddr::from(([0, 0, 0, 0], 9000));
    println!("Server running on http://{}", addr);

    // 启动服务器
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// 主页
async fn index() -> &'static str {
    "PeerJS Server is running"
}

#[derive(Debug, Deserialize)]
struct WsQuery {
    id: Option<String>,
    token: Option<String>,
    key: Option<String>,
}
// 应用状态
#[derive(Clone)]
pub struct AppState {
    room: Arc<Room>,
    config: ServerConfig,
}

// WebSocket处理器
async fn ws_handler( ws: WebSocketUpgrade, Query(query): Query<WsQuery>, State(state): State<AppState>) -> Response {
    
    // 验证参数
    let (id, token, key) = match (query.id, query.token, query.key) {
        (Some(id), Some(token), Some(key)) => (id, token, key),
        _ => {
            return Response::builder()
                .status(400)
                .body("Missing required parameters".into())
                .unwrap();
        }
    };

    // 验证 key
    if key != state.config.key {
        return Response::builder()
            .status(401)
            .body("Invalid key".into())
            .unwrap();
    }

    // 升级HTTP连接到WebSocket
    ws.on_upgrade(move |socket| handle_socket(socket, state, id, token))
}

// 处理WebSocket连接
// async fn handle_socket(mut socket: WebSocket) {
//     println!("New WebSocket connection established");
// 
//     // 发送欢迎消息
//     if let Err(e) = socket.send(Message::Text("Welcome to Axum WebSocket!".to_string())).await {
//         println!("Error sending welcome message: {}", e);
//         return;
//     }
// 
//     // 处理消息循环
//     while let Some(msg) = socket.recv().await {
//         match msg {
//             Ok(Message::Text(text)) => {
//                 println!("Received text message: {}", text);
// 
//                 // 回显消息（可以在这里添加自定义逻辑）
//                 let response = format!("Echo: {}", text);
//                 if let Err(e) = socket.send(Message::Text(response)).await {
//                     println!("Error sending message: {}", e);
//                     break;
//                 }
//             }
//             Ok(Message::Binary(data)) => {
//                 println!("Received binary message: {} bytes", data.len());
// 
//                 // 回显二进制数据
//                 if let Err(e) = socket.send(Message::Binary(data)).await {
//                     println!("Error sending binary message: {}", e);
//                     break;
//                 }
//             }
//             Ok(Message::Ping(data)) => {
//                 // Pong会自动发送
//                 println!("Received ping: {:?}", data);
//             }
//             Ok(Message::Pong(data)) => {
//                 println!("Received pong: {:?}", data);
//             }
//             Ok(Message::Close(frame)) => {
//                 println!("Client disconnected: {:?}", frame);
//                 break;
//             }
//             Err(e) => {
//                 println!("WebSocket error: {}", e);
//                 break;
//             }
//         }
//     }
// 
//     println!("WebSocket connection closed");
// }