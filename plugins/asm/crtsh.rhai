// CrtshSearch - domain_discovery插件
// 描述: Searches for subdomains using crt.sh certificate transparency logs
// 作者: RShield Team
// 版本: 1.0.0
/**
 * Crt.sh Domain Discovery Plugin
 *
 * This plugin uses the crt.sh certificate transparency search service to discover subdomains of a given domain.
 * It searches through certificate transparency logs to find certificates issued for the domain and its subdomains.
 */

/**
 * Returns the plugin manifest with metadata
 */
fn get_manifest() {
    let manifest = #{
        name: "CrtshSearch",
        description: "Searches for subdomains using crt.sh certificate transparency logs",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://crt.sh/",
        params: [],
        result_fields: []
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

    request_params.proxy_url = "http://127.0.0.1:8080";

    // Send the request
    let response_str = http_request(request_params.to_json());
    let response = json_parse(response_str);

    // print_debug(`response: ${response}`);
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
 * Extracts subdomains from search results
 */
fn extract_subdomains(response_body, domain) {
    let subdomains = [];
    let root_domain = domain.to_lower();

    // print_debug(`response_body:${response_body}`);
    if response_body.trim()== "" {
        return subdomains;
    }

    // Parse the JSON response
    let json_data = json_parse(response_body);
    if json_data == () {
        return subdomains;
    }

    // Process each certificate entry
    for entry in json_data {
        if entry.name_value != () {
            let name_value = entry.name_value.to_lower();

            let parts = name_value.split("\n");
            for part in parts {
                let pattern = `[a-zA-Z0-9-]+.${root_domain}`;
                let matches = regex_matches(part, pattern);
                for xmatch in matches {
                    let subdomain = xmatch.to_lower();
                    if subdomain.ends_with(root_domain) && subdomain != root_domain {
                        subdomains.push(subdomain);
                    }
                }
            }
        }
    }

    return subdomains;
}

/**
 * Main function to search crt.sh for subdomains
 */
fn search_crtsh(domain, wildcard, timeout, proxy_url) {
    let search_domain = domain;

    // Use wildcard search if enabled
    if wildcard {
        search_domain = `%.${domain}`;
    }

    // Base API URL
    let api_url = "https://crt.sh/";

    // Prepare query parameters
    let params = #{
        "q": search_domain,
        "output": "json"
    };

    // Prepare request parameters
    let request_params = #{
        url: api_url,
        method: "GET",
        params: params,
        timeout: timeout
    };

    // Add proxy if provided
    if proxy_url != "" {
        request_params.proxy_url = proxy_url;
    }

    let response = send_http_request(request_params);

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
    let request = json_parse(request_json);

    // print_debug(`authorization: ${authorization}`);
    // Extract parameters
    let domain = request.target;
    let wildcard = request.wildcard;
    if request.wildcard == () {
        wildcard = true;
    }
    let timeout = 30;
    if request.timeout != () {
        timeout = request.timeout;
    }
    let proxy_url = "";

    // Search for subdomains
    let search_result = search_crtsh(domain, wildcard, timeout, proxy_url);

    if search_result.error != (){
        return #{
            success: false,
            message: `搜索失败,报错：${search_result.error}`,
        };
    }else{
        let all_subdomains = search_result.subdomains;
        // print_debug(`${all_subdomains}`);
        return #{
            success: all_subdomains.len() > 0,
            message: `CrtshSearch搜索完成，发现 ${all_subdomains.len()} 个子域名`,
            data: #{
                source: "CrtshSearch",
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
