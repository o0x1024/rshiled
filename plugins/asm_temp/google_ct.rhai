/**
 * Google Certificate Transparency Domain Discovery Plugin
 * 
 * This plugin uses Google's Certificate Transparency logs to discover subdomains of a given domain.
 * It searches through certificate transparency logs to find certificates issued for the domain and its subdomains.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "GoogleCTSearch",
        description: "Searches for subdomains using Google's Certificate Transparency logs",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://transparencyreport.google.com/https/certificates",
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
                default_value: 20,
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
    
    let method = "GET";
    if params.method != () {
        method = params.method;
    }
    let timeout = 20;
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
    
    // Parse the JSON response
    let json_data = json_parse(response_body);
    if json_data == () {
        return subdomains;
    }
    
    // Google CT API specific response parsing
    if json_data.certificates != () {
        for cert in json_data.certificates {
            if cert.subject_dns_names != () {
                for dns_name in cert.subject_dns_names {
                    let name = dns_name.to_lower();
                    
                    // Check if it's a subdomain of our target domain
                    if name.ends_with(root_domain) && name != root_domain {
                        if !subdomains.contains(name) {
                            subdomains.push(name);
                        }
                    }
                }
            }
            
            // Also check alternative names
            if cert.subject_alternative_names != () {
                for alt_name in cert.subject_alternative_names {
                    let name = alt_name.to_lower();
                    
                    // Check if it's a subdomain of our target domain
                    if name.ends_with(root_domain) && name != root_domain {
                        if !subdomains.contains(name) {
                            subdomains.push(name);
                        }
                    }
                }
            }
        }
    }
    
    // Fallback to regex extraction if JSON parsing doesn't yield results
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
 * Main function to search Google Certificate Transparency logs for subdomains
 */
fn search_google_ct(domain, timeout, proxy_url) {
    // Use the public Google CT API endpoint
    let api_url = "https://www.gstatic.com/ct/api/v1/getctlog";
    
    // Prepare query parameters
    let params = #{
        "domain": domain
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
    
    // Try an alternative approach using Google's CT search
    if subdomains.len() == 0 {
        // Fall back to using the transparency report API
        let alt_url = "https://transparencyreport.google.com/transparencyreport/api/v3/httpsreport/ct/certsearch";
        let alt_params = #{
            "include_subdomains": "true",
            "domain": domain
        };
        
        let alt_request_params = #{
            url: alt_url,
            method: "GET",
            params: alt_params,
            timeout: timeout
        };
        
        if proxy_url != () {
            alt_request_params.proxy_url = proxy_url;
        }
        
        let alt_response = send_http_request(alt_request_params);
        
        if alt_response.error == () && alt_response.status_code == 200 {
            let alt_subdomains = extract_subdomains(alt_response.body, domain);
            for subdomain in alt_subdomains {
                if !subdomains.contains(subdomain) {
                    subdomains.push(subdomain);
                }
            }
        }
    }
    
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
    
    // Extract parameters
    let domain = parameters.domain;
    let timeout = 20;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = parameters.proxy_url;
    
    // Search for subdomains
    let search_result = search_google_ct(domain, timeout, proxy_url);
    
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