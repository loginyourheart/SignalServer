use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub key: String,
    pub concurrent_limit: usize,
    pub path: String,
    pub allow_discovery: bool,
    pub alive_timeout: u64,
    pub check_interval: u64,
    pub cleanup_out_msgs: u64,
    pub expire_timeout: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            key: "peerjs".to_string(),
            concurrent_limit: 5000,
            path: "/peerjs".to_string(),
            allow_discovery: false,
            alive_timeout: 60000,
            check_interval: 300,
            cleanup_out_msgs: 10000,
            expire_timeout: 5000,
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
            Ok(config)
        } else {
            println!("Config file not found, creating default: {}", path.display());
            let config = Self::default();
            config.save_to_file(path)?;
            Ok(config)
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(path, content)
    }
}
