use log::debug;

use crate::scan::proxy::{HttpRequest, HttpResponse};

/// Simple utility functions for rule processing
pub struct RuleUtils;

impl RuleUtils {
    /// Check if a response contains a specific string
    pub fn response_contains(response: &HttpResponse, pattern: &str) -> bool {
        String::from_utf8_lossy(&response.body).contains(pattern)
    }
    
    /// Check if a request parameter contains a specific string
    pub fn request_param_contains(request: &HttpRequest, param: &str, pattern: &str) -> bool {
        // Simple implementation - just check the URL for now
        request.url.contains(&format!("{}={}", param, pattern))
    }
    
    /// Check if a response is a redirect
    pub fn is_redirect(response: &HttpResponse) -> bool {
        response.status >= 300 && response.status < 400
    }
    
    /// Get redirect location from response
    pub fn get_redirect_location(response: &HttpResponse) -> Option<String> {
        response.headers.iter()
            .find(|(name, _)| name.to_lowercase() == "location")
            .map(|(_, value)| value.clone())
    }
    
    /// Check if a string is a URL
    pub fn is_url(s: &str) -> bool {
        s.starts_with("http://") || s.starts_with("https://")
    }
    
    /// Log debug information
    pub fn log_debug(message: &str) {
        debug!("{}", message);
    }
}
