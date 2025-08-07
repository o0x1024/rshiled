/**
 * AlienVault OTX Domain Search Plugin
 * 
 * This plugin queries the AlienVault Open Threat Exchange (OTX) API for a given domain
 * and extracts subdomains from passive DNS and URL list endpoints.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "AlienVaultDomainSearch",
        description: "Searches for subdomains using AlienVault OTX",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://otx.alienvault.com/",
        params: [
            #{name: "domain",
                key: "domain",
                type: "string",
                description: "Target domain to scan",
                required: true,
                default_value: ()
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
        "Accept": "application/json, text/plain, */*",
        "Accept-Language": "en-US,en;q=0.9"
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
 * Extracts subdomains from AlienVault passive DNS response
 */
fn extract_subdomains_from_passive_dns(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();
    
    // Try to parse as JSON first
    try {
        let json_data = parse_json(response_body);
        
        // Check if 'passive_dns' array exists in the response
        if json_data.passive_dns != () {
            let passive_dns = json_data.passive_dns;
            
            // Iterate through passive_dns entries
            for entry in passive_dns {
                if entry.hostname != () {
                    let hostname = entry.hostname.to_lower();
                    if hostname.ends_with(root_domain) && hostname != root_domain {
                        if !subdomains.contains(hostname) {
                            subdomains.push(hostname);
                        }
                    }
                }
            }
        }
    } catch(error) {
        print(`Error parsing JSON (passive DNS): ${error}`);
    }
    
    return subdomains;
}

/**
 * Extracts subdomains from AlienVault URL list response
 */
fn extract_subdomains_from_url_list(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();
    
    // Try to parse as JSON first
    try {
        let json_data = parse_json(response_body);
        
        // Check if 'url_list' array exists in the response
        if json_data.url_list != () {
            let url_list = json_data.url_list;
            
            // Iterate through URL list entries
            for entry in url_list {
                if entry.hostname != () {
                    let hostname = entry.hostname.to_lower();
                    if hostname.ends_with(root_domain) && hostname != root_domain {
                        if !subdomains.contains(hostname) {
                            subdomains.push(hostname);
                        }
                    }
                }
            }
        }
    } catch(error) {
        print(`Error parsing JSON (URL list): ${error}`);
    }
    
    return subdomains;
}

/**
 * Main function to query AlienVault OTX for subdomains
 */
fn query_alienvault_otx(domain, timeout, proxy_url) {
    let base_url = "https://otx.alienvault.com/api/v1/indicators/domain";
    let passive_dns_url = `${base_url}/${domain}/passive_dns`;
    let url_list_url = `${base_url}/${domain}/url_list`;
    let subdomains = [];
    
    // Query passive DNS endpoint
    let dns_request_params = #{
        url: passive_dns_url,
        method: "GET",
        timeout: timeout
    };
    
    // Add proxy if provided
    if proxy_url != () {
        dns_request_params.proxy_url = proxy_url;
    }
    
    // Send passive DNS request
    let dns_response = send_http_request(dns_request_params);
    
    // Check for errors
    if dns_response.error != () {
        print(`Error querying passive DNS: ${dns_response.error}`);
    } else if dns_response.status_code == 200 {
        // Extract subdomains from passive DNS response
        let dns_subdomains = extract_subdomains_from_passive_dns(dns_response.body, domain);
        
        // Add unique subdomains
        for subdomain in dns_subdomains {
            if !subdomains.contains(subdomain) {
                subdomains.push(subdomain);
            }
        }
    } else {
        print(`Unexpected status code for passive DNS: ${dns_response.status_code}`);
    }
    
    // Query URL list endpoint
    let url_request_params = #{
        url: url_list_url,
        method: "GET",
        timeout: timeout
    };
    
    // Add proxy if provided
    if proxy_url != () {
        url_request_params.proxy_url = proxy_url;
    }
    
    // Send URL list request
    let url_response = send_http_request(url_request_params);
    
    // Check for errors
    if url_response.error != () {
        print(`Error querying URL list: ${url_response.error}`);
    } else if url_response.status_code == 200 {
        // Extract subdomains from URL list response
        let url_subdomains = extract_subdomains_from_url_list(url_response.body, domain);
        
        // Add unique subdomains
        for subdomain in url_subdomains {
            if !subdomains.contains(subdomain) {
                subdomains.push(subdomain);
            }
        }
    } else {
        print(`Unexpected status code for URL list: ${url_response.status_code}`);
    }
    
    return subdomains
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
    let timeout = 10;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }
    let proxy_url = parameters.proxy_url;
    
    // Query AlienVault OTX for subdomains
    let subdomains = query_alienvault_otx(domain, timeout, proxy_url);
    

        // 准备结果
    let result = #{
        success: subdomains.len() > 0,
        message: `百度搜索完成，发现 ${subdomains.len()} 个子域名`,
        data: #{
            search_engine: "Baidu",
            target_domain: target,
            found_count: subdomains.len(),
            pages_searched: max_pages
        },
        raw_output: json_stringify(#{
            searched_pages: max_pages,
               found_domains: subdomains
        }),
        found_domains: subdomains
    };
        
    return json_stringify(result);
        
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