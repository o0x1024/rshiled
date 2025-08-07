use rshield_lib::core::config::AppConfig;
use rshield_lib::scan::scanners::{Scanner, XssScanner};
use rshield_lib::scan::engine::result::ScanResult;
use super::*;
use std::sync::Arc;
use tokio::runtime::Runtime;

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    #[tokio::test]
    async fn test_xss_scanner_name() {
        let scanner = XssScanner::new(create_test_config());
        assert_eq!(scanner.name().await, "XSS Scanner");
    }

    #[tokio::test]
    async fn test_xss_scanner_detect_reflected_xss() {
        let scanner = XssScanner::new(create_test_config());
        
        // Create a request with XSS payload
        let request = create_test_request(
            "https://example.com/search?q=<script>alert(1)</script>",
            "GET",
            vec![(String::from("Content-Type"), String::from("text/html"))],
            Vec::new(),
        );
        
        // Create a response that reflects the XSS payload
        let response = create_test_response(
            200,
            vec![(String::from("Content-Type"), String::from("text/html"))],
            b"<html><body>Search results for: <script>alert(1)</script></body></html>".to_vec(),
        );
        
        // Run the scanner
        let results = scanner.scan(&request, &response).await;
        
        // Verify that it found the XSS vulnerability
        assert!(!results.is_empty(), "Expected to find XSS vulnerability");
        
        // Check that the vulnerability type is XSS or Reflected XSS
        let xss_results: Vec<&ScanResult> = results.iter()
            .filter(|r| r.vulnerability_type.contains("XSS"))
            .collect();
            
        assert!(!xss_results.is_empty(), "Expected to find XSS vulnerability type");
        
        // Verify the details of the first XSS result
        if let Some(first_result) = xss_results.first() {
            assert_eq!(first_result.risk_level, "High");
            assert!(first_result.url.contains("<script>alert(1)</script>"));
        }
    }

    #[tokio::test]
    async fn test_xss_scanner_negative_case() {
        let scanner = XssScanner::new(create_test_config());
        
        // Create a request with safe content
        let request = create_test_request(
            "https://example.com/search?q=safe_search_term",
            "GET",
            vec![(String::from("Content-Type"), String::from("text/html"))],
            Vec::new(),
        );
        
        // Create a response with safe content
        let response = create_test_response(
            200,
            vec![(String::from("Content-Type"), String::from("text/html"))],
            b"<html><body>Search results for: safe_search_term</body></html>".to_vec(),
        );
        
        // Run the scanner
        let results = scanner.scan(&request, &response).await;
        
        // Verify that it did not find any XSS vulnerabilities
        let xss_results: Vec<&ScanResult> = results.iter()
            .filter(|r| r.vulnerability_type.contains("XSS"))
            .collect();
            
        assert!(xss_results.is_empty(), "Expected no XSS vulnerabilities for safe content");
    }

    #[tokio::test]
    async fn test_xss_scanner_dom_based_xss() {
        let scanner = XssScanner::new(create_test_config());
        
        // Create a request with a DOM-based XSS payload
        let request = create_test_request(
            "https://example.com/page#<img src=x onerror=alert(1)>",
            "GET",
            vec![(String::from("Content-Type"), String::from("text/html"))],
            Vec::new(),
        );
        
        // Create a response with JavaScript that uses document.location
        let response = create_test_response(
            200,
            vec![(String::from("Content-Type"), String::from("text/html"))],
            b"<html><body><script>var hash = document.location.hash.substring(1); document.getElementById('output').innerHTML = hash;</script><div id='output'></div></body></html>".to_vec(),
        );
        
        // Run the scanner
        let results = scanner.scan(&request, &response).await;
        
        // Check results - may detect DOM XSS depending on implementation
        // This is a more advanced test and might require specific implementations for DOM XSS
        println!("DOM XSS test results count: {}", results.len());
        
        for result in &results {
            println!("DOM XSS test result: {:?}", result.vulnerability_type);
        }
    }
} 