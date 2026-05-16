pub mod message;
pub mod message_queue;
pub mod client;
pub mod messagehandler;

use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::sync::Arc;
use axum::async_trait;
use tokio::sync::Mutex;
use uuid::Uuid;
use crate::room::client::ClientManage;
use crate::room::message::Message;
use crate::room::message_queue::{MessageQueue, Queue};


// Room管理接口
#[async_trait]
pub trait RoomManage: Send + Sync {
    async fn get_clients_ids(&self) -> Vec<String>;
    async fn get_client_by_id(&self, client_id: &str) -> Option<Arc<dyn ClientManage>>;
    async fn get_clients_ids_with_queue(&self) -> Vec<String>;
    async fn set_client(&self, client: Arc<dyn ClientManage>, id: String);
    async fn remove_client_by_id(&self, id: &str) -> bool;
    async  fn get_message_queue_by_id(&self, id: &str) -> Option<Arc<Mutex<Box<dyn Queue>>>>;
    async fn add_message_to_queue(&self, id: &str, message: Message);
    async fn clear_message_queue(&self, id: &str);
    // 通过自定义生成器生成客户端ID
    async fn generate_client_id(&self, custom_generator: Option<Box<dyn Fn() -> String + Send + Sync>>) -> String;
}

// Realm 实现
#[derive(Clone)]
pub struct Room {
    //  client_id -> 客户端
    clients: Arc<Mutex<HashMap<String, Arc<dyn ClientManage>>>>,
    //  client_id -> 消息队列
    message_queues: Arc<Mutex<HashMap<String, Arc<Mutex<Box<dyn Queue>>>>>>,
}

impl Room {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            message_queues: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl RoomManage for Room {
    async fn get_clients_ids(&self) -> Vec<String> {
        let clients = self.clients.lock().await;
        clients.keys().cloned().collect()
    }

    async fn get_client_by_id(&self, client_id: &str) -> Option<Arc<dyn ClientManage>> {
        let clients = self.clients.lock().await;
        let result = clients.get(client_id).cloned();
        result
    } 

    async fn get_clients_ids_with_queue(&self) -> Vec<String> {
        let queues = self.message_queues.lock().await;
        queues.keys().cloned().collect()
    }

    async  fn set_client(&self, client: Arc<dyn ClientManage>, id: String) {
        let mut clients = self.clients.lock().await;
        clients.insert(id, client);
    }

    async fn remove_client_by_id(&self, id: &str) -> bool {
        let mut clients = self.clients.lock().await;
        clients.remove(id).is_some()
    }

    async  fn get_message_queue_by_id(&self, id: &str) -> Option<Arc<Mutex<Box<dyn Queue>>>> {
        let queues = self.message_queues.lock().await;
        queues.get(id).cloned()
    }

    async fn add_message_to_queue(&self, id: &str, message: Message) {
        let mut queues = self.message_queues.lock().await;

        // 如果队列不存在，创建新队列
        if !queues.contains_key(id) {
            let new_queue: Box<dyn Queue> = Box::new(MessageQueue::new());
            queues.insert(id.to_string(), Arc::new(Mutex::new(new_queue)));
        }

        // 添加消息到队列
        if let Some(queue) = queues.get(id) {
            let mut q = queue.lock().await;
            q.add_message(message);
        }
    }

    async fn clear_message_queue(&self, id: &str) {
        let mut queues = self.message_queues.lock().await;
        queues.remove(id);
    }

    async fn generate_client_id(&self, custom_generator: Option<Box<dyn Fn() -> String + Send + Sync>>) -> String {
        // 如果不提供自定义生成器使用uuid
        let generate_id = |custom: Option<&Box<dyn Fn() -> String + Send + Sync>>| -> String {
            match custom {
                Some(gen) => gen(),
                None => Uuid::new_v4().to_string(),
            }
        };

        loop {
            let client_id = generate_id(custom_generator.as_ref());

            // 检查ID是否已存在
            let clients = self.clients.lock().await;
            if !clients.contains_key(&client_id) {
                return client_id;
            }
        }
    }
}

// 异步版本（使用 tokio）
#[cfg(feature = "async")]
pub mod async_version {
    use super::*;
    use tokio::sync::{Mutex as AsyncMutex, RwLock};
    use std::sync::Arc;

    pub struct AsyncRealm {
        clients: Arc<RwLock<HashMap<String, Arc<dyn ClientManage>>>>,
        message_queues: Arc<RwLock<HashMap<String, Arc<AsyncMutex<Box<dyn Queue>>>>>>,
    }

    impl AsyncRealm {
        pub fn new() -> Self {
            Self {
                clients: Arc::new(RwLock::new(HashMap::new())),
                message_queues: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        pub async fn get_clients_ids(&self) -> Vec<String> {
            let clients = self.clients.read().await;
            clients.keys().cloned().collect()
        }

        pub async fn get_client_by_id(&self, client_id: &str) -> Option<Arc<dyn ClientManage>> {
            let clients = self.clients.read().await;
            clients.get(client_id).cloned()
        }

        pub async fn set_client(&self, client: Arc<dyn ClientManage>, id: String) {
            let mut clients = self.clients.write().await;
            clients.insert(id, client);
        }

        pub async fn remove_client_by_id(&self, id: &str) -> bool {
            let mut clients = self.clients.write().await;
            clients.remove(id).is_some()
        }

        pub async fn add_message_to_queue(&self, id: &str, message: Message) {
            let mut queues = self.message_queues.write().await;

            if !queues.contains_key(id) {
                let new_queue: Box<dyn Queue> = Box::new(MessageQueue::default());
                queues.insert(id.to_string(), Arc::new(AsyncMutex::new(new_queue)));
            }

            if let Some(queue) = queues.get(id) {
                let mut q = queue.lock().await;
                q.add_message(message);
            }
        }

        pub async fn clear_message_queue(&self, id: &str) {
            let mut queues = self.message_queues.write().await;
            queues.remove(id);
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::room::client::Client;
//     use crate::room::message::MessageType;
//     use super::*;
// 
//     #[test]
//     fn test_realm_operations() {
//         let realm = Room::new();
// 
//         // 测试生成客户端ID
//         let id1 = realm.generate_client_id(None);
//         let id2 = realm.generate_client_id(None);
//         assert_ne!(id1, id2);
// 
//         // 测试添加客户端
//         // 测试添加客户端
//         let client = Arc::new(Client::new(
//             id1.clone(),
//             "test_token".to_string(),
//         ));
//         realm.set_client(client.clone(), id1.clone());
// 
//         // 测试获取客户端
//         assert!(realm.get_client_by_id(&id1).is_some());
//         assert!(realm.get_client_by_id(&id2).is_none());
// 
//         // 测试客户端ID列表
//         let ids = realm.get_clients_ids();
//         assert_eq!(ids.len(), 1);
//         assert!(ids.contains(&id1));
// 
//         // 测试消息队列
//         let message = Message {
//             msg_type: MessageType::Offer,
//             src: Some(id1.clone()),
//             dst: Some(id2.clone()),
//             payload: Some("SDP offer data".to_string()),
//         };
//         realm.add_message_to_queue(&id2, message);
// 
//         // 测试获取有队列的客户端ID
//         let queue_ids = realm.get_clients_ids_with_queue();
//         assert_eq!(queue_ids.len(), 1);
//         assert!(queue_ids.contains(&id2));
// 
//         // 测试删除客户端
//         assert!(realm.remove_client_by_id(&id1));
//         assert!(!realm.remove_client_by_id(&id1)); // 再次删除应该返回false
// 
//         // 测试清除消息队列
//         realm.clear_message_queue(&id2);
//         let queue_ids = realm.get_clients_ids_with_queue();
//         assert_eq!(queue_ids.len(), 0);
//     }
// 
//     #[test]
//     fn test_custom_id_generator() {
//         let realm = Room::new();
// 
//         let counter = std::cell::RefCell::new(0);
//         let custom_generator = Box::new(move || {
//             let mut count = counter.borrow_mut();
//             *count += 1;
//             format!("custom-{}", count)
//         });
// 
//         let id = realm.generate_client_id(Some(custom_generator));
//         assert!(id.starts_with("custom-"));
//     }
// }

// Cargo.toml 依赖
// [dependencies]
// uuid = { version = "1.0", features = ["v4"] }
// tokio = { version = "1", features = ["full"], optional = true }
//
// [features]
// async = ["tokio"]