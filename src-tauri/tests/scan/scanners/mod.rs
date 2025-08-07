// Scanner trait and implementation tests

use rshield_lib::core::config::AppConfig;
use rshield_lib::scan::proxy::{HttpRequest, HttpResponse};
use std::collections::HashMap;
use std::sync::Arc;

// Helper function to create a test HTTP request
pub fn create_test_request(
    url: &str,
    method: &str,
    headers: HashMap<String, String>,
    body: Vec<u8>,
) -> HttpRequest {
    let mut params = Vec::new();
    if let Ok(parsed_url) = url::Url::parse(url) {
        params = parsed_url
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
    }

    HttpRequest {
        method: method.to_string(),
        url: url.to_string(),
        headers,
        body,
        params,
    }
}

// Helper function to create a test HTTP response
pub fn create_test_response(
    status: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
) -> HttpResponse {
    HttpResponse {
        status,
        headers,
        body,
    }
}

// Create test app config
pub fn create_test_config() -> Arc<AppConfig> {
    Arc::new(AppConfig {
        proxy: rshield_lib::core::config::ProxyConfig {
            host: Some("127.0.0.1".to_string()),
            port: Some(8080),
            connect_timeout: 10,
            max_retries: 3,
            retry_delay: 1000,
            ca_cert_path: "".to_string(),
            ca_key_path: "".to_string(),
        },
        scanner: rshield_lib::core::config::ScannerConfig {
            mode: "active".to_string(),
            concurrency: 10,
            timeout_ms: 5000,
            save_results: false,
            results_path: "./results.json".to_string(),
        },
        rules: rshield_lib::core::config::RulesConfig {
            enable_builtin: true,
            enable_extensions: false,
            extensions_path: "rules/extensions".to_string(),
            vulnerabilities: rshield_lib::core::config::VulnerabilitiesConfig {
                xss: rshield_lib::core::config::XssConfig {
                    enabled: true,
                    use_ast: true,
                    max_params: 20,
                    max_depth: 3,
                },
                sql_injection: rshield_lib::core::config::SqlInjectionConfig {
                    enabled: true,
                    level: "high".to_string(),
                },
                rce: rshield_lib::core::config::RceConfig {
                    enabled: true,
                    level: "high".to_string(),
                },
                path_traversal: rshield_lib::core::config::PathTraversalConfig { enabled: true },
                open_redirect: rshield_lib::core::config::OpenRedirectConfig { enabled: true },
            },
        },
        logging: rshield_lib::core::config::LoggingConfig {
            level: "info".to_string(),
            file_output: false,
            file_path: "logs/scan.log".to_string(),
            colored_output: true,
        },
    })
}
