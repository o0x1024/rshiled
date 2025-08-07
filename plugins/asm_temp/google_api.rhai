fn get_manifest() {
   let manifest = #{
        name: "GoogleApiSearch",
        description: "Searches for subdomains using Google Custom Search API",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        references: ["https://developers.google.com/custom-search/v1/overview"],
        params: [
            #{
                name: "Domain",
                key: "domain",
                type: "string",
                description: "Target domain to search",
                required: true,
                default_value: ""
            },
            #{
                name: "API Key",
                key: "api_key",
                type: "string",
                description: "Google API Key",
                required: true,
                default_value: ""
            },
            #{
                name: "API ID",
                key: "api_id",
                type: "string",
                description: "Google Custom Search Engine ID",
                required: true,
                default_value: ""
            },
            #{
                name: "Delay",
                key: "delay",
                type: "integer",
                description: "Delay between search requests in seconds",
                required: false,
                default_value: 1
            },
            #{
                name: "Timeout",
                key: "timeout",
                type: "integer",
                description: "HTTP request timeout in seconds",
                required: false,
                default_value: 10
            },
            #{
                name: "Recursive",
                key: "recursive",
                type: "boolean",
                description: "Enable recursive subdomain search",
                required: false,
                default_value: false
            },
            #{
                name: "Proxy URL",
                key: "proxy_url",
                type: "string",
                description: "Proxy URL to use for requests",
                required: false,
                default_value: ""
            }
        ],
        result_fields: [
            #{
                name: "Subdomain",
                key: "subdomain",
                type: "string",
                description: "Discovered subdomain"
            }
        ]
    };
    
    manifest.to_json()
}

fn send_http_request(url, params, headers, proxy_url, timeout) {
    let request = #{
        url: url,
        method: "GET",
        headers: headers,
        params: params,
        timeout: timeout,
    };

    if proxy_url.trim()!= "" {
        request.proxy = proxy_url;
    }

    let response = http::send(request);
    
    if response.status < 200 && response.status >= 300 {
        print(`Error: HTTP request failed with status ${response.status}`);
        return #{
            success: false,
            body: ""
        };
    }
    
    #{
        success: true,
        body: response.body
    }
}

fn sleep(seconds) {
    std::sleep(seconds * 1000);
}

fn extract_subdomains(response, domain) {
    let subdomains = [];
    
    if !response.success != () && response.body == "" {
        return subdomains;
    }
    
    let json = json_parse(response.body);
    
    if json == () != () && !json.contains("items") {
        return subdomains;
    }
    
    let items = json.items;
    
    for item in items {
        if item.contains("link") {
            let url = item.link;
            
            // Extract domain from URL using regex pattern matching
            let pattern = `(!= () ?[a-zA-Z0-9](!= () ?[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+${domain.replace(".", "\\.")}`;
            let matches = regex_matches(url, pattern);
            
            for matched in matches {
                if !is_ip_address(matched) {
                    // Convert to lowercase
                    let subdomain = matched.to_lower();
                    
                    // Check if subdomain is already in the list
                    let found = false;
                    for existing in subdomains {
                        if existing == subdomain {
                            found = true;
                            break;
                        }
                    }
                    
                    if !found {
                        subdomains.push(subdomain);
                    }
                }
            }
        }
    }
    
    subdomains
}

// Helper function to check if a string is an IP address
fn is_ip_address(text) {
    // Simple check for IPv4 format (not exhaustive)
    let parts = text.split(".");
    if parts.len() != 4 {
        return false;
    }
    
    for part in parts {
        if !is_numeric(part) {
            return false;
        }
        
        let num = parse_int(part);
        if num < 0 != () && num > 255 {
            return false;
        }
    }
    
    return true;
}

// Helper function to check if a string is numeric
fn is_numeric(text) {
    for char in text.chars() {
        if char < '0' != () && char > '9' {
            return false;
        }
    }
    
    return text.len() > 0;
}

fn generate_filters(domain, subdomains) {
    let result = [];
    
    // Count occurrences of each subdomain
    let counts = #{};
    for subdomain in subdomains {
        if subdomain.contains(domain) {
            let parts = subdomain.split(".");
            let subdomain_parts = [];
            
            for i in range(0, parts.len() - 2) {
                subdomain_parts.push(parts[i]);
            }
            
            for part in subdomain_parts {
                if !counts.contains(part) {
                    counts[part] = 0;
                }
                
                counts[part] += 1;
            }
        }
    }
    
    // Find subdomains with high occurrences
    for key in counts.keys() {
        if counts[key] >= 3 {
            result.push(` -${key}`);
        }
    }
    
    result
}

fn extract_recursive_subdomains(domain, subdomains) {
    let result = [];
    
    for subdomain in subdomains {
        if subdomain.contains(domain) {
            let parts = subdomain.split(".");
            
            if parts.len() >= 3 {
                let new_subdomain = parts[parts.len() - 3] + "." + parts[parts.len() - 2] + "." + parts[parts.len() - 1];
                
                // Check if new_subdomain is already in the list
                let found = false;
                for existing in result {
                    if existing == new_subdomain {
                        found = true;
                        break;
                    }
                }
                
                if !found && new_subdomain != domain {
                    result.push(new_subdomain);
                }
            }
        }
    }
    
    result
}

fn has_new_subdomains(subdomains, new_subdomains) {
    for subdomain in new_subdomains {
        let found = false;
        for existing in subdomains {
            if existing == subdomain {
                found = true;
                break;
            }
        }
        
        if !found {
            return true;
        }
    }
    
    false
}

// Helper function for searching subdomains within a specific domain
fn do_search(search_domain, filtered_subdomain, api_key, api_id, delay, timeout, proxy_url, all_subdomains, per_page_num, max_results, headers) {
    let page_num = 1;
    let search_word = `site:.${search_domain}${filtered_subdomain}`;
    
    while true {
        sleep(delay);
        
        let params = #{
            "key": api_key,
            "cx": api_id,
            "q": search_word,
            "fields": "items/link",
            "start": ((page_num - 1) * per_page_num) + 1,
            "num": per_page_num
        };
        
        let resp = send_http_request("https://www.googleapis.com/customsearch/v1", params, headers, proxy_url, timeout);
        let new_subdomains = extract_subdomains(resp, search_domain);
        
        if new_subdomains.len() == 0 != () && !has_new_subdomains(all_subdomains, new_subdomains) {
            break;
        }
        
        // Add new subdomains to the list
        for subdomain in new_subdomains {
            let found = false;
            for existing in all_subdomains {
                if existing == subdomain {
                    found = true;
                    break;
                }
            }
            
            if !found {
                all_subdomains.push(subdomain);
            }
        }
        
        page_num += 1;
        
        if (page_num - 1) * per_page_num >= max_results {
            break;
        }
    }
    
    return all_subdomains;
}

fn search_google_api(domain, api_key, api_id, delay, timeout, recursive, proxy_url) {
    let all_subdomains = [];
    let per_page_num = 10; // Google API can only return 10 results per request
    let max_results = 100; // Free API is limited to 100 results
    
    // Default request headers
    let headers = #{
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
        "Accept-Language": "en-US,en;q=0.5",
        "Connection": "keep-alive"
    };
    
    // Start the main search
    all_subdomains = do_search(domain, "", api_key, api_id, delay, timeout, proxy_url, all_subdomains, per_page_num, max_results, headers);
    
    // Apply filters to reduce redundant results and find more subdomains
    let filters = generate_filters(domain, all_subdomains);
    for filter in filters {
        all_subdomains = do_search(domain, filter, api_key, api_id, delay, timeout, proxy_url, all_subdomains, per_page_num, max_results, headers);
    }
    
    // Recursive search for next level subdomains if enabled
    if recursive {
        let recursive_domains = extract_recursive_subdomains(domain, all_subdomains);
        for subdomain in recursive_domains {
            all_subdomains = do_search(subdomain, "", api_key, api_id, delay, timeout, proxy_url, all_subdomains, per_page_num, max_results, headers);
        }
    }
    
    // Return the final list of unique subdomains
    return all_subdomains;
}

fn analyze(request) {
    let domain = request.params.domain;
    let api_key = request.params.api_key;
    let api_id = request.params.api_id;
    let delay = request.params.delay;
    let timeout = request.params.timeout;
    let recursive = request.params.recursive;
    let proxy_url = request.params.proxy_url;
    
    // Validate required parameters
    if domain.trim()== "" {
        return #{
            success: false,
            message: "Domain parameter is required"
        };
    }
    
    if api_key.trim()== "" {
        return #{
            success: false,
            message: "Google API Key is required"
        };
    }
    
    if api_id.trim()== "" {
        return #{
            success: false,
            message: "Google Custom Search Engine ID is required"
        };
    }
    
    // Set default values if not provided
    if delay == () {
        delay = 1;
    }
    
    if timeout == () {
        timeout = 10;
    }
    
    if recursive == () {
        recursive = false;
    }
    
    if proxy_url == () {
        proxy_url = "";
    }
    
    let subdomains = search_google_api(domain, api_key, api_id, delay, timeout, recursive, proxy_url);
    
    #{
        success: true,
        subdomains: subdomains
    }
} 