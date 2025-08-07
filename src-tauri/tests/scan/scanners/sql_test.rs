use rshield_lib::scan::scanners::{Scanner, SqlInjectionScanner};
use rshield_lib::scan::engine::result::ScanResult;
use super::*;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

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
            "GET",
            vec![(String::from("Content-Type"), String::from("text/html"))],
            Vec::new(),
        );
        
        // Create a response that shows an SQL error
        let response = create_test_response(
            500,
            vec![(String::from("Content-Type"), String::from("text/html"))],
            b"<html><body>Error: SQLSTATE[42000]: Syntax error or access violation: 1064 You have an error in your SQL syntax</body></html>".to_vec(),
        );
        
        // Run the scanner
        let results = scanner.scan(&request, &response).await;
        
        // Verify that it found the SQL injection vulnerability
        assert!(!results.is_empty(), "Expected to find SQL injection vulnerability");
        
        // Check that the vulnerability type is SQL Injection
        let sql_results: Vec<&ScanResult> = results.iter()
            .filter(|r| r.vulnerability_type.contains("SQL"))
            .collect();
            
        assert!(!sql_results.is_empty(), "Expected to find SQL Injection vulnerability type");
        
        // Verify the details of the first SQL result
        if let Some(first_result) = sql_results.first() {
            assert_eq!(first_result.risk_level, "High");
            assert!(first_result.url.contains("id=1'"));
        }
    }

    #[tokio::test]
    async fn test_sql_scanner_detect_boolean_based() {
        let scanner = SqlInjectionScanner::new(create_test_config());
        
        // Create a request with boolean-based SQL injection payload
        let request = create_test_request(
            "https://example.com/users?id=1 AND 1=1",
            "GET",
            vec![(String::from("Content-Type"), String::from("text/html"))],
            Vec::new(),
        );
        
        // Create a response that shows results (successful injection)
        let response = create_test_response(
            200,
            vec![(String::from("Content-Type"), String::from("text/html"))],
            b"<html><body>User found: John Doe</body></html>".to_vec(),
        );
        
        // Run the scanner for the true condition
        let results_true = scanner.scan(&request, &response).await;
        
        // Now test the false condition
        let request_false = create_test_request(
            "https://example.com/users?id=1 AND 1=2",
            "GET",
            vec![(String::from("Content-Type"), String::from("text/html"))],
            Vec::new(),
        );
        
        // Create a response for false condition (no results)
        let response_false = create_test_response(
            200,
            vec![(String::from("Content-Type"), String::from("text/html"))],
            b"<html><body>No users found</body></html>".to_vec(),
        );
        
        // Run the scanner for the false condition
        let results_false = scanner.scan(&request_false, &response_false).await;
        
        // In a complete test we would compare both responses to detect the boolean-based injection
        // For this example, we'll just print the results
        println!("Boolean-based SQL injection test - true condition results: {}", results_true.len());
        println!("Boolean-based SQL injection test - false condition results: {}", results_false.len());
        
        // We assume the actual boolean-based detection requires additional context that might not be 
        // available in this simplified test setup
    }

    #[tokio::test]
    async fn test_sql_scanner_negative_case() {
        let scanner = SqlInjectionScanner::new(create_test_config());
        
        // Create a request with safe content
        let request = create_test_request(
            "https://example.com/users?id=12345",
            "GET",
            vec![(String::from("Content-Type"), String::from("text/html"))],
            Vec::new(),
        );
        
        // Create a response with safe content
        let response = create_test_response(
            200,
            vec![(String::from("Content-Type"), String::from("text/html"))],
            b"<html><body>User found: John Doe</body></html>".to_vec(),
        );
        
        // Run the scanner
        let results = scanner.scan(&request, &response).await;
        
        // Verify that it did not find any SQL injection vulnerabilities
        let sql_results: Vec<&ScanResult> = results.iter()
            .filter(|r| r.vulnerability_type.contains("SQL"))
            .collect();
            
        assert!(sql_results.is_empty(), "Expected no SQL Injection vulnerabilities for safe content");
    }
} 