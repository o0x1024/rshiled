use std::collections::HashMap;

#[derive(Debug, Clone,Default)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub params: Vec<(String, String)>,
}

impl HttpRequest {
    pub fn new(url: &str, method: &str, headers: HashMap<String, String>,body: Vec<u8>,params: Vec<(String, String)>) -> Self {
        let headers_vec = headers.into_iter()
            .map(|(k, v)| (k, v))
            .collect();
            
        Self {
            method: method.to_string(),
            url: url.to_string(),
            headers: headers_vec,
            body: body,
            params: params,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new(status: u16, headers: HashMap<String, String>,body: Vec<u8> ) -> Self {
            
        Self {
            status: status,
            headers: headers,
            body,
        }
    }
}

unsafe impl Send for HttpRequest {}
unsafe impl Sync for HttpRequest {}
unsafe impl Send for HttpResponse {}
unsafe impl Sync for HttpResponse {} 