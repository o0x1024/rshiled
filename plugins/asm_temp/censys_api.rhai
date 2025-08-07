/**
 * Censys API Domain Discovery Plugin
 * 
 * This plugin uses the Censys certificate search API to discover subdomains of a given domain.
 * It requires API credentials to access the service.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "CensysApiSearch",
        description: "Searches for subdomains using Censys certificate search API",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://search.censys.io/",
        params: [
            #{name: "domain",
                key: "domain",
                description: "Target domain to search for subdomains",
                type: "string",
                required: true,
                default_value: ()
            },
            #{name: "api_id",
                key: "api_id",
                description: "Censys API ID",
                type: "string",
                required: true,
                default_value: ()
            },
            #{name: "api_secret",
                key: "api_secret",
                description: "Censys API Secret",
                type: "string",
                required: true,
                default_value: ()
            },
            #{name: "delay",
                key: "delay",
                description: "Delay between requests in seconds",
                type: "integer",
                default_value: 3,
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
        "Accept-Language": "en-US,en;q=0.9",
        "Connection": "keep-alive"
    };
    
    // Add API auth header if provided
    if params.api_id != () && params.api_secret != () {
        // Basic auth credentials need to be base64 encoded in the format "username:password"
        let auth_string = `${params.api_id}:${params.api_secret}`;
        let encoded_auth = std::base64_encode(auth_string);
        headers["Authorization"] = `Basic ${encoded_auth}`;
    }
    
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
    
    // Parse the JSON response
    let json_data = json_parse(response_body);
    if json_data == () {
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
    
    // Also try to extract subdomains from the certificates in the result
    if json_data.result != () && json_data.result.hits != () {
        for hit in json_data.result.hits {
            if hit.names != () {
                for name in hit.names {
                    let domain_name = name.to_lower();
                    if domain_name.ends_with(root_domain) && domain_name != root_domain {
                        if !subdomains.contains(domain_name) {
                            subdomains.push(domain_name);
                        }
                    }
                }
            }
        }
    }
    
    return subdomains;
}

/**
 * Main function to search Censys API for subdomains
 */
fn search_censys(domain, api_id, api_secret, delay, timeout, proxy_url) {
    let all_subdomains = [];
    let cursor = ();
    
    // Base API URL
    let api_url = "https://search.censys.io/api/v2/certificates/search";
    
    // Continue querying until all results are fetched
    while true {
        // Sleep to avoid rate limiting (except for the first request)
        if cursor != () {
            sleep(delay);
        }
        
        // Prepare query parameters
        let params = #{
            "q": `names: ${domain}`,
            "per_page": "100"
        };
        
        // Add cursor if we have one (for pagination)
        if cursor != () {
            params.cursor = cursor;
        }
        
        // Prepare request parameters
        let request_params = #{
            url: api_url,
            method: "GET",
            params: params,
            api_id: api_id,
            api_secret: api_secret,
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
        
        // Add new subdomains to the result
        for subdomain in current_subdomains {
            if !all_subdomains.contains(subdomain) {
                all_subdomains.push(subdomain);
            }
        }
        
        // Parse the response to get the next cursor
        let json_data = json_parse(response.body);
        
        if json_data == () != () && json_data.status != "OK" {
            break;
        }
        
        // Check if we have a next page cursor
        if json_data.result != () && 
           json_data.result.links != () && 
           json_data.result.links.next != () {
            cursor = json_data.result.links.next;
        } else {
            break; // No more pages
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
    
    if parameters.api_id == () {
        return #{
            error: "Missing required parameter: api_id",
            result: []
        };
    }
    
    if parameters.api_secret == () {
        return #{
            error: "Missing required parameter: api_secret",
            result: []
        };
    }
    
    // Extract parameters
    let domain = parameters.domain;
    let api_id = parameters.api_id;
    let api_secret = parameters.api_secret;
    let delay = 3;
    let timeout = 10;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    if parameters.delay != () {
        delay = parameters.delay;
    }

    let proxy_url = parameters.proxy_url;
    
    // Search for subdomains
    let search_result = search_censys(domain, api_id, api_secret, delay, timeout, proxy_url);
    
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