/**
 * FOFA API Search Domain Discovery Plugin
 * 
 * This plugin uses the FOFA API to discover subdomains of a given domain.
 * It requires an API email and key to access the service.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "FofaApiSearch",
        description: "Searches for subdomains using FOFA API",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://fofa.info/",
        params: [
            #{name: "domain",
                key: "domain",
                description: "Target domain to search for subdomains",
                type: "string",
                placeholder: "example.com",
                required: true
            },
            #{name: "email",
                key: "email",
                description: "FOFA API Email",
                type: "string",
                placeholder: "your-email@example.com",
                required: true
            },
            #{name: "api_key",
                key: "api_key",
                description: "FOFA API Key",
                type: "string",
                placeholder: "your-api-key",
                required: true
            },
            #{name: "max_results",
                key: "max_results",
                description: "Maximum number of results to return",
                type: "integer",
                default_value: 10000,
                required: false
            },
            #{name: "delay",
                key: "delay",
                description: "Delay between requests in seconds",
                type: "integer",
                default_value: 1,
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
    
    
    // Send the request
    let response = json_parse(http_request(request_params.to_json()));
    // print_info(`response:${response}`);


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



fn search_fofa(domain, email, api_key, max_results, timeout, proxy_url) {
    let all_subdomains = [];
    let page_num = 1;
    let size_per_page = min(1000, max_results);
    
    let query_string = `domain="${domain}"`;
    let query_data = base64_encode(query_string);
    
    while (size_per_page * page_num <= max_results) {
        if page_num > 1 {
            sleep(1);
        }
        
        // Prepare query parameters
        let params = #{
            "email": email,
            "key": api_key,
            "qbase64": query_data,
            "page": ""+page_num,
            "full": "true",
            "size": ""+size_per_page
        };
        
        // API URL
        let url = "https://fofa.info/api/v1/search/all";
        
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
        // print_info(request_params.to_json());
        // Send the request
        let response = send_http_request(request_params);
        
        // Check status code
        if response.status_code != 200 {
            return #{
                error: `Unexpected status code: ${response.status_code}`,
                subdomains: all_subdomains
            };
        }
        
        // Extract subdomains
        let json_data = json_parse(response.body);
        
        for arr in json_data.results {
            let subdomain = arr[0];

            subdomain.replace("http://", "");
            subdomain.replace("https://", "");
            if subdomain.contains(":") {
                subdomain = subdomain.split(":")[0];
            }
            all_subdomains.push(subdomain);
        }
    
        
        // Check if we've reached the end of results
        let json_data = json_parse(response.body);
        if json_data != () && json_data.size != () {
            let size = json_data.size;
            if size < size_per_page {
                break; // No more results
            }
        } else {
            break;
        }
        
        // Update page number for the next request
        page_num += 1;
    }
    
    return #{
        error: (),
        subdomains: all_subdomains
    };
}


fn min(a, b) {
    if a < b {
        return a;
    }
    return b;
}


fn analyze(request_json) {
    // Parse request parameters
    let request = json_parse(request_json);

    
    // Extract parameters
    let domain = request.target;
    let email = "zjgelen@gmail.com";
    let api_key = "f3921c413b0c8de2be67cceb390c8147";
    let max_results = 1000;
    let timeout = 10;
    let proxy_url = "";
    // if request.proxy_url != () {
    //     proxy_url = request.proxy_url;
    // }
    
    // Search for subdomains
    let search_result = search_fofa(domain, email, api_key, max_results, timeout, proxy_url);
    
    if search_result.error != (){
        return #{
            success: false,
            message: `搜索失败,报错：${search_result.error}`,
        };
    }else{
        let all_subdomains = search_result.subdomains;
        return #{
            success: all_subdomains.len() > 0,
            message: `FofaSearch搜索完成，发现 ${all_subdomains.len()} 个子域名`,
            data: #{
                source: "FofaSearch",
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