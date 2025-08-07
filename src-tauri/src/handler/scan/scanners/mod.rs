use crate::handler::scan::engine::ScanResult;
use crate::handler::scan::proxy::{HttpRequest, HttpResponse};
use crate::core::config::AppConfig;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod xss;
pub mod sql;
pub mod rce;
pub mod host_survival;
pub mod port_scanner;
pub mod service_probes;
pub mod ast_analyzer;

/// 扫描器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScannerType {
    pub scanner_type: ScannerTypeEnum,
}

/// 扫描器类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScannerTypeEnum {
    Xss,
    SqlInjection,
    Rce,
}

/// 扫描器特征
#[async_trait]
pub trait Scanner: Send + Sync {
    /// 获取扫描器名称
    async fn name(&self) -> String;
    
    /// 执行扫描
    async fn scan(&self, request: &HttpRequest, response: &HttpResponse) -> Vec<ScanResult>;
}

/// 创建扫描器实例
pub fn create_scanner(scanner_type: ScannerType, config: Arc<AppConfig>) -> Box<dyn Scanner + Send + Sync> {
    match scanner_type.scanner_type {
        ScannerTypeEnum::Xss => Box::new(xss::XssScanner::new(config)),
        ScannerTypeEnum::SqlInjection => {
            // 在实际应用中创建一个SqlInjectionScanner实例
            Box::new(sql::SqlInjectionScanner::new(config))
        },
        ScannerTypeEnum::Rce => Box::new(ScannerType { scanner_type: ScannerTypeEnum::Rce }),
    }
}

// Re-export scanners
pub use xss::XssScanner;
pub use sql::SqlInjectionScanner;
pub use rce::RceScanner;
pub use plugin::manager::PluginManager;

/// Unified scanner type enum for easier management
#[derive(Clone)]
pub enum UnifiedScannerType {
    Xss(XssScanner),
    SqlInjection(SqlInjectionScanner),
    Rce(RceScanner),
}

#[async_trait]
impl Scanner for UnifiedScannerType {
    async fn name(&self) -> String {
        match self {
            UnifiedScannerType::Xss(s) => s.name().await,
            UnifiedScannerType::SqlInjection(s) => s.name().await,
            UnifiedScannerType::Rce(s) => s.name().await,
        }
    }

    async fn scan(&self, request: &HttpRequest, response: &HttpResponse) -> Vec<ScanResult> {
        match self {
            UnifiedScannerType::Xss(s) => s.scan(request, response).await,
            UnifiedScannerType::SqlInjection(s) => s.scan(request, response).await,
            UnifiedScannerType::Rce(s) => s.scan(request, response).await,
        }
    }
}

/// Thread-safe scanner types (excludes Plugin which isn't Send)
pub enum ThreadSafeScannerType {
    Xss(Arc<Mutex<XssScanner>>),
    SqlInjection(Arc<Mutex<SqlInjectionScanner>>),
    Rce(Arc<Mutex<RceScanner>>),
}

#[async_trait]
impl Scanner for ThreadSafeScannerType {
    async fn name(&self) -> String {
        match self {
            ThreadSafeScannerType::Xss(s) => s.lock().await.name().await,
            ThreadSafeScannerType::SqlInjection(s) => s.lock().await.name().await,
            ThreadSafeScannerType::Rce(s) => s.lock().await.name().await,
        }
    }

    async fn scan(&self, request: &HttpRequest, response: &HttpResponse) -> Vec<ScanResult> {
        match self {
            ThreadSafeScannerType::Xss(s) => s.lock().await.scan(request, response).await,
            ThreadSafeScannerType::SqlInjection(s) => s.lock().await.scan(request, response).await,
            ThreadSafeScannerType::Rce(s) => s.lock().await.scan(request, response).await,
        }
    }
}

#[async_trait]
impl Scanner for ScannerType {
    async fn name(&self) -> String {
        match self.scanner_type {
            ScannerTypeEnum::Xss => "XSS Scanner".to_string(),
            ScannerTypeEnum::SqlInjection => "SQL Injection Scanner".to_string(),
            ScannerTypeEnum::Rce => "RCE Scanner".to_string(),
        }
    }
    
    async fn scan(&self, request: &HttpRequest, response: &HttpResponse) -> Vec<ScanResult> {
        match self.scanner_type {
            ScannerTypeEnum::Xss => {
                // 在实际应用中创建一个XssScanner实例
                let config = Arc::new(AppConfig::default());
                let scanner = XssScanner::new(config);
                scanner.scan(request, response).await
            },
            ScannerTypeEnum::SqlInjection => {
                // 在实际应用中创建一个SqlInjectionScanner实例
                let config = Arc::new(AppConfig::default());
                let scanner = SqlInjectionScanner::new(config);
                scanner.scan(request, response).await
            },
            ScannerTypeEnum::Rce => {
                // 在实际应用中创建一个RceScanner实例
                Vec::new()
            },
        }
    }
}

// 导出插件模块
pub mod plugin;
