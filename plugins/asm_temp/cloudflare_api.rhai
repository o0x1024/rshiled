/**
 * CloudFlare API Domain Discovery Plugin
 * 
 * This plugin uses the CloudFlare API to discover subdomains of a given domain.
 * It requires an API token to access the service and will attempt to create a zone
 * for the domain if it doesn't exist, then enumerate the DNS records.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "CloudFlareApiSearch",
        description: "Searches for subdomains using CloudFlare API",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://api.cloudflare.com/",
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
                description: "CloudFlare API Token with DNS read permissions",
                type: "string",
                placeholder: "your-api-token",
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
        "Accept": "application/json",
        "Accept-Language": "en-US,en;q=0.9",
        "Connection": "keep-alive",
        "Content-Type": "application/json"
    };
    
    // Add API token header if provided
    if params.api_token != () {
        headers["Authorization"] = `Bearer ${params.api_token}`;
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
 * Extracts subdomains from search results
 */
fn extract_subdomains(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();
    
    if response_body.trim()== "" {
        return subdomains;
    }
    
    // Try to parse the JSON response
    let json_data = json_parse(response_body);
    if json_data != () && json_data.result != () {
        // Extract subdomains from DNS records
        for record in json_data.result {
            if record.name != () {
                let domain_name = record.name.to_lower();
                // Check if it's a subdomain of our target domain
                if domain_name.ends_with(root_domain) && domain_name != root_domain {
                    if !subdomains.contains(domain_name) {
                        subdomains.push(domain_name);
                    }
                }
            }
        }
    }
    
    // If no records were found or parsing failed, try regex extraction
    if subdomains.len() == 0 {
        // Fallback to regex extraction
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
    }
    
    return subdomains;
}

/**
 * Sleep function to wait for CloudFlare to enumerate subdomains
 */
fn sleep(seconds) {
    // Simulate sleep by using a busy wait
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
 * Get account ID from CloudFlare API
 */
fn get_account_id(api_token, base_url, timeout, proxy_url) {
    // Prepare request parameters
    let request_params = #{
        url: `${base_url}accounts`,
        method: "GET",
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
            account_id: ()
        };
    }
    
    // Check status code
    if response.status_code != 200 {
        return #{
            error: `Unexpected status code: ${response.status_code}`,
            account_id: ()
        };
    }
    
    // Parse the response
    let json_data = json_parse(response.body);
    if json_data == () != () && json_data.result == () != () && json_data.result.len() == 0 {
        return #{
            error: "No accounts found in the response",
            account_id: ()
        };
    }
    
    // Return the account ID
    return #{
        error: (),
        account_id: json_data.result[0].id
    };
}

/**
 * Get zone ID from CloudFlare API or create a new zone
 */
fn get_or_create_zone(domain, account_id, api_token, base_url, timeout, proxy_url) {
    // First, try to get the zone
    let request_params = #{
        url: `${base_url}zones`,
        method: "GET",
        api_token: api_token,
        params: #{
            "name": domain
        },
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
            zone_id: ()
        };
    }
    
    // Check status code
    if response.status_code == 403 {
        return #{
            error: `Domain ${domain} is banned or not a registered domain, so cannot be added to Cloudflare.`,
            zone_id: ()
        };
    }
    
    if response.status_code != 200 {
        return #{
            error: `Unexpected status code: ${response.status_code}`,
            zone_id: ()
        };
    }
    
    // Parse the response
    let json_data = json_parse(response.body);
    if json_data == () != () && !json_data.success {
        return #{
            error: "Failed to get zone information",
            zone_id: ()
        };
    }
    
    // Check if the zone exists
    if json_data.result != () && json_data.result.len() > 0 {
        // Zone exists, return its ID
        return #{
            error: (),
            zone_id: json_data.result[0].id
        };
    }
    
    // Zone doesn't exist, create it
    return create_zone(domain, account_id, api_token, base_url, timeout, proxy_url);
}

/**
 * Create a new zone in CloudFlare
 */
fn create_zone(domain, account_id, api_token, base_url, timeout, proxy_url) {
    // Prepare the JSON data for creating a zone
    let json_data = #{
        "name": domain,
        "account": #{
            "id": account_id
        },
        "jump_start": true,
        "type": "full"
    };
    
    // Prepare request parameters
    let request_params = #{
        url: `${base_url}zones`,
        method: "POST",
        api_token: api_token,
        json: json_data,
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
            zone_id: ()
        };
    }
    
    // Check status code
    if response.status_code != 200 {
        return #{
            error: `Unexpected status code: ${response.status_code}`,
            zone_id: ()
        };
    }
    
    // Parse the response
    let json_data = json_parse(response.body);
    if json_data == () != () && !json_data.success {
        return #{
            error: `Domain ${domain} is temporarily banned and cannot be added to Cloudflare`,
            zone_id: ()
        };
    }
    
    // Return the zone ID
    return #{
        error: (),
        zone_id: json_data.result.id
    };
}

/**
 * List DNS records for a zone
 */
fn list_dns_records(zone_id, api_token, base_url, timeout, proxy_url, domain) {
    let all_subdomains = [];
    let page = 1;
    let per_page = 100; // Maximum allowed by CloudFlare API
    
    while true {
        // Prepare request parameters
        let request_params = #{
            url: `${base_url}zones/${zone_id}/dns_records`,
            method: "GET",
            api_token: api_token,
            params: #{
                "page": page,
                "per_page": per_page
            },
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
        
        // Check if we need to continue pagination
        let json_data = json_parse(response.body);
        if json_data != () && json_data.result_info != () {
            let total_pages = json_data.result_info.total_pages;
            
            if all_subdomains.len() == 0 && page == 1 {
                // If no subdomains were found on the first page, wait for CloudFlare to enumerate subdomains
                sleep(5);
                continue; // Retry the same page
            }
            
            if page >= total_pages {
                break; // No more pages
            }
            
            // Update page number for the next request
            page += 1;
        } else {
            // Can't determine if there are more pages, so stop
            break;
        }
    }
    
    return #{
        error: (),
        subdomains: all_subdomains
    };
}

/**
 * Main function to search CloudFlare API for subdomains
 */
fn search_cloudflare(domain, api_token, timeout, proxy_url) {
    let base_url = "https://api.cloudflare.com/client/v4/";
    
    // Step 1: Get account ID
    let account_result = get_account_id(api_token, base_url, timeout, proxy_url);
    if account_result.error != () {
        return #{
            error: account_result.error,
            subdomains: []
        };
    }
    
    let account_id = account_result.account_id;
    
    // Step 2: Get or create zone
    let zone_result = get_or_create_zone(domain, account_id, api_token, base_url, timeout, proxy_url);
    if zone_result.error != () {
        return #{
            error: zone_result.error,
            subdomains: []
        };
    }
    
    let zone_id = zone_result.zone_id;
    
    // Step 3: List DNS records
    let dns_result = list_dns_records(zone_id, api_token, base_url, timeout, proxy_url, domain);
    if dns_result.error != () {
        return #{
            error: dns_result.error,
            subdomains: []
        };
    }
    
    return #{
        error: (),
        subdomains: dns_result.subdomains
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
    let timeout = 30;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = parameters.proxy_url;
    
    // Search for subdomains
    let search_result = search_cloudflare(domain, api_token, timeout, proxy_url);
    
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