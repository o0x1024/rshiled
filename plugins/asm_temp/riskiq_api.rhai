/**
 * RiskIQ API Domain Search Plugin
 * 
 * This plugin queries the RiskIQ PassiveTotal API for a given domain and extracts subdomains.
 * RiskIQ provides comprehensive threat intelligence data.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "RiskIQAPIDomainSearch",
        description: "Searches for subdomains using RiskIQ PassiveTotal API",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://www.riskiq.com/products/passivetotal/",
        params: [
            #{name: "domain",
                key: "domain",
                type: "string",
                description: "Target domain to scan",
                required: true,
                default_value: ()
            },
            #{name: "username",
                key: "username",
                description: "RiskIQ API Username",
                type: "string",
                required: true,
                default_value: ()
            },
            #{name: "api_key",
                key: "api_key",
                description: "RiskIQ API Key",
                type: "string",
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
        "Accept": "application/json",
        "Accept-Language": "en-US,en;q=0.9"
    };
    
    // Add any additional headers
    if params.headers != () {
        for key in params.headers.keys() {
            headers[key] = params.headers[key];
        }
    }
    
    // Add Basic Authentication header if username and password are provided
    if params.auth != () {
        let auth_string = base64_encode(`${params.auth.username}:${params.auth.password}`);
        headers["Authorization"] = `Basic ${auth_string}`;
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
 * Extracts subdomains from RiskIQ API response
 */
fn extract_subdomains(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();
    
    // Try to parse as JSON first
    try {
        let json_data = parse_json(response_body);
        
        // RiskIQ API returns subdomain prefixes in the 'subdomains' array
        if json_data.subdomains != () {
            let prefixes = json_data.subdomains;
            
            // Append the main domain to each prefix to form complete subdomains
            for prefix in prefixes {
                let subdomain = `${prefix}.${root_domain}`;
                if !subdomains.contains(subdomain) {
                    subdomains.push(subdomain);
                }
            }
        }
    } catch(error) {
        print(`Error parsing JSON: ${error}`);
    }
    
    return subdomains;
}

/**
 * Main function to query RiskIQ API for subdomains
 */
fn query_riskiq_api(domain, username, api_key, timeout, proxy_url) {
    let url = "https://api.riskiq.net/pt/v2/enrichment/subdomains";
    
    // Prepare query parameters
    let params = #{
        "query": domain
    };
    
    // Prepare authentication
    let auth = #{
        "username": username,
        "password": api_key
    };
    
    // Prepare request parameters
    let request_params = #{
        url: url,
        method: "GET",
        params: params,
        auth: auth,
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
    
    if parameters.username == () {
        return #{
            error: "Missing required parameter: username",
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
    let username = parameters.username;
    let api_key = parameters.api_key;
    let timeout = 10;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = parameters.proxy_url;
    
    // Query RiskIQ API for subdomains
    let result = query_riskiq_api(domain, username, api_key, timeout, proxy_url);
    
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