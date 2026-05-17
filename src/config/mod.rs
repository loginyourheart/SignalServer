use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_key")]
    pub key: String,
    #[serde(default = "default_concurrent_limit")]
    pub concurrent_limit: usize,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default)]
    pub allow_discovery: bool,
    #[serde(default = "default_alive_timeout")]
    pub alive_timeout: u64,
    #[serde(default = "default_check_interval")]
    pub check_interval: u64,
    #[serde(default = "default_cleanup_out_msgs")]
    pub cleanup_out_msgs: u64,
    #[serde(default = "default_expire_timeout")]
    pub expire_timeout: u64,
    #[serde(default)]
    pub tls_enabled: bool,
    #[serde(default = "default_cert_path")]
    pub tls_cert_path: String,
    #[serde(default = "default_key_path")]
    pub tls_key_path: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub debug_request_headers: bool,
}

fn default_key() -> String { "peerjs".to_string() }
fn default_concurrent_limit() -> usize { 5000 }
fn default_path() -> String { "/peerjs".to_string() }
fn default_alive_timeout() -> u64 { 60000 }
fn default_check_interval() -> u64 { 300 }
fn default_cleanup_out_msgs() -> u64 { 10000 }
fn default_expire_timeout() -> u64 { 5000 }
fn default_cert_path() -> String { "cert.pem".to_string() }
fn default_key_path() -> String { "key.pem".to_string() }
fn default_log_level() -> String { "info".to_string() }

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            key: default_key(),
            concurrent_limit: default_concurrent_limit(),
            path: default_path(),
            allow_discovery: false,
            alive_timeout: default_alive_timeout(),
            check_interval: default_check_interval(),
            cleanup_out_msgs: default_cleanup_out_msgs(),
            expire_timeout: default_expire_timeout(),
            tls_enabled: false,
            tls_cert_path: default_cert_path(),
            tls_key_path: default_key_path(),
            log_level: default_log_level(),
            debug_request_headers: false,
        }
    }
}

impl ServerConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.as_ref();
        
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let config: ServerConfig = toml::from_str(&content)?;
            
            println!("Loaded config from: {}", path.display());
            println!("  TLS enabled: {}", config.tls_enabled);
            println!("  Log level: {}", config.log_level);
            
            config.save_to_file(path)?;
            Ok(config)
        } else {
            println!("Config file not found, creating default: {}", path.display());
            let config = Self::default();
            config.save_to_file(path)?;
            Ok(config)
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let content = generate_config_file_content(self);
        fs::write(path, content)
    }
}

fn generate_config_file_content(config: &ServerConfig) -> String {
    format!(
r#"
# ========================================
# PeerJS Server Rust - 配置文件
# ========================================

# --------------------------
# 基本配置
# --------------------------

# PeerJS 认证密钥
key = "{}"

# 最大并发连接数
concurrent_limit = {}

# API 路由路径
path = "{}"

# 是否允许列出所有在线客户端（listAllPeers）
allow_discovery = {}

# --------------------------
# 健康检查配置
# --------------------------

# 客户端存活超时（毫秒）
alive_timeout = {}

# 连接检查间隔（秒）
check_interval = {}

# 清理过期消息间隔（毫秒）
cleanup_out_msgs = {}

# 消息过期时间（毫秒）
expire_timeout = {}

# --------------------------
# TLS/HTTPS 配置
# --------------------------

# 是否启用 TLS（HTTPS/WSS）
tls_enabled = {}

# TLS 证书文件路径
tls_cert_path = "{}"

# TLS 私钥文件路径
tls_key_path = "{}"

# --------------------------
# 日志配置
# --------------------------

# 日志级别: trace, debug, info, warn, error
log_level = "{}"

# 是否打印请求头调试信息
debug_request_headers = {}
"#,
        config.key,
        config.concurrent_limit,
        config.path,
        config.allow_discovery,
        config.alive_timeout,
        config.check_interval,
        config.cleanup_out_msgs,
        config.expire_timeout,
        config.tls_enabled,
        config.tls_cert_path,
        config.tls_key_path,
        config.log_level,
        config.debug_request_headers,
    )
}

