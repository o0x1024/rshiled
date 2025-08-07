/**
 * Quake API Search Domain Discovery Plugin
 * 
 * This plugin uses the Quake API (360.net) to discover subdomains of a given domain.
 * It requires an API key to access the service.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "QuakeApiSearch",
        description: "Searches for subdomains using Quake API (360.net)",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        references: ["https://quake.360.net/"],
        params: [
            #{
                name: "Domain",
                key: "domain",
                type: "string",
                description: "Target domain to search for subdomains",
                required: true,
                default_value: ()
            },
            #{
                name: "API Key",
                key: "api_key",
                type: "string",
                description: "Quake API Key",
                required: true,
                default_value: ()
            },
            #{
                name: "Maximum Results",
                key: "max_results",
                type: "integer",
                description: "Maximum number of results to return",
                required: false,
                default_value: 1000
            },
            #{
                name: "Delay",
                key: "delay",
                type: "integer",
                description: "Delay between requests in seconds",
                required: false,
                default_value: 1
            },
            #{
                name: "Timeout",
                key: "timeout",
                type: "integer",
                description: "Request timeout in seconds",
                required: false,
                default_value: 10
            },
            #{
                name: "Proxy URL",
                key: "proxy_url",
                type: "string",
                description: "Proxy URL to use for requests",
                required: false,
                default_value: ()
            }
        ],
        result_fields: [
            #{
                name: "Subdomain",
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
        "Connection": "keep-alive",
        "Content-Type": "application/json"
    };
    
    // Add API key header
    if params.api_key != () {
        headers["X-QuakeToken"] = params.api_key;
    }
    
    // Add any additional headers
    if params.headers != () {
        for key in params.headers.keys() {
            headers[key] = params.headers[key];
        }
    }
    
    let method  = "GET";
    if params.method != (){
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
    
    // Add JSON body if provided
    if params.json != () {
        request_params.body = json_stringify(params.json);
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
 * Sleeps for a specific amount of time to avoid rate limiting
 */
fn sleep(seconds) {
    // Simulate sleep by using a busy wait
    // In a real implementation, this would use a proper sleep function
    let start_time = timestamp();
    let end_time = start_time + (seconds * 1000); // Convert to milliseconds
    
    while (timestamp() < end_time) {
        // Busy wait
    }
}

/**
 * Get current timestamp in milliseconds
 */
fn timestamp() {
    let date = new_date();
    return date.to_milliseconds();
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
    
    // Parse the JSON response
    let json_data = json_parse(response_body);
    if json_data == () {
        return subdomains;
    }
    
    // Extract data from the response
    if json_data.data == () {
        return subdomains;
    }
    
    // Process each result
    for item in json_data.data {
        if item.service != () && item.service.http != () && item.service.http.host != () {
            let host = item.service.http.host.to_lower();
            if host.ends_with(root_domain) && host != root_domain {
                if !subdomains.contains(host) {
                    subdomains.push(host);
                }
            }
        }
    }
    
    // Also try to match subdomains from the raw response
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
    
    return subdomains;
}

/**
 * Main function to search Quake API for subdomains
 */
fn search_quake(domain, api_key, max_results, delay, timeout, proxy_url) {
    let all_subdomains = [];
    let per_page_num = 100;
    let page_num = 0;
    
    // Continue querying until all results are fetched or limit is reached
    while (per_page_num * page_num < max_results) {
        // Sleep to avoid rate limiting
        if page_num > 0 {
            sleep(delay);
        }
        
        // Prepare query parameters
        let query = #{
            "query": `domain:"${domain}"`,
            "start": page_num * per_page_num,
            "size": per_page_num,
            "include": ["service.http.host"]
        };
        
        // API URL
        let url = "https://quake.360.net/api/v3/search/quake_service";
        
        // Prepare request parameters
        let request_params = #{
            url: url,
            method: "POST",
            json: query,
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
        
        // Check if we've reached the end of results
        let json_data = json_parse(response.body);
        if json_data != () && json_data.meta != () && json_data.meta.pagination != () && json_data.meta.pagination.total != () {
            let total = json_data.meta.pagination.total;
            if page_num * per_page_num >= total {
                break;
            }
        } else {
            break;
        }
        
        // Update page number for the next request
        page_num += 1;
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
    let max_results = if parameters.max_results == () { 1000 } else { parameters.max_results };
    let delay = if parameters.delay == () { 1 } else { parameters.delay };
    let timeout = if parameters.timeout == () { 10 } else { parameters.timeout };
    let proxy_url = parameters.proxy_url;
    
    // Search for subdomains
    let search_result = search_quake(domain, api_key, max_results, delay, timeout, proxy_url);
    
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