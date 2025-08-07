use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::{collections::HashMap, sync::atomic::AtomicU64};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    // 唯一ID - 改为String
    pub id: String,
    // HTTP方法
    pub method: String,
    // 主机
    pub host: String,
    // 路径
    pub path: String,
    // 完整URL
    pub url: String,
    // 状态码
    pub status: u16,
    // 时间戳
    pub timestamp: i64,
    // 请求头
    pub request_headers: HashMap<String, String>,
    // 请求体
    pub request_body: String,
    // 响应头
    pub response_headers: HashMap<String, String>,
    // 响应体
    pub response_body: String,
}

impl RequestRecord {
    // 使用指定ID创建请求记录
    pub fn new_with_id(
        id: String, // 改为 String
        method: String,
        url: String,
        request_headers: HashMap<String, String>,
        request_body: String,
    ) -> Self {
        let url_parsed = url::Url::parse(&url).unwrap_or_else(|_| {
            url::Url::parse("http://unknown").unwrap()
        });
        
        let host = url_parsed.host_str().unwrap_or("unknown").to_string();
        let path = url_parsed.path().to_string();
        
        Self {
            id,
            method,
            host,
            path,
            url,
            status: 0,
            timestamp: Utc::now().timestamp_millis(),
            request_headers,
            request_body,
            response_headers: HashMap::new(),
            response_body: String::new(),
        }
    }
    
    pub fn with_response(
        mut self,
        status: u16,
        response_headers: HashMap<String, String>,
        response_body: String,
    ) -> Self {
        self.status = status;
        self.response_headers = response_headers;
        self.response_body = response_body;
        self
    }
    
    // 检查请求记录是否完整
    pub fn is_complete(&self) -> bool {
        // 如果有状态码且不为0，说明有响应
        self.status != 0 && !self.response_body.is_empty()
    }
}

// 拦截的请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptedRequest {
    pub id: String,
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

// 请求存储
pub struct RequestStore {
    records: Arc<RwLock<Vec<RequestRecord>>>,
    intercepted: Arc<RwLock<HashMap<String, InterceptedRequest>>>,
    next_id: AtomicU64,    // 添加连接信息映射
    connection_map: Arc<RwLock<HashMap<String, String>>>,
}

impl RequestStore {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(Vec::new())),
            intercepted: Arc::new(RwLock::new(HashMap::new())),
            next_id: AtomicU64::new(0),
            connection_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn next_request_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
    
    // 添加请求记录
    pub async fn add_record(&self, record: RequestRecord) {
        let mut records = self.records.write().await;
        // 保持历史记录不超过1000条
        if records.len() >= 1000 {
            records.remove(0);
        }
        records.push(record);
    }
    
    // 获取请求记录 (使用 &str ID)
    pub async fn get_record(&self, id: &str) -> Option<RequestRecord> {
        let records = self.records.read().await;
        records.iter().find(|r| r.id == id).cloned()
    }
    
    // 更新请求记录状态 - 保持不变或根据需要调整
    pub async fn update_record_status(&self, id: &str, status: &str) {
        // 简单实现，仅用于记录状态变更
        println!("更新请求记录状态: {} -> {}", id, status);
    }
    
    // 获取所有记录
    pub async fn get_all_records(&self) -> Vec<RequestRecord> {
        let records = self.records.read().await;
        records.clone()
    }
    
    // 清空记录
    pub async fn clear(&self) {
        let mut records = self.records.write().await;
        records.clear();
        // 移除 next_id 重置
    }
    
    // 添加拦截请求
    pub async fn add_intercepted(&self, request: InterceptedRequest) {
        let mut intercepted = self.intercepted.write().await;
        intercepted.insert(request.id.clone(), request);
    }
    
    // 获取拦截请求
    pub async fn get_intercepted(&self, id: &str) -> Option<InterceptedRequest> {
        let intercepted = self.intercepted.read().await;
        intercepted.get(id).cloned()
    }
    
    // 移除拦截请求
    pub async fn remove_intercepted(&self, id: &str) -> Option<InterceptedRequest> {
        let mut intercepted = self.intercepted.write().await;
        intercepted.remove(id)
    }
    
    // 更新请求记录的响应部分 (使用 &str ID)
    pub async fn update_record_with_response(
        &self,
        id: &str,
        status: u16,
        response_headers: HashMap<String, String>,
        response_body: String,
    ) -> Option<RequestRecord> {
        let mut records = self.records.write().await;
        if let Some(record) = records.iter_mut().find(|r| r.id == id) {
            record.status = status;
            record.response_headers = response_headers;
            record.response_body = response_body;
            Some(record.clone())
        } else {
            None
        }
    }
    
    // 保存连接信息
    pub async fn save_connection_info(&self, connection_id: &str, request_id: &str) {
        let mut connection_map = self.connection_map.write().await;
        connection_map.insert(connection_id.to_string(), request_id.to_string());
    }
    
    // 根据连接ID获取请求ID (确保返回 String)
    pub async fn get_request_id_by_connection(&self, connection_id: &str) -> Option<String> {
        let connection_map = self.connection_map.read().await;
        connection_map.get(connection_id).cloned()
    }
    
    // 更新请求记录（仅请求部分）(使用 &str ID)
    pub async fn update_record(&self, id: &str, record: RequestRecord) -> Option<RequestRecord> {
        let mut records = self.records.write().await;
        if let Some(existing_record) = records.iter_mut().find(|r| r.id == id) {
            // 保存原有的响应信息
            let original_status = existing_record.status;
            let original_response_headers = existing_record.response_headers.clone();
            let original_response_body = existing_record.response_body.clone();
            
            // 更新请求信息
            existing_record.method = record.method;
            existing_record.url = record.url;
            existing_record.host = record.host;
            existing_record.path = record.path;
            existing_record.request_headers = record.request_headers;
            existing_record.request_body = record.request_body;
            
            // 保留原始响应信息
            existing_record.status = original_status;
            existing_record.response_headers = original_response_headers;
            existing_record.response_body = original_response_body;
            
            println!("已更新记录 {} 的请求信息", id);
            return Some(existing_record.clone());
        } else {
            println!("未找到ID为 {} 的记录", id);
            return None;
        }
    }
    
    // 获取最近的n条记录
    pub async fn get_recent_records(&self, count: usize) -> Vec<RequestRecord> {
        let records = self.records.read().await;
        let start_idx = if records.len() > count {
            records.len() - count
        } else {
            0
        };
        
        records[start_idx..].to_vec()
    }
    
    // 获取最新的记录 (用于响应处理中的回退逻辑 - 可能需要移除或修改)
    pub async fn get_latest_record(&self) -> Option<RequestRecord> {
        let records = self.records.read().await;
        records.last().cloned()
    }
} 