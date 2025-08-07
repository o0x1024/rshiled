use std::time::Duration;

use crate::internal::dns_handle::match_subdomains;
use log::*;
use reqwest::{self, Client};

pub async fn get_ip138_subdomains(domain: String) -> Vec<String> {
    let mut subdomains = Vec::new();

    let client = Client::builder()
        .timeout(Duration::from_secs(5)) // 设置超时时间
        .build()
        .unwrap();

    // Construct the URL
    let url = format!("https://site.ip138.com/{}/domain.htm", domain);

    // Make the HTTP request
    match client.get(&url).send().await {
        Ok(response) => match response.text().await {
            Ok(html) => {
                subdomains = match_subdomains(domain.as_str(), &html);
            }
            Err(e) => {
                error!("Error reading response text: {}", e);
            }
        },
        Err(e) => {
            error!("Error making request: {:?}", e);
        }
    }

    subdomains
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fuzzy_matching() {
        println!("{:?}", get_ip138_subdomains("mgtv.com".to_string()).await);
    }
}

//新建一个异步测试用例
