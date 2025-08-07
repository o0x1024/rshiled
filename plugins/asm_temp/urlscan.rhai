/**
 * URLScan Domain Search Plugin
 * 
 * This plugin queries the URLScan.io API for a given domain and extracts subdomains.
 * It leverages URLScan's search functionality to find related domains and subdomains.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "URLScanDomainSearch",
        description: "Searches for subdomains using URLScan.io API",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://urlscan.io/",
        params: [
            #{name: "domain",
                key: "domain",
                description: "Target domain to scan",
                type: "string",
                placeholder: "example.com",
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
            #{name: "subdomain",
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
        "Accept-Language": "en-US,en;q=0.9",
        "Content-Type": "application/json"
    };
    
    // Add any additional headers
    if params.headers != () {
        for key in params.headers.keys() {
            headers[key] = params.headers[key];
        }
    }
    
    // Prepare request parameters
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
 * Extracts subdomains from URLScan response
 */
fn extract_subdomains(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();
    
    // Try to parse as JSON first
    try {
        let json_data = parse_json(response_body);
        
        // Check if results array exists in the response
        if json_data.results != () {
            let results = json_data.results;
            
            // Iterate through results and extract domains
            for result in results {
                // Check page domain
                if result.page != () && result.page.domain != () {
                    let page_domain = result.page.domain.to_lower();
                    if page_domain.ends_with(root_domain) && page_domain != root_domain {
                        if !subdomains.contains(page_domain) {
                            subdomains.push(page_domain);
                        }
                    }
                }
                
                // Check task domain
                if result.task != () && result.task.domain != () {
                    let task_domain = result.task.domain.to_lower();
                    if task_domain.ends_with(root_domain) && task_domain != root_domain {
                        if !subdomains.contains(task_domain) {
                            subdomains.push(task_domain);
                        }
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
 * Main function to query URLScan for subdomains
 */
fn query_urlscan(domain, timeout, proxy_url) {
    let url = "https://urlscan.io/api/v1/search/";
    let params = #{
        "q": `domain:${domain}`
    };
    
    // Prepare request parameters
    let request_params = #{
        url: url,
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
    let timeout = 10;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = parameters.proxy_url;
    
    // Query URLScan for subdomains
    let result = query_urlscan(domain, timeout, proxy_url);
    
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