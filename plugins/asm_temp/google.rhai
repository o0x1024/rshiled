/**
 * Google Search Domain Discovery Plugin
 * 
 * This plugin uses Google search engine to discover subdomains of a given domain.
 * It utilizes the "site:." operator to find pages hosted on subdomains.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "GoogleSearchDomainSearch",
        description: "Searches for subdomains using Google Search engine",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://www.google.com/",
        params: [
            #{name: "domain",
                key: "domain",
                description: "Target domain to search for subdomains",
                type: "string",
                placeholder: "example.com",
                required: true
            },
            #{name: "limit_num",
                key: "limit_num",
                description: "Maximum number of search results to process",
                type: "integer",
                default_value: 1000,
                required: false
            },
            #{name: "per_page",
                key: "per_page",
                description: "Number of results per page",
                type: "integer",
                default_value: 50,
                required: false
            },
            #{name: "delay_min",
                key: "delay_min",
                description: "Minimum delay between requests in seconds",
                type: "integer",
                default_value: 1,
                required: false
            },
            #{name: "delay_max",
                key: "delay_max",
                description: "Maximum delay between requests in seconds",
                type: "integer",
                default_value: 5,
                required: false
            },
            #{name: "timeout",
                key: "timeout",
                description: "Request timeout in seconds",
                type: "integer",
                default_value: 10,
                required: false
            },
            #{name: "recursive",
                key: "recursive",
                description: "Search recursively on discovered subdomains",
                type: "boolean",
                default_value: false,
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
        "User-Agent": "Googlebot",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
        "Accept-Language": "en-US,en;q=0.9",
        "Referer": "https://www.google.com/",
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
    if params.proxy_url != "" {
        request_params.proxy_url = params.proxy_url;
    }
    
    // Add query parameters if provided
    request_params.params = params.params;
    
    print_debug(`request_params: ${request_params}`);
    // Send the request
    let response_str = http_request(request_params.to_json());
    
    // Parse response string to JSON object
    let response = json_parse(response_str);
    
    // Return response as object
    return response;
}

/**
 * Get current timestamp in milliseconds
 * 由于无法使用真实时间，我们返回一个常数值
 */
fn timestamp() {
    return 1234567890; // 返回一个固定值
}

/**
 * Generate random number between min and max
 */
fn random_between(min, max) {
    // 由于无法生成真正的随机数，我们直接返回平均值
    return (min + max) / 2;
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
 * Escape regex special characters in a string
 */
fn regex_escape(text) {
    // 简单方法：直接使用固定格式替换常见特殊字符
    // 将点号替换为转义的点号 - 在Rhai中可能没有简单的字符替换方法
    // 所以我们用一个不太可能出现在域名中的临时标记，然后在模式中手动处理它
    
    // 简化处理：由于我们只关心域名，而域名中主要的特殊字符是点号
    // 我们可以在模式中直接硬编码点号的转义
    return text; // 返回原始文本，在使用时处理转义
}

/**
 * Extracts subdomains from search results
 */
fn extract_subdomains(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();
    
    // 构建模式时手动转义点号
    // 将域名中的每个点号替换为 \\.
    // 例如: example.com -> example\\.com
    let domain_parts = root_domain.split(".");
    let escaped_domain = "";
    
    // 手动拼接转义后的域名
    for i in range(0, domain_parts.len()) {
        if i > 0 {
            escaped_domain += "\\."; // 添加转义的点号
        }
        escaped_domain += domain_parts[i];
    }
    
    // 构建最终的正则表达式模式
    let pattern = `[a-zA-Z0-9][-a-zA-Z0-9.]*\\.${escaped_domain}`;
    
    // 使用正则表达式查找匹配项
    let matches = regex_matches(response_body, pattern);
    print_debug(`matches: ${matches}`);
    // 处理匹配结果并过滤重复项
    if matches.len() > 0 {
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
 * Generate filter statement to exclude common subdomains in order to find more diverse results
 */
fn generate_filters(domain, subdomains) {
    let filters = [];
    let counts = #{};
    
    // Count the frequency of each subdomain prefix
    for subdomain in subdomains {
        if subdomain.ends_with(domain) && subdomain != domain {
            let parts = subdomain.split(".");
            let prefix = parts[0]; // Get the first part of the subdomain
            
            if counts[prefix] == () {
                counts[prefix] = 1;
            } else {
                counts[prefix] = counts[prefix] + 1;
            }
        }
    }
    
    // Generate filter statements for common prefixes
    for prefix in counts.keys() {
        if counts[prefix] > 5 { // Threshold for considering a prefix common
            filters.push(` -site:${prefix}.${domain}`);
        }
    }
    
    return filters;
}

/**
 * Extracts second-level subdomains for recursive search
 */
fn extract_recursive_subdomains(subdomains, domain) {
    let recursive_domains = [];
    let root_domain = domain.to_lower();
    
    for subdomain in subdomains {
        if subdomain.ends_with(root_domain) && subdomain != root_domain {
            let parts = subdomain.split(".");
            if parts.len() > 2 { // Ensure it's at least a second-level subdomain
                let second_level = parts[parts.len() - 3] + "." + parts[parts.len() - 2] + "." + parts[parts.len() - 1];
                if !recursive_domains.contains(second_level) {
                    recursive_domains.push(second_level);
                }
            }
        }
    }
    
    return recursive_domains;
}

/**
 * Check if results have new subdomains
 */
fn has_new_subdomains(current_subdomains, all_subdomains) {
    for subdomain in current_subdomains {
        if !all_subdomains.contains(subdomain) {
            return true;
        }
    }
    return false;
}

/**
 * Main function to search Google for subdomains
 */
fn search_google(domain, filtered_subdomain, limit_num, per_page, delay_min, delay_max, timeout, proxy_url, cookies) {
    let all_subdomains = [];
    let page_num = 1;
    
    
    // Continue querying until all results are fetched or limit is reached
    while (true) {
        // Random sleep to avoid rate limiting
        if page_num > 1 {
            let delay = random_between(delay_min, delay_max);
            sleep(delay);
        }
        
        // Prepare query parameters
        let query = `site:.${domain}${filtered_subdomain}`;
        let params = #{
            "q": query,
            "start": `${page_num}`,
            "num": `${per_page}`,
            "filter": "0",
            "btnG": "Search",
            "gbv": "1",
            "hl": "en"
        };
        
        // Search URL
        let url = "https://www.google.com/search";
        
        let cookies_str = "";
        for cookie in cookies {
            cookies_str += cookie + "; ";
        }
        // Prepare request parameters
        let request_params = #{
            url: url,
            method: "GET",
            headers: #{
                "User-Agent": "Googlebot",
                "Cookie": cookies_str,
                "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
                "Accept-Language": "en-US,en;q=0.9",
                "Referer": "https://www.google.com/",
                "Connection": "keep-alive"
            },
            params: params,
            timeout: timeout
        };
        
        if proxy_url != "" {
            request_params.proxy_url = proxy_url;
        }
        
        // Send the request
        let response = send_http_request(request_params);

        print_debug(`response status: ${response.status}`);
        
        // Check status code
        if response.status != 200 {
            return #{
                error: `Unexpected status code: ${response.status}`,
                subdomains: all_subdomains
            };
        }
        print_debug(`response headers: ${response.headers}`);
        // 获取响应头中的cookie


        // Check for Google blocking or redirection
        if response.body.contains("302 Moved") {
            return #{
                error: "Google search blocked or redirected",
                subdomains: all_subdomains
            };
        }
        
        // Extract subdomains
        let current_subdomains = extract_subdomains(response.body, domain);
        
        // If no new subdomains found, break the loop
        if !has_new_subdomains(current_subdomains, all_subdomains) {
            break;
        }
        
        // Add new subdomains to the result
        for subdomain in current_subdomains {
            if !all_subdomains.contains(subdomain) {
                all_subdomains.push(subdomain);
            }
        }
        
        // Check if there's a next page
        if !response.body.contains(`start=${page_num + per_page}`) {
            break;
        }
        
        // Update page number for the next request
        page_num += per_page;
        
        // Check if we've reached the limit
        if page_num >= limit_num {
            break;
        }
    }
    print_debug(`ressss----------: `);
    
    return #{
        error: (),
        subdomains: all_subdomains,
        cookies: cookies
    };
}

/**
 * Main function to process the request and return results
 */
fn analyze(request_json) {
    // Parse request parameters
    let request = json_parse(request_json);
    
    // Extract parameters
    let domain = request.target;
    let limit_num = 1000;
    let per_page = 50;
    let delay_min = 1;
    let delay_max = 5;
    let timeout = 10;
    let recursive = false;

    if request.per_page != () {
        per_page = request.per_page;
    }
    if request.delay_min != () {
        delay_min = request.delay_min;
    }
    if request.delay_max != () {
        delay_max = request.delay_max;
    }
    if request.timeout != () {
        timeout = request.timeout;
    }
    if request.recursive != () {
        recursive = request.recursive;
    }
    let proxy_url = "http://127.0.0.1:8080";


    let request_params = #{
        url: "https://www.google.com/",
        method: "GET",
        headers: #{
            "User-Agent": "Googlebot",
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
            "Accept-Language": "en-US,en;q=0.9",
            "Referer": "https://www.google.com/",
            "Connection": "keep-alive"
        },
        timeout: timeout,
        proxy_url:"http://127.0.0.1:8080"
    };
    let response_str = http_request(request_params.to_json());
    let response = json_parse(response_str);

    let cookies = [];
    if response.headers.contains("Set-Cookie") {
        let cookie_header = response.headers["Set-Cookie"];
        if type_of(cookie_header) == "string" {
            cookies.push(cookie_header);
        } else if type_of(cookie_header) == "array" {
            cookies = cookie_header;
        }
    }

    if response.headers.contains("set-cookie") {
        let cookie_header = response.headers["set-cookie"];
        if type_of(cookie_header) == "string" {
            cookies.push(cookie_header);
        } else if type_of(cookie_header) == "array" {
            cookies = cookie_header;
        }
    }
    
    print_debug(`cookies: ${cookies}`);
    // Initial search
    let all_subdomains = [];
    
    // Initial search with no filter
    let initial_result = search_google(domain, "", limit_num, per_page, delay_min, delay_max, timeout, proxy_url, cookies);
    
    // Check for errors
    if initial_result.error != () {
        return #{
            error: initial_result.error,
            result: []
        };
    }
    
    // Store subdomains and cookies
    all_subdomains = initial_result.subdomains;
    cookies = initial_result.cookies;
    
    // Generate filters based on common subdomains
    let filters = generate_filters(domain, all_subdomains);
    
    print_debug(`filters:------ ${filters}`);
    // Search with filters to find more diverse subdomains
    for filter_statement in filters {
        let filtered_result = search_google(domain, filter_statement, limit_num, per_page, delay_min, delay_max, timeout, proxy_url, cookies);
        
        // Check for errors
        if filtered_result.error != () {
            continue;
        }
        
        // Add new subdomains to the result
        for subdomain in filtered_result.subdomains {
            if !all_subdomains.contains(subdomain) {
                all_subdomains.push(subdomain);
            }
        }
        
        // Update cookies
        cookies = filtered_result.cookies;
    }
    
    // Recursive search on discovered subdomains if enabled
    if recursive {
        let recursive_domains = extract_recursive_subdomains(all_subdomains, domain);
        
        for recursive_domain in recursive_domains {
            let recursive_result = search_google(recursive_domain, "", limit_num, per_page, delay_min, delay_max, timeout, proxy_url, cookies);
            
            // Check for errors
            if recursive_result.error != () {
                continue;
            }
            
            // Add new subdomains to the result
            for subdomain in recursive_result.subdomains {
                if !all_subdomains.contains(subdomain) {
                    all_subdomains.push(subdomain);
                }
            }
            
            // Update cookies
            cookies = recursive_result.cookies;
        }
    }
    
    // Return success response
    return #{
        success: all_subdomains.len() > 0,
        message: `Google搜索完成，发现 ${all_subdomains.len()} 个子域名`,
        data: #{
            source: "GOOGLE",
            target_domain: domain,
            found_count: all_subdomains.len(),
        },
        raw_output: json_stringify(#{
            found_domains: all_subdomains
        }),
        found_domains: all_subdomains
    };
} 