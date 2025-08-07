/**
 * GitHub API Search Domain Discovery Plugin
 * 
 * This plugin uses the GitHub API to discover subdomains of a given domain by searching code repositories.
 * It requires an API token to access the service and has rate limiting considerations.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "GithubApiSearch",
        description: "Searches for subdomains using GitHub Code Search API",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://docs.github.com/en/rest/search/search?apiVersion=2022-11-28#search-code",
        params: [
            #{name: "domain",
                key: "domain",
                description: "Target domain to search for subdomains",
                type: "string",
                placeholder: "example.com",
                required: true
            },
            #{name: "api_token",
                key: "api_token",
                description: "GitHub API Token",
                type: "string",
                placeholder: "your-api-token",
                required: true
            },
            #{name: "max_pages",
                key: "max_pages",
                description: "Maximum number of pages to request (100 results per page, GitHub limits to 1000 results)",
                type: "integer",
                default_value: 10,
                required: false
            },
            #{name: "delay",
                key: "delay",
                description: "Delay between requests in seconds (recommended at least 5 due to GitHub rate limiting)",
                type: "integer",
                default_value: 5,
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
        "Accept": "application/vnd.github.v3.text-match+json",
        "Accept-Language": "en-US,en;q=0.9",
        "Connection": "keep-alive"
    };
    
    // Add API token header
    if params.api_token != () {
        headers["Authorization"] = `token ${params.api_token}`;
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
    
    // Extract from text matches if available
    if json_data.items != () {
        for item in json_data.items {
            // Check in text matches (most accurate, as GitHub API returns matched context)
            if item.text_matches != () {
                for match_info in item.text_matches {
                    if match_info.fragment != () {
                        // Extract subdomains from the matched fragment
                        let matches = regex_matches(match_info.fragment, pattern);
                        
                        for matched in matches {
                            let subdomain = matched.to_lower();
                            if subdomain.ends_with(root_domain) && subdomain != root_domain {
                                if !subdomains.contains(subdomain) {
                                    subdomains.push(subdomain);
                                }
                            }
                        }
                    }
                }
            }
            
            // Check in the name and path as fallback
            if item.name != () {
                let matches = regex_matches(item.name, pattern);
                for matched in matches {
                    let subdomain = matched.to_lower();
                    if subdomain.ends_with(root_domain) && subdomain != root_domain {
                        if !subdomains.contains(subdomain) {
                            subdomains.push(subdomain);
                        }
                    }
                }
            }
            
            if item.path != () {
                let matches = regex_matches(item.path, pattern);
                for matched in matches {
                    let subdomain = matched.to_lower();
                    if subdomain.ends_with(root_domain) && subdomain != root_domain {
                        if !subdomains.contains(subdomain) {
                            subdomains.push(subdomain);
                        }
                    }
                }
            }
        }
    }
    
    // Also try to match subdomains from the raw response
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
 * Main function to search GitHub API for subdomains
 */
fn search_github(domain, api_token, max_pages, delay, timeout, proxy_url) {
    let all_subdomains = [];
    let page = 1;
    
    // Continue querying until all results are fetched or limit is reached
    while (page <= max_pages) {
        // Sleep to avoid rate limiting - GitHub has strict rate limits
        if page > 1 {
            sleep(delay);
        }
        
        // Prepare query parameters
        let params = #{
            "q": domain,
            "per_page": 100,
            "page": page,
            "sort": "indexed",
            "access_token": api_token
        };
        
        // API URL
        let url = "https://api.github.com/search/code";
        
        // Prepare request parameters
        let request_params = #{
            url: url,
            method: "GET",
            params: params,
            api_token: api_token,
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
        
        // Check if we've reached the end of results or GitHub limit
        let json_data = json_parse(response.body);
        if json_data != () && json_data.total_count != () {
            let total_count = json_data.total_count;
            if page * 100 >= total_count != () && page * 100 >= 1000 {
                // GitHub limits to 1000 results total (10 pages at 100 items per page)
                break;
            }
        } else {
            break;
        }
        
        // Update page number for the next request
        page += 1;
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
    
    if parameters.api_token == () {
        return #{
            error: "Missing required parameter: api_token",
            result: []
        };
    }
    
    // Extract parameters
    let domain = parameters.domain;
    let api_token = parameters.api_token;
    let max_pages = 10;
    let delay = 5;
    let timeout = 30;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let delay = 5;
    if parameters.delay != () {
        delay = parameters.delay;
    }
    let max_pages = 10;
    if parameters.max_pages != () {
        max_pages = parameters.max_pages;
    }
    let proxy_url = "";
    if parameters.proxy_url != () {
        proxy_url = parameters.proxy_url;
    }
    // Search for subdomains
    let search_result = search_github(domain, api_token, max_pages, delay, timeout, proxy_url);
    
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