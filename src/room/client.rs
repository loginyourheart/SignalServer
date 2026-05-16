use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use axum::async_trait;
use axum::extract::ws::{Message as wsmessage, WebSocket};
use futures_util::SinkExt;
use futures_util::stream::SplitSink;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::Mutex;

// 客户端管理
#[async_trait]
pub trait ClientManage<T: Serialize = serde_json::Value>: Send + Sync {
    fn get_id(&self) -> &str;
    fn get_token(&self) -> &str;
    async fn get_socket(&self) -> Option<Arc<Mutex<SplitSink<WebSocket,wsmessage>>>>;
    async fn set_socket(&self, socket: Option<Arc<Mutex<SplitSink<WebSocket,wsmessage>>>>);
    async fn get_last_ping(&self) -> u64;
    async fn set_last_ping(&self, last_ping: u64);
    async fn send(&self, data: &T) -> Result<(), String>;
}
// 客户端实现
// 主要功能：
// 1. 管理WebSocket连接
// 2. 跟踪最后的ping时间（用于心跳检测）
// 3. 通过WebSocket发送JSON序列化的消息
#[derive(Debug)]
pub struct Client {
    id: String,
    token: String,
    socket: Arc<Mutex<Option<Arc<Mutex<SplitSink<WebSocket,wsmessage>>>>>>,
    last_ping: Arc<Mutex<u64>>,
}

impl Client {
    pub fn new(id: String, token: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            id,
            token,
            socket: Arc::new(Mutex::new(None)),
            last_ping: Arc::new(Mutex::new(now)),
        }
    }
}

#[async_trait]
impl ClientManage for Client {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_token(&self) -> &str {
        &self.token
    }

    async fn get_socket(&self) -> Option<Arc<Mutex<SplitSink<WebSocket,wsmessage>>>> {
        let socket = self.socket.lock().await;
        socket.clone()
    }

    async fn set_socket(&self, socket: Option<Arc<Mutex<SplitSink<WebSocket,wsmessage>>>>) {
        let mut current_socket = self.socket.lock().await;
        *current_socket = socket;
    }

    async fn get_last_ping(&self) -> u64 {
        let last_ping = self.last_ping.lock().await;
        *last_ping
    }

    async fn set_last_ping(&self, last_ping: u64) {
        let mut current_ping = self.last_ping.lock().await;
        *current_ping = last_ping;
    }

    async fn send(&self, data: &Value) -> Result<(), String> {
        let socket_option = self.get_socket().await;

        if let Some(socket_arc) = socket_option {
            let json = serde_json::to_string(data)
                .map_err(|e| format!("Failed to serialize data: {}", e))?;

            let mut socket = socket_arc.lock().await;
            socket.send(wsmessage::Text(json)).await
                .map_err(|e| format!("Failed to send message: {}", e))
        } else {
            Err("Socket not connected".to_string())
        }
    }
}

// Clone 实现
impl Clone for Client {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            token: self.token.clone(),
            socket: Arc::clone(&self.socket),
            last_ping: Arc::clone(&self.last_ping),
        }
    }
}