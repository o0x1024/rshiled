use std::time::Duration;
use log::*;
use reqwest::{self, Client};
use regex::Regex;
use serde_json::Value;

const API_URL: &str = "https://search.censys.io/api/v2/certificates/search";
const DELAY_SECONDS: u64 = 3;

pub async fn get_censys_subdomains(domain: String) -> Vec<String> {
    let mut subdomains = Vec::new();
    
    // 从环境变量获取 API 凭证
    let (api_id, api_secret) = match (std::env::var("CENSYS_API_ID"), std::env::var("CENSYS_API_SECRET")) {
        (Ok(id), Ok(secret)) => (id, secret),
        _ => {
            error!("Missing CENSYS_API_ID or CENSYS_API_SECRET in environment");
            return subdomains;
        }
    };

    // 构建正则表达式
    let re = match build_regex(&domain) {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to build regex: {}", e);
            return subdomains;
        }
    };

    // 创建 HTTP 客户端
    let client = match Client::builder()
        .timeout(Duration::from_secs(5))
        .build() 
    {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create HTTP client: {}", e);
            return subdomains;
        }
    };

    let mut cursor: Option<String> = None;
    
    loop {
        // 发送 API 请求
        let response = match send_api_request(
            &client,
            &domain,
            &api_id,
            &api_secret,
            cursor.as_deref()
        ).await {
            Ok(r) => r,
            Err(e) => {
                error!("API request failed: {}", e);
                break;
            }
        };

        // 处理 API 响应状态
        if response["status"] != "OK" {
            error!("API returned error status: {}", response["status"]);
            break;
        }

        // 提取子域名
        if let Some(text) = response.to_string().as_str() {
            let mut matches = re.find_iter(text)
                .filter_map(|m| m.as_str().parse().ok())
                .collect();
            subdomains.append(&mut matches);
        }

        // 检查分页
        cursor = response["result"]["links"]["next"]
            .as_str()
            .map(|s| s.to_string());
        
        if cursor.is_none() {
            break;
        }

        // 遵守速率限制
        tokio::time::sleep(Duration::from_secs(DELAY_SECONDS)).await;
    }

    // 去重处理
    subdomains.sort();
    subdomains.dedup();
    subdomains
}

async fn send_api_request(
    client: &Client,
    domain: &str,
    api_id: &str,
    api_secret: &str,
    cursor: Option<&str>,
) -> Result<Value, reqwest::Error> {
    let mut params = vec![
        ("q", format!("names: {}", domain)),
        ("per_page", "100".to_string()),
    ];

    if let Some(cursor) = cursor {
        params.push(("cursor", cursor.to_string()));
    }

    client.get(API_URL)
        .basic_auth(api_id, Some(api_secret))
        .query(&params)
        .send()
        .await?
        .json::<Value>()
        .await
}

fn build_regex(domain: &str) -> Result<Regex, regex::Error> {
    let escaped = regex::escape(domain);
    Regex::new(&format!(r#""names":\["([a-zA-Z0-9.-]+\.{})"\]"#, escaped))
}