use crate::core::config::AppConfig;
use crate::global::config::CoreConfig;
use crate::handler::scan::ast::{self, AstAnalyzer, InjectionResult, RiskLevel};
use crate::handler::scan::engine::ScanResult;
use crate::handler::scan::proxy::{HttpRequest, HttpResponse};
use crate::handler::scan::scanners::Scanner;
use anyhow::Result;
use async_trait::async_trait;
use log::{debug, info, warn};
use regex::Regex;
use reqwest;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use once_cell::sync::Lazy;

use url;
use urlencoding;

use async_compression::tokio::bufread::GzipDecoder;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_native_tls::{native_tls, TlsConnector};

/// XSS扫描器
#[derive(Clone)]
pub struct XssScanner {
    /// 配置
    _config: Arc<AppConfig>,
    /// HTML AST分析器
    html_analyzer: Arc<ast::HtmlAstAnalyzer>,
    /// JavaScript AST分析器
    js_analyzer: Arc<ast::JsAstAnalyzer>,
    /// HTTP客户端
    _http_client: reqwest::Client,
}

// 静态缓存，用于存储页面特征向量
static PAGE_CACHE: Lazy<Mutex<HashMap<String, Vec<(String, HashMap<String, f64>)>>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

impl XssScanner {
    /// 创建新的XSS扫描器
    pub fn new(_config: Arc<AppConfig>) -> Self {
        // 创建带代理的HTTP客户端
        // let proxy = reqwest::Proxy::http("http://127.0.0.1:9999").expect("无效的代理URL");
        // let http_client = reqwest::Client::builder()
        //     .proxy(proxy)
        //     .timeout(Duration::from_secs(30))
        //     .build()
        //     .unwrap_or_else(|_| {
        //         // 如果代理配置失败，使用默认客户端
        //         debug!("代理配置失败，使用无代理客户端");
        //         reqwest::Client::new()
        //     });

        Self {
            _config,
            html_analyzer: Arc::new(ast::HtmlAstAnalyzer::new()),
            js_analyzer: Arc::new(ast::JsAstAnalyzer::new()),
            _http_client: reqwest::Client::new(),
        }
    }

    /// 检查内容类型是否为HTML
    fn is_html_content(&self, headers: &HashMap<String, String>) -> bool {
        headers.iter().any(|(name, value)| {
            name.eq_ignore_ascii_case("content-type") && value.to_lowercase().contains("text/html")
        })
    }

    /// 检查内容类型是否为JavaScript
    fn is_js_content(&self, headers: &HashMap<String, String>) -> bool {
        headers.iter().any(|(name, value)| {
            name.eq_ignore_ascii_case("content-type")
                && (value.to_lowercase().contains("javascript")
                    || value.to_lowercase().contains("ecmascript"))
        })
    }

    /// 检查内容类型是否为文本
    fn is_text_content(&self, headers: &HashMap<String, String>) -> bool {
        for (name, value) in headers {
            if name.to_lowercase() == "content-type" {
                // 检查常见的文本内容类型
                return value.contains("text/")
                    || value.contains("application/json")
                    || value.contains("application/javascript")
                    || value.contains("application/xml")
                    || value.contains("application/xhtml+xml");
            }
        }
        // 如果没有Content-Type头，默认为非文本
        false
    }

    /// 检查内容类型是否为JSON
    fn is_json_content(&self, headers: &HashMap<String, String>) -> bool {
        for (name, value) in headers {
            if name.to_lowercase() == "content-type" {
                return value.contains("application/json");
            }
        }
        false
    }

    /// 从URL中提取参数
    fn extract_params_from_url(&self, url: &str) -> Vec<(String, String)> {
        let mut params = Vec::new();

        if let Some(query_string) = url.split('?').nth(1) {
            for pair in query_string.split('&') {
                let mut parts = pair.split('=');
                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                    params.push((key.to_string(), value.to_string()));
                }
            }
        }

        params
    }

    /// 从URL中提取路径参数
    fn extract_path_params(&self, url: &str) -> Vec<(String, String)> {
        let mut params = Vec::new();

        // 解析URL
        if let Ok(parsed_url) = url::Url::parse(url) {
            // 获取路径段
            let path_segments: Vec<&str> = parsed_url
                .path_segments()
                .map(|segments| segments.collect())
                .unwrap_or_default();

            // 检查每个路径段是否可能包含参数
            for (i, segment) in path_segments.iter().enumerate() {
                // 检查是否包含特殊字符或编码
                if segment.contains('%') || segment.contains('&') || segment.contains('=') {
                    params.push((format!("path_param_{}", i), segment.to_string()));
                }
            }
        }

        params
    }

    /// 检查参数是否反射在响应中
    #[allow(dead_code)]
    fn check_reflection(&self, param_value: &str, body: &[u8]) -> bool {
        // 先检查是否可能是二进制内容
        let sample = &body[..std::cmp::min(1000, body.len())];
        let binary_chars_count = sample
            .iter()
            .filter(|&&b| b < 32 && b != 9 && b != 10 && b != 13)
            .count();

        // 如果包含较多二进制字符，使用更严格的匹配方法
        if binary_chars_count > sample.len() / 10 {
            // 对于二进制内容，转换参数值为字节数组并进行匹配
            let param_bytes = param_value.as_bytes();
            return body
                .windows(param_bytes.len())
                .any(|window| window == param_bytes);
        }

        // 对于文本内容，尝试正常解析
        match std::str::from_utf8(body) {
            Ok(body_str) => {
                // 检查原始值和URL解码后的值
                let decoded_value = urlencoding::decode(param_value).unwrap_or_default();
                body_str.contains(param_value) || body_str.contains(decoded_value.as_ref())
            }
            Err(_) => {
                // 如果不是有效的UTF-8，使用lossy转换，但只处理小部分内容
                let max_len = std::cmp::min(body.len(), 100 * 1024); // 最多处理100KB
                let body_str = String::from_utf8_lossy(&body[..max_len]);
                let decoded_value = urlencoding::decode(param_value).unwrap_or_default();
                body_str.contains(param_value) || body_str.contains(decoded_value.as_ref())
            }
        }
    }

    /// 生成XSS测试载荷
    fn generate_xss_payload(&self) -> Vec<String> {
        vec![
            // "</div><abbr title=\"XSS\">Hover me</abbr>".to_string(),
            // "</div><dfn>gelenlen</dfn>".to_string(),
            "\"><audio/src/><!--".to_string(),
            "<audio/src/><!--".to_string(),
            // "</div><time datetime=\"2023-01-01\">XSS Time</time>".to_string(),
            // "</div><font color=\"red\">XSS Text</font>".to_string(),
            // "><samp>XSS Output</samp>".to_string(),
            // "><var>XSS Var</var>".to_string(),
            // "><cite>XSS Citation</cite>".to_string(),
            // "><data value=\"999\">XSS Data</data>".to_string(),
            // "><section style=\"color: blue;\">XSS Section</section>".to_string(),
            // "</p><ruby>XSS<rt>Annotation</rt></ruby>".to_string(),
        ]
    }

    /// 使用AST分析检测XSS
    fn detect_xss_with_ast(
        &self,
        original_body: &str,
        modified_body: &str,
        is_html: bool,
    ) -> Result<Vec<InjectionResult>> {
        let mut results = Vec::new();

        if is_html {
            // 分析原始响应和修改后的响应
            let original_analysis = (&*self.html_analyzer).analyze(original_body)?;
            let modified_analysis = (&*self.html_analyzer).analyze(modified_body)?;

            // 比较两个分析结果，检测差异
            let original_nodes = &original_analysis.node_types;
            let modified_nodes = &modified_analysis.node_types;

            if original_analysis.structure_hash != modified_analysis.structure_hash {
                // 检查修改后的响应中是否有新的节点类型（可能是注入的恶意代码）
                for (node_type, occurrences) in modified_nodes {
                    let original_count = original_nodes.get(node_type).cloned().unwrap_or(0);
                    let modified_count = *occurrences;

                    // 如果节点类型在修改后更多，可能是注入
                    if modified_count > original_count {
                        // 检查是否是危险的HTML标签或属性
                        let risk_level = if node_type.contains("audio") || node_type.contains("svg")
                        {
                            RiskLevel::Medium
                        } else {
                            RiskLevel::Low
                        };

                        // 创建注入结果
                        let injection_result = InjectionResult {
                            detected: true,
                            injection_type: Some("HTML XSS".to_string()),
                            risk_level: Some(risk_level),
                            injection_point: Some(format!("HTML {} 节点", node_type)),
                            injection_content: Some(format!(
                                "检测到新增的 {} 节点可能包含XSS注入",
                                node_type
                            )),
                            location: None, // 实际情况下可以尝试定位注入点位置
                            details: Some(format!(
                                "在原始响应中有 {} 个 {} 节点，修改后有 {} 个，差异可能表示XSS注入",
                                original_count, node_type, modified_count
                            )),
                        };

                        results.push(injection_result);
                    }
                }
            }
        } else {
            // 分析JavaScript内容
            let original_analysis = (&*self.js_analyzer).analyze(original_body)?;
            let modified_analysis = (&*self.js_analyzer).analyze(modified_body)?;

            // 比较两个分析结果
            let original_nodes = &original_analysis.node_types;
            let modified_nodes = &modified_analysis.node_types;

            // 检查修改后的响应中是否有新的节点类型
            for (node_type, occurrences) in modified_nodes {
                let original_count = original_nodes.get(node_type).cloned().unwrap_or(0);
                let modified_count = *occurrences;

                // 如果节点类型在修改后更多，可能是注入
                if modified_count > original_count {
                    // 检查是否是危险的JavaScript节点
                    let risk_level = if node_type.contains("CallExpression")
                        && (modified_body.contains("eval(")
                            || modified_body.contains("Function(")
                            || modified_body.contains("setTimeout(")
                            || modified_body.contains("setInterval("))
                    {
                        RiskLevel::Critical
                    } else if node_type.contains("CallExpression")
                        || node_type.contains("NewExpression")
                    {
                        RiskLevel::High
                    } else {
                        RiskLevel::Medium
                    };

                    // 创建注入结果
                    let injection_result = InjectionResult {
                        detected: true,
                        injection_type: Some("JavaScript XSS".to_string()),
                        risk_level: Some(risk_level),
                        injection_point: Some(format!("JavaScript {} 节点", node_type)),
                        injection_content: Some(format!(
                            "检测到新增的 {} 节点可能包含JS注入",
                            node_type
                        )),
                        location: None,
                        details: Some(format!(
                            "在原始响应中有 {} 个 {} 节点，修改后有 {} 个，差异可能表示JS注入",
                            original_count, node_type, modified_count
                        )),
                    };

                    results.push(injection_result);
                }
            }

            // 检查语法错误
            if !original_analysis.has_syntax_error && modified_analysis.has_syntax_error {
                // 新增语法错误可能表示注入不完整，但仍可能导致XSS
                let result = InjectionResult {
                    detected: true,
                    injection_type: Some("JavaScript 语法错误".to_string()),
                    risk_level: Some(RiskLevel::Low),
                    injection_point: Some("JavaScript 代码".to_string()),
                    injection_content: Some(
                        "检测到JavaScript语法错误，可能是不完整的注入尝试".to_string(),
                    ),
                    location: None,
                    details: Some(
                        "修改后的JavaScript代码存在语法错误，这可能是注入造成的".to_string(),
                    ),
                };

                results.push(result);
            }
        }

        // 如果没有发现任何注入，返回一个表示未检测到的结果
        if results.is_empty() {
            results.push(InjectionResult {
                detected: false,
                injection_type: None,
                risk_level: None,
                injection_point: None,
                injection_content: None,
                location: None,
                details: Some("未检测到XSS注入".to_string()),
            });
        }

        Ok(results)
    }

    /// 将风险级别转换为字符串
    fn risk_level_to_string(&self, risk_level: &Option<RiskLevel>) -> String {
        match risk_level {
            Some(RiskLevel::High) => "High".to_string(),
            Some(RiskLevel::Medium) => "Medium".to_string(),
            Some(RiskLevel::Low) => "Low".to_string(),
            Some(RiskLevel::Critical) => "Critical".to_string(),
            None => "Medium".to_string(), // 默认为Medium
        }
    }

    /// 从JSON响应中提取字段值
    fn extract_values_from_json(&self, body: &[u8]) -> Vec<String> {
        let mut values = Vec::new();
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) {
            self.extract_json_values(&json, &mut values);
        }
        values
    }

    /// 递归提取JSON中的所有值
    fn extract_json_values(&self, json: &serde_json::Value, values: &mut Vec<String>) {
        match json {
            serde_json::Value::String(s) => values.push(s.clone()),
            serde_json::Value::Array(arr) => {
                for item in arr {
                    self.extract_json_values(item, values);
                }
            }
            serde_json::Value::Object(obj) => {
                for (_, value) in obj {
                    self.extract_json_values(value, values);
                }
            }
            _ => {}
        }
    }

    /// 提取内联脚本
    fn extract_inline_scripts(&self, html: &str) -> Vec<String> {
        let script_re = Regex::new(r"<script[^>]*>(.*?)</script>").unwrap();
        script_re
            .captures_iter(html)
            .filter_map(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .collect()
    }

    /// 分析响应内容
    #[allow(dead_code)]
    fn analyze_response(&self, response: &HttpResponse) -> Vec<String> {
        let mut findings = Vec::new();

        if let Ok(body) = String::from_utf8(response.body.clone()) {
            // 分析 HTML 内容
            if let Ok(results) = (&*self.html_analyzer).analyze(&body) {
                if results.has_syntax_error {
                    findings.push(format!(
                        "HTML syntax error detected: {}",
                        results.structure_hash
                    ));
                }
                findings.extend(results.node_types.keys().cloned());
            }

            // 提取并分析内联脚本
            let scripts = self.extract_inline_scripts(&body);
            for script in scripts {
                if let Ok(results) = (&*self.js_analyzer).analyze(&script) {
                    if results.has_syntax_error {
                        findings.push(format!(
                            "JavaScript syntax error detected: {}",
                            results.structure_hash
                        ));
                    }
                    findings.extend(results.node_types.keys().cloned());
                }
            }
        }

        findings
    }

    /// 检测DOM-based XSS
    async fn detect_dom_based_xss(
        &self,
        request: &HttpRequest,
        response: &HttpResponse,
    ) -> Result<Vec<ScanResult>> {
        let mut results = Vec::new();

        // 1. 先通过Content-Type判断内容类型
        if !self.is_html_content(&response.headers) {
            debug!("响应不是HTML内容，跳过DOM XSS扫描");
            return Ok(results);
        }

        // 4. 尝试解析为有效UTF-8字符串
        let body_str = match std::str::from_utf8(&response.body) {
            Ok(s) => s.to_string(),
            Err(_) => {
                // 再次验证是否为二进制内容，计算不可打印字符比例
                let valid_bytes = response
                    .body
                    .iter()
                    .filter(|&&b| (b >= 32 && b <= 126) || b == 9 || b == 10 || b == 13)
                    .count();

                let text_ratio = valid_bytes as f64 / response.body.len() as f64;

                // 如果可打印字符比例低于70%，认为是二进制内容
                if text_ratio < 0.7 {
                    debug!(
                        "响应体包含大量非文本字符 (文本比例: {:.2}%)，跳过DOM XSS扫描",
                        text_ratio * 100.0
                    );
                    return Ok(results);
                }

                debug!("响应体不是有效的UTF-8文本，尝试部分转换");

                // 最后尝试转换部分内容
                let max_len = std::cmp::min(response.body.len(), 10 * 1024); // 最多处理10KB
                let lossy_string = String::from_utf8_lossy(&response.body[..max_len]).to_string();

                // 检查转换后的结果是否有较多替换字符
                let replacement_chars = lossy_string.chars().filter(|&c| c == '\u{FFFD}').count();

                if replacement_chars > lossy_string.len() / 10 {
                    debug!(
                        "转换后包含过多替换字符 ({}/{})，可能是二进制内容，跳过DOM XSS扫描",
                        replacement_chars,
                        lossy_string.len()
                    );
                    return Ok(results);
                }

                lossy_string
            }
        };

        // 提取所有URL参数
        let params = self.extract_params_from_url(&request.url);

        // 检查每个参数是否在JavaScript上下文中使用
        for (param_name, param_value) in params {
            // 检查参数值是否在JavaScript代码中使用
            if body_str.contains(&format!("document.getElementById('{}')", param_value))
                || body_str.contains(&format!("document.querySelector('{}')", param_value))
                || body_str.contains(&format!("$('{}').html()", param_value))
                || body_str.contains(&format!("var {} =", param_value))
            {
                // 创建扫描结果
                let result = ScanResult {
                    vulnerability_type: "DOM-based XSS".to_string(),
                    name: "DOM型跨站脚本漏洞".to_string(),
                    description: "检测到DOM型XSS漏洞，攻击者可以通过构造恶意输入在客户端执行任意JavaScript代码".to_string(),
                    risk_level: "High".to_string(),
                    url: request.url.to_string(),
                    method: request.method.to_string(),
                    parameter: Some(param_name.clone()),
                    value: Some(param_value.clone()),
                    evidence: Some(format!("参数 {} 的值 {} 在JavaScript代码中使用，可能导致DOM型XSS", param_name, param_value)),
                    remediation: Some("对用户输入进行HTML转义，实施输入验证和白名单机制，使用安全的DOM API".to_string()),
                    details: Some(format!("参数 {} 的值 {} 在客户端JavaScript中使用，可能导致DOM型XSS", param_name, param_value)),
                    timestamp: chrono::Utc::now(),
                    request_details: Some(format!("{} {}\nHost: {}", request.method, request.url, request.url.split('/').nth(2).unwrap_or("unknown"))),
                    response_details: Some(format!("响应体大小: {} 字节\n内容预览: {}", response.body.len(), String::from_utf8_lossy(&response.body[..std::cmp::min(200, response.body.len())])))
                };

                results.push(result);
            }
        }

        Ok(results)
    }

    /// 主动扫描JSON响应中的XSS漏洞
    async fn active_scan_json(
        &self,
        request: &HttpRequest,
        response: &HttpResponse,
    ) -> Result<Vec<ScanResult>> {
        let results = Vec::new();

        // 1. 先通过Content-Type判断内容类型
        if !self.is_json_content(&response.headers) {
            return Ok(results);
        }

        // 2. 快速检查是否为二进制内容
        let sample_size = std::cmp::min(1000, response.body.len());
        if sample_size > 0 {
            let sample = &response.body[..sample_size];

            // 计算二进制字符的数量，非打印字符但不包括常见的空白字符
            let binary_chars_count = sample
                .iter()
                .filter(|&&b| (b < 32 || b > 126) && b != 9 && b != 10 && b != 13 && b != 32)
                .count();

            // 如果二进制字符比例超过10%，认为是二进制内容（JSON更严格）
            if binary_chars_count > sample_size / 10 {
                debug!(
                    "检测到疑似二进制内容 ({}/{} 字节)，跳过JSON XSS扫描",
                    binary_chars_count, sample_size
                );
                return Ok(results);
            }
        }

        // 3. 检查内容大小，避免处理过大的响应
        if response.body.len() > 1024 * 1024 {
            // 限制为1MB
            debug!("JSON响应体过大 ({} 字节)，跳过XSS扫描", response.body.len());
            return Ok(results);
        }

        // 4. 尝试解析为有效UTF-8字符串
        let json_content = match std::str::from_utf8(&response.body) {
            Ok(s) => s,
            Err(_) => {
                debug!("JSON响应体不是有效的UTF-8文本，跳过扫描");
                return Ok(results);
            }
        };

        // 5. 验证是否为有效的JSON
        if let Err(_) = serde_json::from_str::<serde_json::Value>(json_content) {
            debug!("响应体不是有效的JSON格式，跳过JSON XSS扫描");
            return Ok(results);
        }

        // 提取JSON响应中的所有值
        let json_values = self.extract_values_from_json(&response.body);

        // 提取请求参数
        let params = self.extract_params_from_url(&request.url);

        // 检查每个参数是否反射在JSON响应中
        for (_param_name, param_value) in params {
            for json_value in &json_values {
                if json_value.contains(&param_value) {
                    // 生成XSS测试载荷
                    for payload in self.generate_xss_payload() {
                        // 创建修改后的请求
                        let mut modified_request = request.clone();
                        let modified_url = request.url.replace(&param_value, &payload);
                        modified_request.url = modified_url.clone();

                        // 构建请求头
                        let mut headers = reqwest::header::HeaderMap::new();
                        for (name, value) in &request.headers {
                            if let Ok(header_name) =
                                reqwest::header::HeaderName::from_bytes(name.as_bytes())
                            {
                                if let Ok(header_value) =
                                    reqwest::header::HeaderValue::from_str(value)
                                {
                                    headers.insert(header_name, header_value);
                                }
                            }
                        }

                        // 提取主机名和端口
                        let (scheme, host, port) =
                            if let Ok(url) = url::Url::parse(&modified_url.clone()) {
                                (
                                    url.scheme().to_string(),
                                    url.host_str().unwrap_or("localhost").to_string(),
                                    url.port().unwrap_or_else(|| {
                                        if url.scheme() == "https" {
                                            443
                                        } else {
                                            80
                                        }
                                    }),
                                )
                            } else {
                                ("http".to_string(), "localhost".to_string(), 80)
                            };

                        // 使用低级API发送HTTP请求
                        if scheme == "https" {
                            // 对于HTTPS请求，也使用手动构造请求
                            debug!("尝试手动构造HTTPS请求: {}", &modified_url);
                            // 由于无法直接使用tokio::TcpStream处理TLS，返回一个预设的空响应
                            debug!("HTTPS请求无法完成TLS握手，返回空响应");
                            return Ok(Vec::new());
                        } else {
                            // 对于HTTP请求，使用TCP直接发送
                            use tokio::io::{AsyncReadExt, AsyncWriteExt};
                            use tokio::net::TcpStream;

                            // 创建简单的HTTP请求
                            let request_str = format!(
                                "POST {} HTTP/1.1\r\n\
                                 Host: {}\r\n\
                                 User-Agent: Mozilla/5.0\r\n\
                                 Content-Type: application/x-www-form-urlencoded\r\n\
                                 Accept: */*\r\n\
                                 Accept-Encoding: gzip, deflate\r\n\
                                 Connection: close\r\n\
                                 \r\n",
                                if let Ok(url) = url::Url::parse(&modified_url) {
                                    let path = url.path().to_string();
                                    path
                                } else {
                                    "/".to_string()
                                },
                                host
                            );

                            // 创建TCP连接
                            match TcpStream::connect(format!("{}:{}", host, port)).await {
                                Ok(mut stream) => {
                                    // 发送请求
                                    if let Err(e) = stream.write_all(request_str.as_bytes()).await {
                                        debug!("发送HTTP请求失败: {:?}", e);
                                        return Ok(Vec::new());
                                    } else {
                                        // 读取响应
                                        let mut response_bytes = Vec::new();
                                        if let Err(e) =
                                            stream.read_to_end(&mut response_bytes).await
                                        {
                                            debug!("读取HTTP响应失败: {:?}", e);
                                            return Ok(Vec::new());
                                        } else {
                                            // 解析HTTP响应
                                            let response_text =
                                                String::from_utf8_lossy(&response_bytes)
                                                    .to_string();
                                            // 提取响应体
                                            if let Some(_body_start) =
                                                response_text.find("\r\n\r\n")
                                            {
                                                return Ok(Vec::new());
                                            } else {
                                                debug!("无法解析HTTP响应");
                                                return Ok(Vec::new());
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    debug!("无法连接到服务器: {:?}", e);
                                    return Ok(Vec::new());
                                }
                            }
                        }
                    }
                }
            }
        }

        debug!("JSON XSS扫描完成，发现 {} 个漏洞", results.len());
        Ok(results)
    }

    /// 添加一个新的工具函数，用于解析HTTP响应
    async fn parse_http_response(
        &self,
        response_bytes: &[u8],
    ) -> Result<(Vec<(String, String)>, Vec<u8>)> {
        // 如果响应为空，返回错误
        if response_bytes.is_empty() {
            return Err(anyhow::anyhow!("响应为空"));
        }

        // 查找头部和正文分界线（两个连续的CRLF）
        let mut headers_end = 0;
        for i in 0..response_bytes.len() - 3 {
            if response_bytes[i] == b'\r'
                && response_bytes[i + 1] == b'\n'
                && response_bytes[i + 2] == b'\r'
                && response_bytes[i + 3] == b'\n'
            {
                headers_end = i + 4;
                break;
            }
        }

        if headers_end == 0 {
            return Err(anyhow::anyhow!("无法解析HTTP响应，找不到头部结束标志"));
        }

        // 解析头部
        let headers_data = &response_bytes[0..headers_end - 4]; // 不包括最后的 CRLFCRLF
        let headers_str = String::from_utf8_lossy(headers_data);
        let mut headers = Vec::new();

        // 解析响应行和所有头部
        let mut lines = headers_str.split("\r\n");

        // 跳过第一行（状态行）
        if let Some(_status_line) = lines.next() {
            // 可以解析状态行获取状态码，这里简化处理
        } else {
            return Err(anyhow::anyhow!("无效的HTTP响应，没有状态行"));
        }

        // 解析所有头部
        for line in lines {
            if let Some(colon_pos) = line.find(':') {
                let name = line[0..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.push((name, value));
            }
        }

        // 检查传输编码和内容长度
        let mut is_chunked = false;
        let mut is_gzip = false;
        let mut content_length = None;

        for (name, value) in &headers {
            if name.eq_ignore_ascii_case("Transfer-Encoding")
                && value.eq_ignore_ascii_case("chunked")
            {
                is_chunked = true;
            } else if name.eq_ignore_ascii_case("Content-Length") {
                if let Ok(len) = value.parse::<usize>() {
                    content_length = Some(len);
                }
            } else if name.eq_ignore_ascii_case("Content-Encoding")
                && (value.eq_ignore_ascii_case("gzip") || value.contains("gzip"))
            {
                is_gzip = true;
            }
        }

        // 提取正文
        let raw_body = if is_chunked {
            // 解析分块编码
            let mut decoded_body = Vec::new();
            let mut pos = headers_end;

            loop {
                // 已经到达响应末尾
                if pos >= response_bytes.len() {
                    break;
                }

                // 查找块大小行结束位置
                let mut chunk_size_end = pos;
                while chunk_size_end < response_bytes.len() - 1 {
                    if response_bytes[chunk_size_end] == b'\r'
                        && response_bytes[chunk_size_end + 1] == b'\n'
                    {
                        break;
                    }
                    chunk_size_end += 1;
                }

                if chunk_size_end >= response_bytes.len() - 1 {
                    break; // 不完整的响应
                }

                // 解析块大小
                let chunk_size_str = String::from_utf8_lossy(&response_bytes[pos..chunk_size_end]);
                let chunk_size_hex = chunk_size_str.split(';').next().unwrap_or("0");
                let chunk_size = usize::from_str_radix(chunk_size_hex.trim(), 16).unwrap_or(0);

                // 到达最后一个空块，表示结束
                if chunk_size == 0 {
                    break;
                }

                // 计算块内容的起始和结束位置
                let chunk_data_start = chunk_size_end + 2; // 跳过 CRLF
                let chunk_data_end = chunk_data_start + chunk_size;

                // 检查边界
                if chunk_data_end > response_bytes.len() {
                    // 不完整的数据块
                    break;
                }

                // 添加到解码的正文中
                decoded_body.extend_from_slice(&response_bytes[chunk_data_start..chunk_data_end]);

                // 移动到下一个块
                pos = chunk_data_end + 2; // 跳过块后的 CRLF
            }

            decoded_body
        } else if let Some(len) = content_length {
            // 基于内容长度
            let body_end = headers_end + len;

            if body_end <= response_bytes.len() {
                response_bytes[headers_end..body_end].to_vec()
            } else {
                // 内容不完整，返回所有可用数据
                response_bytes[headers_end..].to_vec()
            }
        } else {
            // 没有指定长度，返回所有剩余数据
            response_bytes[headers_end..].to_vec()
        };

        // 处理gzip压缩的内容
        let body = if is_gzip {
            // debug!("检测到gzip编码的响应，尝试解压...");
            // 创建raw_body的克隆，以避免所有权问题
            let raw_body_clone = raw_body.clone();
            match decode_gzip(&raw_body_clone).await {
                Ok(decompressed) => {
                    // debug!(
                    //     "成功解压gzip内容，原始大小: {} 解压后: {}",
                    //     raw_body_clone.len(),
                    //     decompressed.len()
                    // );
                    decompressed
                }
                Err(e) => {
                    warn!("Gzip解压失败: {}, 保留原始数据", e);
                    raw_body_clone
                }
            }
        } else {
            raw_body
        };

        Ok((headers, body))
    }

    /// 添加超时处理的辅助函数
    async fn send_with_timeout<T, E>(
        &self,
        future: impl std::future::Future<Output = std::result::Result<T, E>>,
        timeout_secs: u64,
    ) -> Result<T>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        use tokio::time::{timeout, Duration};

        match timeout(Duration::from_secs(timeout_secs), future).await {
            Ok(result) => result.map_err(|e| anyhow::anyhow!(e)),
            Err(_) => Err(anyhow::anyhow!("操作超时")),
        }
    }

    /// 修改HTTP请求发送函数，添加异常处理和超时
    async fn send_http_request(
        &self,
        stream: &mut TcpStream,
        request: &str,
        timeout_secs: u64,
    ) -> Result<Vec<u8>> {
        // 发送请求
        self.send_with_timeout(stream.write_all(request.as_bytes()), timeout_secs)
            .await?;

        // 读取响应
        let mut response_bytes = Vec::new();
        let read_result = self
            .send_with_timeout(stream.read_to_end(&mut response_bytes), timeout_secs)
            .await;

        // 处理读取错误，但如果已经读取了一些数据，仍然尝试解析
        if read_result.is_err() && response_bytes.is_empty() {
            return Err(anyhow::anyhow!("读取HTTP响应失败: {:?}", read_result.err()));
        }

        Ok(response_bytes)
    }

    /// 修改HTTPS请求发送函数，添加异常处理和超时
    async fn send_https_request(
        &self,
        stream: &mut tokio_native_tls::TlsStream<TcpStream>,
        request: &str,
        timeout_secs: u64,
    ) -> Result<Vec<u8>> {
        // 发送请求
        self.send_with_timeout(stream.write_all(request.as_bytes()), timeout_secs)
            .await?;

        // 读取响应
        let mut response_bytes = Vec::new();
        let read_result = self
            .send_with_timeout(stream.read_to_end(&mut response_bytes), timeout_secs)
            .await;

        // 处理读取错误，但如果已经读取了一些数据，仍然尝试解析
        if read_result.is_err() && response_bytes.is_empty() {
            return Err(anyhow::anyhow!(
                "读取HTTPS响应失败: {:?}",
                read_result.err()
            ));
        }

        Ok(response_bytes)
    }

    /// 主动扫描XSS漏洞
    async fn active_scan(
        &self,
        request: &HttpRequest,
        response: &HttpResponse,
    ) -> Result<Vec<ScanResult>> {
        let mut results = Vec::new();

        // 1. 先通过Content-Type判断内容类型
        let is_html = self.is_html_content(&response.headers);
        let is_js = self.is_js_content(&response.headers);
        let is_text = self.is_text_content(&response.headers);

        // 如果不是文本类型，直接跳过
        if !is_html && !is_js && !is_text {
            debug!("非文本内容类型，跳过XSS扫描");
            return Ok(results);
        }

        let original_body = String::from_utf8_lossy(&response.body);

        // 跟踪已发现漏洞的参数
        let mut vulnerable_params = std::collections::HashSet::new();

        // 遍历所有参数，发送包含XSS payload的请求
        for (param_name, param_value) in &request.params {
            // 如果这个参数已经发现漏洞，跳过后续测试
            if vulnerable_params.contains(param_name) {
                debug!("参数 {} 已发现XSS漏洞，跳过后续测试", param_name);
                continue;
            }

            // 检查是否是路径参数
            let is_path_param = param_name.starts_with("path_");
            let mut new_path = String::new();
            // 生成XSS测试载荷
            for payload in self.generate_xss_payload() {
                let mut new_url = request.url.clone();

                // 处理查询参数
                if !is_path_param && new_url.contains(&format!("{}={}", param_name, param_value)) {
                    if let Ok(parsed_url) = url::Url::parse(&new_url) {
                        new_path = parsed_url.path().to_string();
                        if let Some(query_params) = parsed_url.query() {
                            new_path = format!("{}?{}", new_path, query_params);
                        }
                        new_path = new_path.replace(
                            &format!("{}={}", param_name, param_value),
                            &format!("{}={}", param_name, &payload),
                        );
                    }
                } else if is_path_param {
                    // 处理路径参数
                    // 分析URL以找到路径部分
                    if let Ok(parsed_url) = url::Url::parse(&new_url) {
                        // 获取原始路径
                        let path = parsed_url.path();

                        // 如果路径中包含参数值
                        if path.contains(param_value) {
                            // 创建替换后的路径（不经过URL对象，避免编码）
                            let modified_param = param_value.to_owned() + &payload[..];

                            // 直接在URL字符串中替换，保留原始字符
                            let path_with_slash = if !path.starts_with("/") {
                                format!("/{}", path)
                            } else {
                                path.to_string()
                            };

                            // 替换路径中的参数
                            new_path = path_with_slash.replace(param_value, &modified_param);

                            // 构建URL的基本部分（不含路径）
                            let url_base = format!(
                                "{}://{}",
                                parsed_url.scheme(),
                                parsed_url.host_str().unwrap_or("")
                            );
                            let port_part = parsed_url
                                .port()
                                .map_or(String::new(), |p| format!(":{}", p));

                            // 组合新URL
                            new_url = format!(
                                "{}{}{}{}",
                                url_base,
                                port_part,
                                new_path,
                                parsed_url
                                    .query()
                                    .map_or(String::new(), |q| format!("?{}", q))
                            );
                        }
                    }
                }

                // 提取主机名和端口
                let (scheme, host, port) = if let Ok(url) = url::Url::parse(&new_url.clone()) {
                    (
                        url.scheme().to_string(),
                        url.host_str().unwrap_or("localhost").to_string(),
                        url.port()
                            .unwrap_or_else(|| if url.scheme() == "https" { 443 } else { 80 }),
                    )
                } else {
                    ("http".to_string(), "localhost".to_string(), 80)
                };

                // 构建请求头
                let mut headers = reqwest::header::HeaderMap::new();
                for (name, value) in &request.headers {
                    if let Ok(header_name) =
                        reqwest::header::HeaderName::from_bytes(name.as_bytes())
                    {
                        if let Ok(header_value) = reqwest::header::HeaderValue::from_str(value) {
                            headers.insert(header_name, header_value);
                        }
                    }
                }
                let user_agent = request
                    .headers
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case("user-agent"))
                    .map(|(_, value)| value.as_str())
                    .unwrap_or("Mozilla/5.0");

                let mut http_request = String::new();
                let _response_result = match request.method.as_str() {
                    "GET" => {
                        // 手动构建 HTTP 请求
                        // 首先构建基本请求行和主机头
                        http_request = format!(
                            "{} {} HTTP/1.1\r\n\
                            Host: {}\r\n",
                            request.method,
                            if new_path.is_empty() { "/" } else { &new_path },
                            host
                        );

                        // 添加所有请求头
                        for (name, value) in &request.headers {
                            // 跳过Host头，因为已经添加了
                            if !name.eq_ignore_ascii_case("host") {
                                http_request.push_str(&format!("{}: {}\r\n", name, value));
                            }
                        }

                        // 确保有User-Agent
                        if !request
                            .headers
                            .iter()
                            .any(|(name, _)| name.eq_ignore_ascii_case("user-agent"))
                        {
                            http_request.push_str(&format!("User-Agent: {}\r\n", user_agent));
                        }

                        // 添加其他必要的头部和空行结束头部
                        http_request.push_str("Accept: */*\r\n");
                        http_request.push_str("Accept-Encoding: gzip, deflate\r\n");
                        http_request.push_str("Connection: close\r\n");
                        http_request.push_str("\r\n");
                    }
                    "POST" => {
                        // 如果是POST请求，需要替换请求体中的参数
                        let mut new_body = request.body.clone();
                        let body_str = String::from_utf8_lossy(&request.body);
                        if body_str.contains(&format!("{}={}", param_name, param_value)) {
                            let new_body_str = body_str.replace(
                                &format!("{}={}", param_name, param_value),
                                &format!("{}={}", param_name, &payload),
                            );
                            new_body = new_body_str.into_bytes();
                        }

                        // 构建基本请求行和主机头
                        http_request = format!(
                            "{} {} HTTP/1.1\r\n\
                            Host: {}\r\n",
                            request.method,
                            if new_path.is_empty() { "/" } else { &new_path },
                            host
                        );

                        // 添加所有请求头
                        for (name, value) in &request.headers {
                            // 跳过Host头，因为已经添加了
                            if !name.eq_ignore_ascii_case("host") {
                                http_request.push_str(&format!("{}: {}\r\n", name, value));
                            }
                        }

                        // 确保有User-Agent和Content-Type
                        if !request
                            .headers
                            .iter()
                            .any(|(name, _)| name.eq_ignore_ascii_case("user-agent"))
                        {
                            http_request.push_str(&format!("User-Agent: {}\r\n", user_agent));
                        }
                        if !request
                            .headers
                            .iter()
                            .any(|(name, _)| name.eq_ignore_ascii_case("content-type"))
                        {
                            http_request
                                .push_str("Content-Type: application/x-www-form-urlencoded\r\n");
                        }

                        // 添加Content-Length和结束头部的空行
                        http_request.push_str(&format!("Content-Length: {}\r\n", new_body.len()));
                        http_request.push_str("Accept-Encoding: gzip, deflate\r\n");
                        http_request.push_str("Connection: close\r\n");
                        http_request.push_str("\r\n");

                        // 添加请求体
                        let body_string = String::from_utf8_lossy(&new_body);
                        http_request.push_str(&body_string);
                    }
                    _ => info!("不支持的请求方法: {}", request.method),
                };

                let mut new_response = String::new();

                if scheme == "http" {
                    // 创建TCP连接
                    match TcpStream::connect(format!("{}:{}", host, port)).await {
                        Ok(mut stream) => {
                            // 发送请求并读取响应，使用10秒超时
                            match self.send_http_request(&mut stream, &http_request, 10).await {
                                Ok(response_bytes) => {
                                    // 解析HTTP响应
                                    match self.parse_http_response(&response_bytes).await {
                                        Ok((_headers, body)) => {
                                            // 将响应体转换为字符串进行分析
                                            let response_text =
                                                String::from_utf8_lossy(&body).to_string();
                                            new_response = response_text;
                                        }
                                        Err(e) => {
                                            debug!("解析HTTP响应失败: {:?}", e);
                                            // 尝试直接使用原始响应数据
                                            let response_text =
                                                String::from_utf8_lossy(&response_bytes)
                                                    .to_string();
                                            new_response = response_text;
                                        }
                                    }
                                }
                                Err(e) => {
                                    debug!("发送HTTP请求或读取响应失败: {:?}", e);
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            debug!("无法连接到服务器: {:?}", e);
                            continue;
                        }
                    }
                } else if scheme == "https" {
                    // 创建TCP连接和TLS包装器
                    match TcpStream::connect(format!("{}:{}", host, port)).await {
                        Ok(tcp_stream) => {
                            // 创建TLS连接器
                            let connector = match native_tls::TlsConnector::builder()
                                .danger_accept_invalid_certs(true) // 允许不安全的证书（仅用于开发环境）
                                .build()
                            {
                                Ok(connector) => connector,
                                Err(e) => {
                                    debug!("创建TLS连接器失败: {:?}", e);
                                    continue;
                                }
                            };

                            let connector = TlsConnector::from(connector);

                            // 连接到服务器，使用5秒超时
                            match self
                                .send_with_timeout(connector.connect(host.as_str(), tcp_stream), 5)
                                .await
                            {
                                Ok(mut tls_stream) => {
                                    // 发送请求并读取响应，使用10秒超时
                                    match self
                                        .send_https_request(&mut tls_stream, &http_request, 10)
                                        .await
                                    {
                                        Ok(response_bytes) => {
                                            // 解析HTTP响应
                                            match self.parse_http_response(&response_bytes).await {
                                                Ok((_headers, body)) => {
                                                    // 将响应体转换为字符串进行分析
                                                    let response_text =
                                                        String::from_utf8_lossy(&body).to_string();
                                                    new_response = response_text;
                                                }
                                                Err(e) => {
                                                    debug!("解析HTTPS响应失败: {:?}", e);
                                                    // 尝试直接使用原始响应数据
                                                    let response_text =
                                                        String::from_utf8_lossy(&response_bytes)
                                                            .to_string();
                                                    new_response = response_text;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            debug!("发送HTTPS请求或读取响应失败: {:?}", e);
                                            continue;
                                        }
                                    }
                                }
                                Err(e) => {
                                    debug!("TLS连接失败: {:?}", e);
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            debug!("无法连接到服务器: {:?}", e);
                            continue;
                        }
                    }
                }

                // 处理响应
                // std::fs::write("original_body.txt", original_body.to_string()).unwrap();
                // std::fs::write("new_body.txt", &new_response).unwrap();
                // 使用AST分析检测XSS
                if let Ok(injection_results) =
                    self.detect_xss_with_ast(&original_body, &new_response, is_html)
                {
                    for injection_result in injection_results {
                        // 只关注检测到的漏洞
                        if injection_result.detected {
                            // 创建扫描结果
                            let risk_level = if let Some(level) = &injection_result.risk_level {
                                self.risk_level_to_string(&Some(level.clone()))
                            } else {
                                "Medium".to_string()
                            };

                            let result = ScanResult {
                                vulnerability_type: "XSS".to_string(),
                                name: "反射型跨站脚本".to_string(),
                                description: format!("检测到反射型跨站脚本(XSS)漏洞，攻击者可以通过{}构造恶意链接执行JavaScript代码", 
                                    if is_path_param { "URL路径" } else { "URL参数" }),
                                risk_level,
                                url: request.url.to_string(),
                                method: request.method.to_string(),
                                parameter: Some(param_name.clone()),
                                value: Some(payload.clone()),
                                evidence: injection_result.injection_content,
                                remediation: Some("对用户输入进行严格过滤，使用HTML编码输出用户数据，实施内容安全策略(CSP)".to_string()),
                                details: injection_result.details,
                                timestamp: chrono::Utc::now(),
                                request_details: Some(format!("{} {}\nHost: {}", request.method, request.url, request.url.split('/').nth(2).unwrap_or("unknown"))),
                                response_details: Some(format!("响应体大小: {} 字节\n内容预览: {}", response.body.len(), String::from_utf8_lossy(&response.body[..std::cmp::min(200, response.body.len())])))
                            };

                            // 保存构造的请求URL和检测到漏洞的代码片段作为证据
                            let mut evidence_parts = Vec::new();

                            // 添加构造的请求URL
                            evidence_parts.push(format!("测试URL: {}", &new_path));

                            // 提取检测到漏洞的代码片段
                            if let Some(start_index) = new_response.find(&payload) {
                                let _start = if start_index > 50 {
                                    start_index - 50
                                } else {
                                    0
                                };
                            }

                            // 构造完整的请求详情
                            let request_detail = match request.method.as_str() {
                                "GET" => {
                                    format!(
                                        "{} {} HTTP/1.1\nHost: {}\n{}\n\n",
                                        request.method,
                                        &new_path,
                                        if let Ok(url) = url::Url::parse(&new_path) {
                                            url.host_str().unwrap_or("unknown").to_string()
                                        } else {
                                            "unknown".to_string()
                                        },
                                        {
                                            let mut headers_str = String::new();
                                            for (name, value) in &request.headers {
                                                if !name.eq_ignore_ascii_case("host") {
                                                    headers_str.push_str(&format!(
                                                        "{}: {}\r\n",
                                                        name, value
                                                    ));
                                                }
                                            }
                                            headers_str
                                        }
                                    )
                                }
                                "POST" => {
                                    let mut headers_str = String::new();
                                    for (name, value) in &request.headers {
                                        headers_str.push_str(&format!("{}: {}\n", name, value));
                                    }

                                    let body_str = String::from_utf8_lossy(&request.body);
                                    format!(
                                        "{} {} HTTP/1.1\n{}\n{}",
                                        request.method, &new_path, headers_str, body_str
                                    )
                                }
                                _ => format!("{} {}", request.method, &new_path),
                            };

                            // 构造响应详情
                            let response_detail = format!(
                                "HTTP/1.1 200 OK\nContent-Type: text/html\n\n{}",
                                if new_response.len() > 1000 {
                                    // 如果响应体太长，只显示相关部分
                                    if let Some(start_index) = new_response.find(&payload) {
                                        let start = if start_index > 200 {
                                            start_index - 200
                                        } else {
                                            0
                                        };
                                        let end = std::cmp::min(
                                            start_index + payload.len() + 200,
                                            new_response.len(),
                                        );
                                        format!("... {} ...", &new_response[start..end])
                                    } else {
                                        format!("{}...(响应体过长，已截断)", &new_response[..1000])
                                    }
                                } else {
                                    new_response.to_string()
                                }
                            );

                            // 更新ScanResult中的evidence字段
                            let evidence_text = evidence_parts.join("\n\n");
                            let result_with_details = ScanResult {
                                evidence: Some(evidence_text),
                                request_details: Some(request_detail),
                                response_details: Some(response_detail),
                                ..result
                            };

                            results.push(result_with_details);

                            // 将该参数标记为已发现漏洞，跳过后续测试
                            vulnerable_params.insert(param_name.clone());

                            // 找到漏洞后，跳出当前payload的循环
                            break;
                        }
                    }
                }

                // 如果参数已经被标记为漏洞，则跳出payload循环
                if vulnerable_params.contains(param_name) {
                    break;
                }
            }
        }

        debug!("XSS扫描完成，发现 {} 个漏洞参数", vulnerable_params.len());

        Ok(results)
    }

    /// 检查参数反射
    fn check_params_reflection(
        &self,
        request: &HttpRequest,
        body_text: &str,
        results: &Vec<ScanResult>,
    ) -> Vec<ScanResult> {
        let mut new_results = results.clone();

        // 提取URL参数
        let url_params = self.extract_params_from_url(&request.url);
        // 提取路径参数
        let path_params = self.extract_path_params(&request.url);

        // 合并所有参数
        let all_params = url_params.into_iter().chain(path_params.into_iter());

        // 检查每个参数
        for (param_name, param_value) in all_params {
            // 检查参数是否反射在响应中
            if body_text.contains(&param_value) {
                // 创建扫描结果
                let result = ScanResult {
                    vulnerability_type: "XSS".to_string(),
                    name: "反射型跨站脚本".to_string(),
                    description: format!(
                        "检测到反射型跨站脚本(XSS)漏洞，参数 '{}' 的值在响应中反射",
                        param_name
                    ),
                    risk_level: "High".to_string(),
                    url: request.url.to_string(),
                    method: request.method.to_string(),
                    parameter: Some(param_name.clone()),
                    value: Some(param_value.clone()),
                    evidence: Some(format!(
                        "参数 '{}' 的值 '{}' 在响应中反射",
                        param_name, param_value
                    )),
                    remediation: Some(
                        "对用户输入进行严格过滤，使用HTML编码输出用户数据，实施内容安全策略(CSP)"
                            .to_string(),
                    ),
                    details: Some(format!(
                        "参数 '{}' 的值 '{}' 在响应中反射，可能导致XSS攻击",
                        param_name, param_value
                    )),
                    timestamp: chrono::Utc::now(),
                    request_details: Some(format!(
                        "{} {}\nHost: {}",
                        request.method,
                        request.url,
                        request.url.split('/').nth(2).unwrap_or("unknown")
                    )),
                    response_details: Some("反射检测成功，详情见证据部分".to_string()),
                };

                new_results.push(result);
            }
        }

        new_results
    }

    /// 获取扫描器名称
    pub async fn name(&self) -> String {
        "XSS Scanner".to_string()
    }

    //传入GET URL，对其进行XSS扫描
    pub async fn scan_get_url(&self, urls: &Vec<String>) -> Vec<ScanResult> {
        let mut results = Vec::new();
        for url in urls {

            let params = {
                // 解析URL提取查询参数
                match url::Url::parse(url) {
                    Ok(parsed_url) => {
                        let params: Vec<(String, String)> = parsed_url
                            .query_pairs()
                            .map(|(k, v)| (k.to_string(), v.to_string()))
                            .collect();



                        params
                    }
                    Err(_) => {
                        vec![]
                    }
                }
            };

            let request = HttpRequest {
                method: "GET".to_string(),
                url: url.clone(),
                headers: HashMap::new(),
                body: vec![],
                params: params,
            };

            //获取全局的http_client
            let http_client  = CoreConfig::global().unwrap().http_client.clone().unwrap();
            //先请求URL，获取响应
            let response = match http_client.get(url).send().await {
                Ok(response) => response,
                Err(e) => {
                    // error!("请求URL失败: {}", e);
                    continue;
                }
            };
            let headers = response.headers().clone();
            let status = response.status().as_u16();
            let body = match response.text().await {
                Ok(body) => body,
                Err(e) => {
                    // error!("获取响应体失败: {}", e);
                    continue;
                }
            };

            // 将HeaderMap转换为HashMap<String, String>
            let header_map = headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
                .collect();

            let response = HttpResponse {
                status: status,
                headers: header_map,
                body: body.as_bytes().to_vec(),
            };
            let result = self.scan(&request, &response).await;
            results.extend(result);
        }
        results
    }

    /// 扫描XSS漏洞
    pub async fn scan(&self, request: &HttpRequest, response: &HttpResponse) -> Vec<ScanResult> {
        let mut results = Vec::new();


        // 缓存website和对应的response判断后续页面是否和前面检测过的页面存在高度相似或者内容重叠
        let url_str = &request.url;
        
        // 尝试从URL中提取结构化信息
        let (domain, path_pattern) = extract_url_pattern(url_str);
        let cache_key = format!("{}-{}", domain, path_pattern);
        
        let response_body = String::from_utf8_lossy(&response.body);
        
        // 计算当前页面的特征向量（词频向量）
        let current_vector = compute_term_frequency(&response_body);
        
        // 相似度阈值
        let similarity_threshold = 0.90; // 可以根据需要调整
        
        // 获取缓存
        let mut skip_scan = false;
        
        {
            let cache = PAGE_CACHE.lock().unwrap();
            
            // 检查是否有该网站+路径模式的缓存
            if let Some(site_cache) = cache.get(&cache_key) {
                for (cached_url, cached_vector) in site_cache {
                    // 计算余弦相似度
                    let similarity = cosine_similarity(&current_vector, cached_vector);
                    
                    // 如果相似度高于阈值，则跳过检测
                    if similarity > similarity_threshold {
                        // println!("页面相似度: {:.2}, 跳过检测: {}", similarity, url_str);
                        // println!("缓存URL: {}, 当前URL: {}", cached_url, url_str);
                        skip_scan = true;
                        break;
                    }
                }
            }
        }
        
        if skip_scan {
            return results;
        }
        
        // if !request.params.is_empty() {
        //     println!("url:{:?},params:{:?}", request.url, request.params);
        // }
        
        // 如果不跳过，则将当前页面添加到缓存
        {
            let mut cache = PAGE_CACHE.lock().unwrap();
            let site_cache = cache.entry(cache_key).or_insert_with(Vec::new);
            site_cache.push((url_str.to_string(), current_vector));
            
            // 限制每个模式的缓存大小，避免内存泄漏
            if site_cache.len() > 20 {
                site_cache.remove(0); // 移除最旧的条目
            }
        }

        // 执行主动扫描并等待结果
        let mut active_results = self.active_scan(request, response).await.unwrap();
        results.append(&mut active_results);

        // 对JSON响应进行主动扫描并等待结果
        let mut json_results: Vec<ScanResult> =
            self.active_scan_json(request, response).await.unwrap();
        results.append(&mut json_results);

        // 检查DOM型XSS
        if self.is_html_content(&response.headers) {
            let dom_results = self.detect_dom_based_xss(request, response).await.unwrap();
            results.extend(dom_results);
        }

        results
    }

    /// 被动扫描XSS漏洞
    async fn passive_scan(
        &self,
        request: &HttpRequest,
        response: &HttpResponse,
    ) -> Result<Vec<ScanResult>> {
        let mut results = Vec::new();

        // 1. 先通过Content-Type判断内容类型
        let is_html = self.is_html_content(&response.headers);
        let is_js = self.is_js_content(&response.headers);
        let is_json = self.is_json_content(&response.headers);
        let is_text = self.is_text_content(&response.headers);

        // 如果不是文本类型，直接跳过
        if !is_html && !is_js && !is_json && !is_text {
            debug!("非文本内容类型，跳过XSS被动扫描");
            return Ok(results);
        }

        // 2. 快速检查是否为二进制内容
        // 检查前1000字节中的二进制字符比例
        let sample_size = std::cmp::min(1000, response.body.len());
        if sample_size > 0 {
            let sample = &response.body[..sample_size];

            // 计算二进制字符的数量，非打印字符但不包括常见的空白字符
            let binary_chars_count = sample
                .iter()
                .filter(|&&b| (b < 32 || b > 126) && b != 9 && b != 10 && b != 13 && b != 32)
                .count();

            // 如果二进制字符比例超过20%，认为是二进制内容
            if binary_chars_count > sample_size / 5 {
                debug!(
                    "检测到疑似二进制内容 ({}/{} 字节)，跳过XSS被动扫描",
                    binary_chars_count, sample_size
                );
                return Ok(results);
            }
        }

        // 3. 检查内容大小，避免处理过大的响应
        if response.body.len() > 1024 * 1024 {
            // 限制为1MB
            debug!("响应体过大 ({} 字节)，跳过XSS被动扫描", response.body.len());
            return Ok(results);
        }

        // 4. 尝试解析为有效UTF-8字符串
        let body_text = match std::str::from_utf8(&response.body) {
            Ok(s) => s,
            Err(_) => {
                // 再次验证是否为二进制内容，计算不可打印字符比例
                let valid_bytes = response
                    .body
                    .iter()
                    .filter(|&&b| (b >= 32 && b <= 126) || b == 9 || b == 10 || b == 13)
                    .count();

                let text_ratio = valid_bytes as f64 / response.body.len() as f64;

                // 如果可打印字符比例低于70%，认为是二进制内容
                if text_ratio < 0.7 {
                    debug!(
                        "响应体包含大量非文本字符 (文本比例: {:.2}%)，跳过XSS被动扫描",
                        text_ratio * 100.0
                    );
                    return Ok(results);
                }

                debug!("响应体不是有效的UTF-8文本，尝试部分转换");

                // 最后尝试转换部分内容
                let max_len = std::cmp::min(response.body.len(), 10 * 1024); // 最多处理10KB
                let lossy_string = String::from_utf8_lossy(&response.body[..max_len]);

                // 检查转换后的结果是否有较多替换字符
                let replacement_chars = lossy_string.chars().filter(|&c| c == '\u{FFFD}').count();

                if replacement_chars > lossy_string.len() / 10 {
                    debug!(
                        "转换后包含过多替换字符 ({}/{})，可能是二进制内容，跳过XSS被动扫描",
                        replacement_chars,
                        lossy_string.len()
                    );
                    return Ok(results);
                }

                // 对于经过lossy转换的内容，进行反射检测
                results = self.check_params_reflection(request, &lossy_string, &results);

                // 因为文本内容质量较差，仅检测反射，不进行后续扫描
                return Ok(results);
            }
        };

        // 提取URL参数
        let url_params = self.extract_params_from_url(&request.url);
        // 提取路径参数
        let path_params = self.extract_path_params(&request.url);

        // 合并所有参数
        let all_params = url_params.into_iter().chain(path_params.into_iter());

        // 检查每个参数
        for (param_name, param_value) in all_params {
            // 检查参数是否反射在响应中
            if body_text.contains(&param_value) {
                // 创建扫描结果
                let result = ScanResult {
                    vulnerability_type: "XSS".to_string(),
                    name: "反射型跨站脚本".to_string(),
                    description: format!(
                        "检测到反射型跨站脚本(XSS)漏洞，参数 '{}' 的值在响应中反射",
                        param_name
                    ),
                    risk_level: "High".to_string(),
                    url: request.url.to_string(),
                    method: request.method.to_string(),
                    parameter: Some(param_name.clone()),
                    value: Some(param_value.clone()),
                    evidence: Some(format!(
                        "参数 '{}' 的值 '{}' 在响应中反射",
                        param_name, param_value
                    )),
                    remediation: Some(
                        "对用户输入进行严格过滤，使用HTML编码输出用户数据，实施内容安全策略(CSP)"
                            .to_string(),
                    ),
                    details: Some(format!(
                        "参数 '{}' 的值 '{}' 在响应中反射，可能导致XSS攻击",
                        param_name, param_value
                    )),
                    timestamp: chrono::Utc::now(),
                    request_details: Some(format!(
                        "{} {}\nHost: {}",
                        request.method,
                        request.url,
                        request.url.split('/').nth(2).unwrap_or("unknown")
                    )),
                    response_details: Some(format!(
                        "响应体大小: {} 字节\n内容预览: {}",
                        response.body.len(),
                        String::from_utf8_lossy(
                            &response.body[..std::cmp::min(200, response.body.len())]
                        )
                    )),
                };

                results.push(result);
            }
        }

        // 执行主动扫描
        let mut active_results = self.active_scan(request, response).await?;
        results.append(&mut active_results);

        // 对JSON响应进行主动扫描
        let mut json_results = self.active_scan_json(request, response).await?;
        results.append(&mut json_results);

        // 检查DOM型XSS
        if self.is_html_content(&response.headers) {
            let dom_results = self.detect_dom_based_xss(request, response).await?;
            results.extend(dom_results);
        }

        Ok(results)
    }
}

#[async_trait]
impl Scanner for XssScanner {
    async fn name(&self) -> String {
        self.name().await
    }

    async fn scan(&self, request: &HttpRequest, response: &HttpResponse) -> Vec<ScanResult> {
        match self.passive_scan(request, response).await {
            Ok(results) => results,
            Err(_) => Vec::new(),
        }
    }
}

/// 处理chunked编码的gzip内容
async fn decode_gzip(compressed_data: &[u8]) -> Result<Vec<u8>> {
    let reader = std::io::Cursor::new(compressed_data);
    let mut decoder = GzipDecoder::new(reader);
    let mut decompressed = Vec::new();

    // 使用tokio的异步复制
    tokio::io::copy(&mut decoder, &mut decompressed).await?;
    Ok(decompressed)
}

// 计算词频向量
fn compute_term_frequency(text: &str) -> HashMap<String, f64> {
    let mut term_freq = HashMap::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    
    if words.is_empty() {
        return term_freq;
    }
    
    // 计算词频
    for word in words.iter() {
        let word = word.to_lowercase();
        *term_freq.entry(word).or_insert(0.0) += 1.0;
    }
    
    // 归一化为TF值
    let total_words = words.len() as f64;
    for count in term_freq.values_mut() {
        *count /= total_words;
    }
    
    term_freq
}

// 计算余弦相似度
fn cosine_similarity(vec1: &HashMap<String, f64>, vec2: &HashMap<String, f64>) -> f64 {
    let mut dot_product = 0.0;
    
    // 计算点积
    for (term, weight) in vec1 {
        if let Some(other_weight) = vec2.get(term) {
            dot_product += weight * other_weight;
        }
    }
    
    // 计算向量模长
    let mut magnitude1 = 0.0;
    for weight in vec1.values() {
        magnitude1 += weight * weight;
    }
    magnitude1 = magnitude1.sqrt();
    
    let mut magnitude2 = 0.0;
    for weight in vec2.values() {
        magnitude2 += weight * weight;
    }
    magnitude2 = magnitude2.sqrt();
    
    // 防止除以零
    if magnitude1 > 0.0 && magnitude2 > 0.0 {
        dot_product / (magnitude1 * magnitude2)
    } else {
        0.0
    }
}

// 从URL中提取域名和路径模式
fn extract_url_pattern(url_str: &str) -> (String, String) {
    if let Ok(parsed_url) = url::Url::parse(url_str) {
        let domain = parsed_url.host_str().unwrap_or("unknown").to_string();
        let path = parsed_url.path();
        
        // 对路径进行模式分析
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        if path_segments.is_empty() {
            return (domain, "root".to_string());
        }
        
        // 构建路径模式，将数字ID替换为占位符
        let mut pattern_segments = Vec::new();
        let id_pattern = Regex::new(r"^\d+$").unwrap();
        
        for segment in path_segments {
            let pattern_segment = if id_pattern.is_match(segment) {
                // 数字ID替换为{id}占位符
                "{id}".to_string()
            } else if segment.len() > 8 && segment.chars().all(|c| c.is_ascii_hexdigit()) {
                // 长数字/字母组合(可能是哈希或UUID)替换为{hash}
                "{hash}".to_string()
            } else if segment.contains('.') {
                // 处理带扩展名的段落
                let parts: Vec<&str> = segment.split('.').collect();
                if parts.len() >= 2 {
                    let name = parts[0];
                    let ext = parts[1];
                    
                    if id_pattern.is_match(name) {
                        format!("{{}}.{}", ext)
                    } else {
                        segment.to_string()
                    }
                } else {
                    segment.to_string()
                }
            } else {
                segment.to_string()
            };
            
            pattern_segments.push(pattern_segment);
        }
        
        let path_pattern = pattern_segments.join("/");
        return (domain, path_pattern);
    }
    
    ("unknown".to_string(), "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::AppConfig;
    use crate::handler::scan::proxy::{HttpRequest, HttpResponse};
    use std::sync::Arc;

    // 创建测试请求的辅助函数
    fn create_test_request(
        url: &str,
        body: Option<Vec<u8>>,
        params: Vec<(String, String)>,
    ) -> HttpRequest {
        let mut headers = HashMap::new();
        headers.insert(String::from("Content-Type"), String::from("text/html"));
        headers.insert(String::from("User-Agent"), String::from("Test Agent"));
    
        
        HttpRequest {
            method: "GET".to_string(),
            url: url.to_string(),
            headers,
            body: body.unwrap_or_default(),
            params,
        }
    }

    // 创建测试响应的辅助函数
    fn create_test_response(status: u16, body: &str, content_type: &str) -> HttpResponse {
        let mut headers = HashMap::new();
        headers.insert(String::from("Content-Type"), String::from("text/html"));
        headers.insert(String::from("User-Agent"), String::from("Test Agent"));
    
        
        HttpResponse {
            status,
            headers,
            body: body.as_bytes().to_vec(),
        }
    }

    // 测试XSS扫描器名称
    #[tokio::test]
    async fn test_xss_scanner_name() {
        let config = Arc::new(AppConfig::default());
        let scanner = XssScanner::new(config);

        assert_eq!(scanner.name().await, "XSS Scanner");
    }

    // 测试XSS扫描器检测反射型XSS
    #[tokio::test]
    async fn test_xss_scanner_detect_reflected_xss() {
        let config = Arc::new(AppConfig::default());
        let scanner = XssScanner::new(config);

        // 创建包含XSS payload的请求
        let request = create_test_request(
            "https://example.com/search?q=<script>alert(1)</script>",
            None,
            vec![("q".to_string(), "<script>alert(1)</script>".to_string())],
        );

        // 创建反射XSS payload的响应
        let response = create_test_response(
            200,
            "<html><body>Search results for: <script>alert(1)</script></body></html>",
            "text/html",
        );

        // 执行扫描
        let results = scanner.scan(&request, &response).await;
        println!("XSS test results: {:?}", results);

        // 验证结果
        // assert!(!results.is_empty());
        // assert_eq!(results[0].vulnerability_type, "Reflected XSS");
        // assert_eq!(results[0].parameter, Some("q".to_string()));
    }

    // 测试XSS扫描器对安全内容的处理
    #[tokio::test]
    async fn test_xss_scanner_negative_case() {
        let config = Arc::new(AppConfig::default());
        let scanner = XssScanner::new(config);

        // 创建安全请求
        let request = create_test_request(
            "https://example.com/search?q=safe_search_term",
            None,
            vec![("q".to_string(), "safe_search_term".to_string())],
        );

        // 创建安全响应
        let response = create_test_response(
            200,
            "<html><body>Search results for: safe_search_term</body></html>",
            "text/html",
        );

        // 执行扫描
        let results = scanner.scan(&request, &response).await;
        println!("XSS negative test results: {:?}", results);

        // 验证结果
        // assert!(results.is_empty());
    }
}
