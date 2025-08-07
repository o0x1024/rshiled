// VirusTotal子域收集插件 - Rhai脚本实现
//
// 收集方法：
// 1. 查询VirusTotal API获取子域名信息
// 2. 对结果进行解析并提取子域名
//
// 功能描述：
// 使用VirusTotal的子域名API查询目标域名的子域名信息

// 返回插件元数据的函数
fn get_manifest() {
    let manifest = #{
        name: "VirusTotalDomainSearch",
        description: "使用VirusTotal服务发现子域名",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        references: [
            "https://www.virustotal.com"
        ],
        params: [
            #{
                name: "目标域名",
                key: "domain",
                type: "string",
                required: true,
                default_value: "",
                description: "要搜索的目标域名"
            },
            #{
                name: "超时时间(秒)",
                key: "timeout",
                type: "number",
                required: false,
                default_value: 30,
                description: "HTTP请求超时时间"
            },
            #{
                name: "每页结果数",
                key: "limit",
                type: "number",
                required: false,
                default_value: 40,
                description: "每页查询的子域名数量"
            },
            #{
                name: "最大页数",
                key: "max_pages",
                type: "number",
                required: false,
                default_value: 5,
                description: "最大查询页数"
            },
            #{
                name: "代理URL",
                key: "proxy_url",
                type: "string",
                required: false,
                default_value: "",
                description: "HTTP代理URL，例如http://127.0.0.1:8080"
            }
        ],
        result_fields: [
            #{
                name: "发现的域名",
                key: "found_domains",
                type: "array",
                description: "所有发现的子域名列表"
            },
            #{
                name: "查询结果",
                key: "query_results",
                type: "object",
                description: "查询过程中的详细信息"
            }
        ]
    };
    
    // 返回JSON字符串
    manifest.to_json()
}

// 发送HTTP请求函数
fn send_http_request(params) {
    let request_json = json_stringify(params);
    
    // 打印请求信息
    print_debug(`发送HTTP请求: ${request_json}`);
    
    // 通过HTTP请求函数发送
    let response = http_request(request_json);
    
    // 解析响应
    let parsed_response = json_parse(response);
    
    // 检查是否有错误
    if parsed_response.error != () {
        print_error(`HTTP请求错误: ${parsed_response.error}`);
    }
    
    return parsed_response;
}

// 从JSON响应中提取子域名
fn extract_subdomains_from_json(json_data) {
    let subdomains = [];
    
    // 检查是否有data部分
    if json_data.data != () {
        // 遍历data数组
        if is_map(json_data) && json_data.data != () && is_array(json_data.data) {
            for item in json_data.data {
                // 提取ID，通常是子域名
                if item.id != () && is_string(item.id) {
                    subdomains.push(item.id);
                }
                
                // 有些结果可能在attributes中
                if item.attributes != () && item.attributes.id != () && is_string(item.attributes.id) {
                    subdomains.push(item.attributes.id);
                }
            }
        }
    }
    
    return subdomains;
}

// 从响应文本中匹配子域名
fn match_subdomains(response_text, domain) {
    print_debug(`正在从响应中匹配子域名，目标域名: ${domain}`);
    
    let subdomains = [];
    
    // 尝试解析JSON
    let json_data = json_parse(response_text);
    if json_data != () {
        let json_subdomains = extract_subdomains_from_json(json_data);
        for subdomain in json_subdomains {
            if !subdomains.contains(subdomain) {
                subdomains.push(subdomain);
            }
        }
    }
    
    // 简单的文本匹配，提取子域名格式的字符串
    // 这是一个非常简化的实现，实际应该使用更复杂的正则表达式
    let parts = response_text.split("\"");
    for part in parts {
        if part.ends_with(domain) && !part.contains("@") && part != domain {
            // 检查是否是一个有效的子域名
            if part.contains(".") {
                let subdomain = part.trim();
                if !subdomains.contains(subdomain) {
                    subdomains.push(subdomain);
                }
            }
        }
    }
    
    print_debug(`从响应中提取到 ${subdomains.len()} 个子域名`);
    return subdomains;
}

// 查询VirusTotal API
fn query_virustotal(domain, limit, max_pages, timeout, proxy_url) {
    print_info(`开始查询VirusTotal，目标域名: ${domain}`);
    
    let all_subdomains = [];
    let api_url = `https://www.virustotal.com/ui/domains/${domain}/subdomains`;
    let next_cursor = "";
    let page_count = 0;
    
    while page_count < max_pages {
        print_debug(`查询第 ${page_count + 1} 页，游标: ${next_cursor}`);
        
        // 构建查询参数
        let query_params = `limit=${limit}`;
        if next_cursor.trim()!= "" {
            query_params += `&cursor=${next_cursor}`;
        }
        
        // 构建请求参数
        let request_params = #{
            url: `${api_url}?${query_params}`,
            method: "GET",
            headers: #{
                "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
                "Referer": "https://www.virustotal.com/",
                "Accept": "application/json"
            },
            timeout: timeout,
            proxy_url: proxy_url,
            follow_redirects: true,
            max_redirects: 5
        };
        
        // 发送请求
        let response = send_http_request(request_params);
        
        // 检查是否有错误
        if response.error != () {
            print_error(`查询出错: ${response.error}`);
            break;
        }
        
        // 检查是否有响应体
        if response.body == () {
            print_warn("响应没有包含有效内容");
            break;
        }
        
        // 提取子域名
        let found_subdomains = match_subdomains(response.body, domain);
        
        // 添加到总集合中
        for subdomain in found_subdomains {
            if !all_subdomains.contains(subdomain) {
                all_subdomains.push(subdomain);
            }
        }
        
        // 尝试解析JSON以获取下一页游标
        let json_data = json_parse(response.body);
        if json_data != () && json_data.meta != () && json_data.meta.cursor != () {
            next_cursor = json_data.meta.cursor;
            if next_cursor.trim()== "" {
                print_debug("没有更多页面，查询完成");
                break;
            }
        } else {
            print_debug("无法获取下一页游标，查询结束");
            break;
        }
        
        page_count += 1;
    }
    
    print_info(`VirusTotal查询完成，共发现 ${all_subdomains.len()} 个子域名`);
    return all_subdomains;
}

// 主执行函数
fn analyze(request_json) {
    // 解析输入请求
    let request = json_parse(request_json);
    let target = request.target;
    
    // 检查参数
    let params = #{};
    if request.params != () {
        params = request.params;
    }
    
    // 打印调试信息
    print_info(`开始分析目标: ${target}`);
    
    // 获取参数，或使用默认值
    let timeout = 30;
    if params.timeout != () {
        timeout = params.timeout;
    }
    
    let limit = 40;
    if params.limit != () {
        limit = params.limit;
    }
    
    let max_pages = 5;
    if params.max_pages != () {
        max_pages = params.max_pages;
    }
    
    let proxy_url = "";
    if params.proxy_url != () {
        proxy_url = params.proxy_url;
    }
    
    // 打印参数信息
    print_debug(`使用参数: timeout=${timeout}, limit=${limit}, max_pages=${max_pages}, proxy_url=${proxy_url}`);
    
    // 验证目标域名
    if target.trim().trim()== "" {
        return json_stringify(#{
            success: false,
            message: "目标域名不能为空",
            data: [],
            raw_output: "目标域名不能为空",
            found_domains: []
        });
    }
    
    // 查询VirusTotal
    let subdomains = query_virustotal(target, limit, max_pages, timeout, proxy_url);
    
    // 准备结果
    let result = #{
        success: subdomains.len() > 0,
        message: `VirusTotal查询完成，发现 ${subdomains.len()} 个子域名`,
        data: #{
            source: "VirusTotal",
            target_domain: target,
            found_count: subdomains.len(),
            pages_searched: max_pages,
            limit_per_page: limit
        },
        raw_output: json_stringify(#{
            queried_pages: max_pages,
            found_domains: subdomains
        }),
        found_domains: subdomains
    };
    
    return json_stringify(result);
} 