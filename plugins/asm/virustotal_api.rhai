/**
 * VirusTotal API Domain Search Plugin
 * 
 * This plugin queries the VirusTotal API v3 for a given domain using an API key
 * and extracts subdomains from the results.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "VirusTotalAPIDomainSearch",
        description: "Searches for subdomains using VirusTotal API v3 with API key",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://developers.virustotal.com/reference/subdomains",
        params: [
            #{name: "domain",
                key: "domain",
                description: "Target domain to scan",
                type: "string",
                placeholder: "example.com",
                required: true
            },
            #{name: "api_key",
                key: "api_key",
                description: "VirusTotal API Key",
                type: "string",
                required: true,
                default_value: ()
            },
            #{name: "limit",
                key: "limit",
                description: "Number of results per page",
                type: "integer",
                default_value: 40,
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
    
    // Add query parameters if provided
    if params.params != () {
        request_params.params = params.params;
    }
    
    // Send the request
    let response_str = http_request(request_params.to_json());
    let response = json_parse(response_str);

    // Check for errors
    if response.status_code != 200 {
        return #{
            error: response.error,
            subdomains: []
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
 * Extracts subdomains from VirusTotal API response
 */
 fn extract_subdomains(json_data, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();

    // Process each certificate entry
    for entry in json_data.data {
        if entry.id != () {
            let subdomain = entry.id.to_lower();
            if subdomain.ends_with(root_domain) && subdomain != root_domain {
                subdomains.push(subdomain);
            }
        }
    }

    return subdomains;
}
/**
 * Main function to query VirusTotal API for subdomains
 */
fn query_virustotal_api(domain, api_key, limit, timeout, proxy_url) {
    let all_subdomains = [];
    let url = `https://www.virustotal.com/api/v3/domains/${domain}/subdomains`;
    let next_cursor = "";
    // Continue querying while there's a next cursor
    while (true) {
        // Prepare query parameters
        let params = #{
            "limit": limit.to_string()
        };
        
        if next_cursor!= "" {
            params.cursor = next_cursor;
        }
        
        let headers = #{
            "x-apikey": api_key
        };
        
        // Prepare request parameters
        let request_params = #{
            url: url,
            method: "GET",
            headers: headers,
            params: params,
            timeout: timeout
        };
        

        
        let response = send_http_request(request_params);

        if response.status_code != 200 {
            return #{
                error: `Unexpected status code: ${response.status_code}`,
                subdomains: all_subdomains
            };
        }
        
        let json_data = json_parse(response.body);

        // Extract subdomains
        let subdomains = extract_subdomains(json_data, domain);

        for subdomain in subdomains {
            all_subdomains.push(subdomain);
        }
        // print_info(`status_code: ${response.status_code}`);

        next_cursor = json_data.meta.cursor;
        // print_info(`next_cursor: ${next_cursor}`);
        // Break if no more results (no cursor)
        if next_cursor == () {
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
    let parameters =  json_parse(request_json);
    
    // Validate required parameters
    if parameters.target == () {
        return #{
            error: "Missing required parameter: domain",
            result: []
        };
    }
    // print_info(`parameters: ${parameters}`);
    
    // Extract parameters
    let domain = parameters.target;
    let api_key = "209e09bd70fb39a37987218def266691e0bb834a91b26d56039eed82c02b2e71";
    let limit = 40;
    if parameters.limit != () {
        limit = parameters.limit;
    }
    let timeout = 10;
    if parameters.timeout != () {
        timeout = parameters.timeout;
    }

    
    // Query VirusTotal API for subdomains
    let result = query_virustotal_api(domain, api_key, limit, timeout, "");
    
    // Check for errors
    if result.error != () {
        return #{
            error: result.error,
            result: []
        };
    }
    
    let all_subdomains = result.subdomains;

    return #{
        success: all_subdomains.len() > 0,
        message: `VirusTotalAPI搜索完成,发现 ${all_subdomains.len()} 个子域名`,
        data: #{
            source: "VirusTotalAPI",
            target_domain: domain,
            found_count: all_subdomains.len(),
        },
        raw_output: json_stringify(#{
            found_domains: all_subdomains
        }),
        found_domains: all_subdomains
    };
} 