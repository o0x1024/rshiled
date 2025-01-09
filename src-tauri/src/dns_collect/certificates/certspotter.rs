use reqwest;

use crate::utils::dns_handle::match_subdomains;

pub async fn get_certspotter_subdomains(domain: String) -> Vec<String> {
    let mut subdomains = Vec::new();

    // Construct the URL
    let url = format!("https://api.certspotter.com/v1/issuances?domain={}&include_subdomains=true&expand=dns_names", domain);

    // Make the HTTP request
    match reqwest::get(&url).await {
        Ok(response) => match response.text().await {
            Ok(html) => {
                subdomains = match_subdomains(domain.as_str(), &html);
            }
            Err(e) => {
                eprintln!("Error reading response text: {}", e);
            }
        },
        Err(e) => {
            eprintln!("Error making request: {:?}", e);
        }
    }

    subdomains
}
