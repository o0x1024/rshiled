/**
 * VirusTotal API Domain Search Plugin
 * 
 * This plugin queries the VirusTotal API v3 for a given domain using an API key
 * and extracts subdomains from the results.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "VirusTotalAPIDomainSearch",
        description: "Searches for subdomains using VirusTotal API v3 with API key",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://developers.virustotal.com/reference/subdomains",
        params: [
            #{name: "domain",
                key: "domain",
                description: "Target domain to scan",
                type: "string",
                placeholder: "example.com",
                required: true
            },
            #{name: "api_key",
                key: "api_key",
                description: "VirusTotal API Key",
                type: "string",
                required: true,
                default_value: ()
            },
            #{name: "limit",
                key: "limit",
                description: "Number of results per page",
                type: "integer",
                default_value: 40,
                required: false
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
 * Extracts subdomains from VirusTotal API response
 */
fn extract_subdomains(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();
    
    // Try to parse as JSON first
    try {
        let json_data = parse_json(response_body);
        
        // Check if 'data' array exists in the response (v3 API format)
        if json_data.data != () {
            let data_array = json_data.data;
            
            // Iterate through data entries
            for item in data_array {
                if item.id != () {
                    let subdomain = item.id.to_lower();
                    if subdomain.ends_with(root_domain) && subdomain != root_domain {
                        if !subdomains.contains(subdomain) {
                            subdomains.push(subdomain);
                        }
                    }
                }
                
                // Some results might have the domain in the attributes
                if item.attributes != () && item.attributes.id != () {
                    let subdomain = item.attributes.id.to_lower();
                    if subdomain.ends_with(root_domain) && subdomain != root_domain {
                        if !subdomains.contains(subdomain) {
                            subdomains.push(subdomain);
                        }
                    }
                }
            }
        }
        
        // Check for meta.cursor for pagination
        let next_cursor = "";
        if json_data.meta != () && json_data.meta.cursor != () {
            next_cursor = json_data.meta.cursor;
        }
        
        return #{
            subdomains: subdomains,
            next_cursor: next_cursor
        };
    } catch(error) {
        print(`Error parsing JSON: ${error}`);
        return #{
            subdomains: [],
            next_cursor: ""
        };
    }
}

/**
 * Main function to query VirusTotal API for subdomains
 */
fn query_virustotal_api(domain, api_key, limit, timeout, proxy_url) {
    let all_subdomains = [];
    let url = `https://www.virustotal.com/api/v3/domains/${domain}/subdomains`;
    let next_cursor = "";
    
    // Continue querying while there's a next cursor
    while (true) {
        // Prepare query parameters
        let params = #{
            "limit": limit.to_string()
        };
        
        // Add cursor for pagination if available
        if next_cursor.trim()!= "" {
            params.cursor = next_cursor;
        }
        
        // Prepare headers with API key
        let headers = #{
            "x-apikey": api_key
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
        
        // Extract subdomains and next cursor
        let extraction_result = extract_subdomains(response.body, domain);
        let current_subdomains = extraction_result.subdomains;
        next_cursor = extraction_result.next_cursor;
        
        // Add new subdomains to the result
        for subdomain in current_subdomains {
            if !all_subdomains.contains(subdomain) {
                all_subdomains.push(subdomain);
            }
        }
        
        // Break if no more results (no cursor)
        if next_cursor.trim()== "" {
            break;
        }
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
    let limit = 40;
    if parameters.limit != () {
        limit = parameters.limit;
    }
    let timeout = 10;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = parameters.proxy_url;
    
    // Query VirusTotal API for subdomains
    let result = query_virustotal_api(domain, api_key, limit, timeout, proxy_url);
    
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