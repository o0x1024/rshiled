use chrono::{DateTime, Local};
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Instant, Duration};
use std::str;
use native_tls::TlsConnector;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, State};
use tauri::path::PathResolver;
use serde_json;
use base64;
use h2::client;
use tokio::net::TcpStream as TokioTcpStream;
use tokio::runtime::Runtime;
use tokio_rustls::{TlsConnector as RustlsConnector, rustls};
use http::{Request, Response as HttpResponse, StatusCode, HeaderMap};
use rustls::{ClientConfig, RootCertStore};
use webpki_roots;
use bytes::{Bytes, BytesMut};
use std::sync::Arc;
use futures::prelude::*;
use flate2::read::GzDecoder;

// 保存历史记录的结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestHistory {
    id: String,
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: Option<String>,
    status: u16,
    time: u64,
    timestamp: DateTime<Local>,
    response_headers: HashMap<String, String>,
    response_body: String,
}

// 请求响应的结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: String,
    content_type: String,
    is_binary: bool,
}

// HTTP版本枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HttpVersion {
    Http1,
    Http2,
}

// 保存Repeater设置的结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepeaterSettings {
    // 编辑器设置
    pub font_size: u32,
    pub line_height: f32,
    // 代理设置
    pub use_proxy: bool,
    pub proxy_host: String,
    pub proxy_port: u16,
    pub proxy_type: String,
    pub proxy_user: String,
    pub proxy_password: String,
    pub timeout: u64,
    // HTTP版本设置
    pub default_http_version: HttpVersion,
}

impl Default for RepeaterSettings {
    fn default() -> Self {
        Self {
            font_size: 14,
            line_height: 1.5,
            use_proxy: false,
            proxy_host: String::new(),
            proxy_port: 8080,
            proxy_type: "http".to_string(),
            proxy_user: String::new(),
            proxy_password: String::new(),
            timeout: 30,
            default_http_version: HttpVersion::Http1,
        }
    }
}

// 全局请求历史记录
static REQUEST_HISTORY: Lazy<Mutex<Vec<RequestHistory>>> = Lazy::new(|| Mutex::new(Vec::new()));

// 全局设置
static REPEATER_SETTINGS: Lazy<Mutex<RepeaterSettings>> = Lazy::new(|| {
    Mutex::new(RepeaterSettings::default())
});

// 将HTTP响应转换为我们的数据结构
async fn parse_response(response: Response) -> Result<RequestResponse, String> {
    let status = response.status().as_u16();
    
    // 处理响应头
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            let header_name = name.to_string();
            let header_value = value.to_str().unwrap_or("").to_string();
            (header_name, header_value)
        })
        .collect::<HashMap<String, String>>();
    
    // 检查是否是gzip编码
    let is_gzip = headers.iter().any(|(k, v)| 
        k.eq_ignore_ascii_case("content-encoding") && v.to_lowercase().contains("gzip")
    );
    
    // 获取Content-Type
    let content_type = headers.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default();
    
    // 获取响应体字节
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    
    // 处理响应体
    let body = if is_gzip && !bytes.is_empty() {
        println!("检测到gzip编码，尝试解压，压缩数据大小: {} 字节", bytes.len());
        
        // 尝试解压gzip数据
        let mut decoder = GzDecoder::new(bytes.as_ref());
        let mut buffer = Vec::with_capacity(bytes.len() * 5); // 预估解压后数据大小为压缩数据的5倍
        
        match decoder.read_to_end(&mut buffer) {
            Ok(size) => {
                println!("解压成功，解压后大小: {} 字节", size);
                if size > 0 {
                    // 判断是否为二进制内容
                    let is_binary_content = is_binary_content(&content_type) || !is_valid_utf8(&buffer);
                    
                    if is_binary_content {
                        // 二进制内容，使用Base64编码
                        let encoded = base64::encode(&buffer);
                        format!("{} 字节，已用Base64编码]\n{}", size, encoded)
                    } else {
                        // 文本内容，直接转换
                        let text = String::from_utf8_lossy(&buffer).to_string();
                        format!("{}", text)
                    }
                } else {
                    println!("解压后大小为0，使用原始数据");
                    String::from_utf8_lossy(&bytes).to_string()
                }
            },
            Err(e) => {
                println!("警告: gzip解压失败: {}", e);
                
                // 尝试第二种解压方法
                println!("尝试使用另一种方法解压...");
                let mut decoder2 = GzDecoder::new(bytes.as_ref());
                let mut buffer2 = Vec::new();
                
                // 使用更大的读取缓冲区
                let mut read_buffer = [0u8; 65536]; // 64KB 读取缓冲区
                let mut success = false;
                
                loop {
                    match decoder2.read(&mut read_buffer) {
                        Ok(0) => break, // 读取完成
                        Ok(n) => {
                            buffer2.extend_from_slice(&read_buffer[..n]);
                            success = true;
                        },
                        Err(e2) => {
                            println!("第二次尝试解压也失败: {}", e2);
                            break;
                        }
                    }
                }
                
                // 检查第二种方法是否成功
                if success && !buffer2.is_empty() {
                    println!("第二种方法解压成功，解压后大小: {} 字节", buffer2.len());
                    
                    // 判断是否为二进制内容
                    let is_binary_content = is_binary_content(&content_type) || !is_valid_utf8(&buffer2);
                    
                    if is_binary_content {
                        // 二进制内容，使用Base64编码
                        let encoded = base64::encode(&buffer2);
                        format!("{} 字节，已用Base64编码]\n{}", buffer2.len(), encoded)
                    } else {
                        // 文本内容，直接转换
                        let text = String::from_utf8_lossy(&buffer2).to_string();
                        format!("{}", text)
                    }
                } else {
                    // 两种方法都失败，继续使用原始数据
                    println!("两种解压方法都失败，使用原始数据");
                    String::from_utf8_lossy(&bytes).to_string()
                }
            }
        }
    } else {
        // 非gzip编码，直接处理
        let is_binary_content = is_binary_content(&content_type) || !is_valid_utf8(&bytes);
        
        if is_binary_content {
            // 二进制内容，使用Base64编码
            let encoded = base64::encode(&bytes);
            format!("{} 字节，已用Base64编码]\n{}", bytes.len(), encoded)
        } else {
            // 文本内容，直接转换
            String::from_utf8_lossy(&bytes).to_string()
        }
    };
    
    // 判断是否为二进制内容
    let is_binary = is_binary_content(&content_type) || 
                    (bytes.len() > 0 && !is_valid_utf8(&bytes) && !is_gzip);
    
    Ok(RequestResponse {
        status,
        headers,
        body,
        content_type,
        is_binary,
    })
}

// 辅助函数：查找HTTP头部结束位置
fn find_header_end(data: &[u8]) -> Option<usize> {
    for i in 0..data.len().saturating_sub(3) {
        if data[i] == b'\r' && data[i+1] == b'\n' && data[i+2] == b'\r' && data[i+3] == b'\n' {
            return Some(i);
        }
    }
    None
}

// 辅助函数：检查内容是否为二进制
fn is_binary_content(content_type: &str) -> bool {
    let ct = content_type.to_lowercase();
    ct.contains("image/") || 
    ct.contains("audio/") || 
    ct.contains("video/") || 
    ct.contains("application/octet-stream") ||
    ct.contains("application/pdf") ||
    ct.contains("application/zip") ||
    ct.contains("application/x-") ||
    ct.contains("multipart/form-data")
}

// 辅助函数：检查数据是否为有效UTF-8
fn is_valid_utf8(data: &[u8]) -> bool {
    str::from_utf8(data).is_ok()
}

/// 通过HTTP库发送请求
async fn send_http_request(
    method: &str,
    url: &str,
    headers: &HashMap<String, String>,
    body: &Option<String>,
) -> Result<RequestResponse, String> {
    let client = Client::new();
    
    // 解析HTTP方法
    let http_method = match method.to_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        "HEAD" => Method::HEAD,
        "OPTIONS" => Method::OPTIONS,
        _ => return Err(format!("不支持的HTTP方法: {}", method)),
    };
    
    // 构建请求
    let mut req = client.request(http_method.clone(), url);
    
    // 添加请求头
    for (name, value) in headers {
        req = req.header(name, value);
    }
    
    // 添加请求体
    if let Some(body_content) = body {
        if !body_content.is_empty() {
            req = req.body(body_content.clone());
        }
    }
    
    // 发送请求
    let response = req.send().await.map_err(|e| e.to_string())?;
    
    // 解析响应
    parse_response(response).await
}

/// 通过Socket发送请求
fn send_socket_request(
    raw_request: &str,
    host: &str,
    port: u16,
    use_https: bool,
    timeout_seconds: u64,
) -> Result<RequestResponse, String> {
    let socket_addr = format!("{}:{}", host, port);
    let addrs = socket_addr.to_socket_addrs()
        .map_err(|e| format!("无法解析主机地址: {}", e))?
        .collect::<Vec<_>>();
    
    if addrs.is_empty() {
        return Err(format!("无法解析主机地址: {}", socket_addr));
    }
    
    // 减少默认超时时间
    let timeout = Duration::from_secs(if timeout_seconds > 0 { timeout_seconds } else { 5 });
    
    // 获取当前设置
    let settings = REPEATER_SETTINGS.lock().unwrap().clone();
    
    // 如果设置了代理，则通过代理发送请求
    if settings.use_proxy {
        if use_https {
            // 使用TLS连接通过代理
            send_proxy_tls_request(raw_request, host, &addrs[0], &settings, timeout)
        } else {
            // 使用普通TCP连接通过代理
            send_proxy_request(raw_request, host, &addrs[0], &settings, timeout)
        }
    } else {
        // 不使用代理的原始逻辑
        if use_https {
            // 使用TLS连接
            send_tls_request(raw_request, host, &addrs[0], timeout)
        } else {
            // 使用普通TCP连接
            send_plain_request(raw_request, &addrs[0], timeout)
        }
    }
}

/// 通过普通TCP Socket发送请求
fn send_plain_request(
    raw_request: &str, 
    addr: &std::net::SocketAddr, 
    timeout: Duration
) -> Result<RequestResponse, String> {
    // 设置更短的连接超时，并记录开始时间
    let connect_start = Instant::now();
    let connect_timeout = Duration::from_secs(3); // 连接超时设为3秒
    
    let mut stream = TcpStream::connect_timeout(addr, connect_timeout)
        .map_err(|e| format!("连接服务器失败: {}", e))?;
    
    println!("TCP连接耗时: {}ms", connect_start.elapsed().as_millis());
    
    // 设置读写超时
    stream.set_read_timeout(Some(timeout))
        .map_err(|e| format!("设置读取超时失败: {}", e))?;
    stream.set_write_timeout(Some(timeout))
        .map_err(|e| format!("设置写入超时失败: {}", e))?;
    
    // 添加非阻塞模式
    stream.set_nonblocking(true)
        .map_err(|e| format!("设置非阻塞模式失败: {}", e))?;
    
    // 添加TCP_NODELAY以禁用Nagle算法
    stream.set_nodelay(true)
        .map_err(|e| format!("设置TCP_NODELAY失败: {}", e))?;
    
    // 转换换行符并发送HTTP请求
    let normalized_request = normalize_http_request(raw_request);
    let write_start = Instant::now();
    
    // 非阻塞写入处理
    let mut written = 0;
    let request_bytes = normalized_request.as_bytes();
    let total_len = request_bytes.len();
    
    while written < total_len {
        match stream.write(&request_bytes[written..]) {
            Ok(n) => {
                written += n;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // 等待一小段时间再尝试
                std::thread::sleep(Duration::from_millis(10));
                
                // 检查是否超时
                if write_start.elapsed() > timeout {
                    return Err("发送请求超时".to_string());
                }
                continue;
            }
            Err(e) => return Err(format!("发送请求失败: {}", e)),
        }
    }
    
    println!("请求发送耗时: {}ms", write_start.elapsed().as_millis());
    
    // 切换回阻塞模式读取响应
    stream.set_nonblocking(false)
        .map_err(|e| format!("切换回阻塞模式失败: {}", e))?;
    
    // 读取响应
    println!("开始读取响应...");
    let read_start = Instant::now();
    let result = read_response_from_stream(&mut stream);
    println!("响应读取耗时: {}ms", read_start.elapsed().as_millis());
    
    result
}

/// 确保HTTP请求格式正确（将\n转换为\r\n，确保行尾是\r\n等）
fn normalize_http_request(raw_request: &str) -> String {
    // 如果请求已经包含了正确的CRLF，则不需要替换
    if raw_request.contains("\r\n") {
        return raw_request.to_string();
    }
    
    // 将裸的\n转换为\r\n
    let mut normalized = raw_request.replace("\n", "\r\n");
    
    // 确保请求以\r\n\r\n结尾（表示头部结束）
    if !normalized.ends_with("\r\n\r\n") {
        if normalized.ends_with("\r\n") {
            normalized.push_str("\r\n");
        } else {
            normalized.push_str("\r\n\r\n");
        }
    }
    
    normalized
}

/// 通过TLS Socket发送请求
fn send_tls_request(
    raw_request: &str, 
    host: &str,
    addr: &std::net::SocketAddr, 
    timeout: Duration
) -> Result<RequestResponse, String> {
    // 设置更短的连接超时，并记录开始时间
    let connect_start = Instant::now();
    let connect_timeout = Duration::from_secs(3); // 连接超时设为3秒
    
    let tcp_stream = TcpStream::connect_timeout(addr, connect_timeout)
        .map_err(|e| format!("连接服务器失败: {}", e))?;
    
    println!("TLS TCP连接耗时: {}ms", connect_start.elapsed().as_millis());
    
    // 设置读写超时
    tcp_stream.set_read_timeout(Some(timeout))
        .map_err(|e| format!("设置读取超时失败: {}", e))?;
    tcp_stream.set_write_timeout(Some(timeout))
        .map_err(|e| format!("设置写入超时失败: {}", e))?;
    
    // 添加TCP_NODELAY以禁用Nagle算法
    tcp_stream.set_nodelay(true)
        .map_err(|e| format!("设置TCP_NODELAY失败: {}", e))?;
    
    // 创建并配置TLS连接器
    let tls_start = Instant::now();
    let mut builder = TlsConnector::builder();
    builder.danger_accept_invalid_certs(true);  // 允许自签名证书，类似Burp的功能
    
    let connector = builder.build()
        .map_err(|e| format!("创建TLS连接器失败: {}", e))?;
    
    // 连接到TLS服务器
    let mut stream = connector.connect(host, tcp_stream)
        .map_err(|e| format!("TLS握手失败: {}", e))?;
    
    println!("TLS握手耗时: {}ms", tls_start.elapsed().as_millis());
    
    // 转换换行符并发送HTTP请求
    let normalized_request = normalize_http_request(raw_request);
    let write_start = Instant::now();
    
    // 发送请求
    stream.write_all(normalized_request.as_bytes())
        .map_err(|e| format!("发送请求失败: {}", e))?;
    
    println!("TLS请求发送耗时: {}ms", write_start.elapsed().as_millis());
    
    // 读取响应
    println!("开始读取TLS响应...");
    let read_start = Instant::now();
    let result = read_response_from_stream(&mut stream);
    println!("TLS响应读取耗时: {}ms", read_start.elapsed().as_millis());
    
    result
}

/// 从流中读取HTTP响应
fn read_response_from_stream<T: Read>(stream: &mut T) -> Result<RequestResponse, String> {
    let mut response_buffer = Vec::new();
    let mut buffer = [0; 4096];
    let mut headers_received = false;
    let mut is_chunked = false;
    let mut content_length: Option<usize> = None;
    let mut chunked_ended = false;
    let mut body_start_pos = 0;
    let mut is_gzip = false;
    
    // 添加一个响应读取超时机制
    let start_time = Instant::now();
    let max_wait = Duration::from_secs(5); // 最大等待5秒
    let min_read_threshold = Duration::from_millis(200); // 两次读取间隔阈值
    let mut last_data_time = start_time;
    let mut received_data = false;
    
    loop {
        // 检查是否已经超时
        let now = Instant::now();
        let elapsed_since_last = now.duration_since(last_data_time);
        
        // 如果已经接收到一些数据并且超过一定时间没有新数据，认为响应已完成
        if received_data && elapsed_since_last > min_read_threshold {
            // 如果已经收到头部和一部分正文，认为可能是持久连接，直接结束读取
            if headers_received {
                println!("已超过{}ms没有接收到新数据，认为响应已完成", elapsed_since_last.as_millis());
                break;
            }
        }
        
        // 总体超时检查
        if now.duration_since(start_time) > max_wait {
            if response_buffer.is_empty() {
                return Err("响应读取超时".to_string());
            }
            println!("响应读取达到最大超时时间({}秒)，处理已接收的数据", max_wait.as_secs());
            break;
        }
        
        match stream.read(&mut buffer) {
            Ok(0) => break, // 连接关闭
            Ok(n) => {
                received_data = true;
                last_data_time = Instant::now(); // 更新最后接收数据的时间
                response_buffer.extend_from_slice(&buffer[..n]);
                
                // 如果还没有完整接收到响应头，检查是否已接收
                if !headers_received {
                    if let Some(header_end) = find_header_end(&response_buffer) {
                        headers_received = true;
                        body_start_pos = header_end + 4; // 记录响应体开始位置
                        
                        // 检查传输编码和内容长度
                        if let Ok(headers_str) = str::from_utf8(&response_buffer[..header_end]) {
                            is_chunked = headers_str.to_lowercase().contains("transfer-encoding: chunked");
                            
                            // 检查Content-Length
                            if let Some(cl_pos) = headers_str.to_lowercase().find("content-length:") {
                                let cl_str = &headers_str[cl_pos + 15..];
                                if let Some(end_pos) = cl_str.find("\r\n") {
                                    let cl_str = cl_str[..end_pos].trim();
                                    content_length = cl_str.parse::<usize>().ok();
                                    println!("检测到Content-Length: {}", content_length.unwrap_or(0));
                                }
                            }
                            
                            // 检查是否是gzip编码
                            is_gzip = headers_str.to_lowercase().contains("content-encoding: gzip");
                            if is_gzip {
                                println!("检测到gzip编码的响应");
                            }
                        }
                    }
                }
                
                // 如果这是分块传输，检查最后一个块（0大小块）
                if headers_received && is_chunked {
                    chunked_ended = check_chunked_complete(&response_buffer[body_start_pos..]);
                    if chunked_ended {
                        println!("检测到chunked编码完成标记");
                        break;
                    }
                }
                // 如果有Content-Length，检查是否已接收到足够的数据
                else if headers_received && content_length.is_some() {
                    let body_received = response_buffer.len() - body_start_pos;
                    if body_received >= content_length.unwrap() {
                        println!("已接收完整响应体(Content-Length)");
                        break;
                    }
                }
            }
            Err(e) => {
                // 处理超时错误 - 如果已有数据，尝试处理已接收部分
                if !response_buffer.is_empty() {
                    println!("读取响应遇到错误: {}，处理已接收的数据", e);
                    break;
                }
                return Err(format!("读取响应失败: {}", e));
            }
        }
    }
    
    // 解析响应
    parse_raw_response(&response_buffer)
}

/// 检查chunked编码是否已完成
fn check_chunked_complete(data: &[u8]) -> bool {
    // 尝试寻找表示chunked传输结束的模式: "0\r\n\r\n"
    if data.len() < 5 {
        return false;
    }
    
    // 快速检查结尾是否为0\r\n\r\n
    if data.ends_with(b"0\r\n\r\n") {
        return true;
    }
    
    // 遍历查找0\r\n\r\n序列
    for i in 0..data.len() - 4 {
        if data[i] == b'0' && 
           data[i+1] == b'\r' && 
           data[i+2] == b'\n' && 
           data[i+3] == b'\r' && 
           data[i+4] == b'\n' {
            return true;
        }
    }
    
    false
}

/// 通过代理发送普通HTTP请求
fn send_proxy_request(
    raw_request: &str, 
    target_host: &str,
    target_addr: &std::net::SocketAddr, 
    settings: &RepeaterSettings,
    timeout: Duration
) -> Result<RequestResponse, String> {
    // 暂时简单实现HTTP代理
    // 注意：完整实现需要处理不同代理类型(HTTP/SOCKS5等)
    let proxy_addr = format!("{}:{}", settings.proxy_host, settings.proxy_port);
    let proxy_addrs = proxy_addr.to_socket_addrs()
        .map_err(|e| format!("无法解析代理地址: {}", e))?
        .collect::<Vec<_>>();
    
    if proxy_addrs.is_empty() {
        return Err(format!("无法解析代理地址: {}", proxy_addr));
    }
    
    let mut stream = TcpStream::connect_timeout(&proxy_addrs[0], timeout)
        .map_err(|e| format!("连接代理服务器失败: {}", e))?;
    
    // 设置读写超时
    stream.set_read_timeout(Some(timeout))
        .map_err(|e| format!("设置读取超时失败: {}", e))?;
    stream.set_write_timeout(Some(timeout))
        .map_err(|e| format!("设置写入超时失败: {}", e))?;
    
    // 根据代理类型进行不同处理
    match settings.proxy_type.as_str() {
        "http" | "https" => {
            // 对于HTTP代理，需要发送CONNECT请求
            let target = format!("{}:{}", target_host, target_addr.port());
            let mut connect_req = format!("CONNECT {} HTTP/1.1\r\nHost: {}\r\n", target, target);
            
            // 添加代理认证
            if !settings.proxy_user.is_empty() {
                let auth = format!("{}:{}", settings.proxy_user, settings.proxy_password);
                let auth_base64 = base64::encode(auth);
                connect_req.push_str(&format!("Proxy-Authorization: Basic {}\r\n", auth_base64));
            }
            
            // 结束请求头
            connect_req.push_str("\r\n");
            
            // 发送CONNECT请求
            stream.write_all(connect_req.as_bytes())
                .map_err(|e| format!("发送代理CONNECT请求失败: {}", e))?;
            
            // 读取CONNECT响应
            let mut response_buffer = Vec::new();
            let mut buffer = [0; 4096];
            let mut connected = false;
            
            // 循环读取响应，直到找到空行（表示头部结束）
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) => return Err("代理连接异常关闭".to_string()),
                    Ok(n) => {
                        response_buffer.extend_from_slice(&buffer[..n]);
                        if let Ok(response_str) = str::from_utf8(&response_buffer) {
                            if response_str.contains("\r\n\r\n") {
                                // 检查是否成功建立连接
                                if response_str.starts_with("HTTP/1.1 200") || 
                                   response_str.starts_with("HTTP/1.0 200") {
                                    connected = true;
                                    break;
                                } else {
                                    return Err(format!("代理连接失败: {}", response_str));
                                }
                            }
                        }
                    }
                    Err(e) => return Err(format!("读取代理响应失败: {}", e)),
                }
            }
            
            if !connected {
                return Err("无法通过代理连接到目标服务器".to_string());
            }
            
            // 现在代理连接已建立，发送原始请求
            stream.write_all(raw_request.replace("\n", "\r\n").as_bytes())
                .map_err(|e| format!("通过代理发送请求失败: {}", e))?;
            
            // 读取原始响应
            read_response_from_stream(&mut stream)
        },
        "socks5" => {
            // SOCKS5代理实现（简化版）
            // 1. 发送认证方法协商
            let mut auth_methods = vec![0x00]; // 0x00表示无认证
            if !settings.proxy_user.is_empty() {
                auth_methods.push(0x02); // 0x02表示用户名/密码认证
            }
            
            let auth_request = [
                0x05, // SOCKS版本5
                auth_methods.len() as u8, // 认证方法数量
            ];
            
            stream.write_all(&auth_request)
                .map_err(|e| format!("发送SOCKS5认证请求失败: {}", e))?;
            stream.write_all(&auth_methods)
                .map_err(|e| format!("发送SOCKS5认证方法失败: {}", e))?;
            
            // 2. 接收服务器选择的认证方法
            let mut auth_response = [0u8; 2];
            stream.read_exact(&mut auth_response)
                .map_err(|e| format!("读取SOCKS5认证响应失败: {}", e))?;
            
            if auth_response[0] != 0x05 {
                return Err("不是有效的SOCKS5响应".to_string());
            }
            
            // 3. 如果需要认证，执行认证
            if auth_response[1] == 0x02 && !settings.proxy_user.is_empty() {
                // 用户名/密码认证
                let user_bytes = settings.proxy_user.as_bytes();
                let pass_bytes = settings.proxy_password.as_bytes();
                
                let mut auth_msg = vec![0x01]; // 认证子版本
                auth_msg.push(user_bytes.len() as u8);
                auth_msg.extend_from_slice(user_bytes);
                auth_msg.push(pass_bytes.len() as u8);
                auth_msg.extend_from_slice(pass_bytes);
                
                stream.write_all(&auth_msg)
                    .map_err(|e| format!("发送SOCKS5认证信息失败: {}", e))?;
                
                let mut auth_result = [0u8; 2];
                stream.read_exact(&mut auth_result)
                    .map_err(|e| format!("读取SOCKS5认证结果失败: {}", e))?;
                
                if auth_result[1] != 0x00 {
                    return Err("SOCKS5代理认证失败".to_string());
                }
            } else if auth_response[1] != 0x00 && auth_response[1] != 0x02 {
                return Err(format!("SOCKS5代理不支持无认证或用户名/密码认证: {:x}", auth_response[1]));
            }
            
            // 4. 发送连接请求
            let target_ip = match target_addr.ip() {
                std::net::IpAddr::V4(ipv4) => {
                    let octets = ipv4.octets();
                    vec![0x01, octets[0], octets[1], octets[2], octets[3]]
                },
                std::net::IpAddr::V6(ipv6) => {
                    let mut addr_bytes = vec![0x04]; // IPv6
                    addr_bytes.extend_from_slice(&ipv6.octets());
                    addr_bytes
                }
            };
            
            let port_bytes = target_addr.port().to_be_bytes();
            
            let mut connect_request = vec![
                0x05, // SOCKS版本
                0x01, // 命令：CONNECT
                0x00, // 保留字段
            ];
            connect_request.extend_from_slice(&target_ip);
            connect_request.extend_from_slice(&port_bytes);
            
            stream.write_all(&connect_request)
                .map_err(|e| format!("发送SOCKS5连接请求失败: {}", e))?;
            
            // 5. 接收连接响应
            let mut resp_header = [0u8; 4];
            stream.read_exact(&mut resp_header)
                .map_err(|e| format!("读取SOCKS5连接响应头失败: {}", e))?;
            
            if resp_header[1] != 0x00 {
                return Err(format!("SOCKS5连接失败，错误码: {:x}", resp_header[1]));
            }
            
            // 读取响应中的绑定地址和端口（根据地址类型）
            match resp_header[3] {
                0x01 => { // IPv4
                    let mut ipv4_addr = [0u8; 4];
                    stream.read_exact(&mut ipv4_addr)
                        .map_err(|e| format!("读取SOCKS5绑定IPv4地址失败: {}", e))?;
                },
                0x04 => { // IPv6
                    let mut ipv6_addr = [0u8; 16];
                    stream.read_exact(&mut ipv6_addr)
                        .map_err(|e| format!("读取SOCKS5绑定IPv6地址失败: {}", e))?;
                },
                0x03 => { // 域名
                    let mut domain_len = [0u8; 1];
                    stream.read_exact(&mut domain_len)
                        .map_err(|e| format!("读取SOCKS5域名长度失败: {}", e))?;
                    let mut domain = vec![0u8; domain_len[0] as usize];
                    stream.read_exact(&mut domain)
                        .map_err(|e| format!("读取SOCKS5域名失败: {}", e))?;
                },
                _ => return Err(format!("不支持的SOCKS5地址类型: {:x}", resp_header[3])),
            }
            
            // 读取绑定端口
            let mut port = [0u8; 2];
            stream.read_exact(&mut port)
                .map_err(|e| format!("读取SOCKS5绑定端口失败: {}", e))?;
            
            // 现在SOCKS5连接已建立，发送原始请求
            stream.write_all(raw_request.replace("\n", "\r\n").as_bytes())
                .map_err(|e| format!("通过SOCKS5代理发送请求失败: {}", e))?;
            
            // 读取原始响应
            read_response_from_stream(&mut stream)
        },
        _ => Err(format!("不支持的代理类型: {}", settings.proxy_type)),
    }
}

/// 通过代理发送TLS请求 (通过代理后使用TLS封装)
fn send_proxy_tls_request(
    raw_request: &str, 
    target_host: &str,
    target_addr: &std::net::SocketAddr, 
    settings: &RepeaterSettings,
    timeout: Duration
) -> Result<RequestResponse, String> {
    // 与普通代理请求类似，但在建立代理连接后进行TLS握手
    let proxy_addr = format!("{}:{}", settings.proxy_host, settings.proxy_port);
    let proxy_addrs = proxy_addr.to_socket_addrs()
        .map_err(|e| format!("无法解析代理地址: {}", e))?
        .collect::<Vec<_>>();
    
    if proxy_addrs.is_empty() {
        return Err(format!("无法解析代理地址: {}", proxy_addr));
    }
    
    let mut proxy_stream = TcpStream::connect_timeout(&proxy_addrs[0], timeout)
        .map_err(|e| format!("连接代理服务器失败: {}", e))?;
    
    // 设置读写超时
    proxy_stream.set_read_timeout(Some(timeout))
        .map_err(|e| format!("设置读取超时失败: {}", e))?;
    proxy_stream.set_write_timeout(Some(timeout))
        .map_err(|e| format!("设置写入超时失败: {}", e))?;
    
    // 通过代理建立到目标的TCP连接
    if settings.proxy_type == "http" || settings.proxy_type == "https" {
        // 对于HTTP代理，需要先发送CONNECT请求
        let target = format!("{}:{}", target_host, target_addr.port());
        let mut connect_req = format!("CONNECT {} HTTP/1.1\r\nHost: {}\r\n", target, target);
        
        // 添加代理认证
        if !settings.proxy_user.is_empty() {
            let auth = format!("{}:{}", settings.proxy_user, settings.proxy_password);
            let auth_base64 = base64::encode(auth);
            connect_req.push_str(&format!("Proxy-Authorization: Basic {}\r\n", auth_base64));
        }
        
        // 结束请求头
        connect_req.push_str("\r\n");
        
        // 发送CONNECT请求
        proxy_stream.write_all(connect_req.as_bytes())
            .map_err(|e| format!("发送代理CONNECT请求失败: {}", e))?;
        
        // 读取CONNECT响应
        let mut response_buffer = Vec::new();
        let mut buffer = [0; 4096];
        let mut connected = false;
        
        // 循环读取响应，直到找到空行（表示头部结束）
        loop {
            match proxy_stream.read(&mut buffer) {
                Ok(0) => return Err("代理连接异常关闭".to_string()),
                Ok(n) => {
                    response_buffer.extend_from_slice(&buffer[..n]);
                    if let Ok(response_str) = str::from_utf8(&response_buffer) {
                        if response_str.contains("\r\n\r\n") {
                            // 检查是否成功建立连接
                            if response_str.starts_with("HTTP/1.1 200") || 
                               response_str.starts_with("HTTP/1.0 200") {
                                connected = true;
                                break;
                            } else {
                                return Err(format!("代理连接失败: {}", response_str));
                            }
                        }
                    }
                }
                Err(e) => return Err(format!("读取代理响应失败: {}", e)),
            }
        }
        
        if !connected {
            return Err("无法通过代理连接到目标服务器".to_string());
        }
    } else if settings.proxy_type == "socks5" {
        // SOCKS5代理实现（简化版，与send_proxy_request类似）
        // 在这里实现SOCKS5代理协议，最终建立到目标服务器的连接
        // ...省略实现代码，参考send_proxy_request中的SOCKS5实现...
    } else {
        return Err(format!("代理TLS模式不支持的代理类型: {}", settings.proxy_type));
    }
    
    // 此时proxy_stream已经建立到目标服务器的连接
    // 在该连接上进行TLS握手
    
    // 创建TLS连接器
    let connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)  // 允许自签名证书
        .build()
        .map_err(|e| format!("创建TLS连接器失败: {}", e))?;
    
    // 在代理连接上建立TLS连接
    let mut tls_stream = connector.connect(target_host, proxy_stream)
        .map_err(|e| format!("通过代理进行TLS握手失败: {}", e))?;
    
    // 发送HTTP请求
    tls_stream.write_all(raw_request.replace("\n", "\r\n").as_bytes())
        .map_err(|e| format!("通过TLS代理发送请求失败: {}", e))?;
    
    // 读取响应
    read_response_from_stream(&mut tls_stream)
}

/// 自动检测HTTP请求的版本
fn detect_http_version(raw_request: &str) -> HttpVersion {
    // 检查是否包含HTTP/2相关标识
    if raw_request.contains("HTTP/2") || 
       raw_request.contains("h2") || 
       raw_request.contains("HTTP2") || 
       raw_request.contains("HTTP/2.0") {
        return HttpVersion::Http2;
    }
    
    // 默认使用HTTP/1.1
    HttpVersion::Http1
}

/// 发送HTTP请求 - 支持通过HTTP库或Socket发送
#[tauri::command]
pub async fn repeater_send_request(
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: Option<String>,
    use_socket: Option<bool>,
    target_host: Option<String>,
    target_port: Option<u16>,
    use_https: Option<bool>,
    raw_request: Option<String>,
) -> Result<RequestResponse, String> {
    let start = Instant::now();
    println!("开始处理请求...");
    
    let result = if use_socket == Some(true) {
        // 使用Socket方式发送
        let host = target_host.ok_or("使用Socket模式时必须提供目标主机")?;
        let port = target_port.unwrap_or(80);
        let raw = raw_request.ok_or("使用Socket模式时必须提供原始请求")?;
        let https = use_https.unwrap_or(false);
        
        // 自动检测HTTP版本
        let detected_version = detect_http_version(&raw);
        println!("检测到HTTP版本: {:?}", detected_version);
        
        // 使用更短的超时时间
        let timeout = 5; // 5秒超时
        
        // 根据检测到的版本决定使用哪种发送方式
        match detected_version {
            HttpVersion::Http2 => send_http2_request(&raw, &host, port, https).await,
            HttpVersion::Http1 => send_socket_request(&raw, &host, port, https, timeout)
        }
    } else {
        // 使用HTTP库方式发送
        send_http_request(&method, &url, &headers, &body).await
    }?;
    
    let elapsed = start.elapsed().as_millis() as u64;
    println!("请求总耗时: {}ms，响应状态码: {}", elapsed, result.status);
    
    // 保存到历史记录
    let history_item = RequestHistory {
        id: uuid::Uuid::new_v4().to_string(),
        method: method.clone(),
        url: url.clone(),
        headers: headers.clone(),
        body: body.clone(),
        status: result.status,
        time: elapsed,
        timestamp: Local::now(),
        response_headers: result.headers.clone(),
        response_body: result.body.clone(),
    };
    
    REQUEST_HISTORY.lock().unwrap().push(history_item);
    
    Ok(result)
}

/// 获取请求历史记录
#[tauri::command]
pub fn repeater_get_request_history() -> Result<Vec<RequestHistory>, String> {
    let history = REQUEST_HISTORY.lock().unwrap().clone();
    
    // 返回按时间排序的历史记录
    let mut sorted_history = history;
    sorted_history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    
    Ok(sorted_history)
}

/// 删除历史记录项
#[tauri::command]
pub fn repeater_delete_history_item(id: String) -> Result<(), String> {
    let mut history = REQUEST_HISTORY.lock().unwrap();
    
    let index = history
        .iter()
        .position(|item| item.id == id)
        .ok_or_else(|| "未找到历史记录项".to_string())?;
    
    history.remove(index);
    
    Ok(())
}

/// 获取Repeater设置
#[tauri::command]
pub fn repeater_get_settings(app_handle: AppHandle) -> Result<RepeaterSettings, String> {
    // 尝试从文件读取设置
    let settings_path = get_settings_path(&app_handle)?;
    
    if Path::new(&settings_path).exists() {
        match fs::read_to_string(&settings_path) {
            Ok(content) => {
                match serde_json::from_str::<RepeaterSettings>(&content) {
                    Ok(settings) => {
                        // 更新全局设置
                        *REPEATER_SETTINGS.lock().unwrap() = settings.clone();
                        Ok(settings)
                    },
                    Err(e) => {
                        // 如果解析失败，使用默认设置
                        let default_settings = RepeaterSettings::default();
                        *REPEATER_SETTINGS.lock().unwrap() = default_settings.clone();
                        Err(format!("解析设置文件失败: {}", e))
                    }
                }
            },
            Err(e) => {
                // 如果读取失败，使用默认设置
                let default_settings = RepeaterSettings::default();
                *REPEATER_SETTINGS.lock().unwrap() = default_settings.clone();
                Err(format!("读取设置文件失败: {}", e))
            }
        }
    } else {
        // 如果文件不存在，使用默认设置
        let default_settings = RepeaterSettings::default();
        *REPEATER_SETTINGS.lock().unwrap() = default_settings.clone();
        Ok(default_settings)
    }
}

/// 保存Repeater设置
#[tauri::command]
pub fn repeater_save_settings(settings: RepeaterSettings, app_handle: AppHandle) -> Result<(), String> {
    // 更新全局设置
    *REPEATER_SETTINGS.lock().unwrap() = settings.clone();
    
    // 保存到文件
    let settings_path = get_settings_path(&app_handle)?;
    
    // 确保目录存在
    if let Some(parent) = Path::new(&settings_path).parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建设置目录失败: {}", e))?;
    }
    
    // 序列化设置
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("序列化设置失败: {}", e))?;
    
    // 写入文件
    fs::write(&settings_path, content)
        .map_err(|e| format!("写入设置文件失败: {}", e))?;
    
    Ok(())
}

// 获取设置文件路径
fn get_settings_path(app_handle: &AppHandle) -> Result<String, String> {

    let settings_path = "config/repeater_settings.json";
    
    Ok(settings_path.to_string())
}

/// 发送HTTP/2请求
async fn send_http2_request(
    raw_request: &str,
    host: &str,
    port: u16,
    use_https: bool,
) -> Result<RequestResponse, String> {
    // 解析原始HTTP请求
    let (method, path, headers, body) = parse_raw_http_request(raw_request)?;
    
    // 创建Tokio TCP连接
    let addr = format!("{}:{}", host, port);
    let tcp = TokioTcpStream::connect(addr)
        .await
        .map_err(|e| format!("TCP连接失败: {}", e))?;

    // 创建HTTP/2客户端
    if use_https {
        // 准备TLS配置
        let mut root_store = RootCertStore::empty();
        root_store.add_server_trust_anchors(
            webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
                rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject.as_ref(),
                    ta.subject_public_key_info.as_ref(),
                    ta.name_constraints.as_ref().map(|nc| nc.as_ref()),
                )
            }),
        );
        
        // 创建TLS客户端配置
        let mut tls_config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        
        // 允许无效证书，类似Burp的功能
        tls_config.dangerous().set_certificate_verifier(Arc::new(DangerousVerifier{}));
        
        // 启用ALPN协议，以支持HTTP/2
        tls_config.alpn_protocols = vec![b"h2".to_vec()];
        
        let connector = RustlsConnector::from(Arc::new(tls_config));
        let server_name = rustls::ServerName::try_from(host)
            .map_err(|e| format!("无效的服务器名称: {}", e))?;
        
        // 建立TLS连接
        let tls_stream = connector.connect(server_name, tcp)
            .await
            .map_err(|e| format!("TLS连接失败: {}", e))?;
        
        // 建立HTTP/2客户端
        let (mut client, h2) = client::Builder::new()
            .handshake(tls_stream)
            .await
            .map_err(|e| format!("HTTP/2握手失败: {}", e))?;
        
        // 在后台任务中处理连接
        tokio::spawn(async move {
            if let Err(e) = h2.await {
                eprintln!("HTTP/2连接错误: {}", e);
            }
        });
        
        // 创建HTTP/2请求
        let scheme = "https";
        let uri = format!("{}://{}{}", scheme, host, path)
            .parse::<http::Uri>()
            .map_err(|e| format!("URI解析失败: {}", e))?;
        
        let mut request = Request::builder()
            .method(method)
            .uri(uri);
        
        // 添加请求头
        let mut request_headers = request.headers_mut().unwrap();
        for (name, value) in headers {
            let header_name = http::header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|e| format!("无效的头部名称 {}: {}", name, e))?;
            let header_value = http::header::HeaderValue::from_str(&value)
                .map_err(|e| format!("无效的头部值 {}: {}", value, e))?;
            request_headers.insert(header_name, header_value);
        }
        
        // 构建请求
        let request = request.body(())
            .map_err(|e| format!("创建HTTP/2请求失败: {}", e))?;
        
        // 发送请求
        let (response_future, mut send_stream) = client.send_request(request, false)
            .map_err(|e| format!("发送HTTP/2请求失败: {}", e))?;
        
        // 如果有请求体，发送
        if !body.is_empty() {
            let data = Bytes::from(body);
            send_stream.send_data(data, true)
                .map_err(|e| format!("发送HTTP/2请求体失败: {}", e))?;
        } else {
            send_stream.send_data(Bytes::new(), true)
                .map_err(|e| format!("结束HTTP/2请求失败: {}", e))?;
        }

        // 等待响应
        let response = response_future.await
            .map_err(|e| format!("等待HTTP/2响应失败: {}", e))?;
        
        // 获取HTTP/2响应头
        let status = response.status().as_u16();
        let headers_map = response.headers();
        
        // 收集响应头
        let mut headers = HashMap::new();
        for (name, value) in headers_map.iter() {
            let name_str = name.as_str().to_string();
            let value_str = String::from_utf8_lossy(value.as_bytes()).to_string();
            headers.insert(name_str, value_str);
        }
        
        // 收集响应体
        let mut body_bytes = BytesMut::new();
        let mut body_stream = response.into_body();
        while let Some(chunk_result) = body_stream.data().await {
            match chunk_result {
                Ok(chunk) => {
                    body_bytes.extend_from_slice(&chunk);
                },
                Err(e) => return Err(format!("读取HTTP/2响应体失败: {}", e)),
            }
        }
        
        // 将响应体转换为字符串
        let body = match String::from_utf8(body_bytes.to_vec()) {
            Ok(body) => body,
            Err(_) => {
                // 如果不是有效的UTF-8，转换为Base64
                let encoded = base64::encode(&body_bytes);
                format!("{} 字节，已用Base64编码]\n{}", body_bytes.len(), encoded)
            }
        };
        
        Ok(RequestResponse {
            status,
            headers,
            body,
            content_type: String::new(),
            is_binary: false,
        })
    } else {
        // 非TLS的HTTP/2连接（h2c）
        // 由于裸HTTP/2连接较为少见，这里简化处理
        Err("暂不支持非TLS的HTTP/2连接 (h2c)，请使用HTTPS".to_string())
    }
}

// 危险的证书验证器，允许任何证书（类似Burp的功能）
struct DangerousVerifier {}

impl rustls::client::ServerCertVerifier for DangerousVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        // 危险：接受任何证书
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

// 从原始HTTP请求中解析方法、路径、头部和请求体
fn parse_raw_http_request(raw_request: &str) -> Result<(http::Method, String, HashMap<String, String>, String), String> {
    let lines: Vec<&str> = raw_request.split("\r\n").collect();
    if lines.is_empty() {
        return Err("无效的HTTP请求".to_string());
    }
    
    // 解析请求行
    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("无效的HTTP请求行".to_string());
    }
    
    let method_str = parts[0];
    let method = match method_str {
        "GET" => http::Method::GET,
        "POST" => http::Method::POST,
        "PUT" => http::Method::PUT,
        "DELETE" => http::Method::DELETE,
        "HEAD" => http::Method::HEAD,
        "OPTIONS" => http::Method::OPTIONS,
        "PATCH" => http::Method::PATCH,
        "TRACE" => http::Method::TRACE,
        _ => return Err(format!("不支持的HTTP方法: {}", method_str)),
    };
    
    let path = parts[1].to_string();
    
    // 解析请求头
    let mut headers = HashMap::new();
    let mut i = 1;
    let mut body_start = 0;
    
    while i < lines.len() {
        let line = lines[i];
        if line.is_empty() {
            body_start = i + 1;
            break;
        }
        
        if let Some(idx) = line.find(':') {
            let name = line[..idx].trim().to_string();
            let value = line[idx + 1..].trim().to_string();
            headers.insert(name, value);
        }
        
        i += 1;
    }
    
    // 提取请求体
    let body = if body_start > 0 && body_start < lines.len() {
        lines[body_start..].join("\r\n")
    } else {
        String::new()
    };
    
    Ok((method, path, headers, body))
}

// 解析HTTP响应
fn parse_raw_response(response_data: &[u8]) -> Result<RequestResponse, String> {
    // 先尝试解析响应头，即使整个响应不是有效UTF-8
    let mut headers = HashMap::new();
    let mut status = 0;
    let mut content_type = String::new();
    let mut content_encoding = String::new();
    let mut body_data = response_data;
    let mut is_chunked = false;
    
    // 尝试找到头部和正文的分隔符 \r\n\r\n
    if let Some(header_end) = find_header_end(response_data) {
        // 尝试解析头部（通常头部是ASCII，可以安全转换为UTF-8）
        if let Ok(headers_str) = str::from_utf8(&response_data[..header_end]) {
            // 解析状态行
            let header_lines: Vec<&str> = headers_str.split("\r\n").collect();
            if !header_lines.is_empty() {
                let status_line = header_lines[0];
                let status_parts: Vec<&str> = status_line.split_whitespace().collect();
                if status_parts.len() >= 2 {
                    status = status_parts[1].parse::<u16>().unwrap_or(0);
                }
                
                // 解析头部
                for i in 1..header_lines.len() {
                    let line = header_lines[i];
                    if let Some(index) = line.find(':') {
                        let name = line[..index].trim().to_string();
                        let value = line[index + 1..].trim().to_string();
                        
                        // 记录Content-Type
                        if name.eq_ignore_ascii_case("content-type") {
                            content_type = value.clone();
                        }
                        // 记录Content-Encoding
                        if name.eq_ignore_ascii_case("content-encoding") {
                            content_encoding = value.clone();
                        }
                        // 检查是否chunked
                        if name.eq_ignore_ascii_case("transfer-encoding") && value.to_lowercase().contains("chunked") {
                            is_chunked = true;
                        }
                        headers.insert(name, value);
                    }
                }
            }
        }
        // 分离响应体
        let mut body_start = header_end + 4;
        while response_data[body_start..].starts_with(b"\r\n") {
            body_start += 2;
        }
        body_data = &response_data[body_start..];
    }
    // chunked解码
    let mut decoded_body = Vec::new();
    if is_chunked {
        let mut pos = 0;
        while pos < body_data.len() {
            // 查找chunk长度行结尾
            let mut len_end = pos;
            while len_end < body_data.len() && body_data[len_end] != b'\r' {
                len_end += 1;
            }
            if len_end + 1 >= body_data.len() || body_data[len_end+1] != b'\n' {
                break; // 非法chunk格式
            }
            // 解析chunk长度
            let len_str = match std::str::from_utf8(&body_data[pos..len_end]) {
                Ok(s) => s.trim(),
                Err(_) => break,
            };
            let chunk_len = match usize::from_str_radix(len_str.split(';').next().unwrap_or("0"), 16) {
                Ok(l) => l,
                Err(_) => break,
            };
            if chunk_len == 0 {
                break;
            }
            // 跳过长度行和\r\n
            pos = len_end + 2;
            if pos + chunk_len > body_data.len() {
                break; // 数据不完整
            }
            decoded_body.extend_from_slice(&body_data[pos..pos+chunk_len]);
            pos += chunk_len;
            // 跳过chunk后的\r\n
            if pos + 1 < body_data.len() && body_data[pos] == b'\r' && body_data[pos+1] == b'\n' {
                pos += 2;
            } else {
                break;
            }
        }
        body_data = &decoded_body;
    }
    // 处理gzip压缩
    let mut decompressed_data = Vec::new();
    let is_gzipped = content_encoding.to_lowercase().contains("gzip");
    if is_gzipped && !body_data.is_empty() {
        println!("检测到gzip编码，尝试解压，压缩数据大小: {} 字节", body_data.len());
        let mut buffer = Vec::with_capacity(body_data.len() * 5);
        let mut decoder = GzDecoder::new(body_data);
        match decoder.read_to_end(&mut buffer) {
            Ok(size) => {
                println!("解压成功，解压后大小: {} 字节", size);
                if size > 0 {
                    decompressed_data = buffer;
                    body_data = &decompressed_data;
                    headers.remove("Content-Encoding");
                } else {
                    println!("解压后大小为0，使用原始数据");
                }
            },
            Err(e) => {
                println!("警告: gzip解压失败: {}", e);
                println!("尝试使用另一种方法解压...");
                let mut decoder2 = GzDecoder::new(body_data);
                let mut buffer2 = Vec::new();
                let mut read_buffer = [0u8; 65536];
                let mut success = false;
                loop {
                    match decoder2.read(&mut read_buffer) {
                        Ok(0) => break,
                        Ok(n) => { buffer2.extend_from_slice(&read_buffer[..n]); success = true; },
                        Err(e2) => { println!("第二次尝试解压也失败: {}", e2); break; }
                    }
                }
                if success && !buffer2.is_empty() {
                    println!("第二种方法解压成功，解压后大小: {} 字节", buffer2.len());
                    decompressed_data = buffer2;
                    body_data = &decompressed_data;
                    headers.remove("Content-Encoding");
                } else {
                    println!("两种解压方法都失败，使用原始数据");
                }
            }
        }
    }
    let is_binary = is_binary_content(&content_type) || !is_valid_utf8(body_data);
    let body = if is_binary {
        let encoded = base64::encode(body_data);
        format!("{} 字节，已用Base64编码]\n{}", body_data.len(), encoded)
    } else {
        String::from_utf8_lossy(body_data).to_string()
    };
    Ok(RequestResponse {
        status,
        headers,
        body,
        content_type,
        is_binary,
    })
} 