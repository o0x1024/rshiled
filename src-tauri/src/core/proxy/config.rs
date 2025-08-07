use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    // 唯一标识符
    pub id: Option<String>,
    // 代理监听端口
    pub port: u16,
    // 代理监听接口 (0.0.0.0 或 127.0.0.1)
    pub interface: String,
    // 是否启用HTTPS拦截
    pub https_enabled: bool,
    // HTTP版本 (1 或 2)
    pub http_version: Option<u8>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            id: None,
            port: 8080,
            interface: "127.0.0.1".to_string(),
            https_enabled: true,
            http_version: Some(1),
        }
    }
} 