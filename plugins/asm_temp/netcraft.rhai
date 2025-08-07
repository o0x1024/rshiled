/**
 * Netcraft Domain Search Plugin
 * 
 * This plugin queries the Netcraft SearchDNS service for a given domain and extracts subdomains.
 * Netcraft provides extensive information about domains and their hosting infrastructure.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "NetcraftDomainSearch",
        description: "Searches for subdomains using Netcraft SearchDNS",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://searchdns.netcraft.com/",
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
        "Referer": "https://searchdns.netcraft.com/",
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
 * Sleeps for a random amount of time to avoid rate limiting
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
 * Extracts subdomains from Netcraft HTML response
 */
fn extract_subdomains(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();
    
    // Netcraft responses are HTML, so we need to extract domain patterns
    let pattern = `[a-zA-Z0-9][-a-zA-Z0-9.]*\\.${root_domain.replace(".", "\\.")}`;
    
    // Use regex to find all matches
    let matches = regex_matches(response_body, pattern);
    
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
 * Extract the 'last' parameter from the response for pagination
 */
fn extract_last_param(response_body, domain) {
    // Look for the 'last' parameter in the Next Page link
    let pattern = `&last=.*${domain.replace(".", "\\.")}`;
    let matches = regex_matches(response_body, pattern);
    
    if matches.len() > 0 {
        return matches[0];
    }
    
    return "";
}

/**
 * Check if there's a next page in the response
 */
fn has_next_page(response_body) {
    return response_body.contains("Next Page");
}

/**
 * Main function to query Netcraft for subdomains
 */
fn query_netcraft(domain, max_pages, delay, timeout, proxy_url) {
    let all_subdomains = [];
    let base_url = "https://searchdns.netcraft.com/?restriction=site+contains&position=limited";
    let page_num = 1;
    let per_page_num = 20;
    let page_count = 0;
    let last_param = "";
    
    // Continue querying until all results are fetched or max pages is reached
    while (page_count < max_pages) {
        // Sleep to avoid rate limiting
        if page_count > 0 {
            sleep(delay);
        }
        
        // Prepare query parameters
        let params = #{
            "host": `*.${domain}`,
            "from": page_num.to_string()
        };
        
        // Construct the URL with the last parameter if available
        let url = base_url + last_param;
        
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
        
        // Check if there's a next page
        if !has_next_page(response.body) {
            break;
        }
        
        // Extract the 'last' parameter for the next page
        last_param = extract_last_param(response.body, domain);
        if last_param.trim()== "" {
            break;
        }
        
        // Update page number for the next request
        page_num += per_page_num;
        page_count += 1;
        
        // Check if we've reached the maximum page limit (Netcraft typically limits to 500)
        if page_num > 500 {
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
    
    // Extract parameters
    let domain = parameters.domain;
    let max_pages = 25;
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
    
    // Query Netcraft for subdomains
    let result = query_netcraft(domain, max_pages, delay, timeout, proxy_url);
    
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