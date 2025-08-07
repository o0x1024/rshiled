/**
 * Yahoo Search Domain Discovery Plugin
 * 
 * This plugin uses Yahoo search engine to discover subdomains of a given domain.
 * It utilizes the "site:." operator to find pages hosted on subdomains.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "YahooSearchDomainSearch",
        description: "Searches for subdomains using Yahoo Search engine",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://search.yahoo.com/",
        params: [
            #{name: "domain",
                key: "domain",
                description: "Target domain to search for subdomains",
                type: "string",
                placeholder: "example.com",
                required: true
            },
            #{name: "limit_num",
                key: "limit_num",
                description: "Maximum number of search results to process",
                type: "integer",
                default_value: 1000,
                required: false
            },
            #{name: "per_page",
                key: "per_page",
                description: "Number of results per page",
                type: "integer",
                default_value: 30,
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
            #{name: "recursive",
                key: "recursive",
                description: "Search recursively on discovered subdomains",
                type: "boolean",
                default_value: false,
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
        "Accept-Language": "en-US,en;q=0.9",
        "Referer": "https://search.yahoo.com/",
        "Connection": "keep-alive"
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
    
    // Add cookies if provided
    if params.cookies != () {
        request_params.cookies = params.cookies;
    }
    
    // Send the request
    let response = http_request(request_params);
    
    // Check for errors
    if response.error != () {
        print(`Error in HTTP request: ${response.error}`);
        return #{
            error: response.error,
            status_code: 0,
            body: "",
            cookies: ()
        };
    }
    
    // Return response with simplified structure
    return #{
        status_code: response.status_code,
        body: response.body,
        headers: response.headers,
        cookies: response.cookies
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
    
    // Clean up the HTML by removing bold tags that may break the pattern matching
    let cleaned_text = response_body.replace("<b>", "").replace("</b>", "");
    
    // Pattern to match subdomains
    let pattern = `[a-zA-Z0-9][-a-zA-Z0-9.]*\\.${root_domain.replace(".", "\\.")}`;
    
    // Use regex to find all matches
    let matches = regex_matches(cleaned_text, pattern);
    
    // Process matches and filter duplicates
    if matches.len() > 0 {
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
 * Generate filter statement to exclude common subdomains in order to find more diverse results
 */
fn generate_filters(domain, subdomains) {
    let filters = [];
    let counts = #{};
    
    // Count the frequency of each subdomain prefix
    for subdomain in subdomains {
        if subdomain.ends_with(domain) && subdomain != domain {
            let parts = subdomain.split(".");
            let prefix = parts[0]; // Get the first part of the subdomain
            
            if counts[prefix] == () {
                counts[prefix] = 1;
            } else {
                counts[prefix] = counts[prefix] + 1;
            }
        }
    }
    
    // Generate filter statements for common prefixes
    for prefix in counts.keys() {
        if counts[prefix] > 5 { // Threshold for considering a prefix common
            filters.push(` -site:${prefix}.${domain}`);
        }
    }
    
    return filters;
}

/**
 * Extracts second-level subdomains for recursive search
 */
fn extract_recursive_subdomains(subdomains, domain) {
    let recursive_domains = [];
    let root_domain = domain.to_lower();
    
    for subdomain in subdomains {
        if subdomain.ends_with(root_domain) && subdomain != root_domain {
            let parts = subdomain.split(".");
            if parts.len() > 2 { // Ensure it's at least a second-level subdomain
                let second_level = parts[parts.len() - 3] + "." + parts[parts.len() - 2] + "." + parts[parts.len() - 1];
                if !recursive_domains.contains(second_level) {
                    recursive_domains.push(second_level);
                }
            }
        }
    }
    
    return recursive_domains;
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
 * Main function to search Yahoo for subdomains
 */
fn search_yahoo(domain, filtered_subdomain, limit_num, per_page, delay, timeout, proxy_url, cookies) {
    let all_subdomains = [];
    let page_num = 1;
    
    // First, get cookies from Yahoo's main page
    if cookies == () {
        let init_url = "https://search.yahoo.com/";
        let init_request_params = #{
            url: init_url,
            method: "GET",
            timeout: timeout
        };
        
        // Add proxy if provided
        if proxy_url != () {
            init_request_params.proxy_url = proxy_url;
        }
        
        // Send the initial request to get cookies
        let init_response = send_http_request(init_request_params);
        
        // Check for errors
        if init_response.error != () {
            return #{
                error: init_response.error,
                subdomains: []
            };
        }
        
        // Store cookies for subsequent requests
        cookies = init_response.cookies;
    }
    
    // Continue querying until all results are fetched or limit is reached
    while (page_num < limit_num) {
        // Sleep to avoid rate limiting
        if page_num > 1 {
            sleep(delay);
        }
        
        // Prepare query parameters
        let query = `site:.${domain}${filtered_subdomain}`;
        let params = #{
            "p": query,
            "b": page_num.to_string(),
            "pz": per_page.to_string()
        };
        
        // Search URL
        let url = "https://search.yahoo.com/search";
        
        // Prepare request parameters
        let request_params = #{
            url: url,
            method: "GET",
            params: params,
            cookies: cookies,
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
        if !has_new_subdomains(current_subdomains, all_subdomains) {
            break;
        }
        
        // Add new subdomains to the result
        for subdomain in current_subdomains {
            if !all_subdomains.contains(subdomain) {
                all_subdomains.push(subdomain);
            }
        }
        
        // Check if there's a next page
        if !response.body.contains(">Next</a>") {
            break;
        }
        
        // Update page number for the next request
        page_num += per_page;
    }
    
    return #{
        error: (),
        subdomains: all_subdomains,
        cookies: cookies
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
    let limit_num = 1000;
    let per_page = 30;
    let delay = 2;
    let timeout = 10;
    let recursive = false;
    if parameters.recursive != () {
        recursive = parameters.recursive;
    }

    if parameters.per_page != () {
        per_page = parameters.per_page;
    }
    if parameters.delay != () {
        delay = parameters.delay;
    }
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }

    let proxy_url = parameters.proxy_url;
    
    // Initial search
    let all_subdomains = [];
    let cookies = ();
    
    // Initial search with no filter
    let initial_result = search_yahoo(domain, "", limit_num, per_page, delay, timeout, proxy_url, cookies);
    
    // Check for errors
    if initial_result.error != () {
        return #{
            error: initial_result.error,
            result: []
        };
    }
    
    // Store subdomains and cookies
    all_subdomains = initial_result.subdomains;
    cookies = initial_result.cookies;
    
    // Generate filters based on common subdomains
    let filters = generate_filters(domain, all_subdomains);
    
    // Search with filters to find more diverse subdomains
    for filter_statement in filters {
        let filtered_result = search_yahoo(domain, filter_statement, limit_num, per_page, delay, timeout, proxy_url, cookies);
        
        // Check for errors
        if filtered_result.error != () {
            continue;
        }
        
        // Add new subdomains to the result
        for subdomain in filtered_result.subdomains {
            if !all_subdomains.contains(subdomain) {
                all_subdomains.push(subdomain);
            }
        }
        
        // Update cookies
        cookies = filtered_result.cookies;
    }
    
    // Recursive search on discovered subdomains if enabled
    if recursive {
        let recursive_domains = extract_recursive_subdomains(all_subdomains, domain);
        
        for recursive_domain in recursive_domains {
            let recursive_result = search_yahoo(recursive_domain, "", limit_num, per_page, delay, timeout, proxy_url, cookies);
            
            // Check for errors
            if recursive_result.error != () {
                continue;
            }
            
            // Add new subdomains to the result
            for subdomain in recursive_result.subdomains {
                if !all_subdomains.contains(subdomain) {
                    all_subdomains.push(subdomain);
                }
            }
            
            // Update cookies
            cookies = recursive_result.cookies;
        }
    }
    
    // Return success response
    return #{
        error: (),
        result: all_subdomains
    };
} 