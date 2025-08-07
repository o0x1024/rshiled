/**
 * Gitee Search Domain Discovery Plugin
 * 
 * This plugin searches Gitee code repositories to discover subdomains of a given domain.
 * It uses Gitee's search functionality to find code containing domain references.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "GiteeSearch",
        description: "Searches for subdomains in Gitee code repositories",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://gitee.com/",
        params: [
            #{name: "domain",
                key: "domain",
                description: "Target domain to search for subdomains",
                type: "string",
                placeholder: "example.com",
                required: true
            },
            #{name: "max_pages",
                key: "max_pages",
                description: "Maximum number of search result pages to process",
                type: "integer",
                default_value: 100,
                required: false
            },
            #{name: "delay",
                key: "delay",
                description: "Delay between requests in seconds",
                type: "integer",
                default_value: 2,
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
    
    // Pattern to match subdomains
    let pattern = `[a-zA-Z0-9][-a-zA-Z0-9.]*\\.${root_domain.replace(".", "\\.")}`;
    
    // Extract potential subdomains from the response body
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
 * Check if there are no more search results
 */
fn has_no_more_results(response_body) {
    // Check if the response contains the "empty-box" class, indicating no results
    if response_body.contains("class=\"empty-box\"") {
        return true;
    }
    
    // Check if we've reached the last page
    if response_body.contains("<li class=\"disabled\"><a href=\"###\">") {
        return true;
    }
    
    return false;
}

/**
 * Check if results have new subdomains
 */
fn has_new_subdomains(current_subdomains, all_subdomains) {
    for subdomain in current_subdomains {
        if !all_subdomains.contains(subdomain) {
            return true;
        }
    }
    return false;
}

/**
 * Main function to search Gitee for subdomains
 */
fn search_gitee(domain, max_pages, delay, timeout, proxy_url) {
    let all_subdomains = [];
    let page_num = 1;
    
    // Continue querying until all results are fetched or limit is reached
    while (page_num <= max_pages) {
        // Sleep to avoid rate limiting
        if page_num > 1 {
            sleep(delay);
        }
        
        // Prepare query parameters
        let params = #{
            "pageno": page_num,
            "q": domain,
            "type": "code"
        };
        
        // Search URL
        let url = "https://search.gitee.com/";
        
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
        
        // Check if there are no more results
        if has_no_more_results(response.body) {
            break;
        }
        
        // Extract subdomains
        let current_subdomains = extract_subdomains(response.body, domain);
        
        // If no new subdomains found, break the loop
        if !has_new_subdomains(current_subdomains, all_subdomains) {
            break;
        }
        
        // Add new subdomains to the result
        for subdomain in current_subdomains {
            if !all_subdomains.contains(subdomain) {
                all_subdomains.push(subdomain);
            }
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
    
    // Extract parameters
    let domain = parameters.domain;
    let max_pages = 100;
    let delay = 2;
    let timeout = 10;
    if parameters.max_pages != () {
        max_pages = parameters.max_pages;
    }
    if parameters.delay != () {
        delay = parameters.delay;
    }
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = parameters.proxy_url;
    
    // Search for subdomains
    let search_result = search_gitee(domain, max_pages, delay, timeout, proxy_url);
    
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