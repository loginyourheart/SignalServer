use crate::room::client::{Client, ClientManage};
use crate::room::message::{Message, MessageType};
use crate::room::{Room, RoomManage};
use crate::AppState;
use axum::extract::ws::{CloseFrame, Message as wsmessage, WebSocket};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use crate::room::messagehandler::{MessageHandler, MessageHandlerImpl};

// 错误类型枚举
#[derive(Debug, Clone, Serialize)]
enum ErrorType {
    InvalidWsParameters,
    InvalidKey,
    ConnectionLimitExceed,
    PeerUnavailable,
}

impl ErrorType {
    fn as_str(&self) -> &'static str {
        match self {
            ErrorType::InvalidWsParameters => "Invalid WebSocket parameters",
            ErrorType::InvalidKey => "Invalid key provided",
            ErrorType::ConnectionLimitExceed => "Connection limit exceeded",
            ErrorType::PeerUnavailable => "Peer unavailable",
        }
    }
}
// 错误消息结构
#[derive(Debug, Serialize)]
struct ErrorMessage {
    #[serde(rename = "type")]
    msg_type: MessageType,
    payload: ErrorPayload,
}
#[derive(Debug, Serialize)]
struct ErrorPayload {
    msg: String,
}

// 处理 WebSocket 连接
pub async fn handle_socket(mut socket: WebSocket, state: AppState, id: String, token: String) {
    let handler = MessageHandlerImpl::new(state.room.clone());
    // split
    let (mut sender, mut receiver) = socket.split();
    let arcsender = Arc::new(Mutex::new(sender));
    let arcreceiver = Arc::new(Mutex::new(receiver));
    // 检查客户端是否已存在
    if let Some(existing_client) = state.room.get_client_by_id(&id).await {
        if existing_client.get_token() != token {
            // ID 已被占用，token 不匹配
            send_error(
                arcsender,
                ErrorMessage {
                    msg_type: MessageType::IdTaken,
                    payload: ErrorPayload {
                        msg: "ID is taken".to_string(),
                    },
                },
            )
            .await;
            return;
        }
        // 如果 token 匹配，继续处理（可能是重连）
    } else {
        // 检查并发限制
        if state.room.get_clients_ids().await.len() >= state.config.concurrent_limit {
            send_error(
                arcsender,
                ErrorMessage {
                    msg_type: MessageType::Error,
                    payload: ErrorPayload {
                        msg: ErrorType::ConnectionLimitExceed.as_str().to_string(),
                    },
                },
            )
            .await;
            return;
        }
    }

    // 注册客户端
    let client = Client::new(id.clone(), token.clone());
    client.set_socket(Some(arcsender.clone())).await;
    let arc1 = Arc::new(client);
    state.room.set_client(arc1.clone(), id.clone()).await;

    // 发送client OPEN 消息
    if let Ok(open_msg) = serde_json::to_string(&Message::new(MessageType::Open, None, None, None))
    {
        let _ = arcsender.lock().await.send(wsmessage::Text(open_msg)).await;
    }
    let (tx, mut rx) = broadcast::channel::<String>(100);

    // 克隆必要的变量
    let client_id = id.clone();
    let state_clone = state.clone();
    
    
    // 任务1：从广播通道接收消息并发送给客户端
    let mut send_task = tokio::spawn({
        async move {
            while let Ok(msg) = rx.recv().await {
                
            }
        }
    });

    // 任务2：从客户端接收消息并处理
    // 创建一个消息队列
    let (msg_tx, mut msg_rx) = mpsc::channel::<Message>(100);

    // 接收任务只负责接收和转发
    let mut recv_task = {
        let msg_tx = msg_tx.clone();
        tokio::spawn(async move {
            loop {
                let msg_result = {
                    let mut socket = arcreceiver.lock().await;
                    socket.next().await
                };

                match msg_result {
                    Some(Ok(wsmessage::Text(text))) => {
                        if let Ok(mut message) = serde_json::from_str::<Message>(&text) {
                            message.src = Some(client_id.clone());
                            handler.handle(arc1.clone(), message).await;
                        }else { 
                            // 处理解析错误
                          println!("Received invalid message: {:?}", text);
                        }
                    }
                    Some(Ok(wsmessage::Close(_))) => break,
                    Some(Err(e)) => {
                        println!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        })
    };
    

    // 等待任一任务完成
    tokio::select! {
         _ = (&mut send_task) => {
            recv_task.abort();
        }

        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }

    let arc = state
        .room
        .get_client_by_id(&id)
        .await
        .unwrap()
        .get_socket()
        .await
        .unwrap();

    if Arc::as_ptr(&arcsender) == Arc::as_ptr(&arc) {
        state.room.remove_client_by_id(&id).await;
        // let mut socket = arcsocket.lock().await;
        //     let close_frame = CloseFrame {
        //         code: 1000,
        //         reason: "Send task completed".into(),
        //     };
        // let _ = socket.send(wsmessage::Close(Some(close_frame))).await;
        // println!("Sent close frame");
        println!("Client {} removed", id);
    } else {
        println!("Socket not match");
        // 即使不匹配，也发送 Close 消息
       
    }

    // 发送错误消息
    async fn send_error(sender: Arc<Mutex<SplitSink<WebSocket,wsmessage>>>, error: ErrorMessage) {
        if let Ok(msg) = serde_json::to_string(&error) {
            let _ = sender.lock().await.send(wsmessage::Text(msg)).await;
        } else {
            println!("Failed to serialize error message");
        }
    }
}
