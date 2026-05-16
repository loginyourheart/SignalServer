use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use axum::async_trait;
use futures_util::SinkExt;
use tokio::sync::RwLock;
use tokio::time::{interval, Interval};
use crate::room::RoomManage;
use tokio_util::sync::CancellationToken;
use crate::room::client::ClientManage;

const DEFAULT_CHECK_INTERVAL: u64 = 300; // 300秒

// 配置trait
pub trait IConfig {
    fn alive_timeout(&self) -> u64;
}

// 自定义配置，只包含需要的字段
#[derive(Clone)]
pub struct CustomConfig {
    pub alive_timeout: u64,
}

impl IConfig for CustomConfig {
    fn alive_timeout(&self) -> u64 {
        self.alive_timeout
    }
}


// Socket trait
#[async_trait]
pub trait ISocket: Send + Sync {
    async fn close(&self) -> Result<(), Box<dyn std::error::Error>>;
}

// 回调函数类型
pub type OnCloseCallback = Arc<dyn Fn(Arc<dyn ClientManage>) + Send + Sync>;

pub struct CheckBrokenConnections {
    pub check_interval: Duration,
    realm: Arc<dyn RoomManage>,
    config: CustomConfig,
    on_close: Option<OnCloseCallback>,
    cancellation_token: CancellationToken,
    is_running: Arc<RwLock<bool>>,
}

impl CheckBrokenConnections {
    pub fn new(
        realm: Arc<dyn RoomManage>,
        config: CustomConfig,
        check_interval: Option<u64>,
        on_close: Option<OnCloseCallback>,
    ) -> Self {
        let check_interval = Duration::from_secs(check_interval.unwrap_or(DEFAULT_CHECK_INTERVAL));

        Self {
            check_interval,
            realm,
            config,
            on_close,
            cancellation_token: CancellationToken::new(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            // 如果已经在运行，先停止
            self.cancellation_token.cancel();
            // 等待停止完成
            while *is_running {
                drop(is_running);
                tokio::time::sleep(Duration::from_millis(10)).await;
                is_running = self.is_running.write().await;
            }
        }

        *is_running = true;
        drop(is_running);

        let realm = Arc::clone(&self.realm);
        let config = self.config.clone();
        let on_close = self.on_close.as_ref().map(|f| f as *const _);
        let check_interval = self.check_interval;
        let cancellation_token = self.cancellation_token.clone();
        let is_running = Arc::clone(&self.is_running);

        let on_close = self.on_close.clone();


        tokio::spawn(async move {
            let mut interval = interval(check_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        Self::check_connections_impl(
                            &realm,
                            &config,
                            on_close.as_ref()
                        ).await;
                    }
                    _ = cancellation_token.cancelled() => {
                        break;
                    }
                }
            }

            let mut running = is_running.write().await;
            *running = false;
        });
    }

    pub async fn stop(&self) {
        self.cancellation_token.cancel();

        // 等待任务完全停止
        while *self.is_running.read().await {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn check_connections_impl(
        realm: &Arc<dyn RoomManage>,
        config: &CustomConfig,
        on_close: Option<&OnCloseCallback>,
    ) {
        let client_ids = realm.get_clients_ids().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let alive_timeout = config.alive_timeout();

        for client_id in client_ids {
            let client = match realm.get_client_by_id(&client_id).await {
                Some(client) => client,
                None => continue,
            };

            let last_ping = client.get_last_ping().await;
            let time_since_last_ping = now - last_ping;

            if time_since_last_ping < alive_timeout {
                continue;
            }

            // todo关闭socket并清理资源

            // 清理资源
            realm.clear_message_queue(&client_id).await;
            realm.remove_client_by_id(&client_id).await;
            client.set_socket(None).await;

            // 调用回调函数
            if let Some(callback) = on_close {
                callback(client);
            }
        }
    }
}

// 使用示例
#[cfg(test)]
mod tests {
    use axum::async_trait;
    use serde_json::Value;
    use crate::room::client::{Client, ClientManage};
    use crate::room::Room;
    use super::*;
    

    #[tokio::test]
    async fn test_check_broken_connections() {
        let config = CustomConfig {
            alive_timeout: 1000, // 1秒
        };

        let realm = Arc::new(Room::new());
        let client = Arc::new(Client::new("client1".to_string (), "token1".to_string()));
        realm.set_client(client.clone(), "client1".to_string()).await;
        let on_close = Some(Arc::new(|client: Arc<dyn ClientManage>| {
            println!("Client {} closed", client.get_id());
        }) as OnCloseCallback);

        let checker = CheckBrokenConnections::new(
            realm,
            config,
            Some(1), // 每1秒检查一次
            on_close,
        );

        checker.start().await;

        // 让它运行一段时间
        tokio::time::sleep(Duration::from_secs(3)).await;

        checker.stop().await;
    }
}