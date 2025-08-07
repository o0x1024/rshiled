use crate::core::config::AppConfig;
use crate::handler::scan::proxy::{HttpRequest, HttpResponse};
use crate::handler::scan::engine::ScanResult;
use crate::handler::scan::scanners::Scanner;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;

/// RCE扫描器
#[derive(Clone)]
pub struct RceScanner {
    /// 配置
    config: Arc<AppConfig>,
    /// RCE错误模式
    error_patterns: HashMap<String, Vec<Regex>>,
    /// RCE测试载荷
    payloads: HashMap<String, Vec<String>>,
}

impl RceScanner {
    /// 创建新的RCE扫描器
    pub fn new(config: Arc<AppConfig>) -> Self {
        // 初始化RCE错误模式
        let mut error_patterns = HashMap::new();
        
        // PHP错误
        let mut php_patterns = Vec::new();
        php_patterns.push(Regex::new(r"Warning: system\(").unwrap());
        php_patterns.push(Regex::new(r"Warning: exec\(").unwrap());
        php_patterns.push(Regex::new(r"Warning: shell_exec\(").unwrap());
        php_patterns.push(Regex::new(r"Warning: passthru\(").unwrap());
        php_patterns.push(Regex::new(r"Warning: popen\(").unwrap());
        php_patterns.push(Regex::new(r"Warning: proc_open\(").unwrap());
        php_patterns.push(Regex::new(r"Fatal error: Uncaught Error: Call to undefined function").unwrap());
        
        // Java错误
        let mut java_patterns = Vec::new();
        java_patterns.push(Regex::new(r"java\.io\.IOException").unwrap());
        java_patterns.push(Regex::new(r"java\.lang\.RuntimeException").unwrap());
        java_patterns.push(Regex::new(r"java\.lang\.ProcessBuilder").unwrap());
        java_patterns.push(Regex::new(r"java\.lang\.Process").unwrap());
        java_patterns.push(Regex::new(r"org\.apache\.commons\.exec").unwrap());
        
        // Python错误
        let mut python_patterns = Vec::new();
        python_patterns.push(Regex::new(r"Traceback \(most recent call last\)").unwrap());
        python_patterns.push(Regex::new(r"File .+, line \d+").unwrap());
        python_patterns.push(Regex::new(r"subprocess\.").unwrap());
        python_patterns.push(Regex::new(r"os\.system").unwrap());
        python_patterns.push(Regex::new(r"os\.popen").unwrap());
        
        // Node.js错误
        let mut nodejs_patterns = Vec::new();
        nodejs_patterns.push(Regex::new(r"Error: Command failed:").unwrap());
        nodejs_patterns.push(Regex::new(r"child_process").unwrap());
        nodejs_patterns.push(Regex::new(r"ReferenceError:").unwrap());
        nodejs_patterns.push(Regex::new(r"SyntaxError:").unwrap());
        
        // 通用错误
        let mut common_patterns = Vec::new();
        common_patterns.push(Regex::new(r"sh: \d+:").unwrap());
        common_patterns.push(Regex::new(r"/bin/sh").unwrap());
        common_patterns.push(Regex::new(r"/bin/bash").unwrap());
        common_patterns.push(Regex::new(r"command not found").unwrap());
        common_patterns.push(Regex::new(r"Permission denied").unwrap());
        common_patterns.push(Regex::new(r"No such file or directory").unwrap());
        
        error_patterns.insert("php".to_string(), php_patterns);
        error_patterns.insert("java".to_string(), java_patterns);
        error_patterns.insert("python".to_string(), python_patterns);
        error_patterns.insert("nodejs".to_string(), nodejs_patterns);
        error_patterns.insert("common".to_string(), common_patterns);
        
        // 初始化RCE测试载荷
        let mut payloads = HashMap::new();
        
        // 命令注入载荷
        let mut cmd_payloads = Vec::new();
        cmd_payloads.push(";id".to_string());
        cmd_payloads.push("& id".to_string());
        cmd_payloads.push("| id".to_string());
        cmd_payloads.push("|| id".to_string());
        cmd_payloads.push("&& id".to_string());
        cmd_payloads.push("`id`".to_string());
        cmd_payloads.push("$(id)".to_string());
        cmd_payloads.push(";ls -la".to_string());
        cmd_payloads.push("& ls -la".to_string());
        cmd_payloads.push("| ls -la".to_string());
        cmd_payloads.push("|| ls -la".to_string());
        cmd_payloads.push("&& ls -la".to_string());
        cmd_payloads.push("`ls -la`".to_string());
        cmd_payloads.push("$(ls -la)".to_string());
        
        // 高级命令注入载荷
        let mut advanced_cmd_payloads = Vec::new();
        advanced_cmd_payloads.push(";cat /etc/passwd".to_string());
        advanced_cmd_payloads.push("& cat /etc/passwd".to_string());
        advanced_cmd_payloads.push("| cat /etc/passwd".to_string());
        advanced_cmd_payloads.push("|| cat /etc/passwd".to_string());
        advanced_cmd_payloads.push("&& cat /etc/passwd".to_string());
        advanced_cmd_payloads.push("`cat /etc/passwd`".to_string());
        advanced_cmd_payloads.push("$(cat /etc/passwd)".to_string());
        advanced_cmd_payloads.push(";type C:\\Windows\\win.ini".to_string());
        advanced_cmd_payloads.push("& type C:\\Windows\\win.ini".to_string());
        advanced_cmd_payloads.push("| type C:\\Windows\\win.ini".to_string());
        advanced_cmd_payloads.push("|| type C:\\Windows\\win.ini".to_string());
        advanced_cmd_payloads.push("&& type C:\\Windows\\win.ini".to_string());
        advanced_cmd_payloads.push("`type C:\\Windows\\win.ini`".to_string());
        advanced_cmd_payloads.push("$(type C:\\Windows\\win.ini)".to_string());
        
        // 代码注入载荷
        let mut code_payloads = Vec::new();
        code_payloads.push("';phpinfo();//".to_string());
        code_payloads.push("\";phpinfo();//".to_string());
        code_payloads.push("'.system('id').'".to_string());
        code_payloads.push("\".system('id').\"".to_string());
        code_payloads.push("'.exec('id').'".to_string());
        code_payloads.push("\".exec('id').\"".to_string());
        code_payloads.push("'.shell_exec('id').'".to_string());
        code_payloads.push("\".shell_exec('id').\"".to_string());
        
        payloads.insert("cmd".to_string(), cmd_payloads);
        payloads.insert("advanced_cmd".to_string(), advanced_cmd_payloads);
        payloads.insert("code".to_string(), code_payloads);
        
        Self {
            config,
            error_patterns,
            payloads,
        }
    }
    
    /// 检查响应中是否包含RCE错误
    fn check_rce_error(&self, body: &[u8]) -> Option<String> {
        // 使用from_utf8_lossy避免UTF-8编码问题
        let body_str = String::from_utf8_lossy(body);
        
        for (lang, patterns) in &self.error_patterns {
            for pattern in patterns {
                if let Some(captures) = pattern.captures(&body_str) {
                    if let Some(m) = captures.get(0) {
                        return Some(format!("{}: {}", lang, m.as_str()));
                    }
                }
            }
        }
        None
    }
    
    /// 获取当前检测级别的载荷
    fn get_payloads_for_level(&self, level: &str) -> Vec<String> {
        let mut result = Vec::new();
        
        // 命令注入载荷对所有级别都有效
        if let Some(payloads) = self.payloads.get("cmd") {
            result.extend(payloads.clone());
        }
        
        // 中高级别添加代码注入载荷
        if level == "medium" || level == "high" {
            if let Some(payloads) = self.payloads.get("code") {
                result.extend(payloads.clone());
            }
        }
        
        // 高级别添加高级命令注入载荷
        if level == "high" {
            if let Some(payloads) = self.payloads.get("advanced_cmd") {
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
}

#[async_trait]
impl Scanner for RceScanner {
    async fn name(&self) -> String {
        "RCE Scanner".to_string()
    }
    
    async fn scan(&self, request: &HttpRequest, response: &HttpResponse) -> Vec<ScanResult> {
        let mut results = Vec::new();
        
        // 获取检测级别
        let level = &self.config.rules.vulnerabilities.rce.level;
        
        // 获取响应体
        let body = response.body.as_ref();
        
        // 检查响应中是否包含RCE错误
        if let Some(error) = self.check_rce_error(body) {
            // 检查所有请求参数（URL参数和POST参数）
            for (param_name, param_value) in &request.params {
                // 检查参数值是否可能导致RCE
                let payloads = self.get_payloads_for_level(level);
                
                for payload in payloads {
                    if param_value.contains(&payload) {
                        // 创建扫描结果
                        let result = ScanResult {
                            vulnerability_type: "RCE".to_string(),
                            name: "远程命令执行漏洞".to_string(),
                            description: "检测到远程命令执行(RCE)漏洞，攻击者可以通过构造恶意输入执行任意系统命令".to_string(),
                            risk_level: "Critical".to_string(),
                            url: request.url.to_string(),
                            method: request.method.to_string(),
                            parameter: Some(param_name.clone()),
                            value: Some(param_value.clone()),
                            evidence: Some(error.clone()),
                            remediation: Some("避免使用危险函数执行系统命令，对用户输入进行严格过滤，实施输入验证和白名单机制".to_string()),
                            details: Some(format!("参数 {} 的值 {} 可能导致远程命令执行，响应中包含RCE错误: {}", param_name, param_value, error)),
                            timestamp: chrono::Utc::now(),
                            request_details: Some(format!("{} {}\nHost: {}", request.method, request.url, request.url.split('/').nth(2).unwrap_or("unknown"))),
                            response_details: Some(format!("响应包含RCE错误特征: ")),
                        };
                        
                        results.push(result);
                        break;
                    }
                }
            }
        }
        
        results
    }
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::*;
    use crate::core::config::AppConfig;

    // Helper function to create a test HTTP request
    fn create_test_request(url: &str, method: &str, headers: HashMap<String, String>, body: Vec<u8>) -> HttpRequest {
        // 解析URL并提取参数
        let parsed_url = Url::parse(url).unwrap();
        let params: Vec<(String, String)> = parsed_url.query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
            
        HttpRequest {
            method: method.to_string(),
            url: url.to_string(),
            headers,
            body,
            params,
        }
    }

    // Helper function to create a test HTTP response
    fn create_test_response(status: u16, headers: HashMap<String, String>, body: Vec<u8>) -> HttpResponse {
        HttpResponse {
            status,
            headers,
            body,
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
    async fn test_rce_scanner_name() {
        let scanner = RceScanner::new(create_test_config());
        assert_eq!(scanner.name().await, "RCE Scanner");
    }

} 