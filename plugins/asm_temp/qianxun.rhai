/**
 * QianXun (千寻) Domain Discovery Plugin
 * 
 * This plugin uses the QianXun (dnsscan.cn) service to discover subdomains of a given domain.
 * It queries the web service and extracts subdomain information from the results.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "QianXunSearch",
        description: "Searches for subdomains using QianXun (dnsscan.cn) service",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://www.dnsscan.cn/",
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
                default_value: 20,
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
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
        "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
        "Connection": "keep-alive",
        "Referer": "https://www.dnsscan.cn/",
        "Content-Type": "application/x-www-form-urlencoded"
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
    
    // Add body data if this is a POST request
    if params.method == "POST" && params.data != () {
        request_params.body = params.data;
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
    
    // Pattern to match subdomains in HTML content
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
 * Checks if there are more pages of results
 */
fn has_more_pages(response_body) {
    // Check if pagination exists
    if !response_body.contains("<div id=\"page\" class=\"pagelist\">") {
        return false;
    }
    
    // Check if we're on the last page
    if response_body.contains("<li class=\"disabled\"><span>&raquo;</span></li>") {
        return false;
    }
    
    return true;
}

/**
 * Main function to search QianXun for subdomains
 */
fn search_qianxun(domain, max_pages, timeout, proxy_url) {
    let all_subdomains = [];
    let page_num = 1;
    
    // Continue querying until all results are fetched or limit is reached
    while (page_num <= max_pages) {
        // Construct the search URL
        let url = `https://www.dnsscan.cn/dns.html?keywords=${domain}&page=${page_num}`;
        
        // Prepare POST data
        let data = `ecmsfrom=&show=&num=&classid=0&keywords=${domain}`;
        
        // Prepare request parameters
        let request_params = #{
            url: url,
            method: "POST",
            data: data,
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
        
        // Check if there are more pages
        if !has_more_pages(response.body) {
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
    
    // Extract parameters
    let domain = parameters.domain;
    let max_pages = 20;
    let timeout = 30;
    if parameters.max_pages != () {
        max_pages = parameters.max_pages;
    }
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = "";
    if parameters.proxy_url != () {
        proxy_url = parameters.proxy_url;
    }
    
    // Search for subdomains
    let search_result = search_qianxun(domain, max_pages, timeout, proxy_url);
    
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