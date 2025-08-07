use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 代理主机地址
    pub host: String,
    /// 代理端口
    pub port: u16,
    /// 连接超时（秒）
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,
    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
    /// 重试延迟（毫秒）
    #[serde(default = "default_retry_delay")]
    pub retry_delay: u64,
}

/// 默认连接超时（秒）
fn default_connect_timeout() -> u64 {
    10
}

/// 默认最大重试次数
fn default_max_retries() -> usize {
    3
}

/// 默认重试延迟（毫秒）
fn default_retry_delay() -> u64 {
    1000
}

/// 扫描器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    /// 扫描模式：passive（被动）, active（主动）, mixed（混合）
    pub mode: String,
    /// 并发扫描任务数
    pub concurrency: usize,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
    /// 是否保存扫描结果
    pub save_results: bool,
    /// 扫描结果保存路径
    pub results_path: String,
}

/// XSS漏洞配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XssConfig {
    /// 是否启用
    pub enabled: bool,
    /// 是否使用AST分析
    pub use_ast: bool,
    /// 插桩参数的最大数量
    pub max_params: usize,
    /// 插桩测试的最大深度
    pub max_depth: usize,
}

/// SQL注入漏洞配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlInjectionConfig {
    /// 是否启用
    pub enabled: bool,
    /// 检测级别：low, medium, high
    pub level: String,
}

/// RCE漏洞配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RceConfig {
    /// 是否启用
    pub enabled: bool,
    /// 检测级别：low, medium, high
    pub level: String,
}

/// 路径遍历漏洞配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathTraversalConfig {
    /// 是否启用
    pub enabled: bool,
}

/// 开放重定向漏洞配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRedirectConfig {
    /// 是否启用
    pub enabled: bool,
}

/// 漏洞配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilitiesConfig {
    /// XSS漏洞配置
    pub xss: XssConfig,
    /// SQL注入漏洞配置
    pub sql_injection: SqlInjectionConfig,
    /// RCE漏洞配置
    pub rce: RceConfig,
    /// 路径遍历漏洞配置
    pub path_traversal: PathTraversalConfig,
    /// 开放重定向漏洞配置
    pub open_redirect: OpenRedirectConfig,
}

/// 规则配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesConfig {
    /// 是否启用内置规则
    pub enable_builtin: bool,
    /// 是否启用扩展规则
    pub enable_extensions: bool,
    /// 扩展规则路径
    pub extensions_path: String,
    /// 漏洞配置
    pub vulnerabilities: VulnerabilitiesConfig,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别：error, warn, info, debug, trace
    pub level: String,
    /// 是否输出到文件
    pub file_output: bool,
    /// 日志文件路径
    pub file_path: String,
    /// 是否在控制台显示彩色日志
    pub colored_output: bool,
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 代理配置
    pub proxy: ProxyConfig,
    /// 扫描器配置
    pub scanner: ScannerConfig,
    /// 规则配置
    pub rules: RulesConfig,
    /// 日志配置
    pub logging: LoggingConfig,
}

impl AppConfig {
    /// 从文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config_str = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;
        
        let config: AppConfig = serde_yaml::from_str(&config_str)
            .with_context(|| format!("Failed to parse config file: {:?}", path.as_ref()))?;
        
        Ok(config)
    }
    
    /// 获取默认配置
    pub fn default() -> Self {
        // 尝试从默认路径加载
        if let Ok(config) = Self::from_file("config/config.yaml") {
            return config;
        }
        
        // 返回硬编码的默认配置
        Self {
            proxy: ProxyConfig {
                host: "127.0.0.1".to_string(),
                port: 8889,
                connect_timeout: default_connect_timeout(),
                max_retries: default_max_retries(),
                retry_delay: default_retry_delay(),
            },
            scanner: ScannerConfig {
                mode: "active".to_string(),
                concurrency: 10,
                timeout_ms: 5000,
                save_results: true,
                results_path: "results".to_string(),
            },
            rules: RulesConfig {
                enable_builtin: true,
                enable_extensions: true,
                extensions_path: "rules/extensions".to_string(),
                vulnerabilities: VulnerabilitiesConfig {
                    xss: XssConfig {
                        enabled: true,
                        use_ast: true,
                        max_params: 20,
                        max_depth: 3,
                    },
                    sql_injection: SqlInjectionConfig {
                        enabled: true,
                        level: "high".to_string(),
                    },
                    rce: RceConfig {
                        enabled: true,
                        level: "high".to_string(),
                    },
                    path_traversal: PathTraversalConfig {
                        enabled: true,
                    },
                    open_redirect: OpenRedirectConfig {
                        enabled: true,
                    },
                },
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file_output: true,
                file_path: "logs/passvia_scan.log".to_string(),
                colored_output: true,
            },
        }
    }
} 