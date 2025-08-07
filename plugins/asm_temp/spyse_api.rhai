/**
 * Spyse API Domain Search Plugin
 * 
 * This plugin queries the Spyse API for a given domain and extracts subdomains.
 * Spyse is a search engine for Internet connected devices and provides comprehensive information.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "SpyseAPIDomainSearch",
        description: "Searches for subdomains using Spyse API",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://spyse.com/",
        params: [
            #{
                name: "domain",
                key: "domain",
                type: "string",
                description: "Target domain to scan",
                required: true,
                default_value: ()
            },
            #{
                name: "api_token",
                key: "api_token",
                type: "string",
                description: "Spyse API Token",
                required: true,
                default_value: ()
            },
            #{
                name: "limit",
                key: "limit",
                type: "integer",
                description: "Number of results per page",
                default_value: 100,
                required: false
            },
            #{
                name: "max_pages",
                key: "max_pages",
                type: "integer",
                description: "Maximum number of pages to fetch",
                default_value: 10,
                required: false
            },
            #{
                name: "timeout",
                key: "timeout",
                type: "integer",
                description: "Request timeout in seconds",
                default_value: 10,
                required: false
            },
            #{name: "proxy_url",
                key: "proxy_url",
                type: "string",
                description: "Proxy URL to use for requests",
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

        }]
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
        "Accept-Language": "en-US,en;q=0.9"
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
    
    let timeout = 10;
    if params.timeout != () {
        timeout = params.timeout;
    }
    // Prepare request parameters
    let request_params = #{
        url: params.url,
        method: method,
        headers: headers,
        timeout:timeout
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
 * Extracts subdomains from API response or from text
 */
fn extract_subdomains(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();
    
    // Try to parse as JSON first
    try {
        let json_data = parse_json(response_body);
        
        // Check if data and items exist in the response
        if json_data.data != () && json_data.data.items != () {
            let items = json_data.data.items;
            
            // Iterate through items
            for item in items {
                if item.name != () {
                    let subdomain = item.name.to_lower();
                    if subdomain.ends_with(root_domain) && subdomain != root_domain {
                        if !subdomains.contains(subdomain) {
                            subdomains.push(subdomain);
                        }
                    }
                }
            }
        }
    } catch(error) {
        print(`Error parsing JSON: ${error}`);
        
        // Fallback to simple pattern matching
        let pattern = `[a-zA-Z0-9-]+\\.${root_domain.replace(".", "\\.")}`;
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
 * Main function to query Spyse API for subdomains
 */
fn query_spyse_api(domain, api_token, limit, max_pages, timeout, proxy_url) {
    let all_subdomains = [];
    let url = "https://api.spyse.com/v3/data/domain/subdomain";
    let offset = 0;
    let page_count = 0;
    
    // Add Authorization header
    let headers = #{
        "Authorization": `Bearer ${api_token}`
    };
    
    // Continue querying until all results are fetched or max pages is reached
    while (page_count < max_pages) {
        // Prepare query parameters
        let params = #{
            "domain": domain,
            "offset": offset.to_string(),
            "limit": limit.to_string()
        };
        
        // Prepare request parameters
        let request_params = #{
            url: url,
            method: "GET",
            headers: headers,
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
        
        // If no subdomains found, break the loop
        if current_subdomains.len() == 0 {
            break;
        }
        
        // Add new subdomains to the result
        for subdomain in current_subdomains {
            if !all_subdomains.contains(subdomain) {
                all_subdomains.push(subdomain);
            }
        }
        
        // Try to parse JSON to check items count
        try {
            let json_data = parse_json(response.body);
            
            // Check if we have fewer items than the limit, indicating last page
            if json_data.data != () && json_data.data.items != () {
                let items = json_data.data.items;
                if items.len() < limit {
                    break;
                }
            }
        } catch(error) {
            print(`Error parsing JSON for pagination check: ${error}`);
        }
        
        // Update offset for the next page
        offset += limit;
        page_count += 1;
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
    
    if parameters.api_token == () {
        return #{
            error: "Missing required parameter: api_token",
            result: []
        };
    }
    
    // Extract parameters
    let domain = parameters.domain;
    let api_token = parameters.api_token;
    let limit = 100;
    let max_pages = 10;
    let timeout = 10;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    if parameters.max_pages != () {
        max_pages = parameters.max_pages;
    }
    if parameters.limit != () {
        limit = parameters.limit;
    }
        let max_pages = 10;
    if parameters.max_pages != () {
        max_pages = parameters.max_pages;
    }
        let timeout = 10;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = parameters.proxy_url;
    
    // Query Spyse API for subdomains
    let result = query_spyse_api(domain, api_token, limit, max_pages, timeout, proxy_url);
    
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