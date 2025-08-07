/**
 * BinaryEdge API Domain Discovery Plugin
 * 
 * This plugin uses the BinaryEdge API to discover subdomains of a given domain.
 * It requires an API key to access the service.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "BinaryEdgeApiSearch",
        description: "Searches for subdomains using BinaryEdge API",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://api.binaryedge.io/",
        params: [
            #{name: "domain",
                key: "domain",
                description: "Target domain to search for subdomains",
                type: "string",
                placeholder: "example.com",
                required: true
            },
            #{name: "api_key",
                key: "api_key",
                description: "BinaryEdge API Key",
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
        "Accept": "application/json",
        "Accept-Language": "en-US,en;q=0.9",
        "Connection": "keep-alive"
    };
    
    // Add API key header if provided
    if params.api_key != () {
        headers["X-Key"] = params.api_key;
    }
    
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
    if json_data != () {
        // BinaryEdge API specific response parsing
        if json_data.subdomains != () {
            for subdomain in json_data.subdomains {
                let domain_name = subdomain.to_lower();
                // Ensure it's a full domain name by appending the root domain if necessary
                if !domain_name.contains(".") {
                    domain_name = `${domain_name}.${root_domain}`;
                }
                
                // Check if it's a subdomain of our target domain
                if domain_name.ends_with(root_domain) && domain_name != root_domain {
                    if !subdomains.contains(domain_name) {
                        subdomains.push(domain_name);
                    }
                }
            }
        }
        
        // If no subdomains were found or the structure is unexpected, try to find subdomains in the entire response
        if subdomains.len() == 0 {
            // Fallback to regex extraction
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
    } else {
        // Fallback to regex extraction if JSON parsing fails
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
 * Main function to search BinaryEdge API for subdomains
 */
fn search_binaryedge(domain, api_key, timeout, proxy_url) {
    // BinaryEdge API URL for subdomain search
    let api_url = `https://api.binaryedge.io/v2/query/domains/subdomain/${domain}`;
    
    // Prepare request parameters
    let request_params = #{
        url: api_url,
        method: "GET",
        api_key: api_key,
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
    
    if parameters.api_key == () {
        return #{
            error: "Missing required parameter: api_key",
            result: []
        };
    }
    
    // Extract parameters
    let domain = parameters.domain;
    let api_key = parameters.api_key;
    let timeout = 30;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = parameters.proxy_url;
    
    // Search for subdomains
    let search_result = search_binaryedge(domain, api_key, timeout, proxy_url);
    
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