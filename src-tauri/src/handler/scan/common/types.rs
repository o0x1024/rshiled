// src-tauri/src/handler/scan/common/types.rs
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TargetType {
    Website,
    IP,
    IPRange,
    Domain,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub value: String,
    pub target_type: TargetType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedPortScanOptions {
    pub enabled: Option<bool>,
    pub ports: Option<String>, // e.g., "1-1000", "80,443,22", "top100", "top1000", "all"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedVulnerabilityScanOptions {
    pub enabled: Option<bool>,
    pub plugins: Option<Vec<String>>, // e.g., ["xss", "sqli"] or ["all"]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedServiceBruteforceOptions {
    pub enabled: Option<bool>,
    pub services: Option<Vec<String>>, // e.g., ["ssh", "ftp"]
    pub usernames: Option<String>,    // File path or comma-separated list
    pub passwords: Option<String>,    // File path or comma-separated list
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedScanOptions {
    pub host_survival: Option<bool>,
    pub port_scan: Option<DetailedPortScanOptions>,
    pub vulnerability_scan: Option<DetailedVulnerabilityScanOptions>,
    pub web_sensitive_info: Option<bool>,
    pub service_bruteforce: Option<DetailedServiceBruteforceOptions>,
    pub fingerprint_scan: Option<bool>,
    pub nuclei_scan: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveScanConfig {
    pub targets: Vec<String>,
    pub scan_type: String, // "full", "quick", "custom", "nuclei"
    pub threads: u32,
    pub timeout: u32,
    pub save_results: bool,
    pub results_path: Option<String>,
    pub detailed_scan_options: Option<DetailedScanOptions>,
}

impl ActiveScanConfig {
    // 判断目标类型
    pub fn identify_target_type(target: &str) -> TargetType {
        // 检测IP地址
        if let Ok(_) = IpAddr::from_str(target) {
            return TargetType::IP;
        }
        
        // 检测IP范围 (例如: 192.168.1.1-192.168.1.255 或 192.168.1.0/24)
        if target.contains("-") || target.contains("/") {
            return TargetType::IPRange;
        }
        
        // 检测网站URL
        if target.starts_with("http://") || target.starts_with("https://") {
            return TargetType::Website;
        }
        
        // 检测域名
        if target.contains(".") && !target.contains(" ") {
            return TargetType::Domain;
        }
        
        TargetType::Unknown
    }
    
    // 获取处理后的目标列表，包含目标类型
    pub fn get_processed_targets(&self) -> Vec<Target> {
        self.targets.iter()
            .map(|t| Target {
                value: t.clone(),
                target_type: Self::identify_target_type(t),
            })
            .collect()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PassiveScanConfig { // Renamed from ScanConfig to avoid conflict if ScanConfig is used elsewhere
    pub port: u16,
    pub save_results: bool,
    pub results_path: Option<String>,
    pub intercept_tls: bool,
    pub use_plugins: bool, // Kept for passive scan context if needed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityDetail {
    pub note: String,
    pub request: String,
    pub response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: u32,
    pub vulnerability_type: String,
    pub name: String,
    pub url: String,
    pub risk_level: String, // "Critical", "High", "Medium", "Low", "Info"
    pub timestamp: String,
    pub description: String,
    pub solution: String,
    pub parameter: Option<String>,
    pub value: Option<String>,
    pub evidence: Option<String>,
    pub details: Option<VulnerabilityDetail>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

// Definition for ScannerStatus, moved here for centralization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerStatus {
    pub running: bool,
    pub proxy_address: String,
    pub proxy_port: u16,
    pub scan_count: usize,
    pub vulnerability_count: usize,
    pub last_update: Option<String>,
    pub last_stop_time: Option<String>,
    pub message: Option<String>,
}

// You might also want to move ScannerStatus here if it's broadly used
// For now, assuming it stays in state.rs or api.rs (to be refactored) 