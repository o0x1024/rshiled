use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Emitter, WebviewWindow};
use log;
use chrono;
use crate::scan::proxy::Proxy;
use anyhow::{Result, anyhow};

// Import types from the new common location
use crate::handler::scan::common::types::{ScannerStatus, Vulnerability, VulnerabilityDetail};

/// 应用全局状态
pub struct AppState {
    /// 代理服务实例
    pub proxy: Arc<Mutex<Option<Arc<Proxy>>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            proxy: Arc::new(Mutex::new(None)),
        }
    }
}

pub struct ScannerState {
    pub running: Arc<Mutex<bool>>,
    pub status: Arc<Mutex<ScannerStatus>>,
    pub vulnerabilities: Arc<Mutex<Vec<Vulnerability>>>,
    pub window: Arc<WebviewWindow>,
    pub proxy: Arc<Mutex<Option<Arc<Proxy>>>>,
}

impl ScannerState {
    pub fn new(window: WebviewWindow) -> Self {
        Self {
            running: Arc::new(Mutex::new(false)),
            status: Arc::new(Mutex::new(ScannerStatus {
                running: false,
                proxy_address: String::new(),
                proxy_port: 0,
                scan_count: 0,
                vulnerability_count: 0,
                last_update: None,
                last_stop_time: None,
                message: None,
            })),
            vulnerabilities: Arc::new(Mutex::new(Vec::new())),
            window: Arc::new(window),
            proxy: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn update_status(&self, update_fn: impl FnOnce(&mut ScannerStatus)) {
        let mut status = self.status.lock().await;
        update_fn(&mut status);
        status.last_update = Some(chrono::Utc::now().to_utc().to_string());
        
        if let Err(e) = self.window.as_ref().emit(
            "scan_status_update",
            serde_json::json!({
                "status": &*status,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        ) {
            log::error!("Failed to emit scan status update: {}", e);
        }
    }

    pub async fn add_vulnerability(&self, vulnerability: Vulnerability) {
        let mut vulnerabilities = self.vulnerabilities.lock().await;
        vulnerabilities.push(vulnerability.clone());
        
        self.update_status(|status| {
            status.vulnerability_count = vulnerabilities.len();
        }).await;
        
        if let Err(e) = self.window.as_ref().emit(
            "vulnerability_found",
            serde_json::json!({
                "vulnerability": vulnerability,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        ) {
            log::error!("Failed to emit vulnerability found event: {}", e);
        }
    }
}

/// 尝试获取全局AppState
pub fn get_app_state() -> Result<Arc<AppState>> {
    Err(anyhow!("当前环境下无法获取全局AppState"))
} 