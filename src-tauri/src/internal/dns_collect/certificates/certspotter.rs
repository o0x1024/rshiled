use std::time::Duration;

use crate::internal::dns_handle::match_subdomains;
use log::*;
use reqwest::{self, Client};

pub async fn get_certspotter_subdomains(domain: String) -> Vec<String> {
    let mut subdomains = Vec::new();

    let client = Client::builder()
        .timeout(Duration::from_secs(5)) // 设置超时时间
        .build()
        .unwrap();
    // Construct the URL
    let url = format!("https://api.certspotter.com/v1/issuances?domain={}&include_subdomains=true&expand=dns_names", domain);

    // Make the HTTP request
    match client.get(&url).send().await {
        Ok(response) => match response.text().await {
            Ok(html) => {
                subdomains = match_subdomains(domain.as_str(), &html);
            }
            Err(_) => {
                // error!("Error reading response text: {}", e);
            }
        },
        Err(e) => {
            error!("Error making request: {:?}", e);
        }
    }

    subdomains
}
