/**
 * Racent Domain Discovery Plugin
 * 
 * This plugin uses the Racent certificate search service to discover subdomains of a given domain.
 * It searches through certificate databases to find certificates issued for the domain and its subdomains.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "RacentSearch",
        description: "Searches for subdomains using Racent certificate database",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://rapiddns.io/",
        params: [
            #{name: "domain",
                key: "domain",
                description: "Target domain to search for subdomains",
                type: "string",
                placeholder: "example.com",
                required: true
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
    
    // Pattern to match subdomains in HTML table rows
    // RapidDNS typically presents results in an HTML table
    let pattern = `<tr><td>([a-zA-Z0-9][-a-zA-Z0-9.]*\\.${root_domain.replace(".", "\\.")})</td>`;
    let matches = regex_matches(response_body, pattern);
    
    // Extract from matches - typically the first capture group contains the subdomain
    for matched in matches {
        // First try to get the subdomain from capture group
        let extract_pattern = `<tr><td>([a-zA-Z0-9][-a-zA-Z0-9.]*\\.${root_domain.replace(".", "\\.")})`;
        let capture_matches = regex_matches(matched, extract_pattern);
        if capture_matches.len() > 0 {
            for capture in capture_matches {
                let subdomain_pattern = `([a-zA-Z0-9][-a-zA-Z0-9.]*\\.${root_domain.replace(".", "\\.")})`;
                let subdomain_matches = regex_matches(capture, subdomain_pattern);
                if subdomain_matches.len() > 0 {
                    let subdomain = subdomain_matches[0].to_lower();
                    if subdomain.ends_with(root_domain) && subdomain != root_domain {
                        if !subdomains.contains(subdomain) {
                            subdomains.push(subdomain);
                        }
                    }
                }
            }
        } else {
            // Fallback to direct match if capture group doesn't work
            let subdomain_pattern = `[a-zA-Z0-9][-a-zA-Z0-9.]*\\.${root_domain.replace(".", "\\.")}`;
            let subdomain_matches = regex_matches(matched, subdomain_pattern);
            for subdomain_match in subdomain_matches {
                let subdomain = subdomain_matched.to_lower();
                if subdomain.ends_with(root_domain) && subdomain != root_domain {
                    if !subdomains.contains(subdomain) {
                        subdomains.push(subdomain);
                    }
                }
            }
        }
    }
    
    // If we couldn't extract subdomains from the table format, try a more general approach
    if subdomains.len() == 0 {
        // Fallback to general regex extraction
        let fallback_pattern = `[a-zA-Z0-9][-a-zA-Z0-9.]*\\.${root_domain.replace(".", "\\.")}`;
        let fallback_matches = regex_matches(response_body, fallback_pattern);
        
        for matched in fallback_matches {
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
 * Main function to search Racent (via RapidDNS) for subdomains
 */
fn search_racent(domain, timeout, proxy_url) {
    // RapidDNS is a common replacement for Racent
    let api_url = "https://rapiddns.io/subdomain";
    
    // Prepare query parameters
    let params = #{
        "domain": domain,
        "full": "1"  // Request full results
    };
    
    // Prepare request parameters
    let request_params = #{
        url: api_url,
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
 * Alternative method to search for subdomains
 */
fn search_alternative(domain, timeout, proxy_url) {
    // Alternative source: DNS Dumpster (which may have similar data)
    let api_url = "https://dnsdumpster.com/";
    
    // First, get the CSRF token
    let initial_request = #{
        url: api_url,
        method: "GET",
        timeout: timeout
    };
    
    if proxy_url != () {
        initial_request.proxy_url = proxy_url;
    }
    
    // Send the initial request to get the CSRF token
    let initial_response = send_http_request(initial_request);
    
    if initial_response.error != () != () && initial_response.status_code != 200 {
        return #{
            error: "Failed to get CSRF token for search",
            subdomains: []
        };
    }
    
    // Extract CSRF token
    let csrf_pattern = `name="csrfmiddlewaretoken" value="([^"]+)"`;
    let csrf_matches = regex_matches(initial_response.body, csrf_pattern);
    
    if csrf_matches.len() == 0 {
        return #{
            error: "Could not extract CSRF token",
            subdomains: []
        };
    }
    
    let csrf_token = csrf_matches[0].replace(csrf_pattern, "$1");
    
    // Prepare the search request with the CSRF token
    let headers = #{
        "Referer": api_url,
        "Content-Type": "application/x-www-form-urlencoded",
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.102 Safari/537.36"
    };
    
    let search_request = #{
        url: api_url,
        method: "POST",
        headers: headers,
        timeout: timeout,
        body: `csrfmiddlewaretoken=${csrf_token}&targetip=${domain}`
    };
    
    if proxy_url != () {
        search_request.proxy_url = proxy_url;
    }
    
    // Send the search request
    let search_response = send_http_request(search_request);
    
    if search_response.error != () != () && search_response.status_code != 200 {
        return #{
            error: "Failed to perform search",
            subdomains: []
        };
    }
    
    // Extract subdomains
    let alt_subdomains = extract_subdomains(search_response.body, domain);
    
    return #{
        error: (),
        subdomains: alt_subdomains
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
    let timeout = 30;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = parameters.proxy_url;
    
    // Search for subdomains
    let search_result = search_racent(domain, timeout, proxy_url);
    
    // Check for errors or no results
    if search_result.error != () != () && search_result.subdomains.len() == 0 {
        // Try alternative method
        let alt_result = search_alternative(domain, timeout, proxy_url);
        
        // If alternative method worked and found subdomains, use those results
        if alt_result.error == () && alt_result.subdomains.len() > 0 {
            search_result = alt_result;
        }
    }
    
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