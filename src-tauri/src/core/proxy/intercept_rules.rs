use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use regex::Regex;
use std::path::Path;
use std::fs;
use anyhow::{Result, anyhow};

// 定义规则数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptionRule {
    pub id: String,
    pub enabled: bool,
    pub operator: String, // "and" 或 "or"
    pub match_type: String, // "domain", "ip", "protocol", "method", "extension", "path", "header", "statusCode"
    pub match_relationship: String, // "matches" 或 "not_matches"
    pub match_condition: String,
}

// 定义规则集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptionRules {
    pub request_rules: Vec<InterceptionRule>,
    pub response_rules: Vec<InterceptionRule>,
}

impl Default for InterceptionRules {
    fn default() -> Self {
        Self {
            request_rules: Vec::new(),
            response_rules: Vec::new(),
        }
    }
}

// 规则管理器
pub struct RuleManager {
    rules: Arc<RwLock<InterceptionRules>>,
    rules_file: String,
}

impl RuleManager {
    // 创建新的规则管理器
    pub fn new(rules_file: &str) -> Result<Self> {
        let rules = if Path::new(rules_file).exists() {
            let content = fs::read_to_string(rules_file)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            InterceptionRules::default()
        };

        Ok(Self {
            rules: Arc::new(RwLock::new(rules)),
            rules_file: rules_file.to_string(),
        })
    }

    // 保存规则到文件
    async fn save_rules(&self) -> Result<()> {
        let rules = self.rules.read().await;
        let json = serde_json::to_string_pretty(&*rules)?;
        
        let dir = Path::new(&self.rules_file).parent().unwrap();
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
        
        fs::write(&self.rules_file, json)?;
        Ok(())
    }

    // 获取请求拦截规则
    pub async fn get_request_rules(&self) -> Vec<InterceptionRule> {
        self.rules.read().await.request_rules.clone()
    }

    // 获取响应拦截规则
    pub async fn get_response_rules(&self) -> Vec<InterceptionRule> {
        self.rules.read().await.response_rules.clone()
    }

    // 设置请求拦截规则
    pub async fn set_request_rules(&self, rules: Vec<InterceptionRule>) -> Result<()> {
        {
            let mut rules_lock = self.rules.write().await;
            rules_lock.request_rules = rules;
        }
        self.save_rules().await
    }

    // 设置响应拦截规则
    pub async fn set_response_rules(&self, rules: Vec<InterceptionRule>) -> Result<()> {
        {
            let mut rules_lock = self.rules.write().await;
            rules_lock.response_rules = rules;
        }
        self.save_rules().await
    }

    // 检查请求是否应该被拦截
    pub async fn should_intercept_request(
        &self, 
        method: &str, 
        url: &str, 
        headers: &HashMap<String, String>
    ) -> bool {
        let rules = self.rules.read().await.request_rules.clone();
        
        // 如果没有规则，默认拦截所有请求
        if rules.is_empty() {
            return true;
        }
        
        let mut should_intercept = false;
        
        for (i, rule) in rules.iter().enumerate() {
            if !rule.enabled {
                continue;
            }
            
            let matches = self.rule_matches_request(rule, method, url, headers);
            
            if i == 0 {
                // 第一条规则直接设置初始值
                should_intercept = matches;
            } else {
                // 后续规则根据操作符连接
                match rule.operator.as_str() {
                    "and" => should_intercept = should_intercept && matches,
                    "or" => should_intercept = should_intercept || matches,
                    _ => {}
                }
            }
        }
        
        should_intercept
    }
    
    // 检查响应是否应该被拦截
    pub async fn should_intercept_response(
        &self, 
        status: u16, 
        url: &str, 
        headers: &HashMap<String, String>
    ) -> bool {
        let rules = self.rules.read().await.response_rules.clone();
        
        // 如果没有规则，默认拦截所有响应
        if rules.is_empty() {
            return true;
        }
        
        let mut should_intercept = false;
        
        for (i, rule) in rules.iter().enumerate() {
            if !rule.enabled {
                continue;
            }
            
            let matches = self.rule_matches_response(rule, status, url, headers);
            
            if i == 0 {
                // 第一条规则直接设置初始值
                should_intercept = matches;
            } else {
                // 后续规则根据操作符连接
                match rule.operator.as_str() {
                    "and" => should_intercept = should_intercept && matches,
                    "or" => should_intercept = should_intercept || matches,
                    _ => {}
                }
            }
        }
        
        should_intercept
    }
    
    // 判断请求是否匹配规则
    fn rule_matches_request(
        &self, 
        rule: &InterceptionRule, 
        method: &str, 
        url: &str, 
        headers: &HashMap<String, String>
    ) -> bool {
        // 解析URL
        let url_parsed = url::Url::parse(url).unwrap_or_else(|_| {
            url::Url::parse("http://unknown").unwrap()
        });
        
        let domain = url_parsed.host_str().unwrap_or("unknown");
        let path = url_parsed.path();
        let protocol = url_parsed.scheme();
        
        // 根据匹配类型判断
        let matches = match rule.match_type.as_str() {
            "domain" => {
                self.pattern_matches(&rule.match_condition, domain)
            },
            "ip" => {
                // 这里简化处理，实际应用可能需要更复杂的IP匹配逻辑
                domain == rule.match_condition
            },
            "protocol" => {
                protocol == rule.match_condition
            },
            "method" => {
                // 不区分大小写
                method.to_lowercase() == rule.match_condition.to_lowercase()
            },
            "extension" => {
                // 检查文件扩展名
                path.ends_with(&rule.match_condition)
            },
            "path" => {
                self.pattern_matches(&rule.match_condition, path)
            },
            "header" => {
                // 检查请求头，格式为 "Header-Name: Header-Value"
                if let Some(idx) = rule.match_condition.find(':') {
                    let (header_name, header_value) = rule.match_condition.split_at(idx);
                    let header_name = header_name.trim();
                    // 跳过冒号和空格
                    let header_value = header_value[1..].trim();
                    
                    if let Some(actual_value) = headers.get(header_name) {
                        actual_value == header_value
                    } else {
                        false
                    }
                } else {
                    // 只检查头部名称存在
                    headers.contains_key(&rule.match_condition)
                }
            },
            _ => false,
        };
        
        // 应用匹配关系
        match rule.match_relationship.as_str() {
            "matches" => matches,
            "not_matches" => !matches,
            _ => false,
        }
    }
    
    // 判断响应是否匹配规则
    fn rule_matches_response(
        &self, 
        rule: &InterceptionRule, 
        status: u16, 
        url: &str, 
        headers: &HashMap<String, String>
    ) -> bool {
        // 解析URL
        let url_parsed = url::Url::parse(url).unwrap_or_else(|_| {
            url::Url::parse("http://unknown").unwrap()
        });
        
        let domain = url_parsed.host_str().unwrap_or("unknown");
        let path = url_parsed.path();
        
        // 根据匹配类型判断
        let matches = match rule.match_type.as_str() {
            "domain" => {
                self.pattern_matches(&rule.match_condition, domain)
            },
            "path" => {
                self.pattern_matches(&rule.match_condition, path)
            },
            "statusCode" => {
                if let Ok(status_code) = rule.match_condition.parse::<u16>() {
                    status == status_code
                } else {
                    false
                }
            },
            "header" => {
                // 检查响应头
                if let Some(idx) = rule.match_condition.find(':') {
                    let (header_name, header_value) = rule.match_condition.split_at(idx);
                    let header_name = header_name.trim();
                    // 跳过冒号和空格
                    let header_value = header_value[1..].trim();
                    
                    if let Some(actual_value) = headers.get(header_name) {
                        actual_value == header_value
                    } else {
                        false
                    }
                } else {
                    // 只检查头部名称存在
                    headers.contains_key(&rule.match_condition)
                }
            },
            _ => false,
        };
        
        // 应用匹配关系
        match rule.match_relationship.as_str() {
            "matches" => matches,
            "not_matches" => !matches,
            _ => false,
        }
    }
    
    // 模式匹配，支持通配符和正则表达式
    fn pattern_matches(&self, pattern: &str, value: &str) -> bool {
        // 尝试作为正则表达式处理
        if let Ok(re) = Regex::new(pattern) {
            return re.is_match(value);
        }
        
        // 处理简单的通配符 (*.example.com)
        if pattern.starts_with("*.") {
            let suffix = &pattern[1..]; // 移除 *
            return value.ends_with(suffix);
        }
        
        // 直接相等比较
        pattern == value
    }
}
