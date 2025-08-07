/**
 * DNSDumpster Domain Search Plugin
 * 
 * This plugin queries the DNSDumpster service for a given domain and extracts subdomains.
 * DNSDumpster is a free domain research tool that can discover DNS records for a domain.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "DNSDumpsterDomainSearch",
        description: "Searches for subdomains using DNSDumpster",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://dnsdumpster.com/",
        params: [
            #{name: "domain",
                key: "domain",
                type: "string",
                description: "Target domain to scan",
                required: true,
                default_value: ()
            },
            #{name: "timeout",
                key: "timeout",
                description: "Request timeout in seconds",
                type: "integer",
                default_value: 10,
                required: false
            },
            #{name: "proxy_url",
                key: "proxy_url",
                description: "Proxy URL to use for requests",
                type: "string",
                default_value: (),
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

/**
 * Sends HTTP request and handles response
 */
fn send_http_request(params) {
    // Default headers for the request
    let headers = #{
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.102 Safari/537.36",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
        "Accept-Language": "en-US,en;q=0.9",
        "Connection": "keep-alive"
    };
    
    // Add any additional headers
    if params.headers != () {
        for key in params.headers.keys() {
            headers[key] = params.headers[key];
        }
    }
    
    let timeout = 10;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let method = "GET";
    if params.method != () {
        method = params.method;
    }
    
    // Prepare request parameters
    let request_params = #{
        url: params.url,
        method: method,
        headers: headers,
        timeout: timeout
    };
    
    // Add body data if provided (for POST requests)
    if params.data != () {
        request_params.body = params.data;
    }
    
    // Add proxy if provided
    if params.proxy_url != () {
        request_params.proxy = params.proxy_url;
    }
    
    // Add cookies if provided
    if params.cookies != () {
        request_params.cookies = params.cookies;
    }
    
    // Send the request
    let response = http_request(request_params);
    
    // Check for errors
    if response.error != () {
        print(`Error in HTTP request: ${response.error}`);
        return #{
            error: response.error,
            status_code: 0,
            body: "",
            cookies: []
        };
    }
    
    // Return response with simplified structure
    return #{
        status_code: response.status_code,
        body: response.body,
        headers: response.headers,
        cookies: response.cookies
    };
}

/**
 * Extracts CSRF token from cookies
 */
fn extract_csrf_token(cookies) {
    if cookies.csrftoken != () {
        return cookies.csrftoken;
    }
    return "";
}

/**
 * Extracts subdomains from HTML response
 */
fn extract_subdomains(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();
    
    // Convert HTML to lowercase for easier matching
    let html_lower = response_body.to_lower();
    
    // Simple pattern matching for subdomains in the HTML
    // Look for domain patterns in the HTML
    let pattern = `[a-zA-Z0-9-]+\\.${root_domain.replace(".", "\\.")}`;
    
    // Use regex to find all matches
    let matches = regex_matches(html_lower, pattern);
    
    // Process matches and filter duplicates
    if matches.len() > 0 {
        for matched in matches {
            let subdomain = matched.to_lower();
            if subdomain.ends_with(root_domain) && subdomain != root_domain {
                if !subdomains.contains(subdomain) {
                    subdomains.push(subdomain);
                }
            }
        }
    }
    
    return subdomains;
}

/**
 * Main function to query DNSDumpster for subdomains
 */
fn query_dnsdumpster(domain, timeout, proxy_url) {
    let url = "https://dnsdumpster.com/";
    
    // First request to get CSRF token
    let first_request_params = #{
        url: url,
        method: "GET",
        timeout: timeout
    };
    
    // Add proxy if provided
    if proxy_url != () {
        first_request_params.proxy_url = proxy_url;
    }
    
    // Send initial request to get CSRF token
    let first_response = send_http_request(first_request_params);
    
    // Check for errors
    if first_response.error != () {
        return #{
            error: first_response.error,
            subdomains: []
        };
    }
    
    // Check status code
    if first_response.status_code != 200 {
        return #{
            error: `Unexpected status code in first request: ${first_response.status_code}`,
            subdomains: []
        };
    }
    
    // Extract CSRF token from cookies
    let csrf_token = extract_csrf_token(first_response.cookies);
    
    if csrf_token.trim()== "" {
        return #{
            error: "Failed to extract CSRF token",
            subdomains: []
        };
    }
    
    // Prepare POST data
    let post_data = #{
        "csrfmiddlewaretoken": csrf_token,
        "targetip": domain,
        "user": "free"
    };
    
    // Add Referer header
    let headers = #{
        "Referer": "https://dnsdumpster.com",
        "Content-Type": "application/x-www-form-urlencoded"
    };
    
    // Prepare second request parameters
    let second_request_params = #{
        url: url,
        method: "POST",
        headers: headers,
        data: post_data,
        cookies: first_response.cookies,
        timeout: timeout
    };
    
    // Add proxy if provided
    if proxy_url != () {
        second_request_params.proxy_url = proxy_url;
    }
    
    // Send second request with POST data
    let second_response = send_http_request(second_request_params);
    
    // Check for errors
    if second_response.error != () {
        return #{
            error: second_response.error,
            subdomains: []
        };
    }
    
    // Check status code
    if second_response.status_code != 200 {
        return #{
            error: `Unexpected status code in second request: ${second_response.status_code}`,
            subdomains: []
        };
    }
    
    // Extract subdomains from response
    let subdomains = extract_subdomains(second_response.body, domain);
    
    return #{
        error: (),
        subdomains: subdomains
    };
}

/**
 * Main function to process the request and return results
 */
fn analyze(request_json) {
    // Parse request parameters
    let parameters = request_json.parameters;
    
    // Validate required parameters
    if parameters.domain == () {
        return #{
            error: "Missing required parameter: domain",
            result: []
        };
    }
    
    // Extract parameters
    let domain = parameters.domain;
    let timeout = 10;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = parameters.proxy_url;
    
    // Query DNSDumpster for subdomains
    let result = query_dnsdumpster(domain, timeout, proxy_url);
    
    // Check for errors
    if result.error != () {
        return #{
            error: result.error,
            result: []
        };
    }
    
    // Return success response
    return #{
        error: (),
        result: result.subdomains
    };
} 