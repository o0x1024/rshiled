// src-tauri/src/handler/scan/scanners/host_survival.rs
use std::process::Command;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use std::net::{IpAddr, ToSocketAddrs};
use log::{info, error, debug, warn};
use tokio::time::timeout;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io;
use tokio::sync::mpsc;
use std::time::Instant;

// 引入pnet库
use pnet::packet::icmp::{echo_request, IcmpTypes, IcmpPacket, MutableIcmpPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{Ipv4Packet, MutableIpv4Packet};
use pnet::packet::Packet;
use pnet::transport::{icmp_packet_iter, transport_channel};
use pnet::transport::TransportChannelType::Layer4;
use pnet::transport::TransportProtocol::Ipv4;

// 主机存活检测结果
#[derive(Debug, Clone)]
pub struct HostSurvivalResult {
    pub target: String,
    pub is_alive: bool,
    pub method: String, // ping, tcp, http等
    pub latency_ms: Option<u32>,
    pub details: Option<String>,
}

// 检查主机是否存活，使用多种方式
pub async fn check_host_survival(target: &str, timeout_seconds: u32) -> Result<HostSurvivalResult, String> {
    // 尝试解析目标为IP地址
    let target_str = target.to_string();
    let timeout_duration = Duration::from_secs(timeout_seconds as u64);
    
    // 先尝试使用ping
    if let Ok(ping_result) = ping_host(&target_str, timeout_seconds).await {
        if ping_result.is_alive {
            return Ok(ping_result);
        }
    }
    
    // 如果ping失败，尝试TCP连接测试
    let ports = vec![80, 443, 22, 21, 8080];
    for port in ports {
        if let Ok(tcp_result) = check_tcp_port(&target_str, port, timeout_seconds).await {
            if tcp_result.is_alive {
                return Ok(tcp_result);
            }
        }
    }
    
    // 如果是HTTP/HTTPS URL，尝试HTTP请求
    if target_str.starts_with("http://") || target_str.starts_with("https://") {
        if let Ok(http_result) = check_http(&target_str, timeout_seconds).await {
            if http_result.is_alive {
                return Ok(http_result);
            }
        }
    }
    
    // 所有方法都失败，返回目标离线
    Ok(HostSurvivalResult {
        target: target_str,
        is_alive: false,
        method: "all".to_string(),
        latency_ms: None,
        details: Some("目标主机无响应".to_string()),
    })
}

// 使用pnet发送ICMP Ping请求
async fn ping_host(target: &str, timeout_seconds: u32) -> Result<HostSurvivalResult, String> {
    let target = target.to_string();
    let clean_target = if target.starts_with("http://") || target.starts_with("https://") {
        target.replace("http://", "").replace("https://", "").split('/').next().unwrap_or(&target).to_string()
    } else {
        target.clone()
    };
    
    debug!("使用ICMP Ping检测主机存活: {}", clean_target);
    
    // 将目标解析为IP地址
    let ip_addr = match IpAddr::from_str(&clean_target) {
        Ok(ip) => ip,
        Err(_) => {
            // 尝试DNS解析
            match tokio::net::lookup_host(format!("{}:0", clean_target)).await {
                Ok(mut addrs) => {
                    if let Some(addr) = addrs.next() {
                        addr.ip()
                    } else {
                        return Err(format!("无法解析主机地址: {}", clean_target));
                    }
                },
                Err(e) => return Err(format!("DNS解析失败: {} - {}", clean_target, e)),
            }
        }
    };
    
    // 如果是IPv6，目前我们只处理IPv4
    let ip_v4 = match ip_addr {
        IpAddr::V4(ipv4) => ipv4,
        IpAddr::V6(_) => return Err("IPv6 ping尚未实现".to_string()),
    };
    
    // 创建一个通道来在异步上下文中传递结果
    let (tx, mut rx) = mpsc::channel::<Result<HostSurvivalResult, String>>(1);
    
    // 创建一个布尔标志来指示何时停止接收线程
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();
    
    // 在单独的线程中处理ping操作，因为pnet API是同步的
    let ping_handle = tokio::task::spawn_blocking(move || {
        let ping_result = perform_ping(ip_v4, timeout_seconds, stop_flag);
        
        if let Err(e) = tx.blocking_send(ping_result) {
            warn!("无法发送ping结果: {}", e);
        }
    });
    
    // 设置超时
    match timeout(Duration::from_secs(timeout_seconds as u64), rx.recv()).await {
        Ok(Some(result)) => {
            result
        },
        Ok(None) => {
            // 通道已关闭，但没有接收到值
            stop_flag_clone.store(true, Ordering::SeqCst);
            Err("Ping操作通道关闭，未收到结果".to_string())
        },
        Err(_) => {
            // 超时
            stop_flag_clone.store(true, Ordering::SeqCst);
            debug!("Ping操作超时: {}", clean_target);
            Err(format!("Ping操作超时: {}", clean_target))
        }
    }
}

// 执行实际的ping操作（同步函数，在spawn_blocking内部调用）
fn perform_ping(ip_addr: std::net::Ipv4Addr, timeout_seconds: u32, stop_flag: Arc<AtomicBool>) -> Result<HostSurvivalResult, String> {
    // 尝试创建原始套接字通道用于ICMP
    let protocol = Layer4(Ipv4(IpNextHeaderProtocols::Icmp));
    let (mut tx, mut rx) = match transport_channel(4096, protocol) {
        Ok((tx, rx)) => (tx, rx),
        Err(e) => return Err(format!("无法创建传输通道: {}", e)),
    };
    
    // 创建ICMP Echo请求包
    let mut echo_packet = [0u8; 64];
    let mut icmp_packet = MutableIcmpPacket::new(&mut echo_packet[..]).unwrap();
    
    // 设置ICMP包类型和代码
    icmp_packet.set_icmp_type(IcmpTypes::EchoRequest);
    icmp_packet.set_icmp_code(echo_request::IcmpCodes::NoCode);
    
    // 设置标识符和序列号
    let ident = rand::random::<u16>();
    let seq_num = 1;
    
    // 填充ICMP数据部分
    let mut payload = vec![0; 32];
    for i in 0..payload.len() {
        payload[i] = i as u8;
    }
    
    // 将数据复制到ICMP包中
    let icmp_packet_size = MutableIcmpPacket::minimum_packet_size();
    
    // 先释放icmp_packet的借用，以便我们可以修改echo_packet
    {
        // 设置ICMP数据包内容
        for (i, byte) in payload.iter().enumerate() {
            if i + icmp_packet_size < echo_packet.len() {
                echo_packet[icmp_packet_size + i] = *byte;
            }
        }
    }
    
    // 设置Echo请求标识符和序列号
    {
        echo_packet[4] = (ident >> 8) as u8;
        echo_packet[5] = (ident & 0xff) as u8;
        
        echo_packet[6] = (seq_num >> 8) as u8;
        echo_packet[7] = (seq_num & 0xff) as u8;
    }
    
    // 重新封装icmp_packet以计算校验和
    let mut icmp_packet = MutableIcmpPacket::new(&mut echo_packet[..]).unwrap();
    
    // 计算校验和
    icmp_packet.set_checksum(pnet::util::checksum(&icmp_packet.packet(), 1));
    
    // 开始计时
    let start_time = Instant::now();
    
    // 发送ICMP包
    match tx.send_to(icmp_packet, IpAddr::V4(ip_addr)) {
        Ok(_) => {},
        Err(e) => return Err(format!("发送ICMP包失败: {}", e)),
    }
    
    // 创建一个接收迭代器
    let mut iter = icmp_packet_iter(&mut rx);
    
    // 设置接收超时
    let timeout_duration = Duration::from_secs(timeout_seconds as u64);
    let end_time = start_time + timeout_duration;
    
    // 接收响应
    while Instant::now() < end_time {
        // 检查是否应该停止
        if stop_flag.load(Ordering::SeqCst) {
            return Err("Ping操作被终止".to_string());
        }
        
        // 使用select/poll机制间隔性地检查数据
        // pnet的TransportReceiver不支持设置超时，所以使用短间隔轮询
        match iter.next_with_timeout(Duration::from_millis(100)) {
            Ok(Some((packet, addr))) => {
                if addr == IpAddr::V4(ip_addr) {
                    // 检查是否是Echo回复
                    if packet.get_icmp_type() == IcmpTypes::EchoReply {
                        // 提取接收到的标识符
                        let recv_id = ((packet.packet()[4] as u16) << 8) | (packet.packet()[5] as u16);
                        
                        // 检查标识符是否匹配
                        if recv_id == ident {
                            let latency = start_time.elapsed().as_millis() as u32;
                            
                            return Ok(HostSurvivalResult {
                                target: ip_addr.to_string(),
                                is_alive: true,
                                method: "icmp_ping".to_string(),
                                latency_ms: Some(latency),
                                details: Some(format!("ICMP Ping成功: {} ({}ms)", ip_addr, latency)),
                            });
                        }
                    }
                }
            },
            Ok(None) => {
                // 没有收到数据包，继续等待
                continue;
            },
            Err(e) => {
                // 真正的错误
                if e.kind() != io::ErrorKind::TimedOut && e.kind() != io::ErrorKind::WouldBlock {
                    return Err(format!("接收ICMP包失败: {}", e));
                }
            }
        }
    }
    
    // 超时，主机可能不在线
    Err(format!("ICMP Ping超时，无响应: {}", ip_addr))
}

// 从ping输出中解析延迟时间 (已不再使用，但保留作参考)
fn parse_ping_latency(ping_output: &str) -> Option<u32> {
    // Windows ping输出格式
    if ping_output.contains("Average =") {
        if let Some(avg_part) = ping_output.split("Average =").nth(1) {
            if let Some(ms_part) = avg_part.split("ms").next() {
                if let Ok(latency) = ms_part.trim().parse::<u32>() {
                    return Some(latency);
                }
            }
        }
    }
    // Unix ping输出格式
    else if ping_output.contains("avg/") {
        if let Some(stats_line) = ping_output.lines().find(|line| line.contains("avg/")) {
            if let Some(avg_part) = stats_line.split("=").nth(1) {
                if let Some(avg_value) = avg_part.split("/").nth(1) {
                    if let Ok(latency) = avg_value.trim().parse::<f32>() {
                        return Some(latency as u32);
                    }
                }
            }
        }
    }
    
    None
}

// 使用TCP连接测试主机存活
async fn check_tcp_port(target: &str, port: u16, timeout_seconds: u32) -> Result<HostSurvivalResult, String> {
    let target = target.to_string();
    let clean_target = if target.starts_with("http://") || target.starts_with("https://") {
        target.replace("http://", "").replace("https://", "").split('/').next().unwrap_or(&target).to_string()
    } else {
        target.clone()
    };
    
    debug!("使用TCP连接测试主机存活: {}:{}", clean_target, port);
    
    let socket_addr = format!("{}:{}", clean_target, port);
    let tcp_future = async {
        match socket_addr.to_socket_addrs() {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    let start_time = Instant::now();
                    match tokio::net::TcpStream::connect(addr).await {
                        Ok(_) => {
                            let latency = start_time.elapsed().as_millis() as u32;
                            Ok(HostSurvivalResult {
                                target: target.clone(),
                                is_alive: true,
                                method: format!("tcp:{}", port),
                                latency_ms: Some(latency),
                                details: Some(format!("TCP端口{}连接成功 ({}ms)", port, latency)),
                            })
                        },
                        Err(e) => {
                            debug!("TCP连接失败: {}:{} - {}", clean_target, port, e);
                            Err(format!("TCP连接失败: {}", e))
                        }
                    }
                } else {
                    Err("无法解析地址".to_string())
                }
            },
            Err(e) => {
                debug!("解析地址失败: {} - {}", clean_target, e);
                Err(format!("解析地址失败: {}", e))
            }
        }
    };
    
    match timeout(Duration::from_secs(timeout_seconds as u64), tcp_future).await {
        Ok(result) => result,
        Err(_) => {
            debug!("TCP连接超时: {}:{}", clean_target, port);
            Err(format!("TCP连接超时: {}:{}", clean_target, port))
        }
    }
}

// 使用HTTP请求测试主机存活
async fn check_http(url: &str, timeout_seconds: u32) -> Result<HostSurvivalResult, String> {
    debug!("使用HTTP请求测试主机存活: {}", url);
    
    let http_future = async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds as u64))
            .build()
            .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
        
        let start_time = Instant::now();
        match client.get(url).send().await {
            Ok(response) => {
                let latency = start_time.elapsed().as_millis() as u32;
                let status = response.status();
                Ok(HostSurvivalResult {
                    target: url.to_string(),
                    is_alive: true,
                    method: "http".to_string(),
                    latency_ms: Some(latency),
                    details: Some(format!("HTTP请求成功: 状态码 {} ({}ms)", status, latency)),
                })
            },
            Err(e) => {
                debug!("HTTP请求失败: {} - {}", url, e);
                Err(format!("HTTP请求失败: {}", e))
            }
        }
    };
    
    match timeout(Duration::from_secs(timeout_seconds as u64), http_future).await {
        Ok(result) => result,
        Err(_) => {
            debug!("HTTP请求超时: {}", url);
            Err(format!("HTTP请求超时: {}", url))
        }
    }
}

// 批量检查多个目标的存活状态
pub async fn scan_hosts(targets: &[String], threads: u32, timeout: u32) -> Result<Vec<HostSurvivalResult>, String> {
    use futures::stream::{self, StreamExt};
    
    info!("开始批量检查主机存活，目标数量: {}", targets.len());
    
    let max_concurrent = threads as usize;
    let results = stream::iter(targets.iter().map(|target| target.clone()))
        .map(|target| async move {
            match check_host_survival(&target, timeout).await {
                Ok(result) => {
                    if result.is_alive {
                        info!("主机存活检测: {} 在线 (方法: {})", target, result.method);
                    } else {
                        info!("主机存活检测: {} 离线", target);
                    }
                    result
                },
                Err(e) => {
                    error!("主机存活检测失败: {} - {}", target, e);
                    HostSurvivalResult {
                        target: target.clone(),
                        is_alive: false,
                        method: "error".to_string(),
                        latency_ms: None,
                        details: Some(e),
                    }
                }
            }
        })
        .buffer_unordered(max_concurrent)
        .collect::<Vec<_>>()
        .await;
    
    info!("主机存活检测完成，结果数量: {}", results.len());
    Ok(results)
} 