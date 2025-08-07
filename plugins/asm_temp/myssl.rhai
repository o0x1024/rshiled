/**
 * MySSL Domain Discovery Plugin
 * 
 * This plugin uses the MySSL certificate search service to discover subdomains of a given domain.
 * It searches through certificate databases to find certificates issued for the domain and its subdomains.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "MySslSearch",
        description: "Searches for subdomains using MySSL certificate database",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://myssl.com/",
        params: [
            #{name: "domain",
                key: "domain",
                description: "Target domain to search for subdomains",
                type: "string",
                required: true,
                default_value: ()
            },
            #{name: "timeout",
                key: "timeout",
                description: "Request timeout in seconds",
                type: "integer",
                default_value: 30,
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
        "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
        "Connection": "keep-alive",
        "Referer": "https://myssl.com/"
    };
    
    // Add any additional headers
    if params.headers != () {
        for key in params.headers.keys() {
            headers[key] = params.headers[key];
        }
    }
    
    let method = "GET";
    if params.method != () {
        method = params.method;
    }
    let timeout = 30;
    if params.timeout != () {
        timeout = params.timeout;
    }
    // Prepare request parameters
    let request_params = #{
        url: params.url,
        method: method,
        headers: headers,
        timeout: timeout
    };
    
    // Add query parameters if provided
    if params.params != () {
        request_params.params = params.params;
    }
    
    // Add proxy if provided
    if params.proxy_url != () {
        request_params.proxy = params.proxy_url;
    }
    
    // Send the request
    let response = http_request(request_params);
    
    // Check for errors
    if response.error != () {
        print(`Error in HTTP request: ${response.error}`);
        return #{
            error: response.error,
            status_code: 0,
            body: ""
        };
    }
    
    // Return response with simplified structure
    return #{
        status_code: response.status_code,
        body: response.body,
        headers: response.headers
    };
}

/**
 * Extracts subdomains from search results
 */
fn extract_subdomains(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();
    
    if response_body.trim()== "" {
        return subdomains;
    }
    
    // Try to parse the JSON response
    let json_data = json_parse(response_body);
    if json_data != () && json_data.data != () && json_data.data.domains != () {
        for item in json_data.data.domains {
            if item.domain != () {
                let subdomain = item.domain.to_lower();
                if subdomain.ends_with(root_domain) && subdomain != root_domain {
                    if !subdomains.contains(subdomain) {
                        subdomains.push(subdomain);
                    }
                }
            }
        }
    } else {
        // Fallback to regex extraction if JSON parsing doesn't yield results
        let pattern = `[a-zA-Z0-9][-a-zA-Z0-9.]*\\.${root_domain.replace(".", "\\.")}`;
        let matches = regex_matches(response_body, pattern);
        
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
 * Main function to search MySSL for subdomains
 */
fn search_myssl(domain, timeout, proxy_url) {
    // Use the MySSL API endpoint for domain search
    let api_url = "https://myssl.com/api/v1/discover_sub_domain";
    
    // Prepare query parameters
    let params = #{
        "domain": domain
    };
    
    // Prepare request parameters
    let request_params = #{
        url: api_url,
        method: "GET",
        params: params,
        timeout: timeout
    };
    
    // Add proxy if provided
    if proxy_url != () {
        request_params.proxy_url = proxy_url;
    }
    
    // Send the request
    let response = send_http_request(request_params);
    
    // Check for errors
    if response.error != () {
        return #{
            error: response.error,
            subdomains: []
        };
    }
    
    // Check status code
    if response.status_code != 200 {
        // Try alternative approach
        return search_myssl_alternative(domain, timeout, proxy_url);
    }
    
    // Extract subdomains
    let subdomains = extract_subdomains(response.body, domain);
    
    // If no results, try alternative approach
    if subdomains.len() == 0 {
        let alt_result = search_myssl_alternative(domain, timeout, proxy_url);
        if alt_result.error == () {
            return alt_result;
        }
    }
    
    return #{
        error: (),
        subdomains: subdomains
    };
}

/**
 * Alternative approach to search MySSL for subdomains
 */
fn search_myssl_alternative(domain, timeout, proxy_url) {
    // Use the MySSL web interface for domain search
    let web_url = "https://myssl.com/discover_sub_domain.html";
    
    // Prepare query parameters
    let params = #{
        "domain": domain
    };
    
    // Prepare request parameters
    let request_params = #{
        url: web_url,
        method: "GET",
        params: params,
        timeout: timeout
    };
    
    // Add proxy if provided
    if proxy_url != () {
        request_params.proxy_url = proxy_url;
    }
    
    // Send the request
    let response = send_http_request(request_params);
    
    // Check for errors
    if response.error != () {
        return #{
            error: response.error,
            subdomains: []
        };
    }
    
    // Check status code
    if response.status_code != 200 {
        return #{
            error: `Unexpected status code: ${response.status_code}`,
            subdomains: []
        };
    }
    
    // Extract subdomains
    let subdomains = extract_subdomains(response.body, domain);
    
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
    let timeout = 30;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = parameters.proxy_url;
    
    // Search for subdomains
    let search_result = search_myssl(domain, timeout, proxy_url);
    
    // Check for errors
    if search_result.error != () {
        return #{
            error: search_result.error,
            result: []
        };
    }
    
    // Return success response
    return #{
        error: (),
        result: search_result.subdomains
    };
} 