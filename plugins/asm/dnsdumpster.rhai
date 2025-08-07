// DNSDumpsterDomainSearch - domain_discovery插件
// 描述: Searches for subdomains using DNSDumpster
// 作者: RShield Team
// 版本: 1.0.0

fn get_manifest() {
    let manifest = #{
        name: "DNSDumpsterDomainSearch",
        description: "Searches for subdomains using DNSDumpster",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://dnsdumpster.com/ ",
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
    
    let timeout = 10;
    if params.timeout != 0 {
        timeout = params.timeout;
    }
    let method = "GET";
    if params.method != () {
        method = params.method;
    }
    
    // Prepare request parameters
    let request_params = #{
        url: params.url,
        method: method,
        headers: headers,
        timeout: timeout
    };
    
    // Add body data if provided (for POST requests)
    if params.data != () {
        request_params.body = params.data;
    }
    
    
    // Send the request
    let response_str = http_request(request_params.to_json());
    let response = json_parse(response_str);

    // print_debug(`response: ${response}`);

    if response.status_code == 200 {
        return #{
            status_code: response.status_code,
            body: response.body,
            headers: response.headers,
            cookies: response.cookies
        };
    }
}

/**
 * Extracts CSRF token from cookies
 */
fn extract_Authorization(body) {
    let authorization = regex_matches(body, `eyJ[A-Za-z0-9_/+\-]{10,}={0,2}\.[A-Za-z0-9_/+\-\\]{15,}={0,2}\.[A-Za-z0-9_/+\-\\]{10,}={0,2}`);
    // print_debug(`authorization: ${authorization}`);
    if authorization.len() > 0 {
        return authorization[0];
    }
    return "";
}

/**
 * Extracts subdomains from HTML response
 */
fn extract_subdomains(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();
    
    // Convert HTML to lowercase for easier matching
    let html_lower = response_body.to_lower();
    
    // Simple pattern matching for subdomains in the HTML
    // Look for domain patterns in the HTML
    let pattern = `target=((?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?.)*${root_domain})`;
    // print_debug(`pattern: ${pattern}`);
    // Use regex to find all matches
    let matches = regex_matches(html_lower, pattern);

    // Process matches and filter duplicates
    if matches.len() > 0 {
        for matched in matches {
            let subdomain = matched.to_lower();
            if subdomain.ends_with(root_domain) && subdomain != root_domain {
                if !subdomains.contains(subdomain) {
                    // print_debug(`subdomain: ${subdomain}`);
                    // print_info(`type_of(subdomain): ${type_of(subdomain)}`);
                    subdomain.trim();
                    subdomain.replace("target=", "");
                    // print_debug(`subdomain: ${subdomain}`);
                    subdomains.push(subdomain);
                }
            }
        }
    }
    
    return subdomains;
}

/**
 * Main function to query DNSDumpster for subdomains
 */
fn query_dnsdumpster(domain, timeout, proxy_url) {
    let url = "https://dnsdumpster.com/";
    
    // First request to get CSRF token
    let first_request_params = #{
        url: url,
        method: "GET",
        timeout: timeout
    };
    
    // Add proxy if provided
    if proxy_url != () {
        first_request_params.proxy_url = proxy_url;
    }

    // Send initial request to get CSRF token
    let first_response = send_http_request(first_request_params);

    // Check for errors
    if first_response.status_code != 200 {
        return #{
            error: first_response.error,
            subdomains: []
        };
    }

    
    // Extract CSRF token from cookies
    let Authorization = extract_Authorization(first_response.body);
    if Authorization== "" {
        return #{
            error: "Failed to extract CSRF token",
            subdomains: []
        };
    }
    // print_info(`Authorization: ${Authorization}`);
    // Prepare POST data
    let post_data = "target=" + domain;
    
    // Add Referer header
    let headers = #{
        "Referer": "https://dnsdumpster.com",
        "Content-Type": "application/x-www-form-urlencoded",
        "Authorization": Authorization
    };
    
    // Prepare second request parameters
    let second_request_params = #{
        url: "https://api.dnsdumpster.com/htmld/",
        method: "POST",
        headers: headers,
        data: post_data,
        timeout: timeout
    };
    // Send second request with POST data
    let second_response = send_http_request(second_request_params);
    // print_info(`second_response: ${second_response}`);
    // Check status code
    if second_response!= () && second_response.status_code != 200 {
        return #{
            error: `Unexpected status code in second request: ${second_response.status_code}`,
            subdomains: []
        };
    }
    
    // Extract subdomains from response
    let subdomains = extract_subdomains(second_response.body, domain);
    // print_debug(`subdomains: ${subdomains}`);
    return #{
        error: (),
        subdomains: subdomains
    };
}

/**
 * Main function to process the request and return results
 */
fn analyze(request_json) {
    let request = json_parse(request_json);

    // Validate required parameters
    if request.target == () {
        return #{
            error: "Missing required parameter: domain",
            result: []
        };
    };
    
    let domain = request.target;
    let timeout = 10;
    if request.timeout != () {
        timeout = request.timeout;
    }
    let proxy_url = "";
    

    let result = query_dnsdumpster(domain, timeout, proxy_url);

    let all_subdomains = result.subdomains;
    
    if result.error != () {
        return #{
            success: false,
            message: result.error,
            data: ()
        };
    }else{
        return #{
            success: all_subdomains.len() > 0,
            message: `DNSDumpster搜索完成，发现 ${all_subdomains.len()} 个子域名`,
            data: #{
            source: "DNSDumpster",
            target_domain: domain,
            found_count: all_subdomains.len(),
            },
            raw_output: json_stringify(#{
                found_domains: all_subdomains
            }),
            found_domains: all_subdomains
        };
    }
} 