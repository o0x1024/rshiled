/**
 * FullHunt API Domain Search Plugin
 * 
 * This plugin queries the FullHunt API for a given domain and extracts subdomains.
 * FullHunt is a threat intelligence platform that provides attackv surface data.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "FullHuntAPIDomainSearch",
        description: "Searches for subdomains using FullHunt API",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://fullhunt.io/",
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
                description: "FullHunt API Key",
                type: "string",
                placeholder: "your-api-key-here",
                required: true
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
        "Accept": "application/json, text/plain, */*",
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
 * Extracts subdomains from FullHunt API response
 */
fn extract_subdomains(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();
    
    // Try to parse as JSON first
    try {
        let json_data = parse_json(response_body);
        
        // Check if hosts array exists in the response
        if json_data.hosts != () {
            let hosts = json_data.hosts;
            
            // Iterate through hosts (which are the subdomains)
            for host in hosts {
                if host.to_lower().ends_with(root_domain) && host.to_lower() != root_domain {
                    if !subdomains.contains(host) {
                        subdomains.push(host);
                    }
                }
            }
        }
    } catch(error) {
        print(`Error parsing JSON: ${error}`);
    }
    
    return subdomains;
}

/**
 * Main function to query FullHunt API for subdomains
 */
fn query_fullhunt_api(domain, api_key, timeout, proxy_url) {
    let url = `https://fullhunt.io/api/v1/domain/${domain}/subdomains`;
    
    // Add API key header
    let headers = #{
        "X-API-KEY": api_key
    };
    
    // Prepare request parameters
    let request_params = #{
        url: url,
        method: "GET",
        headers: headers,
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
    let timeout = 10;   
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = parameters.proxy_url;
    
    // Query FullHunt API for subdomains
    let result = query_fullhunt_api(domain, api_key, timeout, proxy_url);
    
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