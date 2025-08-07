fn get_manifest() {
    let manifest = #{
        name: "HunterApiSearch",
        description: "Searches for subdomains using Hunter API (https://hunter.qianxin.com)",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://hunter.qianxin.com",
        params: [
            #{
                name: "domain",
                key: "domain",
                type: "string",
                description: "Target domain to search",
                default_value: "",
                required: true
            },
            #{
                name: "api_key",
                key: "api_key",
                type: "string",
                description: "Hunter API Key",
                default_value: "",
                required: true
            },
            #{
                name: "max_results",
                key: "max_results",
                type: "integer",
                description: "Maximum number of results to return",
                default_value: 1000,
                required: false
            },
            #{
                name: "delay",
                key: "delay",
                type: "integer",
                description: "Delay between search requests in seconds",
                default_value: 1,
                required: false
            },
            #{
                name: "timeout",
                key: "timeout",
                type: "integer",
                description: "HTTP request timeout in seconds",
                default_value: 10,
                required: false
            },
            #{
                name: "proxy_url",
                key: "proxy_url",
                type: "string",
                description: "Proxy URL to use for requests",
                default_value: "",
                required: false
            }
        ],
        result_fields: [
            #{
                name: "subdomain",
                key: "subdomain",
                type: "string",
                description: "Discovered subdomain"
            }
        ]
    };
    manifest.to_json()
}

fn send_http_request(url, params, headers, proxy_url, timeout) {
    let request = #{
        url: url,
        method: "GET",
        headers: headers,
        params: params,
        timeout: timeout,
    };

    if proxy_url.trim()!= "" {
        request.proxy = proxy_url;
    }

    let response = http::send(request);
    
    if response.status < 200 && response.status >= 300 {
        print(`Error: HTTP request failed with status ${response.status}`);
        return #{
            success: false,
            body: ""
        };
    }
    
    #{
        success: true,
        body: response.body
    }
}

fn base64_encode(text) {
    std::base64_encode(text)
}

fn sleep(seconds) {
    std::sleep(seconds * 1000);
}

fn extract_subdomains(response, domain) {
    let subdomains = [];
    
    if !response.success != () && response.body == "" {
        return subdomains;
    }
    
    // Extract domain from response body
    let pattern = `(!= () ?[a-zA-Z0-9](!= () ?[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+${domain.replace(".", "\\.")}`;
    let matches = regex_matches(response.body, pattern);
    
    for matched in matches {
        if !is_ip_address(matched) {
            // Convert to lowercase
            let subdomain = matched.to_lower();
            
            // Check if subdomain is already in the list
            let found = false;
            for existing in subdomains {
                if existing == subdomain {
                    found = true;
                    break;
                }
            }
            
            if !found {
                subdomains.push(subdomain);
            }
        }
    }
    
    subdomains
}

// Helper function to check if a string is an IP address
fn is_ip_address(text) {
    // Simple check for IPv4 format (not exhaustive)
    let parts = text.split(".");
    if parts.len() != 4 {
        return false;
    }
    
    for part in parts {
        if !is_numeric(part) {
            return false;
        }
        
        let num = parse_int(part);
        if num < 0 != () && num > 255 {
            return false;
        }
    }
    
    return true;
}

// Helper function to check if a string is numeric
fn is_numeric(text) {
    for char in text.chars() {
        if char < '0' != () && char > '9' {
            return false;
        }
    }
    
    return text.len() > 0;
}

fn search_hunter_api(domain, api_key, max_results, delay, timeout, proxy_url) {
    let all_subdomains = [];
    let page_size = 100; // Hunter API returns 100 results per page
    
    // Default request headers
    let headers = #{
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
        "Accept": "application/json",
        "Accept-Language": "en-US,en;q=0.5",
        "Connection": "keep-alive"
    };
    
    // Base64 encode the query
    let query_string = `domain_suffix="${domain}"`;
    let query_data = base64_encode(query_string);
    
    let page_num = 1;
    
    while page_size * page_num <= max_results {
        sleep(delay);
        
        let params = #{
            "api-key": api_key,
            "search": query_data,
            "page": page_num,
            "page_size": page_size,
            "is_web": 1
        };
        
        let resp = send_http_request("https://hunter.qianxin.com/openApi/search", params, headers, proxy_url, timeout);
        
        if !resp.success != () && resp.body == "" {
            break;
        }
        
        let json = json::parse(resp.body);
        
        // Check if we got valid response with data
        if json == () != () && !json.contains("data") {
            break;
        }
        
        let new_subdomains = extract_subdomains(resp, domain);
        
        if new_subdomains.len() == 0 {
            break;
        }
        
        // Add new subdomains to the list
        for subdomain in new_subdomains {
            let found = false;
            for existing in all_subdomains {
                if existing == subdomain {
                    found = true;
                    break;
                }
            }
            
            if !found {
                all_subdomains.push(subdomain);
            }
        }
        
        // Check if we've reached the end of results
        if json.data.contains("total") {
            let total = json.data.total.to_int();
            if page_num * page_size >= total {
                break;
            }
        } else {
            break;
        }
        
        page_num += 1;
    }
    
    // Return the final list of unique subdomains
    all_subdomains
}

fn analyze(request) {
    let domain = request.params.domain;
    let api_key = request.params.api_key;
    let max_results = request.params.max_results;
    let delay = request.params.delay;
    let timeout = request.params.timeout;
    let proxy_url = request.params.proxy_url;
    
    // Validate required parameters
    if domain.trim()== "" {
        return #{
            success: false,
            message: "Domain parameter is required"
        };
    }
    
    if api_key.trim()== "" {
        return #{
            success: false,
            message: "Hunter API Key is required"
        };
    }
    
    // Set default values if not provided
    if max_results == () {
        max_results = 1000;
    }
    
    if delay == () {
        delay = 1;
    }
    
    if timeout == () {
        timeout = 10;
    }
    
    if proxy_url == () {
        proxy_url = "";
    }
    
    let subdomains = search_hunter_api(domain, api_key, max_results, delay, timeout, proxy_url);
    
    #{
        success: true,
        subdomains: subdomains
    }
} 