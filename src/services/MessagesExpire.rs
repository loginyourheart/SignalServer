use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use axum::async_trait;
use serde_json::Value::Array;
use tokio::sync::RwLock;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use crate::room::message::{Message, MessageType};
use crate::room::messagehandler::{MessageHandler, MessageHandlerImpl};
use crate::room::{Room, RoomManage};

// 配置trait
pub trait IConfig {
    fn cleanup_out_msgs(&self) -> u64; // 毫秒
    fn expire_timeout(&self) -> u64;   // 毫秒
}

// 自定义配置
#[derive(Clone)]
pub struct CustomConfig {
    pub cleanup_out_msgs: u64,
    pub expire_timeout: u64,
}

impl IConfig for CustomConfig {
    fn cleanup_out_msgs(&self) -> u64 {
        self.cleanup_out_msgs
    }

    fn expire_timeout(&self) -> u64 {
        self.expire_timeout
    }
}

// 消息过期接口
#[async_trait]
pub trait IMessagesExpire: Send + Sync {
    async fn start_messages_expiration(&self);
    async fn stop_messages_expiration(&self);
}

pub struct MessagesExpire {
    realm: Arc<dyn RoomManage>,
    config: CustomConfig,
    message_handler: Arc<dyn MessageHandler + Send + Sync>,
    cancellation_token: Arc<RwLock<CancellationToken>>,
    is_running: Arc<RwLock<bool>>,
}

impl MessagesExpire {
    pub fn new(
        realm: Arc<dyn RoomManage>,
        config: CustomConfig,
        message_handler: Arc<dyn MessageHandler + Send + Sync>,
    ) -> Self {
        Self {
            realm,
            config,
            message_handler,
            cancellation_token: Arc::new(RwLock::new(CancellationToken::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    async fn prune_outstanding(&self) {
        let destination_client_ids = self.realm.get_clients_ids_with_queue().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let max_diff = self.config.expire_timeout();
        let mut seen: HashMap<String, bool> = HashMap::new();

        for destination_client_id in destination_client_ids {
            let message_queue = match self.realm.get_message_queue_by_id(&destination_client_id).await {
                Some(queue) => queue,
                None => continue,
            };
            
            let mutex_queue = message_queue.lock().await;
            
            let last_read_at =  mutex_queue.get_last_read_time();
            let last_read_diff = now - last_read_at;

            if last_read_diff < max_diff {
                continue;
            }
            // 所有超时的消息
            let messages = mutex_queue.get_messages();
            for message in messages {
                let seen_key = format!(
                    "{}_{}",
                    message.src.as_deref().unwrap_or(""),
                    message.dst.as_deref().unwrap_or("")
                );

                if !seen.contains_key(&seen_key) {
                    // let expire_message = Message {
                    //     msg_type: MessageType::Expire,
                    //     src: message.dst.clone(),
                    //     dst: message.src.clone(),
                    //     payload: None,
                    // };

                    // self.message_handler.handle(None, expire_message).await;
                    seen.insert(seen_key, true);
                }
            }
            self.realm.clear_message_queue(&destination_client_id).await;
        }
    }
}

#[async_trait]
impl IMessagesExpire for MessagesExpire {
    async fn start_messages_expiration(&self) {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            // 如果已经在运行，先停止
            self.cancellation_token.read().await.cancel();
            // 等待停止完成
            while *is_running {
                drop(is_running);
                tokio::time::sleep(Duration::from_millis(10)).await;
                is_running = self.is_running.write().await;
            }
        }

        *is_running = true;
        drop(is_running);

        // 创建新的 cancellation token
        let new_token = CancellationToken::new();
        let token_for_task = new_token.clone();
        *self.cancellation_token.write().await = new_token;

        let realm = Arc::clone(&self.realm);
        let config = self.config.clone();
        let message_handler = Arc::clone(&self.message_handler);
        let cleanup_interval = Duration::from_millis(config.cleanup_out_msgs());
        let is_running = Arc::clone(&self.is_running);

        // 创建 MessagesExpire 的引用用于调用 prune_outstanding
        let self_clone = Self {
            realm,
            config,
            message_handler,
            cancellation_token: Arc::new(RwLock::new(CancellationToken::new())), // 临时的，不会被使用
            is_running: Arc::clone(&is_running),
        };

        tokio::spawn(async move {
            let mut interval = interval(cleanup_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        self_clone.prune_outstanding().await;
                    }
                    _ = token_for_task.cancelled() => {
                        break;
                    }
                }
            }

            let mut running = is_running.write().await;
            *running = false;
        });
    }

    async fn stop_messages_expiration(&self) {
        self.cancellation_token.read().await.cancel();

        // 等待任务完全停止
        while *self.is_running.read().await {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}


// 便利构造函数
impl MessagesExpire {
    pub fn builder() -> MessagesExpireBuilder {
        MessagesExpireBuilder::new()
    }
}

pub struct MessagesExpireBuilder {
    realm: Option<Arc<dyn RoomManage>>,
    config: Option<CustomConfig>,
    message_handler: Arc<dyn MessageHandler + Send + Sync>,
}

impl MessagesExpireBuilder {
    pub fn new() -> Self {
        Self {
            realm: None,
            config: None,
            message_handler: Arc::new(MessageHandlerImpl::new(Arc::new(Room::new()))),
        }
    }

    pub fn realm(mut self, realm: Arc<dyn RoomManage>) -> Self {
        self.realm = Some(realm);
        self
    }

    pub fn config(mut self, config: CustomConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn message_handler(mut self, handler: Arc<dyn MessageHandler + Send + Sync>) -> Self {
        self.message_handler = handler;
        self
    }

    pub fn build(self) -> Result<MessagesExpire, &'static str> {
        let realm = self.realm.ok_or("Realm is required")?;
        let config = self.config.ok_or("Config is required")?;
        let message_handler = self.message_handler;

        Ok(MessagesExpire::new(realm, config, message_handler))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::client::Client;
    use crate::room::messagehandler::MessageHandlerImpl;
    use crate::room::Room;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tokio::sync::Mutex;
    use crate::room::message_queue::{MessageQueue, Queue};

    // Mock MessageQueue for testing
    struct TestMessageQueue {
        messages: Vec<Message>,
        last_read_time: u64,
    }

    impl TestMessageQueue {
        fn new() -> Self {
            Self {
                messages: vec![],
                last_read_time: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            }
        }

        fn add_message(&mut self, message: Message) {
            self.messages.push(message);
        }

        fn set_last_read_time(&mut self, time: u64) {
            self.last_read_time = time;
        }
    }
    #[tokio::test]
    async fn test_messages_expire() {
        let config = CustomConfig {
            cleanup_out_msgs: 1000, // 1秒检查一次
            expire_timeout: 5000,   // 5秒过期
        };


        let realm = Arc::new(Room::new());
        let client = Arc::new(Client::new("client1".to_string (), "token1".to_string()));
        realm.set_client(client.clone(), "client1".to_string()).await;

        let message_handler = Arc::new(MessageHandlerImpl::new(realm.clone()));

        let messages_expire = MessagesExpire::new(realm, config, message_handler);

        // 启动消息过期检查
        messages_expire.start_messages_expiration().await;

        // 让它运行一段时间
        tokio::time::sleep(Duration::from_secs(3)).await;

        // 停止消息过期检查
        messages_expire.stop_messages_expiration().await;
    }
    #[tokio::test]
    async fn test_messages_expire_basic_functionality() {
        let config = CustomConfig {
            cleanup_out_msgs: 500,  // 检查间隔 500ms
            expire_timeout: 1000,    // 超时时间 1秒
        };

        let realm = Arc::new(Room::new());
        let message_handler = Arc::new(MessageHandlerImpl::new(realm.clone()));

        let messages_expire = MessagesExpire::new(
            realm.clone(),
            config,
            message_handler
        );

        // 启动消息过期检查
        messages_expire.start_messages_expiration().await;

        // 确保服务正在运行
        assert!(*messages_expire.is_running.read().await);

        // 等待一段时间
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 停止消息过期检查
        messages_expire.stop_messages_expiration().await;

        // 确保服务已停止
        assert!(!*messages_expire.is_running.read().await);
    }

    #[tokio::test]
    async fn test_messages_expire_with_timeout() {
        let config = CustomConfig {
            cleanup_out_msgs: 100,   // 100ms 检查一次
            expire_timeout: 500,     // 500ms 过期
        };

        let realm = Arc::new(Room::new());

        // 创建客户端并添加消息队列
        let client_id = "test_client_1";
        let client = Arc::new(Client::new(client_id.to_string(), "token1".to_string()));
        realm.set_client(client.clone(), client_id.to_string()).await;

        // 创建一个过期的消息
        let mut message_queue = MessageQueue::new();
        let old_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64 - 1000; // 1秒前
        

        // 添加消息到队列
        let test_message = Message {
            msg_type: MessageType::Offer,
            src: Some("client_a".to_string()),
            dst: Some(client_id.to_string()),
            payload: Some(serde_json::json!({"data": "test"})),
        };
        message_queue.add_message(test_message.clone());

        // 设置消息队列到 realm
        realm.add_message_to_queue(client_id, test_message).await;

        let message_handler = Arc::new(MessageHandlerImpl::new(realm.clone()));
        let messages_expire = MessagesExpire::new(
            realm.clone(),
            config,
            message_handler
        );

        // 确保消息队列存在
        assert!(realm.get_message_queue_by_id(client_id).await.is_some());

        // 启动过期检查
        messages_expire.start_messages_expiration().await;

        // 等待足够的时间让清理任务执行
        tokio::time::sleep(Duration::from_millis(300)).await;

        // 检查消息队列是否被清理
        let queue_after = realm.get_message_queue_by_id(client_id).await;
        if let Some(queue) = queue_after {
            let locked_queue = queue.lock().await;
            assert_eq!(locked_queue.get_messages().len(), 0, "消息队列应该被清空");
        }

        messages_expire.stop_messages_expiration().await;
    }

    #[tokio::test]
    async fn test_messages_not_expired() {
        let config = CustomConfig {
            cleanup_out_msgs: 100,   // 100ms 检查一次
            expire_timeout: 2000,    // 2秒过期
        };

        let realm = Arc::new(Room::new());
        let client_id = "test_client_2";
        let client = Arc::new(Client::new(client_id.to_string(), "token2".to_string()));
        realm.set_client(client.clone(), client_id.to_string()).await;

        // 创建一个新的消息队列（不过期）
        let mut message_queue = MessageQueue::new();
        
        // 添加消息
        let test_message = Message {
            msg_type: MessageType::Answer,
            src: Some("client_b".to_string()),
            dst: Some(client_id.to_string()),
            payload: Some(serde_json::json!({"data": "test2"})),
        };
        message_queue.add_message(test_message.clone());

        realm.add_message_to_queue(client_id, test_message).await;
        
        let message_handler = Arc::new(MessageHandlerImpl::new(realm.clone()));
        let messages_expire = MessagesExpire::new(
            realm.clone(),
            config,
            message_handler
        );

        // 启动过期检查
        messages_expire.start_messages_expiration().await;

        // 等待一段时间
        tokio::time::sleep(Duration::from_millis(300)).await;

        // 检查消息队列是否仍然存在（不应该被清理）
        let queue_after = realm.get_message_queue_by_id(client_id).await;
        assert!(queue_after.is_some());

        if let Some(queue) = queue_after {
            let locked_queue = queue.lock().await;
            assert_eq!(locked_queue.get_messages().len(), 1, "消息不应该被清空");
        }

        messages_expire.stop_messages_expiration().await;
    }

    #[tokio::test]
    async fn test_multiple_restarts() {
        let config = CustomConfig {
            cleanup_out_msgs: 100,
            expire_timeout: 1000,
        };

        let realm = Arc::new(Room::new());
        let message_handler = Arc::new(MessageHandlerImpl::new(realm.clone()));
        let messages_expire = MessagesExpire::new(
            realm.clone(),
            config,
            message_handler
        );

        // 多次启动和停止
        for i in 0..3 {
            println!("Iteration {}", i);

            messages_expire.start_messages_expiration().await;
            assert!(*messages_expire.is_running.read().await);

            tokio::time::sleep(Duration::from_millis(200)).await;

            messages_expire.stop_messages_expiration().await;
            assert!(!*messages_expire.is_running.read().await);
        }
    }

    #[tokio::test]
    async fn test_builder_pattern() {
        let realm = Arc::new(Room::new());
        let config = CustomConfig {
            cleanup_out_msgs: 1000,
            expire_timeout: 5000,
        };
        let message_handler = Arc::new(MessageHandlerImpl::new(realm.clone()));

        // 使用 builder 模式创建
        let messages_expire = MessagesExpire::builder()
            .realm(realm)
            .config(config)
            .message_handler(message_handler)
            .build()
            .expect("Failed to build MessagesExpire");

        messages_expire.start_messages_expiration().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        messages_expire.stop_messages_expiration().await;
    }

    #[tokio::test]
    async fn test_concurrent_start_stop() {
        let config = CustomConfig {
            cleanup_out_msgs: 50,
            expire_timeout: 1000,
        };

        let realm = Arc::new(Room::new());
        let message_handler = Arc::new(MessageHandlerImpl::new(realm.clone()));
        let messages_expire = Arc::new(MessagesExpire::new(
            realm.clone(),
            config,
            message_handler
        ));

        // 并发启动多个任务
        let mut handles = vec![];

        for i in 0..5 {
            let expire_clone = messages_expire.clone();
            let handle = tokio::spawn(async move {
                if i % 2 == 0 {
                    expire_clone.start_messages_expiration().await;
                } else {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    expire_clone.stop_messages_expiration().await;
                }
            });
            handles.push(handle);
        }

        // 等待所有任务完成
        for handle in handles {
            handle.await.unwrap();
        }

        // 最终停止
        messages_expire.stop_messages_expiration().await;
    }
}