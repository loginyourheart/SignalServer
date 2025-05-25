use std::collections::HashMap;
use std::sync::Arc;
use axum::async_trait;
use axum::extract::ws::{CloseFrame, Message as wsmessage, WebSocket};
use futures_util::SinkExt;
use futures_util::stream::SplitSink;
use tokio::sync::Mutex;
use crate::room::client::ClientManage;
use crate::room::message::{Message, MessageType};
use crate::room::RoomManage;


// 处理器函数类型
pub type AsyncHandler = Arc<dyn Fn(Arc<dyn ClientManage + Send + Sync>, Message) -> futures_util::future::BoxFuture<'static, bool> + Send + Sync>;

// 处理器注册表接口
#[async_trait]
pub trait HandlersRegistry {
    fn register_handler(&mut self, message_type: MessageType, handler: AsyncHandler);
    async fn handle(&self, client: Arc<dyn ClientManage + Send + Sync>, message: Message) -> bool;
}


// 处理器注册表实现
pub struct HandlersRegistryImpl {
    handlers: HashMap<MessageType, AsyncHandler>,
}

impl HandlersRegistryImpl {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }
}

#[async_trait]
impl HandlersRegistry for HandlersRegistryImpl {
    fn register_handler(&mut self, message_type: MessageType, handler: AsyncHandler) {
        self.handlers.insert(message_type, handler);
    }

    async fn handle(&self, client: Arc<dyn ClientManage + Send + Sync>, message: Message) -> bool {
        if let Some(handler) = self.handlers.get(&message.msg_type) {
            handler(client, message).await
        } else {
            false
        }
    }
}


#[async_trait]
pub trait MessageHandler {
    async fn handle(&self, client: Arc<dyn ClientManage + Send + Sync>, message: Message) -> bool;
}
// 消息处理器实现
pub struct MessageHandlerImpl {
    handlers_registry: Box<dyn HandlersRegistry + Send + Sync>,
}

impl MessageHandlerImpl {
    pub fn new(room: Arc<dyn RoomManage + Send + Sync>) -> Self {
        let mut handlers_registry = Box::new(HandlersRegistryImpl::new());

        // 创建传输处理器
        let transmission_handler = create_transmission_handler(room.clone());

        // 创建心跳处理器
        let heartbeat_handler = create_heartbeat_handler();

        // 注册处理器
        // 注册处理器
        handlers_registry.register_handler(MessageType::Heartbeat, heartbeat_handler);
        handlers_registry.register_handler(MessageType::Offer, transmission_handler.clone());
        handlers_registry.register_handler(MessageType::Answer, transmission_handler.clone());
        handlers_registry.register_handler(MessageType::Candidate, transmission_handler.clone());
        handlers_registry.register_handler(MessageType::Leave, transmission_handler.clone());
        handlers_registry.register_handler(MessageType::Expire, transmission_handler);

        Self {
            handlers_registry,
        }
    }
}

#[async_trait]
impl MessageHandler for MessageHandlerImpl {
    async fn handle(&self, client: Arc<dyn ClientManage + Send + Sync>, message:Message) -> bool {
        self.handlers_registry.handle(client, message).await
    }
}


// 创建心跳处理器的工厂函数
fn create_heartbeat_handler() -> AsyncHandler {
    Arc::new(|_client: Arc<dyn ClientManage + Send + Sync>, _message:Message| {
        Box::pin(async move {
            // 这里实现心跳处理逻辑
            // 相当于原来的 HeartbeatHandler
            println!("Handling heartbeat from client");
            true
        })
    })
}


// 创建传输处理器的工厂函数

// 创建传输处理器的工厂函数
fn create_transmission_handler(room: Arc<dyn RoomManage + Send + Sync>) -> AsyncHandler {
    Arc::new(move |_client:  Arc<dyn ClientManage + Send + Sync>, message: Message| {
        let realm = room.clone();
        Box::pin(async move {
            handle_transmission(realm, message).await
        })
    })
}

async fn handle_transmission(realm: Arc<dyn RoomManage + Send + Sync>, message: Message) -> bool {
    let message_type = message.msg_type;
    let src_id = message.src.clone();
    let dst_id = message.dst.clone();
    if let Some(dst_id) = &dst_id {
        // 查找目标客户端
        if let Some(destination_client) = realm.get_client_by_id(dst_id).await {
            // 用户已连接！
            if let Some(socket) = destination_client.get_socket().await {
                // 尝试发送消息
                match send_message_to_socket(socket.clone(), message.clone()).await {
                    Ok(_) => {
                        println!( "Message sent to {}: {:?}", dst_id, message_type);
                    }
                    Err(_) => {
                        // 发送失败，处理断连情况
                        handle_connection_error(
                            realm.clone(),
                            destination_client.as_ref(),
                            Some(socket),
                            &src_id,
                            dst_id,
                        ).await;
                    }
                }
            } else {
                // 既没有 socket 也没有 res 可用。对等端死了？
                handle_connection_error(
                    realm.clone(),
                    destination_client.as_ref(),
                    None,
                    &src_id,
                    dst_id,
                ).await;
            }
            
        } else {
            // 等待此客户端连接/重连接（XHR）以处理重要消息
            let ignored_types = [MessageType::Leave, MessageType::Expire];

            if !ignored_types.contains(&message_type) {
                realm.add_message_to_queue(dst_id, message).await;
            } else if message_type == MessageType::Leave {
                // 这种情况在上面已经处理了（dst_id 存在的情况）
            } else {
                // 指定了不可用的目标，消息类型为 LEAVE 或 EXPIRE
                // 忽略
            }
        }
    } else {
        // dst_id 为 None 的情况
        if message_type == MessageType::Leave {
            if let Some(src_id) = &src_id {
                realm.remove_client_by_id(src_id).await;
            }
        } else {
            // 其他情况忽略
        }
    }

    true
}

async fn send_message_to_socket(
    socket: Arc<Mutex<SplitSink<WebSocket, wsmessage>>>,
    message: Message,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    
    let data = serde_json::to_string(&message)?;
    let mut socket_guard = socket.lock().await;
    socket_guard.send(wsmessage::Text(data)).await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    
}

async fn handle_connection_error(
    realm: Arc<dyn RoomManage + Send + Sync>,
    destination_client: &dyn ClientManage,
    socket: Option<Arc<Mutex<SplitSink<WebSocket, wsmessage>>>>,
    src_id: &Option<String>,
    dst_id: &str,
) {
    // 这在对等端断开连接而不关闭连接且关联的 WebSocket 未关闭时发生。
    // 告诉另一端停止尝试。
    if let Some(socket) = socket {
        let mut socket_guard = socket.lock().await;
        let _ = socket_guard.send(wsmessage::Close(None)).await;
    } else {
        realm.remove_client_by_id(&destination_client.get_id()).await;
    }

    // 发送 LEAVE 消息
    if let Some(src_id) = src_id {
        let leave_message = Message::new(
            MessageType::Leave,
            Some(src_id.clone()),
            Some(dst_id.to_string()),
            None,
        );

        // 递归调用处理 LEAVE 消息
        Box::pin(handle_transmission(realm, leave_message)).await;
    }
}
