// Main test module
pub mod scan;

// Re-export test utilities for use in integration tests
pub mod test_utils {
    pub use crate::scan::scanners::*;
}

// Mock implementations for testing
pub mod mocks {
    use rshield_lib::scan::engine::result::ScanResult;
    use rshield_lib::scan::proxy::{HttpRequest, HttpResponse};

    // Mock scanner implementation that doesn't implement Scanner trait
    // due to async_trait lifetime complexities
    pub struct MockScanner {
        pub name: String,
        pub results: Vec<ScanResult>,
    }

    impl MockScanner {
        pub fn new(name: &str, results: Vec<ScanResult>) -> Self {
            Self {
                name: name.to_string(),
                results,
            }
        }

        // Mock methods matching the Scanner trait but not implementing it
        pub async fn get_name(&self) -> String {
            self.name.clone()
        }

        pub async fn scan_request(
            &self,
            _request: &HttpRequest,
            _response: &HttpResponse,
        ) -> Vec<ScanResult> {
            self.results.clone()
        }
    }
}
