/**
 * IPv4Info API Domain Discovery Plugin
 * 
 * This plugin uses the IPv4Info API to discover subdomains of a given domain.
 * It requires an API key to access the service.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "IPv4InfoApiSearch",
        description: "Searches for subdomains using IPv4Info API",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "http://ipv4info.com/",
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
                description: "IPv4Info API Key",
                type: "string",
                placeholder: "your-api-key",
                required: true
            },
            #{name: "max_pages",
                key: "max_pages",
                description: "Maximum number of search result pages to process (max 50)",
                type: "integer",
                default_value: 50,
                required: false
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
                placeholder: "http://127.0.0.1:8080",
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
        // First try to extract from the Subdomains array if available
        if json_data.Subdomains != () {
            for subdomain in json_data.Subdomains {
                let domain_name = subdomain.to_lower();
                if domain_name.ends_with(root_domain) && domain_name != root_domain {
                    if !subdomains.contains(domain_name) {
                        subdomains.push(domain_name);
                    }
                }
            }
        }
        
        // If no subdomains were found in the array or it doesn't exist, 
        // try to find subdomains in the entire response
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
 * Main function to search IPv4Info API for subdomains
 */
fn search_ipv4info(domain, api_key, max_pages, timeout, proxy_url) {
    let all_subdomains = [];
    let page = 0;
    
    // IPv4Info API URL
    let api_url = "http://ipv4info.com/api_v1/";
    
    // Continue querying until all results are fetched or limit is reached
    while (page < max_pages) {
        // Prepare query parameters
        let params = #{
            "type": "SUBDOMAINS",
            "key": api_key,
            "value": domain,
            "page": page
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
                subdomains: all_subdomains
            };
        }
        
        // Check status code
        if response.status_code != 200 {
            return #{
                error: `Unexpected status code: ${response.status_code}`,
                subdomains: all_subdomains
            };
        }
        
        // Extract subdomains
        let current_subdomains = extract_subdomains(response.body, domain);
        
        // If no new subdomains found, break the loop
        if current_subdomains.len() == 0 {
            break;
        }
        
        // Add new subdomains to the result
        for subdomain in current_subdomains {
            if !all_subdomains.contains(subdomain) {
                all_subdomains.push(subdomain);
            }
        }
        
        // Check if we need to continue pagination
        // IPv4Info API returns at most 300 results per page
        let json_data = json_parse(response.body);
        if json_data != () && json_data.Subdomains != () {
            if json_data.Subdomains.len() < 300 {
                break; // No more pages
            }
        } else {
            break; // Can't determine if there are more pages, so stop
        }
        
        // Update page number for the next request
        page += 1;
    }
    
    return #{
        error: (),
        subdomains: all_subdomains
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
    let max_pages = 50;
    let timeout = 30;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    if parameters.max_pages != () {
        max_pages = parameters.max_pages;
    }
    let proxy_url = parameters.proxy_url;
    
    // Search for subdomains
    let search_result = search_ipv4info(domain, api_key, max_pages, timeout, proxy_url);
    
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