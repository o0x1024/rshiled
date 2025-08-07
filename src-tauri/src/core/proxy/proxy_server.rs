use flate2::read::GzDecoder;
use http::Method;
use serde_json::{self, json};
use std::collections::HashMap;
use std::io::Read;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::RwLock;
use url::Url;


use crate::core::proxy::config::ProxyConfig;
use crate::core::proxy::http_interceptor::{
    HttpInterceptor, RequestInterceptControl, ResponseInterceptControl,
};
use crate::core::proxy::store::{RequestRecord, RequestStore};
use crate::core::proxy::ProxyState;
use crate::internal::certificate::CertificateAuthority;

use http_body_util::{BodyExt, Full};
use hudsucker::hyper::{Request, Response};
use hudsucker::Body;
use hudsucker::{
    certificate_authority::OpensslAuthority,
    hyper,
    hyper::body::Bytes,
    openssl::{hash::MessageDigest, pkey::PKey, x509::X509},
    rustls::crypto::aws_lc_rs,
    tokio_tungstenite::tungstenite::Message,
    HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler, *,
};
use hyper::header::{HeaderName, HeaderValue};
use std::fs;
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant};
use std::net::TcpListener;

// Cookie存储结构
struct CookieStore {
    // 每个域名对应的Cookie映射，键是域名，值是该域名下的Cookie映射(键是Cookie名，值是Cookie详情)
    cookies: RwLock<HashMap<String, HashMap<String, String>>>,
}

impl CookieStore {
    fn new() -> Self {
        Self {
            cookies: RwLock::new(HashMap::new()),
        }
    }

    // 从Set-Cookie头解析并存储Cookie
    async fn store_cookies(&self, url: &str, headers: &HashMap<String, String>) {
        println!("尝试从URL: {} 存储Cookie", url);
        
        // 解析URL获取域名
        let domain = match Url::parse(url) {
            Ok(parsed_url) => match parsed_url.host_str() {
                Some(host) => host.to_string(),
                None => {
                    println!("无法解析URL中的域名: {}", url);
                    return;  // 没有有效域名，不处理
                }
            },
            Err(e) => {
                println!("URL解析失败: {} - {}", url, e);
                return;    // URL解析失败，不处理
            }
        };

        println!("从URL: {} 解析出域名: {}", url, domain);

        // 获取所有Set-Cookie头
        let set_cookies: Vec<&String> = headers.iter()
            .filter_map(|(k, v)| if k.to_lowercase() == "set-cookie" { Some(v) } else { None })
            .collect();

        if set_cookies.is_empty() {
            println!("没有找到Set-Cookie头");
            return;  // 没有Set-Cookie，直接返回
        }

        println!("找到 {} 个Set-Cookie头", set_cookies.len());

        let mut domain_cookies = match self.cookies.write().await.get(&domain).cloned() {
            Some(cookies) => {
                println!("已有域名 {} 的Cookie存储，合并新Cookie", domain);
                cookies
            },
            None => {
                println!("为域名 {} 创建新的Cookie存储", domain);
                HashMap::new()
            }
        };

        // 处理每个Set-Cookie头
        for set_cookie in set_cookies {
            println!("处理Set-Cookie: {}", set_cookie);
            // 简单解析cookie：name=value; 其他属性
            if let Some(cookie_main) = set_cookie.split(';').next() {
                if let Some((name, value)) = cookie_main.split_once('=') {
                    println!("存储Cookie: {}={}", name.trim(), value.trim());
                    domain_cookies.insert(name.trim().to_string(), value.trim().to_string());
                }
            }
        }

        // 保存解析后的cookies
        self.cookies.write().await.insert(domain.to_string(), domain_cookies);
        println!("域名 {} 的Cookie已更新", domain);
    }

    // 为请求添加Cookie头
    async fn apply_cookies(&self, url: &str, headers: &mut HashMap<String, String>) {
        println!("尝试为URL: {} 应用Cookie", url);
        
        // 解析URL获取域名
        let domain = match Url::parse(url) {
            Ok(parsed_url) => match parsed_url.host_str() {
                Some(host) => host.to_string(),
                None => {
                    println!("无法解析URL中的域名: {}", url);
                    return;  // 没有有效域名，不处理
                }
            },
            Err(e) => {
                println!("URL解析失败: {} - {}", url, e);
                return;    // URL解析失败，不处理
            }
        };

        println!("从URL: {} 解析出域名: {}", url, domain);

        // 读取该域名的cookies
        let cookies_lock = self.cookies.read().await;
        let domain_cookies = match cookies_lock.get(&domain) {
            Some(cookies) => {
                println!("找到域名 {} 的Cookie存储，包含 {} 个Cookie", domain, cookies.len());
                cookies
            },
            None => {
                println!("域名 {} 没有存储的Cookie", domain);
                return; // 该域名没有存储cookies
            }
        };

        // 如果已有Cookie头，则附加，否则创建新的
        let has_cookie = headers.contains_key("cookie");
        let cookie_header = headers.entry("cookie".to_string()).or_insert("".to_string());
        
        if !domain_cookies.is_empty() {
            let mut cookie_parts = Vec::new();
            
            // 添加已有cookie（如果有）
            if !cookie_header.is_empty() {
                println!("请求中已有Cookie: {}", cookie_header);
                cookie_parts.push(cookie_header.clone());
            }
            
            // 添加存储的cookies
            for (name, value) in domain_cookies {
                println!("添加存储的Cookie: {}={}", name, value);
                cookie_parts.push(format!("{}={}", name, value));
            }
            
            // 更新Cookie头
            *cookie_header = cookie_parts.join("; ");
            println!("应用Cookie后，请求的Cookie头为: {}", cookie_header);
            
            if !has_cookie {
                println!("为请求新增Cookie头");
            } else {
                println!("更新请求的Cookie头");
            }
        }
    }
}

// 新增共享拦截状态结构
struct SharedInterceptState {
    request_enabled: AtomicBool,
    response_enabled: AtomicBool,
    intercept_enabled: AtomicBool,
    cookie_store: CookieStore,  // 新增Cookie存储
}

impl SharedInterceptState {
    fn new(request_enabled: bool, response_enabled: bool, intercept_enabled: bool) -> Self {
        Self {
            request_enabled: AtomicBool::new(request_enabled),
            response_enabled: AtomicBool::new(response_enabled),
            intercept_enabled: AtomicBool::new(intercept_enabled),
            cookie_store: CookieStore::new(),  // 初始化Cookie存储
        }
    }

    fn is_request_enabled(&self) -> bool {
        self.request_enabled.load(Ordering::Relaxed)
    }

    fn set_request_enabled(&self, enabled: bool) {
        self.request_enabled.store(enabled, Ordering::Relaxed);
    }

    fn is_intercept_enabled(&self) -> bool {
        self.intercept_enabled.load(Ordering::Relaxed)
    }

    fn set_intercept_enabled(&self, enabled: bool) {
        self.intercept_enabled.store(enabled, Ordering::Relaxed);
    }
    fn is_response_enabled(&self) -> bool {
        self.response_enabled.load(Ordering::Relaxed)
    }

    fn set_response_enabled(&self, enabled: bool) {
        self.response_enabled.store(enabled, Ordering::Relaxed);
    }
}

// 代理服务器
pub struct ProxyServer {
    config: ProxyConfig,
    pub app: AppHandle,
    store: Arc<RequestStore>,
    certificate_authority: Arc<CertificateAuthority>,
    http_interceptor: Arc<HttpInterceptor>,
    request_control_tx: mpsc::Sender<RequestInterceptControl>,
    response_control_tx: mpsc::Sender<ResponseInterceptControl>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    intercept_enabled: bool,
    intercept_request_enabled: bool,
    intercept_response_enabled: bool,
    intercept_state: Arc<SharedInterceptState>,
}

impl Clone for ProxyServer {
    fn clone(&self) -> Self {
        // 创建新的oneshot通道
        let (shutdown_tx, _) = oneshot::channel();

        Self {
            config: self.config.clone(),
            app: self.app.clone(),
            store: self.store.clone(),
            certificate_authority: self.certificate_authority.clone(),
            http_interceptor: self.http_interceptor.clone(),
            request_control_tx: self.request_control_tx.clone(),
            response_control_tx: self.response_control_tx.clone(),
            shutdown_tx: Some(shutdown_tx),
            intercept_enabled: self.intercept_enabled,
            intercept_request_enabled: self.intercept_request_enabled,
            intercept_response_enabled: self.intercept_response_enabled,
            intercept_state: self.intercept_state.clone(),
        }
    }
}

impl ProxyServer {
    pub fn new(
        config: ProxyConfig,
        app: AppHandle,
        store: Arc<RequestStore>,
        intercept_enabled: bool,
        intercept_request_enabled: bool,
        intercept_response_enabled: bool,
    ) -> Result<Self, String> {
        let cert_dir = PathBuf::from("certs");
        let certificate_authority = Arc::new(CertificateAuthority::new(&cert_dir));
        let http_interceptor = Arc::new(HttpInterceptor::new(app.clone(), Arc::clone(&store)));
        let request_control_tx = http_interceptor.get_request_sender();
        let response_control_tx = http_interceptor.get_response_sender();

        Ok(Self {
            config,
            app,
            store,
            certificate_authority,
            http_interceptor,
            request_control_tx,
            response_control_tx,
            shutdown_tx: None,
            intercept_request_enabled,
            intercept_response_enabled,
            intercept_enabled,
            intercept_state: Arc::new(SharedInterceptState::new(
                intercept_request_enabled,
                intercept_response_enabled,
                intercept_enabled,
            )),
        })
    }

    // 向后兼容的ProxyServer::new函数，自动将响应拦截设为关闭状态
    pub fn new_compat(
        config: ProxyConfig,
        app: AppHandle,
        store: Arc<RequestStore>,
        intercept_request_enabled: bool,
        intercept_enabled: bool,
    ) -> Result<Self, String> {
        // 默认响应拦截为关闭状态
        let intercept_response_enabled = false;

        // 新建拦截状态
        let intercept_state = Arc::new(SharedInterceptState::new(
            intercept_request_enabled,
            intercept_response_enabled,
            intercept_enabled,
        ));

        // 获取证书目录
        let cert_dir = match std::env::current_dir() {
            Ok(mut dir) => {
                dir.push("certs");
                dir
            }
            Err(e) => return Err(format!("无法获取当前目录: {}", e)),
        };

        // 创建组件
        let certificate_authority = Arc::new(CertificateAuthority::new(&cert_dir));
        let http_interceptor = Arc::new(HttpInterceptor::new(app.clone(), Arc::clone(&store)));
        let request_control_tx = http_interceptor.get_request_sender();
        let response_control_tx = http_interceptor.get_response_sender();

        Ok(Self {
            config,
            app,
            store,
            certificate_authority,
            http_interceptor,
            request_control_tx,
            response_control_tx,
            shutdown_tx: None,
            intercept_request_enabled,
            intercept_response_enabled,
            intercept_enabled,
            intercept_state,
        })
    }

    // 启动代理服务器
    pub async fn start(mut self) -> Result<Self, String> {
        let port = self.config.port;
        let interface = self.config.interface.clone();

        // 在启动前先检查端口是否可用
        if !is_port_available(port) {
            return Err(format!("端口 {} 已被占用，请尝试使用其他端口或确保之前的代理已完全关闭", port));
        }

        // 创建关闭信号通道
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        // 加载或生成CA证书
        if !self.certificate_authority.has_ca_certificate() {
            println!("没有找到CA证书，将生成新的CA证书");
            self.app
                .emit(
                    "proxy-ca-missing",
                    json!({
                        "message": "没有找到CA证书，正在生成新证书..."
                    }),
                )
                .map_err(|e| format!("发送CA证书缺失事件失败: {}", e))?;
        }

        match self.certificate_authority.generate_ca().await {
            Ok(_) => {
                println!("CA证书已成功加载或生成");
                self.app
                    .emit(
                        "proxy-ca-ready",
                        json!({
                            "message": "CA证书已就绪"
                        }),
                    )
                    .map_err(|e| format!("发送CA证书就绪事件失败: {}", e))?;
            }
            Err(e) => {
                let error_msg = format!("加载或生成CA证书失败: {}", e);
                println!("{}", error_msg);
                self.app
                    .emit(
                        "proxy-ca-error",
                        json!({
                            "message": error_msg
                        }),
                    )
                    .map_err(|e| format!("发送CA证书错误事件失败: {}", e))?;
                return Err(error_msg);
            }
        }

        // 获取CA证书和私钥
        let private_key_bytes =
            fs::read("certs/RShield_CA.key").map_err(|e| format!("读取私钥文件失败: {}", e))?;
        let ca_cert_bytes =
            fs::read("certs/RShield_CA.crt").map_err(|e| format!("读取CA证书文件失败: {}", e))?;

        // 解析证书和私钥
        let private_key = PKey::private_key_from_pem(&private_key_bytes)
            .map_err(|e| format!("解析私钥失败: {}", e))?;
        let ca_cert =
            X509::from_pem(&ca_cert_bytes).map_err(|e| format!("解析CA证书失败: {}", e))?;

        // 创建CA
        let ca: OpensslAuthority = OpensslAuthority::new(
            private_key,
            ca_cert,
            MessageDigest::sha256(),
            1_000,
            aws_lc_rs::default_provider(),
        );

        // 创建代理处理器
        let interceptor_clone = Arc::clone(&self.http_interceptor);
        let store_clone = Arc::clone(&self.store);
        let app_clone = self.app.clone();
        let intercept_state_clone = Arc::clone(&self.intercept_state);

        let handler = ProxyHandler {
            app: app_clone,
            store: store_clone,
            interceptor: interceptor_clone,
            intercept_state: intercept_state_clone,
        };

        // 设置监听地址
        let addr = format!("{}:{}", interface, port)
            .parse::<SocketAddr>()
            .map_err(|e| format!("解析监听地址失败: {}", e))?;

        // 创建和启动代理
        let proxy = Proxy::builder()
            .with_addr(addr)
            .with_ca(ca)
            .with_rustls_client(aws_lc_rs::default_provider())
            .with_http_handler(handler.clone())
            .with_websocket_handler(handler)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .build()
            .map_err(|e| format!("创建代理失败: {}", e))?;

        // 启动代理土ff
        let app_handle = self.app.clone();
        tokio::spawn(async move {
            match proxy.start().await {
                Ok(_) => {
                    println!("代理服务已停止");
                    let _ = app_handle.emit(
                        "proxy-stopped",
                        json!({
                            "message": "代理服务已停止"
                        }),
                    );
                }
                Err(e) => {
                    println!("代理服务运行出错: {}", e);
                    let _ = app_handle.emit(
                        "proxy-error",
                        json!({
                            "message": format!("代理服务运行出错: {}", e)
                        }),
                    );
                }
            }
        });

        println!("代理服务已在 {} 启动", addr);
        self.app
            .emit(
                "proxy-started",
                json!({
                    "message": format!("代理服务已在 {} 启动", addr)
                }),
            )
            .map_err(|e| format!("发送代理启动事件失败: {}", e))?;

        Ok(self)
    }

    // 停止代理服务器
    pub async fn stop(self) -> Result<(), String> {
        if let Some(tx) = self.shutdown_tx {
            // 发送关闭信号
            match tx.send(()) {
                Ok(_) => {
            println!("已发送停止代理服务信号");
                    
                    // 等待一小段时间，确保关闭信号被处理
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    
                    // 记录停止事件
                    println!("代理服务已停止，端口 {} 将被释放", self.config.port);
                    Ok(())
                },
                Err(_) => {
                    // 如果接收端已关闭，说明代理可能已经停止
                    println!("代理服务可能已经停止，发送信号失败");
                    Ok(())
                }
            }
        } else {
            return Err("代理服务未运行".to_string());
        }
    }

    // 丢弃拦截的请求
    pub async fn drop_intercepted(&self, request_id: &str) -> Result<(), String> {
        println!("尝试丢弃请求: ID={}", request_id);

        // 检查请求是否仍然存在于拦截列表中
        let exists = self.store.get_intercepted(request_id).await.is_some();
        if !exists {
            println!(
                "警告: 请求 ID={} 已不在拦截列表中，可能已被处理",
                request_id
            );
            return Ok(()); // 请求已被处理，不需要再次丢弃
        }

        // 创建响应通道
        let (response_tx, response_rx) = oneshot::channel();

        // 发送丢弃命令
        match self
            .request_control_tx
            .send(RequestInterceptControl::Drop {
                req_id: request_id.to_string(),
                response_tx,
            })
            .await
        {
            Ok(_) => {
                println!("发送丢弃命令成功，等待处理: ID={}", request_id);
            }
            Err(e) => {
                let error_msg = format!("发送丢弃命令失败: {}", e);
                println!("{}", error_msg);
                return Err(error_msg);
            }
        }

        // 等待操作完成，增加超时处理
        match tokio::time::timeout(tokio::time::Duration::from_secs(5), response_rx).await {
            Ok(result) => {
                match result {
                    Ok(result) => match result {
                        Ok(_) => {
                            println!("成功丢弃请求: ID={}", request_id);
                            Ok(())
                        }
                        Err(e) => {
                            println!("丢弃请求失败: {}", e);
                            Err(e)
                        }
                    },
                    Err(e) => {
                        let error_msg = format!("等待丢弃请求响应失败: {}", e);
                        println!("{}", error_msg);
                        // 超时情况下，我们仍认为请求已被丢弃（或者即将被丢弃）
                        Ok(())
                    }
                }
            }
            Err(_) => {
                println!("等待丢弃请求响应超时");
                // 超时情况下，我们仍认为请求已被丢弃（或者即将被丢弃）
                Ok(())
            }
        }
    }

    // 转发拦截的请求，可能带有修改
    pub async fn forward_intercepted(&self, request_data: String) -> Result<String, String> {
        println!("转发被拦截的请求: {}", request_data);

        // 解析拦截请求数据
        let request: serde_json::Value =
            serde_json::from_str(&request_data).map_err(|e| format!("解析拦截请求失败: {}", e))?;

        let request_id = match request.get("id") {
            Some(id) => id.as_str().unwrap_or("").to_string(),
            None => return Err("请求ID不存在".to_string()),
        };

        let original_url = match request.get("url") {
            Some(url) => url.as_str().unwrap_or("").to_string(),
            None => return Err("URL不存在".to_string()),
        };

        // 获取最近的30条记录，看是否有匹配的请求
        let recent_records = self.store.get_recent_records(30).await;
        println!("获取到 {} 条最近记录", recent_records.len());

        // 尝试查找匹配的记录
        let found_record = recent_records
            .iter()
            .find(|r| r.id.to_string() == request_id);

        if let Some(record) = found_record {
            println!("找到匹配的记录 ID: {}", record.id);

            // 从请求中提取相关信息
            let method = match request.get("method") {
                Some(m) => m.as_str().unwrap_or("GET").to_string(),
                None => record.method.clone(),
            };

            let headers = match request.get("headers") {
                Some(h) => {
                    let mut map = HashMap::new();
                    if let Some(obj) = h.as_object() {
                        for (key, value) in obj {
                            if let Some(v) = value.as_str() {
                                map.insert(key.clone(), v.to_string());
                            }
                        }
                    }
                    map
                }
                None => record.request_headers.clone(),
            };

            let body = match request.get("body") {
                Some(b) => b.as_str().unwrap_or("").to_string(),
                None => record.request_body.clone(),
            };

            // 检查请求是否有修改
            let has_changes = record.url != original_url
                || record.method != method
                || record.request_headers != headers
                || record.request_body != body;

            if has_changes {
                println!("请求已被修改，更新记录");

                // 解析URL以获取主机和路径
                let host;
                let path;
                if let Ok(parsed_url) = url::Url::parse(&original_url) {
                    host = parsed_url.host_str().unwrap_or("unknown").to_string();
                    path = parsed_url.path().to_string();
                } else {
                    host = "unknown".to_string();
                    path = "/".to_string();
                }

                // 创建新的记录对象
                let new_record = RequestRecord::new_with_id(
                    record.id.clone(), // 克隆ID以避免移动错误
                    method.clone(),
                    original_url.clone(),
                    headers.clone(),
                    body.clone(),
                );

                // 更新记录
                if let Some(updated) = self.store.update_record(&request_id, new_record).await {
                    println!("已更新记录: {}", updated.id);
                } else {
                    println!("更新记录失败: {}", request_id);
                }
            } else {
                println!("请求未修改，无需更新记录");
            }
        } else {
            println!("未找到匹配记录，使用原始请求ID: {}", request_id);
        }

        // 如果是CONNECT请求，直接结束
        if let Some(method) = request.get("method") {
            if method.as_str() == Some("CONNECT") {
                println!("CONNECT请求，直接返回");
                return Ok("".to_string());
            }
        }

        // 创建请求控制通道
        let (response_tx, response_rx) = oneshot::channel();

        // 从请求中提取数据
        let method = request
            .get("method")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());
        let url = request
            .get("url")
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());
        let headers = if let Some(h) = request.get("headers") {
            if let Some(obj) = h.as_object() {
                let mut map = HashMap::new();
                for (key, value) in obj {
                    if let Some(v) = value.as_str() {
                        map.insert(key.clone(), v.to_string());
                    }
                }
                Some(map)
            } else {
                None
            }
        } else {
            None
        };
        let body = request
            .get("body")
            .and_then(|b| b.as_str())
            .map(|s| s.to_string());

        // 发送转发命令
        match self
            .request_control_tx
            .send(RequestInterceptControl::Forward {
                req_id: request_id.clone(),
                method,
                url,
                headers,
                body,
                response_tx,
            })
            .await
        {
            Ok(_) => {
                println!("发送转发命令成功，等待处理: ID={}", request_id);
            }
            Err(e) => {
                let error_msg = format!("发送转发命令失败: {}", e);
                println!("{}", error_msg);
                return Err(error_msg);
            }
        }

        // 等待操作完成，增加超时处理
        match tokio::time::timeout(tokio::time::Duration::from_secs(10), response_rx).await {
            Ok(result) => match result {
                Ok(result) => match result {
                    Ok(_) => {
                        println!("成功转发请求: ID={}", request_id);
                        Ok("请求已成功转发".to_string())
                    }
                    Err(e) => {
                        println!("转发请求失败: {}", e);
                        Err(e)
                    }
                },
                Err(e) => {
                    let error_msg = format!("等待转发请求响应失败: {}", e);
                    println!("{}", error_msg);
                    Err(error_msg)
                }
            },
            Err(_) => {
                println!("等待转发请求响应超时");
                Err("转发请求超时".to_string())
            }
        }
    }

    // 更新拦截状态
    pub async fn update_intercept_status(&self, enabled: bool) -> Result<(), String> {
        // 更新状态
        self.intercept_state.set_intercept_enabled(enabled);

        // 直接向代理发送拦截状态更新事件
        self.app
            .emit(
                "proxy-intercept-status-change",
                serde_json::json!({
                    "enabled": enabled
                }),
            )
            .map_err(|e| format!("发送拦截状态更新事件失败: {}", e))?;

        Ok(())
    }

    // 更新请求拦截状态
    pub async fn update_intercept_request_status(&self, enabled: bool) -> Result<(), String> {
        // 更新状态
        self.intercept_state.set_request_enabled(enabled);

        // 如果关闭拦截，自动转发所有当前被拦截的请求
        if !enabled {
            // 获取所有活跃的拦截请求ID
            let active_request_ids = self.http_interceptor.get_active_request_ids().await;
            println!(
                "关闭拦截，自动转发 {} 个未处理的请求",
                active_request_ids.len()
            );

            // 转发每个未处理的请求
            for req_id in active_request_ids {
                println!("自动转发请求: ID={}", req_id);
                // 使用原始请求信息转发
                match self.forward_intercepted(req_id.to_string()).await {
                    Ok(_) => println!("自动转发拦截请求成功: {}", req_id),
                    Err(e) => println!("自动转发拦截请求失败: {} - {}", req_id, e),
                }
            }
        }

        // 直接向代理发送拦截状态更新事件
        // self.app.emit("proxy-intercept-status-change", serde_json::json!({
        //     "enabled": enabled
        // })).map_err(|e| format!("发送拦截状态更新事件失败: {}", e))?;

        Ok(())
    }

    // 获取请求拦截状态
    pub fn get_intercept_request_status(&self) -> bool {
        self.intercept_state.is_request_enabled()
    }

    // 更新响应拦截状态
    pub async fn update_intercept_response_status(&self, enabled: bool) -> Result<(), String> {
        // 更新响应拦截状态
        self.intercept_state.set_response_enabled(enabled);

        // 直接向代理发送响应拦截状态更新事件
        self.app
            .emit(
                "proxy-intercept-response-status-change",
                serde_json::json!({
                    "enabled": enabled
                }),
            )
            .map_err(|e| format!("发送响应拦截状态更新事件失败: {}", e))?;

        Ok(())
    }

    // 获取响应拦截状态
    pub fn get_intercept_response_status(&self) -> bool {
        self.intercept_state.is_response_enabled()
    }

    // 获取响应控制发送器 - 用于外部处理响应
    pub fn get_response_control_tx(&self) -> mpsc::Sender<ResponseInterceptControl> {
        self.response_control_tx.clone()
    }

    /// 获取代理配置
    pub fn config(&self) -> &ProxyConfig {
        &self.config
    }
}

// 将等待端口释放的函数设置为公共函数，以便其他模块可以使用
pub fn wait_for_port_release(port: u16, max_wait_ms: u64) -> bool {
    let addr = format!("127.0.0.1:{}", port);
    let start = std::time::Instant::now();
    
    // 设置更长的最小等待时间，确保有足够时间释放端口
    let max_wait = if max_wait_ms < 10000 { 10000 } else { max_wait_ms };
    
    // 先等待一段时间，让操作系统有机会释放端口
    thread::sleep(Duration::from_millis(1000));
    
    // 设置重试次数和间隔
    let mut retry_count = 0;
    let max_retries = 5;
    let retry_interval_ms = max_wait / (max_retries as u64 + 1);
    
    while start.elapsed().as_millis() < max_wait as u128 {
        // 尝试绑定端口
        if let Ok(listener) = TcpListener::bind(&addr) {
            // 成功绑定，先释放监听器
            drop(listener);
            
            // 再次等待一小段时间，确保端口完全释放
            thread::sleep(Duration::from_millis(500));
            
            // 进行二次验证，确保端口真的可用
            if TcpListener::bind(&addr).is_ok() {
                println!("端口 {} 已成功释放", port);
                return true;
            }
        }
        
        // 记录重试次数
        retry_count += 1;
        println!("等待端口 {} 释放中... (尝试 {}/{})", port, retry_count, max_retries);
        
        // 如果端口还未释放，等待一段时间后重试
        thread::sleep(Duration::from_millis(retry_interval_ms));
    }
    
    // 超过最大等待时间，端口仍未释放
    println!("等待端口 {} 释放超时，已等待 {}ms", port, max_wait);
    false
}

// 检查端口是否可用
pub fn is_port_available(port: u16) -> bool {
    let addr = format!("127.0.0.1:{}", port);
    match TcpListener::bind(&addr) {
        Ok(listener) => {
            // 成功绑定，释放监听器
            drop(listener);
            true
        }
        Err(_) => false
    }
}

#[derive(Clone)]
struct ProxyHandler {
    app: AppHandle,
    store: Arc<RequestStore>,
    interceptor: Arc<HttpInterceptor>,
    intercept_state: Arc<SharedInterceptState>,
}

impl HttpHandler for ProxyHandler {
    async fn handle_request(&mut self, ctx: &HttpContext, req: Request<Body>) -> RequestOrResponse {
        if req.method() == Method::CONNECT {
            return RequestOrResponse::Request(req);
        }

        let (parts, body) = req.into_parts();

        // 记录请求信息
        let method = parts.method.to_string();
        let uri = parts.uri.to_string();
        let version = format!("{:?}", parts.version);
        let parts_clone = parts.clone();
        let bytes: Bytes = body.collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&bytes.to_vec()).to_string();

        // 转换请求头为HashMap
        let mut headers_map = HashMap::new();
        for (name, value) in parts_clone.headers.iter() {
            if let Ok(v) = value.to_str() {
                headers_map.insert(name.to_string(), v.to_string());
            }
        }

        println!("收到请求 {} {} - {}", method, uri, version);

        // 应用已保存的Cookie到请求中
        self.intercept_state.cookie_store.apply_cookies(&uri, &mut headers_map).await;

        // 生成UUID作为请求ID
        let request_id = self.store.next_request_id().to_string();
        println!("生成请求ID: {}", request_id);

        // 构建连接ID - 使用多种格式以增加后续匹配的成功率
        let standard_connection_id = format!("{}:{}", ctx.client_addr, request_id);

        // 保存不同格式的连接ID，以便后续匹配
        let connection_ids = vec![
            format!("{}", ctx.client_addr),                // 只有客户端地址
            format!("{}:{}", ctx.client_addr, request_id), // 客户端地址+请求ID
            format!("{}+{}", ctx.client_addr, request_id), // 使用+分隔
            request_id.clone(),                            // 只使用请求ID
        ];

        println!("连接ID: {}", standard_connection_id);

        // 创建请求记录 - 使用UUID作为ID
        let record = RequestRecord::new_with_id(
            request_id.clone(),
            method.clone(),
            uri.clone(),
            headers_map.clone(), // 使用更新后的headers（可能包含Cookie）
            body_str.clone(),
        );

        // 额外保存所有可能的连接ID格式，用于后续匹配响应
        for conn_id in &connection_ids {
            self.store.save_connection_info(conn_id, &request_id).await;
            println!("保存连接信息: {} -> {}", conn_id, request_id);
        }

        // 如果启用了拦截并且启用了请求拦截
        if self.intercept_state.is_intercept_enabled() && self.intercept_state.is_request_enabled()
        {
            // 先使用规则判断是否需要拦截
            let proxy_state: tauri::State<'_, ProxyState> =
                tauri::Manager::state::<ProxyState>(&self.app);
            let should_intercept = proxy_state
                .should_intercept_request(&method, &uri, &headers_map.clone())
                .await;

            if should_intercept {
                // 如果规则匹配，进行拦截处理
                match self
                    .interceptor
                    .intercept_request(&method, &uri, headers_map.clone(), body_str.clone())
                    .await
                {
                    Ok((Some(new_method), Some(new_url), Some(new_headers), Some(new_body))) => {
                        // 用户修改了全部内容：方法、URL、请求头和请求体
                        let mut updated_record = record.clone();
                        updated_record.method = new_method.clone();
                        updated_record.url = new_url.clone();
                        updated_record.request_headers = new_headers.clone();
                        updated_record.request_body = new_body.clone();

                        // 保存最终的请求记录
                        self.store.add_record(updated_record.clone()).await;
                        println!("请求记录已保存(完全修改): ID={}", request_id);

                        // 发送请求已接收事件
                        self.app
                            .emit("proxy-request-received", updated_record)
                            .unwrap_or_else(|e| {
                                println!("发送请求接收事件失败: {}", e);
                            });

                        // 解析新的URI
                        let new_uri = new_url.parse::<http::Uri>().unwrap_or(parts.uri);

                        // 解析HTTP方法
                        let method = http::Method::from_bytes(new_method.as_bytes())
                            .unwrap_or(parts.method.clone());

                        // 使用修改后的方法、URL、请求头和请求体创建新请求
                        let mut request_builder = Request::builder()
                            .method(method)
                            .uri(new_uri)
                            .version(parts.version);

                        // 添加头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in new_headers {
                            if let (Ok(name), Ok(value)) =
                                (HeaderName::from_str(&name), HeaderValue::from_str(&value))
                            {
                                headers.insert(name, value);
                            }
                        }

                        // 创建新的请求体
                        let new_body = Body::from(new_body);
                        let new_request = request_builder.body(new_body).unwrap();

                        return new_request.into();
                    }
                    Ok((Some(new_method), Some(new_url), Some(new_headers), None)) => {
                        // 用户修改了方法、URL和请求头
                        let mut updated_record = record.clone();
                        updated_record.method = new_method.clone();
                        updated_record.url = new_url.clone();
                        updated_record.request_headers = new_headers.clone();

                        // 保存最终的请求记录
                        self.store.add_record(updated_record.clone()).await;
                        println!("请求记录已保存(修改方法、URL和头部): ID={}", request_id);

                        // 发送请求已接收事件
                        self.app
                            .emit("proxy-request-received", updated_record)
                            .unwrap_or_else(|e| {
                                println!("发送请求接收事件失败: {}", e);
                            });

                        // 解析新的URI
                        let new_uri = new_url.parse::<http::Uri>().unwrap_or(parts.uri);

                        // 解析HTTP方法
                        let method = http::Method::from_bytes(new_method.as_bytes())
                            .unwrap_or(parts.method.clone());

                        let mut request_builder = Request::builder()
                            .method(method)
                            .uri(new_uri)
                            .version(parts.version);

                        // 添加头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in new_headers {
                            if let (Ok(name), Ok(value)) =
                                (HeaderName::from_str(&name), HeaderValue::from_str(&value))
                            {
                                headers.insert(name, value);
                            }
                        }

                        // 使用原始请求体
                        let new_request = request_builder.body(Body::from(body_str)).unwrap();

                        return new_request.into();
                    }
                    Ok((Some(new_method), Some(new_url), None, Some(new_body))) => {
                        // 用户修改了方法、URL和请求体
                        let mut updated_record = record.clone();
                        updated_record.method = new_method.clone();
                        updated_record.url = new_url.clone();
                        updated_record.request_body = new_body.clone();

                        // 保存最终的请求记录
                        self.store.add_record(updated_record.clone()).await;
                        println!("请求记录已保存(修改方法、URL和请求体): ID={}", request_id);

                        // 发送请求已接收事件
                        self.app
                            .emit("proxy-request-received", updated_record)
                            .unwrap_or_else(|e| {
                                println!("发送请求接收事件失败: {}", e);
                            });

                        // 解析新的URI
                        let new_uri = new_url.parse::<http::Uri>().unwrap_or(parts.uri);

                        // 解析HTTP方法
                        let method = http::Method::from_bytes(new_method.as_bytes())
                            .unwrap_or(parts.method.clone());

                        let mut request_builder = Request::builder()
                            .method(method)
                            .uri(new_uri)
                            .version(parts.version);

                        // 添加原始头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in parts.headers {
                            if let Some(name) = name {
                                headers.insert(name, value);
                            }
                        }

                        // 使用新请求体
                        let new_request = request_builder.body(Body::from(new_body)).unwrap();

                        return new_request.into();
                    }
                    Ok((Some(new_method), Some(new_url), None, None)) => {
                        // 用户修改了方法和URL
                        println!(
                            "处理修改方法和URL的情况: 原方法={}, 新方法={}, 原URL={}, 新URL={}",
                            method, new_method, uri, new_url
                        );
                        let mut updated_record = record.clone();
                        updated_record.method = new_method.clone();
                        updated_record.url = new_url.clone();

                        // 保存最终的请求记录
                        self.store.add_record(updated_record.clone()).await;
                        println!("请求记录已保存(修改方法和URL): ID={}", request_id);

                        // 发送请求已接收事件
                        self.app
                            .emit("proxy-request-received", updated_record)
                            .unwrap_or_else(|e| {
                                println!("发送请求接收事件失败: {}", e);
                            });

                        // 解析新的URI
                        let new_uri = new_url.parse::<http::Uri>().unwrap_or(parts.uri);

                        // 解析HTTP方法
                        let method = http::Method::from_bytes(new_method.as_bytes())
                            .unwrap_or(parts.method.clone());
                        println!("构建请求使用新方法: {}, 新URL: {}", method, new_uri);

                        let mut request_builder = Request::builder()
                            .method(method)
                            .uri(new_uri)
                            .version(parts.version);

                        // 添加原始头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in parts.headers {
                            if let Some(name) = name {
                                headers.insert(name, value);
                            }
                        }

                        // 使用原始请求体
                        let new_request = request_builder.body(Body::from(body_str)).unwrap();

                        return new_request.into();
                    }
                    Ok((None, Some(new_url), None, None)) => {
                        // 用户只修改了URL
                        println!("处理只修改URL的情况: 原URL={}, 新URL={}", uri, new_url);
                        let mut updated_record = record.clone();
                        updated_record.url = new_url.clone();

                        // 保存最终的请求记录
                        self.store.add_record(updated_record.clone()).await;
                        println!("请求记录已保存(仅修改URL): ID={}", request_id);

                        // 发送请求已接收事件
                        self.app
                            .emit("proxy-request-received", updated_record)
                            .unwrap_or_else(|e| {
                                println!("发送请求接收事件失败: {}", e);
                            });

                        // 解析新的URI
                        let new_uri = new_url.parse::<http::Uri>().unwrap_or(parts.uri);
                        println!("构建请求使用新URL: {}", new_uri);

                        let mut request_builder = Request::builder()
                            .method(parts.method.clone())
                            .uri(new_uri)
                            .version(parts.version);

                        // 添加原始头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in parts.headers {
                            if let Some(name) = name {
                                headers.insert(name, value);
                            }
                        }

                        // 使用原始请求体
                        let new_request = request_builder.body(Body::from(body_str)).unwrap();

                        return new_request.into();
                    }
                    Ok((None, None, Some(new_headers), Some(new_body))) => {
                        // 用户修改了头部和请求体
                        println!("处理修改头部和请求体的情况");
                        let mut updated_record = record.clone();
                        updated_record.request_headers = new_headers.clone();
                        updated_record.request_body = new_body.clone();

                        // 保存最终的请求记录
                        self.store.add_record(updated_record.clone()).await;
                        println!("请求记录已保存(修改头部和请求体): ID={}", request_id);

                        // 发送请求已接收事件
                        self.app
                            .emit("proxy-request-received", updated_record)
                            .unwrap_or_else(|e| {
                                println!("发送请求接收事件失败: {}", e);
                            });

                        // 使用修改后的头部和请求体创建新请求
                        let mut request_builder = Request::builder()
                            .method(parts.method.clone())
                            .uri(parts.uri)
                            .version(parts.version);

                        // 添加头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in new_headers {
                            if let (Ok(name), Ok(value)) =
                                (HeaderName::from_str(&name), HeaderValue::from_str(&value))
                            {
                                headers.insert(name, value);
                            }
                        }

                        // 创建新的请求体
                        let new_body = Body::from(new_body);
                        let new_request = request_builder.body(new_body).unwrap();

                        return new_request.into();
                    }
                    Ok((None, Some(new_url), Some(new_headers), Some(new_body))) => {
                        // 用户修改了URL、头部和请求体
                        println!(
                            "处理修改URL、头部和请求体的情况: 原URL={}, 新URL={}",
                            uri, new_url
                        );
                        let mut updated_record = record.clone();
                        updated_record.url = new_url.clone();
                        updated_record.request_headers = new_headers.clone();
                        updated_record.request_body = new_body.clone();

                        // 保存最终的请求记录
                        self.store.add_record(updated_record.clone()).await;
                        println!("请求记录已保存(修改URL、头部和请求体): ID={}", request_id);

                        // 发送请求已接收事件
                        self.app
                            .emit("proxy-request-received", updated_record)
                            .unwrap_or_else(|e| {
                                println!("发送请求接收事件失败: {}", e);
                            });

                        // 解析新的URI
                        let new_uri = new_url.parse::<http::Uri>().unwrap_or(parts.uri);

                        // 使用修改后的头部和请求体创建新请求
                        let mut request_builder = Request::builder()
                            .method(parts.method.clone())
                            .uri(new_uri)
                            .version(parts.version);

                        // 添加头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in new_headers {
                            if let (Ok(name), Ok(value)) =
                                (HeaderName::from_str(&name), HeaderValue::from_str(&value))
                            {
                                headers.insert(name, value);
                            }
                        }

                        // 创建新的请求体
                        let new_body = Body::from(new_body);
                        let new_request = request_builder.body(new_body).unwrap();

                        return new_request.into();
                    }
                    Ok((None, None, None, Some(new_body))) => {
                        // 用户只修改了请求体
                        println!("处理只修改请求体的情况");
                        let mut updated_record = record.clone();
                        updated_record.request_body = new_body.clone();

                        // 保存最终的请求记录
                        self.store.add_record(updated_record.clone()).await;
                        println!("请求记录已保存(仅修改请求体): ID={}", request_id);

                        let mut request_builder = Request::builder()
                            .method(parts.method.clone())
                            .uri(parts.uri)
                            .version(parts.version);

                        // 添加原始头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in parts.headers {
                            if let Some(name) = name {
                                headers.insert(name, value);
                            }
                        }

                        // 使用新请求体
                        let new_request = request_builder.body(Body::from(new_body)).unwrap();

                        return new_request.into();
                    }
                    Ok((None, None, Some(new_headers), None)) => {
                        // 用户只修改了请求头
                        println!("处理只修改头部的情况");
                        let mut updated_record = record.clone();
                        updated_record.request_headers = new_headers.clone();

                        // 保存最终的请求记录
                        self.store.add_record(updated_record.clone()).await;
                        println!("请求记录已保存(仅修改头部): ID={}", request_id);

                        // 发送请求已接收事件
                        self.app
                            .emit("proxy-request-received", updated_record)
                            .unwrap_or_else(|e| {
                                println!("发送请求接收事件失败: {}", e);
                            });

                        // 使用修改后的头部创建新请求
                        let mut request_builder = Request::builder()
                            .method(parts.method.clone())
                            .uri(parts.uri)
                            .version(parts.version);

                        // 添加头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in new_headers {
                            if let (Ok(name), Ok(value)) =
                                (HeaderName::from_str(&name), HeaderValue::from_str(&value))
                            {
                                headers.insert(name, value);
                            }
                        }

                        // 使用原始请求体
                        let new_request = request_builder.body(Body::from(body_str)).unwrap();

                        return new_request.into();
                    }
                    Ok((None, Some(new_url), None, Some(new_body))) => {
                        // 用户修改了URL和请求体
                        println!(
                            "处理修改URL和请求体的情况: 原URL={}, 新URL={}",
                            uri, new_url
                        );
                        let mut updated_record = record.clone();
                        updated_record.url = new_url.clone();
                        updated_record.request_body = new_body.clone();

                        // 保存最终的请求记录
                        self.store.add_record(updated_record.clone()).await;
                        println!("请求记录已保存(修改URL和请求体): ID={}", request_id);

                        // 发送请求已接收事件
                        self.app
                            .emit("proxy-request-received", updated_record)
                            .unwrap_or_else(|e| {
                                println!("发送请求接收事件失败: {}", e);
                            });

                        // 解析新的URI
                        let new_uri = new_url.parse::<http::Uri>().unwrap_or(parts.uri);

                        let mut request_builder = Request::builder()
                            .method(parts.method.clone())
                            .uri(new_uri)
                            .version(parts.version);

                        // 添加原始头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in parts.headers {
                            if let Some(name) = name {
                                headers.insert(name, value);
                            }
                        }

                        // 使用新请求体
                        let new_request = request_builder.body(Body::from(new_body)).unwrap();

                        return new_request.into();
                    }
                    Ok((None, Some(new_url), Some(new_headers), None)) => {
                        // 用户修改了URL和请求头
                        println!("处理修改URL和头部的情况: 原URL={}, 新URL={}", uri, new_url);
                        let mut updated_record = record.clone();
                        updated_record.url = new_url.clone();
                        updated_record.request_headers = new_headers.clone();

                        // 保存最终的请求记录
                        self.store.add_record(updated_record.clone()).await;
                        println!("请求记录已保存(修改URL和头部): ID={}", request_id);

                        // 发送请求已接收事件
                        self.app
                            .emit("proxy-request-received", updated_record)
                            .unwrap_or_else(|e| {
                                println!("发送请求接收事件失败: {}", e);
                            });

                        // 解析新的URI
                        let new_uri = new_url.parse::<http::Uri>().unwrap_or(parts.uri);

                        let mut request_builder = Request::builder()
                            .method(parts.method.clone())
                            .uri(new_uri)
                            .version(parts.version);

                        // 添加头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in new_headers {
                            if let (Ok(name), Ok(value)) =
                                (HeaderName::from_str(&name), HeaderValue::from_str(&value))
                            {
                                headers.insert(name, value);
                            }
                        }

                        // 使用原始请求体
                        let new_request = request_builder.body(Body::from(body_str)).unwrap();

                        return new_request.into();
                    }
                    Ok((Some(new_method), None, None, None)) => {
                        // 用户只修改了方法
                        println!(
                            "处理只修改方法的情况: 原方法={}, 新方法={}",
                            method, new_method
                        );
                        let mut updated_record = record.clone();
                        updated_record.method = new_method.clone();

                        // 保存最终的请求记录
                        self.store.add_record(updated_record.clone()).await;
                        println!("请求记录已保存(仅修改方法): ID={}", request_id);

                        // 发送请求已接收事件
                        self.app
                            .emit("proxy-request-received", updated_record)
                            .unwrap_or_else(|e| {
                                println!("发送请求接收事件失败: {}", e);
                            });

                        // 解析HTTP方法
                        let method = http::Method::from_bytes(new_method.as_bytes())
                            .unwrap_or(parts.method.clone());

                        let mut request_builder = Request::builder()
                            .method(method)
                            .uri(parts.uri)
                            .version(parts.version);

                        // 添加原始头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in parts.headers {
                            if let Some(name) = name {
                                headers.insert(name, value);
                            }
                        }

                        // 使用原始请求体
                        let new_request = request_builder.body(Body::from(body_str)).unwrap();

                        return new_request.into();
                    }
                    Ok((None, None, None, None)) => {
                        // 用户未修改请求，保存原始记录
                        self.store.add_record(record.clone()).await;
                        println!("请求记录已保存(未修改): ID={}", request_id);

                        // 发送请求已接收事件
                        self.app
                            .emit("proxy-request-received", record)
                            .unwrap_or_else(|e| {
                                println!("发送请求接收事件失败: {}", e);
                            });

                        // 重新构建请求
                        let mut request_builder = Request::builder()
                            .method(parts.method.clone())
                            .uri(parts.uri)
                            .version(parts.version);

                        // 添加原始头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in parts.headers {
                            if let Some(name) = name {
                                headers.insert(name, value);
                            }
                        }

                        let new_request = request_builder.body(Body::from(body_str)).unwrap();

                        return new_request.into();
                    }
                    // 处理其他未明确处理的组合模式
                    Ok(pattern) => {
                        println!("处理未明确匹配的请求修改模式: {:?}", pattern);
                        // 直接保存原始记录，不进行修改
                        self.store.add_record(record.clone()).await;
                        println!("请求记录已保存(未知模式): ID={}", request_id);

                        // 发送请求已接收事件
                        self.app
                            .emit("proxy-request-received", record)
                            .unwrap_or_else(|e| {
                                println!("发送请求接收事件失败: {}", e);
                            });

                        // 重新构建请求
                        let mut request_builder = Request::builder()
                            .method(parts.method.clone())
                            .uri(parts.uri)
                            .version(parts.version);

                        // 添加原始头部
                        let headers = request_builder.headers_mut().unwrap();
                        for (name, value) in parts.headers {
                            if let Some(name) = name {
                                headers.insert(name, value);
                            }
                        }

                        let new_request = request_builder.body(Body::from(body_str)).unwrap();

                        return new_request.into();
                    }
                    Err(e) => {
                        if e.contains("dropped") {
                            // 用户丢弃了请求，返回403响应
                            println!("请求被用户丢弃: {}", request_id);

                            // 保存记录并标记为丢弃
                            let mut dropped_record = record.clone();
                            dropped_record.status = 403; // 使用403表示拒绝
                            dropped_record.response_body = "请求被丢弃".to_string();
                            self.store.add_record(dropped_record.clone()).await;

                            // 发送请求已接收事件
                            self.app
                                .emit("proxy-request-received", dropped_record)
                                .unwrap_or_else(|e| {
                                    println!("发送请求接收事件失败: {}", e);
                                });

                            let response = Response::builder()
                                .status(403)
                                .body(Body::from("请求被丢弃"))
                                .unwrap();
                            return response.into();
                        } else {
                            // 其他拦截错误，记录原始请求
                            self.store.add_record(record.clone()).await;
                            println!("请求拦截错误: {}, ID={}", e, request_id);

                            // 发送请求已接收事件
                            self.app
                                .emit("proxy-request-received", record)
                                .unwrap_or_else(|e| {
                                    println!("发送请求接收事件失败: {}", e);
                                });

                            // 返回错误响应
                            let response = Response::builder()
                                .status(500)
                                .body(Body::from(format!("请求拦截错误: {}", e)))
                                .unwrap();

                            return response.into();
                        }
                    }
                }
            } else {
                // 规则不匹配，直接转发请求
                println!("请求不符合拦截规则，直接转发: {} {}", method, uri);

                // 保存最终的请求记录
                self.store.add_record(record.clone()).await;

                // 发送请求已接收事件
                self.app
                    .emit("proxy-request-received", record)
                    .unwrap_or_else(|e| {
                        println!("发送请求接收事件失败: {}", e);
                    });

                // 重新构建请求
                let mut request_builder = Request::builder()
                    .method(parts.method.clone())
                    .uri(parts.uri)
                    .version(parts.version);

                // 使用headers_map而不是原始headers，确保应用Cookie
                let headers = request_builder.headers_mut().unwrap();
                for (name, value) in &headers_map {
                    if let (Ok(header_name), Ok(header_value)) = 
                       (HeaderName::from_str(name), HeaderValue::from_str(value)) {
                        headers.insert(header_name, header_value);
                    }
                }

                let new_request = request_builder.body(Body::from(body_str)).unwrap();

                return new_request.into();
            }
        } else {
            // 拦截未启用，直接保存原始记录并转发
            self.store.add_record(record.clone()).await;
            println!("请求记录已保存(拦截未启用): ID={}", request_id);

            // 发送请求已接收事件
            self.app
                .emit("proxy-request-received", record)
                .unwrap_or_else(|e| {
                    println!("发送请求接收事件失败: {}", e);
                });

            // 重新构建请求
            let mut request_builder = Request::builder()
                .method(parts.method.clone())
                .uri(parts.uri)
                .version(parts.version);

            // 使用headers_map而不是原始headers，确保应用Cookie
            let headers = request_builder.headers_mut().unwrap();
            for (name, value) in &headers_map {
                if let (Ok(header_name), Ok(header_value)) = 
                   (HeaderName::from_str(name), HeaderValue::from_str(value)) {
                    headers.insert(header_name, header_value);
                }
            }

            let new_request = request_builder.body(Body::from(body_str)).unwrap();

            return new_request.into();
        }
    }

    async fn handle_response(&mut self, ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        let (parts, body) = res.into_parts();
        let parts_clone = parts.clone();

        // 记录响应信息
        let status = parts_clone.status.as_u16();
        let version = format!("{:?}", parts_clone.version);
        let bytes: Bytes = match body.collect().await {
            Ok(bytes) => bytes.to_bytes(),
            Err(e) => {
                println!("读取响应体失败: {}", e);
                return Response::builder()
                    .status(500)
                    .body(Body::from("读取响应体失败"))
                    .unwrap()
                    .into();
            }
        };
        println!("收到响应: 状态码={}, 大小={}字节", status, bytes.len());

        // 判断body是否gzip，如果是则解压再保存
        let body_str = String::from_utf8_lossy(&bytes.to_vec()).to_string();

        let body_str = if let Some(encoding) = parts_clone.headers.get("content-encoding") {
            if encoding.to_str().unwrap_or("") == "gzip" {
                println!("检测到gzip编码，尝试解压");

                // 确保有数据可以解压
                if bytes.len() > 0 {
                    // 使用std::io的GzDecoder代替tokio的GzipDecoder
                    let mut decoder = GzDecoder::new(bytes.as_ref());
                    let mut buffer = Vec::new();

                    match decoder.read_to_end(&mut buffer) {
                        Ok(size) => {
                            println!("解压后大小: {}", size);
                            if size > 0 {
                                String::from_utf8_lossy(&buffer).to_string()
                            } else {
                                println!("解压后大小为0，使用原始数据");
                                body_str
                            }
                        }
                        Err(e) => {
                            println!("解压失败: {}", e);
                            // 解压失败时返回原始数据
                            body_str
                        }
                    }
                } else {
                    println!("空的gzip数据");
                    body_str
                }
            } else {
                println!("未知的编码方式: {}", encoding.to_str().unwrap_or(""));
                body_str
            }
        } else {
            body_str
        };
        println!("解析后的响应体长度: {}", body_str.len());

        // 转换响应头为HashMap
        let mut headers_map = HashMap::new();
        for (name, value) in parts_clone.headers.iter() {
            if let Ok(v) = value.to_str() {
                headers_map.insert(name.to_string(), v.to_string());
            }
        }

        // 构建多种可能的连接标识符形式，用于查找关联的请求ID
        let mut connection_keys = Vec::new();

        // 1. 只包含客户端地址的标准连接键
        connection_keys.push(format!("{}", ctx.client_addr));

        // 2. 尝试获取最近的记录，检查它们的ID是否能用于构建连接ID
        let recent_records = self.store.get_recent_records(10).await;
        for record in &recent_records {
            // 添加使用客户端地址和记录ID组合的可能形式
            connection_keys.push(format!("{}:{}", ctx.client_addr, record.id));
            connection_keys.push(format!("{}+{}", ctx.client_addr, record.id));
            // 也尝试仅使用记录ID
            connection_keys.push(record.id.to_string());
        }

        // 3. 在连接键周围搜索 (添加索引以处理可能的连接序列)
        for i in 0..5 {
            connection_keys.push(format!("{}:{}", ctx.client_addr, i));
        }

        println!(
            "尝试使用 {} 种连接标识符查找关联请求",
            connection_keys.len()
        );

        // 保存查找结果
        let mut request_id_opt = None;
        let mut request_url = String::new();

        // 尝试所有可能的键进行查找
        for key in &connection_keys {
            if let Some(id) = self.store.get_request_id_by_connection(key).await {
                println!("找到关联请求ID: {} (来自连接: {})", id, key);
                request_id_opt = Some(id.clone());
                
                // 获取请求URL以存储cookie
                if let Some(record) = self.store.get_record(&id).await {
                    request_url = record.url.clone();
                }
                break;
            }
        }

        // 如果有关联的请求URL，存储响应中的cookie
        if !request_url.is_empty() {
            self.intercept_state.cookie_store.store_cookies(&request_url, &headers_map).await;
        }

        // 如果找不到或获取ID失败，则无法关联响应
        if request_id_opt.is_none() {
            println!("警告: 无法找到关联的请求ID，尝试查找未完成的记录");

            // 如果找不到关联，查找未完成的记录
            if let Some(recent_record) = recent_records.into_iter().find(|r| !r.is_complete()) {
                println!("找到未完成记录: ID={}", recent_record.id);
                let _ = self
                    .store
                    .update_record_with_response(
                        &recent_record.id,
                        status,
                        headers_map.clone(),
                        body_str.clone(),
                    )
                    .await;

                // 发送请求完成事件
                let updated_record = self
                    .store
                    .get_record(&recent_record.id)
                    .await
                    .unwrap_or(recent_record);
                self.app
                    .emit("proxy-request-completed", updated_record)
                    .unwrap_or_else(|e| {
                        println!("发送请求完成事件失败: {}", e);
                    });
            } else if let Some(latest_record) = self.store.get_latest_record().await {
                println!("无未完成记录，回退到更新最新记录: ID={}", latest_record.id);
                let _ = self
                    .store
                    .update_record_with_response(
                        &latest_record.id,
                        status,
                        headers_map.clone(),
                        body_str.clone(),
                    )
                    .await;

                // 发送请求完成事件
                self.app
                    .emit("proxy-request-completed", latest_record)
                    .unwrap_or_else(|e| {
                        println!("发送请求完成事件失败: {}", e);
                    });
            } else {
                println!("错误: 没有找到任何记录来更新响应信息");
            }

            // 由于无法关联，直接返回原始响应
            return Response::from_parts(parts, Full::new(bytes).into());
        }

        let request_id = request_id_opt.unwrap();
        println!("关联请求ID: {}", request_id);

        // 检查是否启用了响应拦截
        if self.intercept_state.is_intercept_enabled() && self.intercept_state.is_response_enabled()
        {
            println!("启用了响应拦截");
            // 从请求记录获取URL
            let request_url = if let Some(record) = self.store.get_record(&request_id).await {
                record.url
            } else {
                println!("警告: 找不到ID={}的请求记录来获取URL", request_id);
                "unknown".to_string()
            };

            let proxy_state = tauri::Manager::state::<ProxyState>(&self.app);
            let should_intercept = proxy_state
                .should_intercept_response(status, &request_url, &headers_map.clone())
                .await;

            if should_intercept {
                // 如果规则匹配，进行拦截处理
                match self
                    .interceptor
                    .intercept_response(&request_id, status, headers_map.clone(), body_str.clone())
                    .await
                {
                    Ok((Some(new_status), Some(new_headers), Some(new_body))) => {
                        // 用户修改了状态码、头部和主体
                        println!("用户修改了响应 [状态码、头部和主体]");

                        // 更新记录，使用修改后的响应
                        if let Some(updated_record) = self
                            .store
                            .update_record_with_response(
                                &request_id,
                                new_status,
                                new_headers.clone(),
                                new_body.clone(),
                            )
                            .await
                        {
                            // 发送请求完成事件
                            self.app
                                .emit("proxy-request-completed", updated_record)
                                .unwrap_or_else(|e| {
                                    println!("发送请求完成事件失败: {}", e);
                                });
                        }

                        // 构建修改后的响应
                        let mut response_builder = Response::builder()
                            .status(new_status)
                            .version(parts_clone.version);
                        let headers = response_builder.headers_mut().unwrap();
                        for (name, value) in new_headers {
                            if let (Ok(name), Ok(value)) =
                                (HeaderName::from_str(&name), HeaderValue::from_str(&value))
                            {
                                headers.insert(name, value);
                            }
                        }
                        return response_builder.body(Body::from(new_body)).unwrap();
                    }
                    Ok((Some(new_status), Some(new_headers), None)) => {
                        // 用户只修改了状态码和头部
                        println!("用户修改了响应 [状态码和头部]");

                        // 更新记录，使用修改后的状态码和头部，原始主体
                        if let Some(updated_record) = self
                            .store
                            .update_record_with_response(
                                &request_id,
                                new_status,
                                new_headers.clone(),
                                body_str.clone(),
                            )
                            .await
                        {
                            // 发送请求完成事件
                            self.app
                                .emit("proxy-request-completed", updated_record)
                                .unwrap_or_else(|e| {
                                    println!("发送请求完成事件失败: {}", e);
                                });
                        }

                        // 构建修改后的响应
                        let mut response_builder = Response::builder()
                            .status(new_status)
                            .version(parts_clone.version);

                        // 添加修改后的头部
                        let headers = response_builder.headers_mut().unwrap();
                        for (name, value) in new_headers {
                            if let (Ok(name), Ok(value)) =
                                (HeaderName::from_str(&name), HeaderValue::from_str(&value))
                            {
                                headers.insert(name, value);
                            }
                        }

                        // 使用原始主体
                        return response_builder.body(Full::new(bytes).into()).unwrap();
                    }
                    Ok((Some(new_status), None, Some(new_body))) => {
                        // 用户只修改了状态码和主体
                        println!("用户修改了响应 [状态码和主体]");

                        // 更新记录，使用修改后的状态码和主体，原始头部
                        if let Some(updated_record) = self
                            .store
                            .update_record_with_response(
                                &request_id,
                                new_status,
                                headers_map.clone(),
                                new_body.clone(),
                            )
                            .await
                        {
                            // 发送请求完成事件
                            self.app
                                .emit("proxy-request-completed", updated_record)
                                .unwrap_or_else(|e| {
                                    println!("发送请求完成事件失败: {}", e);
                                });
                        }

                        // 构建修改后的响应
                        let mut response_builder = Response::builder()
                            .status(parts_clone.status)
                            .version(parts_clone.version);

                        // 添加原始头部
                        let headers = response_builder.headers_mut().unwrap();
                        for (name, value) in parts_clone.headers {
                            if let Some(name) = name {
                                headers.insert(name, value);
                            }
                        }

                        // 使用修改后的主体
                        return response_builder.body(Body::from(new_body)).unwrap();
                    }
                    Ok((Some(new_status), None, None)) => {
                        // 用户只修改了状态码
                        println!("用户修改了响应 [仅状态码]");

                        // 更新记录，使用修改后的状态码，原始头部和主体
                        if let Some(updated_record) = self
                            .store
                            .update_record_with_response(
                                &request_id,
                                new_status,
                                headers_map.clone(),
                                body_str.clone(),
                            )
                            .await
                        {
                            // 发送请求完成事件
                            self.app
                                .emit("proxy-request-completed", updated_record)
                                .unwrap_or_else(|e| {
                                    println!("发送请求完成事件失败: {}", e);
                                });
                        }

                        // 构建修改后的响应
                        let mut response_builder = Response::builder()
                            .status(new_status)
                            .version(parts_clone.version);

                        // 添加原始头部
                        let headers = response_builder.headers_mut().unwrap();
                        for (name, value) in parts_clone.headers {
                            if let Some(name) = name {
                                headers.insert(name, value);
                            }
                        }

                        // 使用原始主体
                        return response_builder.body(Full::new(bytes).into()).unwrap();
                    }
                    Ok((None, Some(new_headers), Some(new_body))) => {
                        // 用户只修改了头部和主体
                        println!("用户修改了响应 [头部和主体]");

                        // 更新记录，使用原始状态码，修改后的头部和主体
                        if let Some(updated_record) = self
                            .store
                            .update_record_with_response(
                                &request_id,
                                status,
                                new_headers.clone(),
                                new_body.clone(),
                            )
                            .await
                        {
                            // 发送请求完成事件
                            self.app
                                .emit("proxy-request-completed", updated_record)
                                .unwrap_or_else(|e| {
                                    println!("发送请求完成事件失败: {}", e);
                                });
                        }

                        // 构建修改后的响应
                        let mut response_builder = Response::builder()
                            .status(parts_clone.status)
                            .version(parts_clone.version);

                        // 添加修改后的头部
                        let headers = response_builder.headers_mut().unwrap();
                        for (name, value) in new_headers {
                            if let (Ok(name), Ok(value)) =
                                (HeaderName::from_str(&name), HeaderValue::from_str(&value))
                            {
                                headers.insert(name, value);
                            }
                        }

                        // 使用修改后的主体
                        return response_builder.body(Body::from(new_body)).unwrap();
                    }
                    Ok((None, Some(new_headers), None)) => {
                        // 用户只修改了头部
                        println!("用户修改了响应 [仅头部]");

                        // 更新记录，使用原始状态码和主体，修改后的头部
                        if let Some(updated_record) = self
                            .store
                            .update_record_with_response(
                                &request_id,
                                status,
                                new_headers.clone(),
                                body_str.clone(),
                            )
                            .await
                        {
                            // 发送请求完成事件
                            self.app
                                .emit("proxy-request-completed", updated_record)
                                .unwrap_or_else(|e| {
                                    println!("发送请求完成事件失败: {}", e);
                                });
                        }

                        // 构建修改后的响应
                        let mut response_builder = Response::builder()
                            .status(parts_clone.status)
                            .version(parts_clone.version);

                        // 添加修改后的头部
                        let headers = response_builder.headers_mut().unwrap();
                        for (name, value) in new_headers {
                            if let (Ok(name), Ok(value)) =
                                (HeaderName::from_str(&name), HeaderValue::from_str(&value))
                            {
                                headers.insert(name, value);
                            }
                        }

                        // 使用原始主体
                        return response_builder.body(Full::new(bytes).into()).unwrap();
                    }
                    Ok((None, None, Some(new_body))) => {
                        // 用户只修改了主体
                        println!("用户修改了响应 [仅主体]");

                        // 更新记录，使用原始状态码和头部，修改后的主体
                        if let Some(updated_record) = self
                            .store
                            .update_record_with_response(
                                &request_id,
                                status,
                                headers_map.clone(),
                                new_body.clone(),
                            )
                            .await
                        {
                            // 发送请求完成事件
                            self.app
                                .emit("proxy-request-completed", updated_record)
                                .unwrap_or_else(|e| {
                                    println!("发送请求完成事件失败: {}", e);
                                });
                        }

                        // 构建修改后的响应
                        let mut response_builder = Response::builder()
                            .status(parts_clone.status)
                            .version(parts_clone.version);

                        // 添加原始头部
                        let headers = response_builder.headers_mut().unwrap();
                        for (name, value) in parts_clone.headers {
                            if let Some(name) = name {
                                headers.insert(name, value);
                            }
                        }

                        // 使用修改后的主体
                        return response_builder.body(Body::from(new_body)).unwrap();
                    }
                    Ok((None, None, None)) => {
                        // 用户未修改响应，使用原始响应
                        println!("用户未修改响应，使用原始响应");

                        // 更新记录，使用原始状态码、头部和主体
                        if let Some(updated_record) = self
                            .store
                            .update_record_with_response(&request_id, status, headers_map, body_str)
                            .await
                        {
                            // 发送请求完成事件
                            self.app
                                .emit("proxy-request-completed", updated_record)
                                .unwrap_or_else(|e| {
                                    println!("发送请求完成事件失败: {}", e);
                                });
                        }

                        // 使用原始响应
                        return Response::from_parts(parts, Full::new(bytes).into());
                    }
                    Err(e) => {
                        // 错误处理
                        println!("响应拦截处理错误: {}", e);

                        if e.contains("dropped") {
                            // 用户丢弃了响应，返回错误页面
                            println!("响应被用户丢弃");

                            // 更新记录为丢弃状态
                            let _ = self
                                .store
                                .update_record_with_response(
                                    &request_id,
                                    502,
                                    HashMap::new(),
                                    "响应已被丢弃".to_string(),
                                )
                                .await;

                            // 返回错误页面
                            let error_body = format!(
                                "<html><body><h1>响应已被拦截</h1><p>{}</p></body></html>",
                                e
                            );
                            return Response::builder()
                                .status(502)
                                .header("Content-Type", "text/html; charset=utf-8")
                                .body(Body::from(error_body))
                                .unwrap();
                        } else {
                            // 其他错误，记录并返回原始响应
                            println!("响应拦截过程中发生其他错误: {}", e);
                            let _ = self
                                .store
                                .update_record_with_response(
                                    &request_id,
                                    status,
                                    headers_map.clone(),
                                    body_str.clone(),
                                )
                                .await;

                            // 记录错误但返回原始响应
                            return Response::from_parts(parts, Full::new(bytes).into());
                        }
                    }
                }
            } else {
                // 规则不匹配，直接转发响应
                println!("响应不符合拦截规则，直接转发: Status {}", status);

                // 更新记录，使用原始状态码、头部和主体
                if let Some(updated_record) = self
                    .store
                    .update_record_with_response(&request_id, status, headers_map, body_str)
                    .await
                {
                    // 发送请求完成事件
                    self.app
                        .emit("proxy-request-completed", updated_record)
                        .unwrap_or_else(|e| {
                            println!("发送请求完成事件失败: {}", e);
                        });
                }

                // 使用原始响应
                return Response::from_parts(parts, Full::new(bytes).into());
            }
        } else {
            // 拦截未启用，直接更新记录并返回原始响应
            if let Some(updated_record) = self
                .store
                .update_record_with_response(&request_id, status, headers_map, body_str)
                .await
            {
                // 发送请求完成事件
                self.app
                    .emit("proxy-request-completed", updated_record)
                    .unwrap_or_else(|e| {
                        println!("发送请求完成事件失败: {}", e);
                    });
            }

            // 未启用响应拦截，直接返回原始响应
            return Response::from_parts(parts, Full::new(bytes).into());
        }
    }
}

impl WebSocketHandler for ProxyHandler {
    async fn handle_message(&mut self, ctx: &WebSocketContext, msg: Message) -> Option<Message> {
        // WebSocket消息处理
        match &msg {
            Message::Text(text) => {
                println!("[WebSocket] 文本消息: {}", text);
            }
            Message::Binary(data) => {
                println!("[WebSocket] 二进制消息: {} 字节", data.len());
            }
            Message::Ping(_) => {
                println!("[WebSocket] Ping");
            }
            Message::Pong(_) => {
                println!("[WebSocket] Pong");
            }
            Message::Close(_) => {
                println!("[WebSocket] 关闭连接");
            }
            _ => {
                println!("[WebSocket] 其他类型消息");
            }
        }

        // 返回原始消息，不做修改
        Some(msg)
    }
}
