// 服务器配置
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub(crate) key: String,
    pub(crate) concurrent_limit: usize,
    path: String,
    pub allow_discovery: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            key: "peerjs".to_string(),
            concurrent_limit: 5000,
            path: "/peerjs".to_string(),
            allow_discovery: false,
        }
    }
}