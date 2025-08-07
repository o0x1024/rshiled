// src-tauri/src/handler/scan/scanners/port_scanner.rs
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::Duration;
use futures::{stream, StreamExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio::io::{AsyncReadExt, AsyncWriteExt}; // 添加异步IO traits
use std::collections::{HashMap, HashSet};
use log::{info, error, debug, warn};
use std::str::FromStr;
use once_cell::sync::Lazy;
use std::sync::Arc;

// 从兄弟模块导入
use super::service_probes::{NmapServiceProbes, OpenPortDetail, NmapMatch, NmapProbe};

// 全局Nmap服务指纹实例 (使用OnceCell或Lazy确保只加载一次)
// TODO: 实际项目中，"./config/nmap-service-probes.txt" 路径需要正确配置或通过参数传入
static GLOBAL_NMAP_PROBES: Lazy<Arc<Result<NmapServiceProbes, String>>> = Lazy::new(|| {
    Arc::new(NmapServiceProbes::load_from_file("nmap-service-probes.txt"))
});

// 端口扫描结果 (现在包含详细的服务信息)
#[derive(Debug, Clone)]
pub struct PortScanResult {
    pub target: String,
    // pub open_ports: Vec<u16>, // 旧版
    pub open_ports_details: Vec<OpenPortDetail>, // 新版，包含服务信息
    pub scan_time_ms: u64,
    pub scan_type: String,
}

// 端口扫描设置
#[derive(Debug, Clone)]
pub struct PortScanOptions {
    pub ports: Vec<u16>,
    pub timeout_ms: u64,
    pub threads: u32,
    pub identify_services: bool, // 新增：是否进行服务识别
    pub nmap_probes: Option<Arc<NmapServiceProbes>>, // 允许传入自定义的probes实例，主要用于测试
}

// 默认常用端口列表
const TOP_PORTS_20: &[u16] = &[21, 22, 23, 25, 53, 80, 110, 111, 135, 139, 143, 443, 445, 993, 995, 1723, 3306, 3389, 5900, 8080];
const TOP_PORTS_100: &[u16] = &[7, 9, 13, 21, 22, 23, 25, 26, 37, 53, 79, 80, 81, 88, 106, 110, 111, 113, 119, 135, 139, 143, 144, 179, 199, 389, 427, 443, 445, 465, 513, 514, 515, 543, 544, 548, 554, 587, 631, 646, 873, 990, 993, 995, 1025, 1026, 1027, 1028, 1029, 1110, 1433, 1720, 1723, 1755, 1900, 2000, 2001, 2049, 2121, 2717, 3000, 3128, 3306, 3389, 3986, 4899, 5000, 5009, 5051, 5060, 5101, 5190, 5357, 5432, 5631, 5666, 5800, 5900, 6000, 6001, 6646, 7070, 8000, 8008, 8009, 8080, 8081, 8443, 8888, 9100, 9999, 10000, 32768, 49152, 49153, 49154, 49155, 49156, 49157];

// 解析端口范围字符串
pub fn parse_port_range(range_str: &str) -> Result<Vec<u16>, String> {
    match range_str.to_lowercase().as_str() {
        "top20" => Ok(TOP_PORTS_20.to_vec()),
        "top100" => Ok(TOP_PORTS_100.to_vec()),
        "top1000" => {
            let mut ports: HashSet<u16> = (1..1025).collect();
            ports.extend(TOP_PORTS_100);
            Ok(ports.into_iter().collect())
        }
        "all" => Ok((1u16..=65535u16).collect()), // Corrected range for all ports
        _ => {
            let mut ports = HashSet::new();
            for part in range_str.split(',') {
                if part.contains('-') {
                    let range_parts: Vec<&str> = part.split('-').collect();
                    if range_parts.len() != 2 {
                        return Err(format!("无效的端口范围格式: {}", part));
                    }
                    let start = range_parts[0].trim().parse::<u16>().map_err(|_| format!("无效的起始端口: {}", range_parts[0]))?;
                    let end = range_parts[1].trim().parse::<u16>().map_err(|_| format!("无效的结束端口: {}", range_parts[1]))?;
                    if start > end {
                        return Err(format!("无效的端口范围: {} > {}", start, end));
                    }
                    for port in start..=end {
                        ports.insert(port);
                    }
                } else {
                    let port = part.trim().parse::<u16>().map_err(|_| format!("无效的端口号: {}", part))?;
                    ports.insert(port);
                }
            }
            if ports.is_empty() {
                return Err("未指定任何端口".to_string());
            }
            Ok(ports.into_iter().collect())
        }
    }
}

// 解析目标地址
async fn resolve_target(target: &str) -> Result<Vec<IpAddr>, String> {
    let clean_target = if target.starts_with("http://") || target.starts_with("https://") {
        target.replace("http://", "").replace("https://", "").split('/').next().unwrap_or(target).to_string()
    } else {
        target.to_string()
    };
    if let Ok(ip) = IpAddr::from_str(&clean_target) {
        return Ok(vec![ip]);
    }
    let socket_addr = format!("{}:80", clean_target); // Use a common port for DNS resolution
    match tokio::net::lookup_host(socket_addr).await { // Use tokio's async DNS resolution
        Ok(addrs) => {
            let ips: Vec<IpAddr> = addrs.map(|addr| addr.ip()).collect();
            if ips.is_empty() {
                Err(format!("无法解析目标地址: {}", target))
            } else {
                Ok(ips)
            }
        }
        Err(e) => Err(format!("无法解析目标地址: {} - {}", target, e)),
    }
}

// 扫描单个端口并尝试识别服务
async fn scan_port_and_identify_service(
    ip: IpAddr,
    port: u16,
    options: &PortScanOptions,
) -> Option<OpenPortDetail> {
    let socket_addr = SocketAddr::new(ip, port);
    let connection_timeout = Duration::from_millis(options.timeout_ms);

    match timeout(connection_timeout, TcpStream::connect(socket_addr)).await {
        Ok(Ok(mut stream)) => {
            debug!("端口 {}:{} 开放, 尝试服务识别...", ip, port);
            let mut detail = OpenPortDetail {
                port,
                service_name: None,
                version: None,
                product: None,
                extrainfo: None,
                hostname: None,
                ostype: None,
                devicetype: None,
                is_ssl: false, // 将在探测过程中确定
                banner: None,
            };

            if !options.identify_services {
                return Some(detail); // 如果不进行服务识别，直接返回端口开放
            }

            let probes_container = match &options.nmap_probes {
                Some(p_arc) => p_arc.clone(),
                None => {
                    match GLOBAL_NMAP_PROBES.as_ref().as_ref() {
                        Ok(probes) => Arc::new(probes.clone()),
                        Err(e) => {
                            warn!("加载全局Nmap指纹失败: {}，跳过服务识别 for {}:{}", e, ip, port);
                            return Some(detail);
                        }
                    }
                }
            };
            
            // TODO: SSL/TLS detection and wrapping if needed.
            // For now, let's assume non-SSL for simplicity, or that probes handle SSL negotiation implicitly if needed.

            let mut read_buffer = vec![0; 4096]; // 缓冲区用于读取响应
            let mut initial_banner: Option<Vec<u8>> = None;

            // 尝试读取初始Banner (很多服务会立即发送)
            // 设置一个较短的超时，因为不是所有服务都会立即响应
            match timeout(Duration::from_millis(2000), stream.read(&mut read_buffer)).await {
                Ok(Ok(bytes_read)) => {
                    if bytes_read > 0 {
                        let banner_data = read_buffer[..bytes_read].to_vec();
                        detail.banner = Some(String::from_utf8_lossy(&banner_data).to_string());
                        initial_banner = Some(banner_data);
                        debug!("收到 {}:{} 的初始Banner ({} bytes)", ip, port, bytes_read);
                    } else {
                        debug!("{}:{} 连接成功但未收到初始Banner (连接关闭?)", ip, port);
                    }
                }
                Ok(Err(e)) => {
                    debug!("读取 {}:{} 的初始Banner失败: {}", ip, port, e);
                }
                Err(_) => { // Timeout
                    debug!("读取 {}:{} 的初始Banner超时", ip, port);
                }
            }

            // 遍历所有Probes (或者更智能地选择Probes)
            // 简单起见，我们先尝试NULL Probe，然后是其他Probe
            // 实际Nmap会根据端口号、rarity等进行更复杂的选择
            let relevant_probes = probes_container.probes.iter() // probes_container is Arc<NmapServiceProbes>
                .filter(|p| p.protocol.to_uppercase() == "TCP") // 只关心TCP探针
                .collect::<Vec<_>>();

            for probe_to_send in relevant_probes {
                // 检查此探针是否已针对此服务识别出结果
                if detail.service_name.is_some() && detail.product.is_some() { break; }

                // 发送探针数据
                debug!("向 {}:{} 发送探针: {}", ip, port, probe_to_send.name);
                if let Err(_) = timeout(Duration::from_millis(500), stream.write_all(&probe_to_send.data)).await {
                    warn!("发送探针 {} 到 {}:{} 失败", probe_to_send.name, ip, port);
                    break; // 如果发送失败，可能连接已断开，停止对此端口的探测
                }
                
                // 接收响应 (需要处理超时)
                // totalwaitms可以用于此处的超时，但简单起见先用固定值
                let wait_duration = Duration::from_millis(probe_to_send.totalwaitms.unwrap_or(5000) as u64);
                match timeout(wait_duration, stream.read(&mut read_buffer)).await {
                    Ok(Ok(bytes_read)) => {
                        if bytes_read == 0 {
                            debug!("发送探针 {} 后，从 {}:{} 未收到响应 (连接可能已关闭)", probe_to_send.name, ip, port);
                            // 不一定是错误，有些探针可能就是为了关闭连接
                            // 但如果之前有banner，且没有匹配，则可能需要用banner匹配
                            if detail.banner.is_none() { continue; }
                        }
                        let response_data = &read_buffer[..bytes_read];
                        if detail.banner.is_none() { // 如果没有初始banner，则将第一次探针响应视为banner
                             detail.banner = Some(String::from_utf8_lossy(response_data).to_string());
                        }
                        debug!("收到 {}:{} 对探针 {} 的响应 ({} bytes)", ip, port, probe_to_send.name, bytes_read);
                        
                        // 尝试匹配响应
                        match_response_to_probes(response_data, probe_to_send, &mut detail, &probes_container);
                        
                        // 如果有软匹配，但没有版本信息，继续尝试其他探针
                        if detail.service_name.is_some() && detail.version.is_none() && detail.product.is_none() {
                            let is_soft = probe_to_send.matches.iter().any(|m| m.service == detail.service_name.as_deref().unwrap_or_default() && m.soft_match);
                            if is_soft {
                                debug!("软匹配 {} for {}:{}, 继续探测以获取版本信息", detail.service_name.as_deref().unwrap_or(""), ip, port);
                            } else if detail.service_name.is_some() { // 强匹配
                                break; // 找到强匹配，停止探测
                            }
                        } else if detail.service_name.is_some() { // 强匹配且可能有版本信息
                            break;
                        }
                    }
                    Ok(Err(e)) => {
                        warn!("读取 {}:{} 对探针 {} 的响应失败: {}", ip, port, probe_to_send.name, e);
                        break; // 读取失败，停止对此端口的探测
                    }
                    Err(_) => { // Timeout
                        debug!("读取 {}:{} 对探针 {} 的响应超时", ip, port, probe_to_send.name);
                        // 超时不一定意味着后续探针无效，但如果连续超时则可能是问题
                    }
                }
            }
            
            // 如果所有探针都发送完后，服务名仍然未知，但有初始banner，则尝试用初始banner匹配所有探针规则
            if detail.service_name.is_none() {
                if let Some(ref banner_bytes) = initial_banner {
                    debug!("无探针直接匹配 {}:{}，尝试使用初始Banner进行匹配", ip, port);
                    for probe_def in probes_container.probes.iter().filter(|p| p.protocol.to_uppercase() == "TCP") {
                        match_response_to_probes(banner_bytes, probe_def, &mut detail, &probes_container);
                        if detail.service_name.is_some() { break; }
                    }
                }
            }

            // 如果最终没有识别出服务名，但端口是开放的，则标记为unknown
            if detail.service_name.is_none() {
                detail.service_name = Some("unknown".to_string());
            }

            Some(detail)
        }
        _ => None, // 连接失败或超时，端口不开放或不可达
    }
}

fn match_response_to_probes(response_data: &[u8], probe_definition: &NmapProbe, detail: &mut OpenPortDetail, probes_container_arc: &Arc<NmapServiceProbes>) {
    // 打印Arc的强引用计数，这会"使用"probes_container_arc，并可能用于调试
    debug!("Probes_container Arc strong count (match_response_to_probes): {}", Arc::strong_count(probes_container_arc));
    let probes_container_ref = &**probes_container_arc; 

    for match_rule in &probe_definition.matches {
        let response_str = String::from_utf8_lossy(response_data);
        
        if let Some(captures) = match_rule.pattern_compiled.captures(&response_str) {
            debug!("探针 '{}' 的规则 '{}' 匹配了服务 '{}' for port {}", 
                   probe_definition.name, match_rule.service, match_rule.service, detail.port);

            if detail.service_name.is_some() && !match_rule.soft_match {
                // 硬匹配逻辑
            } else if detail.service_name.is_some() && match_rule.soft_match {
                if detail.service_name.as_deref() != Some(&match_rule.service) {
                    let previous_was_soft = probes_container_ref.probes.iter()
                        .flat_map(|p| &p.matches)
                        .any(|m| m.service == detail.service_name.as_deref().unwrap_or_default() && m.soft_match);
                    if !previous_was_soft { continue; } 
                }
            }

            detail.service_name = Some(match_rule.service.clone());

            if let Some(p_template) = &match_rule.version_info.product_capture {
                detail.product = Some(apply_capture_template(p_template, &captures));
            }
            if let Some(v_template) = &match_rule.version_info.version_capture {
                detail.version = Some(apply_capture_template(v_template, &captures));
            }
            if let Some(i_template) = &match_rule.version_info.info_capture {
                detail.extrainfo = Some(apply_capture_template(i_template, &captures));
            }
            if let Some(h_template) = &match_rule.version_info.hostname_capture {
                detail.hostname = Some(apply_capture_template(h_template, &captures));
            }
            if let Some(o_template) = &match_rule.version_info.ostype_capture {
                detail.ostype = Some(apply_capture_template(o_template, &captures));
            }
            if let Some(d_template) = &match_rule.version_info.devicetype_capture {
                detail.devicetype = Some(apply_capture_template(d_template, &captures));
            }
            
            if !match_rule.soft_match {
                return; 
            }
        }
    }
}

// Helper to apply capture group data to Nmap version string templates (e.g., $1, $2)
fn apply_capture_template(template: &str, captures: &regex::Captures) -> String {
    let mut result = template.to_string();
    for i in (1..=captures.len()-1).rev() { // Iterate backwards to avoid issues with $10 vs $1
        if let Some(cap) = captures.get(i) {
            result = result.replace(&format!("${}", i), cap.as_str());
        }
    }
    result
}

// 扫描单个目标的所有指定端口
pub async fn scan_target(
    target_str: &str,
    options: &PortScanOptions,
) -> Result<PortScanResult, String> {
    let start_time = std::time::Instant::now();
    
    let ips = match resolve_target(target_str).await {
        Ok(ips) => ips,
        Err(e) => return Err(e),
    };
    
    if ips.is_empty() {
        return Err(format!("无法解析目标: {}", target_str));
    }
    
    let ip_to_scan = ips[0]; // 优先使用解析到的第一个IP
    debug!("开始扫描目标 {} (IP: {}), 端口数量: {}", target_str, ip_to_scan, options.ports.len());
    
    let max_concurrent = options.threads as usize;
    
    let mut open_ports_details = Vec::new();
    
    let results = stream::iter(options.ports.iter().copied())
        .map(|port_num| {
            // Clone necessary data for the async block
            let current_ip = ip_to_scan;
            let current_options = options.clone(); // Clone options for each task
            async move {
                scan_port_and_identify_service(current_ip, port_num, &current_options).await
            }
        })
        .buffer_unordered(max_concurrent)
        .collect::<Vec<_>>()
        .await;
    
    // 处理每个扫描结果
    for result in results {
        if let Some(detail) = result {
            open_ports_details.push(detail);
        }
    }
    
    let scan_time = start_time.elapsed().as_millis() as u64;
    
    let scan_type_str = if options.ports.len() <= 20 {
        "精确扫描".to_string()
    } else if options.ports.len() <= 100 {
        "TOP100端口扫描".to_string()
    } else if options.ports.len() <= 1000 {
        "TOP1000端口扫描".to_string()
    } else {
        "全端口扫描".to_string()
    };
    
    Ok(PortScanResult {
        target: target_str.to_string(),
        open_ports_details: open_ports_details,
        scan_time_ms: scan_time,
        scan_type: scan_type_str,
    })
}

// 批量扫描多个目标
pub async fn scan_targets(
    targets: &[String],
    port_range_str: &str,
    threads_per_target: u32,
    timeout_seconds: u32,
    identify_services_flag: bool, // 新增参数
) -> Result<HashMap<String, PortScanResult>, String> {
    info!("开始批量端口扫描，目标数量: {}, 端口范围: {}, 服务识别: {}", targets.len(), port_range_str, identify_services_flag);
    
    let ports_to_scan = parse_port_range(port_range_str)?;
    let scan_timeout_ms = (timeout_seconds as u64) * 1000;
    
    // 获取全局指纹的引用
    let nmap_probes_instance = match GLOBAL_NMAP_PROBES.as_ref().as_ref() {
        Ok(probes) => Some(Arc::new(probes.clone())),
        Err(e) => {
            warn!("加载全局Nmap服务指纹失败: {}。服务识别将不可用。", e);
            None
        }
    };
    
    let scan_options = PortScanOptions {
        ports: ports_to_scan,
        timeout_ms: scan_timeout_ms,
        threads: threads_per_target,
        identify_services: identify_services_flag,
        nmap_probes: nmap_probes_instance, 
    };
    
    let max_concurrent_targets = std::cmp::min(10, threads_per_target); // 限制同时扫描的目标数量
    
    let mut all_results = HashMap::new();
    
    let individual_scan_results = stream::iter(targets.iter())
        .map(|target_ref| {
            let current_target = target_ref.clone();
            let current_options = scan_options.clone(); // Clone options for each target scan
            async move {
                let result = scan_target(&current_target, &current_options).await;
                (current_target, result)
            }
        })
        .buffer_unordered(max_concurrent_targets as usize)
        .collect::<Vec<_>>()
        .await;
    
    for (target_name, result_outcome) in individual_scan_results {
        match result_outcome {
            Ok(scan_data) => {
                let open_ports_count = scan_data.open_ports_details.len();
                info!(
                    "端口扫描完成: {} - 发现 {} 个开放端口 (服务识别: {}), 用时: {}ms",
                    target_name, open_ports_count, if identify_services_flag {"启用"} else {"禁用"}, scan_data.scan_time_ms
                );
                all_results.insert(target_name, scan_data);
            }
            Err(e) => {
                error!("扫描目标 {} 失败: {}", target_name, e);
            }
        }
    }
    
    info!("批量端口扫描完成，成功处理目标数量: {}", all_results.len());
    Ok(all_results)
} 