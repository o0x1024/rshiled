pub mod types;

use http::Method;
pub use types::{HttpRequest, HttpResponse};

use hudsucker::Body;
use hudsucker::{
    certificate_authority::RcgenAuthority,
    hyper,
    hyper::body::Bytes,
    rcgen::{CertificateParams, KeyPair},
    rustls::crypto::aws_lc_rs,
    tokio_tungstenite::tungstenite::Message,
    HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler, *,
};

use crate::core::config::ProxyConfig;
use crate::internal::certificate::CertificateAuthority;
use anyhow::Result;
use http_body_util::{BodyExt, Full};
use log::{error, info};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};
use uuid::Uuid;

// Hudsucker相关导入
use hudsucker::hyper::{Request, Response};
use hudsucker::{
    builder::ProxyBuilder, Proxy as HudsuckerProxy,
};
use tokio::net::TcpStream;
use tokio::sync::Mutex as TokioMutex;

/// 拦截处理器 - 实现hudsucker的HttpHandler trait
#[derive(Clone)]
struct InterceptHandler {
    tx: mpsc::Sender<(HttpRequest, HttpResponse)>,
    // 存储请求映射，key是连接标识符，value是(请求ID, HttpRequest)
    requests: Arc<Mutex<HashMap<String, (String, HttpRequest)>>>,
    // 存储连接地址到URI的映射
    uri_map: Arc<Mutex<HashMap<String, String>>>,
}

impl InterceptHandler {
    fn new(tx: mpsc::Sender<(HttpRequest, HttpResponse)>) -> Self {
        Self { 
            tx,
            requests: Arc::new(Mutex::new(HashMap::new())),
            uri_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    // 生成连接标识符
    fn create_connection_id(client_addr: &std::net::SocketAddr) -> String {
        format!("{}", client_addr)
    }
}

impl HttpHandler for InterceptHandler {
    async fn handle_request(&mut self, ctx: &HttpContext, req: Request<Body>) -> RequestOrResponse {
        if req.method() == Method::CONNECT {
            return RequestOrResponse::Request(req);
        }

        let (parts, body) = req.into_parts();

        // 记录请求信息
        let method = parts.method.to_string();
        let uri = parts.uri.to_string();
        let parts_clone = parts.clone();
        let bytes: Bytes = body.collect().await.unwrap().to_bytes().clone();

        // 转换请求头为HashMap
        let mut headers_map = HashMap::new();
        for (name, value) in parts_clone.headers.iter() {
            if let Ok(v) = value.to_str() {
                headers_map.insert(name.to_string(), v.to_string());
            }
        }
        
        println!("uri: {}", uri);
        //从uri中提取参数
        let params = uri.split("?").collect::<Vec<&str>>();
        let params = {
            if params.len() > 1 {
                let params = params[1].split("&").collect::<Vec<&str>>();
                let params = params
                    .iter()
                    .map(|param| param.split("=").collect::<Vec<&str>>())
                    .collect::<Vec<Vec<&str>>>();
                params
                    .iter()
                    .map(|param| {
                        if param.len() >= 2 {
                            (param[0].to_string(), param[1].to_string())
                        } else {
                            (param[0].to_string(), "".to_string())
                        }
                    })
                    .collect::<Vec<(String, String)>>()
            } else {
                vec![]
            }
        };

        // 创建请求对象
        let http_req = HttpRequest::new(&uri, &method, headers_map.clone(), bytes.to_vec(), params);
        
        // 生成请求ID
        let req_id = Uuid::new_v4().to_string();
        
        // 生成连接标识符 - 只使用客户端地址
        let conn_id = Self::create_connection_id(&ctx.client_addr);
        println!("请求: 连接ID={}, 请求ID={}, URI={}", conn_id, req_id, uri);
        
        // 存储请求对象和URI映射
        {
            let mut requests = self.requests.lock().await;
            requests.insert(conn_id.clone(), (req_id.clone(), http_req));
            
            // 保存连接ID到URI的映射
            let mut uri_map = self.uri_map.lock().await;
            uri_map.insert(conn_id, uri.clone());
        }
        
        // 重新构建请求 - 不修改任何头部
        let mut request_builder = Request::builder()
            .method(parts.method.clone())
            .uri(parts.uri.clone())
            .version(parts.version);
            
        // 添加原始头部
        let headers = request_builder.headers_mut().unwrap();
        for (name, value) in parts_clone.headers.iter() {
            headers.insert(name, value.clone());
        }
        
        let body_str = String::from_utf8_lossy(&bytes.to_vec()).to_string();
        let new_request = request_builder.body(Body::from(body_str)).unwrap();

        return new_request.into();
    }

    async fn handle_response(&mut self, ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        let (parts, body) = res.into_parts();
        // 提取响应状态码和头部信息
        let parts_clone = parts.clone();

        // 记录响应信息
        let status = parts_clone.status.as_u16();
        let headers = parts_clone.headers.clone();
        
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
        
        // 使用客户端地址作为连接ID
        let conn_id = Self::create_connection_id(&ctx.client_addr);
        let mut found_request = None;
        let mut found_req_id = String::new();
        
        {
            let mut requests = self.requests.lock().await;
            
            // 尝试直接通过连接ID找到请求
            if let Some((req_id, req)) = requests.remove(&conn_id) {
                println!("响应: 找到匹配的请求 连接ID={}, 请求ID={}", conn_id, req_id);
                found_request = Some(req);
                found_req_id = req_id;
                
                // 同时清理URI映射
                let mut uri_map = self.uri_map.lock().await;
                uri_map.remove(&conn_id);
            }
        }
                
        // 构建HttpResponse
        let http_res = HttpResponse {
            status,
            headers: headers.iter()
                .filter_map(|(name, value)| {
                    if let Ok(v) = value.to_str() {
                        Some((name.to_string(), v.to_string()))
                    } else {
                        None
                    }
                })
                .collect(),
            body: bytes.to_vec(),
        };
        
        // 如果找到匹配的请求，发送到漏洞扫描模块
        if let Some(http_req) = found_request {
            println!("发送请求和响应到漏洞扫描模块，请求ID={}", found_req_id);
            
            // 发送请求和响应
            if let Err(e) = self.tx.send((http_req, http_res.clone())).await {
                eprintln!("发送请求和响应到漏洞扫描模块失败: {}", e);
            } else {
                println!("成功发送请求和响应到漏洞扫描模块");
            }
        } else {
            println!("警告: 未找到与响应匹配的请求");
        }
        
        // 返回原始响应
        return Response::from_parts(parts, Full::new(bytes).into());
    }
}

/// 代理服务器
pub struct Proxy {
    /// 代理配置
    config: ProxyConfig,
    /// 请求/响应通道发送端
    tx: mpsc::Sender<(HttpRequest, HttpResponse)>,
    /// hudsucker代理实例
    proxy: Option<
        Arc<
            TokioMutex<
                HudsuckerProxy<Body, Full<Bytes>, RcgenAuthority, InterceptHandler, TcpStream>,
            >,
        >,
    >,
    /// 关闭信号发送端
    shutdown_tx: Arc<TokioMutex<Option<oneshot::Sender<()>>>>,
    /// 证书管理器（用于TLS拦截）
    cert_manager: Option<Arc<CertificateAuthority>>,
    /// 是否拦截TLS流量
    intercept_tls: bool,
    /// 是否已经提示用户安装CA证书
    ca_install_prompted: Arc<std::sync::atomic::AtomicBool>,
}

impl Proxy {
    /// 创建新的代理服务器
    pub fn new(config: ProxyConfig, tx: mpsc::Sender<(HttpRequest, HttpResponse)>) -> Self {
        let (shutdown_tx, _) = oneshot::channel();
        Self {
            config,
            tx,
            proxy: None,
            shutdown_tx: Arc::new(TokioMutex::new(Some(shutdown_tx))),
            cert_manager: None,
            intercept_tls: false,
            ca_install_prompted: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// 创建新的代理服务器，带TLS拦截配置
    pub fn new_with_tls_settings(
        config: ProxyConfig,
        tx: mpsc::Sender<(HttpRequest, HttpResponse)>,
        cert_manager: Option<Arc<CertificateAuthority>>,
        intercept_tls: bool,
    ) -> Self {
        let (shutdown_tx, _) = oneshot::channel();
        Self {
            config,
            tx,
            proxy: None,
            shutdown_tx: Arc::new(TokioMutex::new(Some(shutdown_tx))),
            cert_manager,
            intercept_tls,
            ca_install_prompted: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn get_certificate_manager(&self) -> Option<Arc<CertificateAuthority>> {
        self.cert_manager.clone()
    }

    /// 启动代理服务器
    pub async fn start(&self) -> Result<(), String> {
        let host = self.config.host.clone().unwrap_or("127.0.0.1".to_string());
        let port = self.config.port.clone().unwrap_or(8889);
        let addr = format!("{}:{}", host, port);
        let listen_addr: std::net::SocketAddr = addr
            .parse()
            .map_err(|e| {
                let err_msg = format!("无法解析地址 {}: {}", addr, e);
                error!("{}", err_msg);
                err_msg
            })
            .map_err(|e| format!("无法解析地址: {}", e))?;

        // 准备拦截处理器
        let handler = InterceptHandler::new(self.tx.clone());

        if let Some(cert_manager) = &self.cert_manager {
            // 获取CA证书和私钥
            let (cert_pem, key_pem) = cert_manager.get_ca_cert_and_key()?;

            // 解析私钥和证书
            let key_pair = KeyPair::from_pem(&String::from_utf8_lossy(&key_pem))
                .map_err(|e| format!("解析私钥失败: {}", e))?;
            let ca_cert = CertificateParams::from_ca_cert_pem(&String::from_utf8_lossy(&cert_pem))
                .map_err(|e| format!("解析CA证书失败: {}", e))?
                .self_signed(&key_pair)
                .map_err(|e| format!("签名CA证书失败: {}", e))?;

            // 创建CA
            let ca = RcgenAuthority::new(
                key_pair,
                ca_cert,
                1_000,
                aws_lc_rs::default_provider(),
            );

            // 创建关闭通道
            let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
            
            // 更新shutdown_tx
            {
                let mut tx_guard = self.shutdown_tx.lock().await;
                *tx_guard = Some(shutdown_tx);
            }

            // 构建代理
            let proxy = ProxyBuilder::new()
                .with_addr(listen_addr)
                .with_ca(ca)
                .with_rustls_client(aws_lc_rs::default_provider())
                .with_http_handler(handler)
                .build()
                .map_err(|e| format!("创建代理失败: {}", e))?;

            // 启动代理服务
            tokio::spawn(async move {
                // 创建两个Future
                let proxy_future = proxy.start();
                let shutdown_future = shutdown_rx;
                
                // 使用select等待任一Future完成
                tokio::select! {
                    result = proxy_future => {
                        match result {
                            Ok(_) => {
                                println!("代理服务已正常停止");
                            }
                            Err(e) => {
                                println!("代理服务运行出错: {}", e);
                            }
                        }
                    }
                    _ = shutdown_future => {
                        println!("收到关闭信号，代理服务即将停止");
                        // 这里不需要做额外操作，因为select会自动取消另一个future
                    }
                }
                
                println!("代理服务已完全停止");
            });

            info!("代理服务已在 {}:{} 启动", host, port);
            return Ok(());
        } else {
            return Err("TLS拦截已启用,但证书管理器未初始化".to_string());
        }
    }

    /// 停止代理服务器
    pub async fn stop(&self) {
        info!("正在停止代理服务器...");
        
        // 发送关闭信号
        self.shutdown_proxy().await;
        
        // 等待端口释放（最多等待3秒）
        let port = self.config.port.unwrap_or(8889);
        if !self.wait_for_port_release(port, 3000).await {
            error!("等待端口 {} 释放超时", port);
        } else {
            info!("代理服务器已成功停止，端口 {} 已释放", port);
        }
    }
    
    /// 发送关闭信号
    async fn shutdown_proxy(&self) {
        let mut shutdown_tx = self.shutdown_tx.lock().await;
        if let Some(tx) = shutdown_tx.take() {
            match tx.send(()) {
                Ok(_) => {
                    info!("已发送代理服务器关闭信号");
                }
                Err(_) => {
                    error!("发送代理服务器关闭信号失败，接收端可能已关闭");
                }
            }
        } else {
            info!("代理服务器已经停止或未启动");
        }
    }
    
    /// 等待端口释放
    async fn wait_for_port_release(&self, port: u16, timeout_ms: u64) -> bool {
        let host = self.config.host.clone().unwrap_or("127.0.0.1".to_string());
        let addr = format!("{}:{}", host, port);
        
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);
        
        while start_time.elapsed() < timeout {
            // 尝试绑定端口，如果成功则表示端口已释放
            match tokio::net::TcpListener::bind(&addr).await {
                Ok(_) => return true,
                Err(_) => {
                    // 等待一小段时间再重试
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        }
        
        false
    }

    /// 检查端口是否可用
    pub async fn try_bind(&self) -> Result<(), String> {
        // 尝试绑定端口
        let addr = format!(
            "{}:{}",
            self.config.host.clone().unwrap_or("127.0.0.1".to_string()),
            self.config.port.clone().unwrap_or(8889)
        );
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::AddrInUse {
                    Err(format!(
                        "端口 {} 已被占用",
                        self.config.port.clone().unwrap_or(8889)
                    ))
                } else {
                    Err(format!("绑定端口失败: {}", e))
                }
            }
        }
    }

    /// 启动代理服务器并记录请求数量
    pub async fn start_with_request_counter(&self, request_count_tx: mpsc::Sender<()>) -> Result<(), String> {
        let host = self.config.host.clone().unwrap_or("127.0.0.1".to_string());
        let port = self.config.port.clone().unwrap_or(8889);
        let addr = format!("{}:{}", host, port);
        let listen_addr: std::net::SocketAddr = addr
            .parse()
            .map_err(|e| {
                let err_msg = format!("无法解析地址 {}: {}", addr, e);
                error!("{}", err_msg);
                err_msg
            })
            .map_err(|e| format!("无法解析地址: {}", e))?;

        // 创建一个能够统计请求的拦截处理器
        let handler = InterceptHandlerWithCounter::new(self.tx.clone(), request_count_tx);

        if let Some(cert_manager) = &self.cert_manager {
            // 获取CA证书和私钥
            let (cert_pem, key_pem) = cert_manager.get_ca_cert_and_key()?;

            // 解析私钥和证书
            let key_pair = KeyPair::from_pem(&String::from_utf8_lossy(&key_pem))
                .map_err(|e| format!("解析私钥失败: {}", e))?;
            let ca_cert = CertificateParams::from_ca_cert_pem(&String::from_utf8_lossy(&cert_pem))
                .map_err(|e| format!("解析CA证书失败: {}", e))?
                .self_signed(&key_pair)
                .map_err(|e| format!("签名CA证书失败: {}", e))?;

            // 创建CA
            let ca = RcgenAuthority::new(
                key_pair,
                ca_cert,
                1_000,
                aws_lc_rs::default_provider(),
            );

            // 创建关闭通道
            let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
            
            // 更新shutdown_tx
            {
                let mut tx_guard = self.shutdown_tx.lock().await;
                *tx_guard = Some(shutdown_tx);
            }

            // 构建代理
            let proxy = ProxyBuilder::new()
                .with_addr(listen_addr)
                .with_ca(ca)
                .with_rustls_client(aws_lc_rs::default_provider())
                .with_http_handler(handler)
                .build()
                .map_err(|e| format!("创建代理失败: {}", e))?;

            // 启动代理服务
            tokio::spawn(async move {
                // 创建两个Future
                let proxy_future = proxy.start();
                let shutdown_future = shutdown_rx;
                
                // 使用select等待任一Future完成
                tokio::select! {
                    result = proxy_future => {
                        match result {
                            Ok(_) => {
                                println!("代理服务已正常停止");
                            }
                            Err(e) => {
                                println!("代理服务运行出错: {}", e);
                            }
                        }
                    }
                    _ = shutdown_future => {
                        println!("收到关闭信号，代理服务即将停止");
                        // 这里不需要做额外操作，因为select会自动取消另一个future
                    }
                }
                
                println!("代理服务已完全停止");
            });

            info!("代理服务已在 {}:{} 启动", host, port);
            return Ok(());
        } else {
            return Err("TLS拦截已启用,但证书管理器未初始化".to_string());
        }
    }
}

/// 带计数功能的拦截处理器
#[derive(Clone)]
struct InterceptHandlerWithCounter {
    tx: mpsc::Sender<(HttpRequest, HttpResponse)>,
    // 请求计数发送器
    request_count_tx: mpsc::Sender<()>,
    // 存储请求映射，key是连接标识符，value是(请求ID, HttpRequest)
    requests: Arc<Mutex<HashMap<String, (String, HttpRequest)>>>,
    // 存储连接地址到URI的映射
    uri_map: Arc<Mutex<HashMap<String, String>>>,
}

impl InterceptHandlerWithCounter {
    fn new(tx: mpsc::Sender<(HttpRequest, HttpResponse)>, request_count_tx: mpsc::Sender<()>) -> Self {
        Self { 
            tx,
            request_count_tx,
            requests: Arc::new(Mutex::new(HashMap::new())),
            uri_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    // 生成连接标识符
    fn create_connection_id(client_addr: &std::net::SocketAddr) -> String {
        format!("{}", client_addr)
    }
}

impl HttpHandler for InterceptHandlerWithCounter {
    async fn handle_request(&mut self, ctx: &HttpContext, req: Request<Body>) -> RequestOrResponse {
        // 增加请求计数
        if let Err(e) = self.request_count_tx.send(()).await {
            eprintln!("发送请求计数失败: {}", e);
        }

        if req.method() == Method::CONNECT {
            return RequestOrResponse::Request(req);
        }

        let (parts, body) = req.into_parts();

        // 记录请求信息
        let method = parts.method.to_string();
        let uri = parts.uri.to_string();
        let parts_clone = parts.clone();
        let bytes: Bytes = body.collect().await.unwrap().to_bytes().clone();

        // 转换请求头为HashMap
        let mut headers_map = HashMap::new();
        for (name, value) in parts_clone.headers.iter() {
            if let Ok(v) = value.to_str() {
                headers_map.insert(name.to_string(), v.to_string());
            }
        }
        
        println!("uri: {}", uri);
        //从uri中提取参数
        let params = uri.split("?").collect::<Vec<&str>>();
        let params = {
            if params.len() > 1 {
                let params = params[1].split("&").collect::<Vec<&str>>();
                let params = params
                    .iter()
                    .map(|param| param.split("=").collect::<Vec<&str>>())
                    .collect::<Vec<Vec<&str>>>();
                params
                    .iter()
                    .map(|param| {
                        if param.len() >= 2 {
                            (param[0].to_string(), param[1].to_string())
                        } else {
                            (param[0].to_string(), "".to_string())
                        }
                    })
                    .collect::<Vec<(String, String)>>()
            } else {
                vec![]
            }
        };

        // 创建请求对象
        let http_req = HttpRequest::new(&uri, &method, headers_map.clone(), bytes.to_vec(), params);
        
        // 生成请求ID
        let req_id = Uuid::new_v4().to_string();
        
        // 生成连接标识符 - 只使用客户端地址
        let conn_id = Self::create_connection_id(&ctx.client_addr);
        println!("请求: 连接ID={}, 请求ID={}, URI={}", conn_id, req_id, uri);
        
        // 存储请求对象和URI映射
        {
            let mut requests = self.requests.lock().await;
            requests.insert(conn_id.clone(), (req_id.clone(), http_req));
            
            // 保存连接ID到URI的映射
            let mut uri_map = self.uri_map.lock().await;
            uri_map.insert(conn_id, uri.clone());
        }
        
        // 重新构建请求 - 不修改任何头部
        let mut request_builder = Request::builder()
            .method(parts.method.clone())
            .uri(parts.uri.clone())
            .version(parts.version);
            
        // 添加原始头部
        let headers = request_builder.headers_mut().unwrap();
        for (name, value) in parts_clone.headers.iter() {
            headers.insert(name, value.clone());
        }
        
        let body_str = String::from_utf8_lossy(&bytes.to_vec()).to_string();
        let new_request = request_builder.body(Body::from(body_str)).unwrap();

        return new_request.into();
    }

    async fn handle_response(&mut self, ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        let (parts, body) = res.into_parts();
        // 提取响应状态码和头部信息
        let parts_clone = parts.clone();

        // 记录响应信息
        let status = parts_clone.status.as_u16();
        let headers = parts_clone.headers.clone();
        
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
        
        // 使用客户端地址作为连接ID
        let conn_id = Self::create_connection_id(&ctx.client_addr);
        let mut found_request = None;
        let mut found_req_id = String::new();
        
        {
            let mut requests = self.requests.lock().await;
            
            // 尝试直接通过连接ID找到请求
            if let Some((req_id, req)) = requests.remove(&conn_id) {
                println!("响应: 找到匹配的请求 连接ID={}, 请求ID={}", conn_id, req_id);
                found_request = Some(req);
                found_req_id = req_id;
                
                // 同时清理URI映射
                let mut uri_map = self.uri_map.lock().await;
                uri_map.remove(&conn_id);
            }
        }
                
        // 构建HttpResponse
        let http_res = HttpResponse {
            status,
            headers: headers.iter()
                .filter_map(|(name, value)| {
                    if let Ok(v) = value.to_str() {
                        Some((name.to_string(), v.to_string()))
                    } else {
                        None
                    }
                })
                .collect(),
            body: bytes.to_vec(),
        };
        
        // 如果找到匹配的请求，发送到漏洞扫描模块
        if let Some(http_req) = found_request {
            println!("发送请求和响应到漏洞扫描模块，请求ID={}", found_req_id);
            
            // 发送请求和响应
            if let Err(e) = self.tx.send((http_req, http_res.clone())).await {
                eprintln!("发送请求和响应到漏洞扫描模块失败: {}", e);
            } else {
                println!("成功发送请求和响应到漏洞扫描模块");
            }
        } else {
            println!("警告: 未找到与响应匹配的请求");
        }
        
        // 返回原始响应
        return Response::from_parts(parts, Full::new(bytes).into());
    }
}
