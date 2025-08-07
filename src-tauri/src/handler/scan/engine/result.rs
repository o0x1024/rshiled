use chrono;
use serde::{Deserialize, Serialize};

/// 漏洞扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// 漏洞类型
    pub vulnerability_type: String,
    /// 漏洞名称
    pub name: String,
    /// 漏洞描述
    pub description: String,
    /// 风险级别
    pub risk_level: String,
    /// 请求URL
    pub url: String,
    /// 请求方法
    pub method: String,
    /// 漏洞参数
    pub parameter: Option<String>,
    /// 漏洞值
    pub value: Option<String>,
    /// 漏洞证据
    pub evidence: Option<String>,
    /// 修复建议
    pub remediation: Option<String>,
    /// 详细信息
    pub details: Option<String>,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 请求详情
    pub request_details: Option<String>,
    /// 响应详情
    pub response_details: Option<String>,
}
