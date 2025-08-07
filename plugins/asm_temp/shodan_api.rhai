/**
 * Shodan API Search Domain Discovery Plugin
 * 
 * This plugin uses the Shodan API to discover subdomains of a given domain.
 * It requires an API key to access the service.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "ShodanApiSearch",
        description: "Searches for subdomains using Shodan API",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://shodan.io/",
        params: [
            #{
                name: "domain",
                key: "domain",
                type: "string",
                description: "Target domain to search for subdomains",
                required: true,
                default_value: ()
            },
            #{
                name: "api_key",
                key: "api_key",
                type: "string",
                description: "Shodan API Key",
                required: true,
                default_value: ()
            },
            #{
                name: "timeout",
                key: "timeout",
                type: "integer",
                description: "Request timeout in seconds",
                default_value: 10,
                required: false
            },
            #{
                name: "proxy_url",
                key: "proxy_url",
                type: "string",
                description: "Proxy URL to use for requests",
                default_value: (),
                required: false
            }],
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
    
    // Parse the JSON response
    let json_data = json_parse(response_body);
    if json_data == () {
        return subdomains;
    }
    
    // Extract subdomain names from the response
    if json_data.subdomains != () {
        // Shodan returns subdomain prefixes without the base domain
        // We need to append the base domain to each prefix
        for name in json_data.subdomains {
            let full_subdomain = `${name}.${root_domain}`.to_lower();
            if !subdomains.contains(full_subdomain) {
                subdomains.push(full_subdomain);
            }
        }
    }
    
    // Additionally, try to extract domains from the raw response using regex
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
 * Main function to search Shodan API for subdomains
 */
fn search_shodan(domain, api_key, timeout, proxy_url) {
    // Prepare the API URL
    let url = `https://api.shodan.io/dns/domain/${domain}?key=${api_key}`;
    
    // Prepare request parameters
    let request_params = #{
        url: url,
        method: "GET",
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
    
    if parameters.api_key == () {
        return #{
            error: "Missing required parameter: api_key",
            result: []
        };
    }
    
    // Extract parameters
    let domain = parameters.domain;
    let api_key = parameters.api_key;
    
    let timeout = 10;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    
    let proxy_url = parameters.proxy_url;
    
    // Search for subdomains
    let search_result = search_shodan(domain, api_key, timeout, proxy_url);
    
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