// ThreatMiner子域收集插件 - Rhai脚本实现
//
// 收集方法：
// 1. 查询ThreatMiner API获取子域名信息
// 2. 解析响应中的子域名
//
// 功能描述：
// 使用ThreatMiner情报平台API获取目标域名的子域名

// 返回插件元数据的函数
fn get_manifest() {
    let manifest = #{
        name: "ThreatMinerDomainSearch",
        description: "使用ThreatMiner情报平台API发现子域名",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        references: [
            "https://www.threatminer.org/"
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

// 从响应中提取子域名
fn extract_domains_from_response(response_body, domain) {
    print_debug(`解析ThreatMiner响应，目标域名: ${domain}`);
    
    let subdomains = [];
    let parsed = json_parse(response_body);
    
    if parsed.status_code == 200 && parsed.results != () && is_array(parsed.results) {
        for subdomain in parsed.results {
            if is_string(subdomain) && subdomain.ends_with(domain) && subdomain != domain {
                if !subdomains.contains(subdomain) {
                    subdomains.push(subdomain);
                }
            }
        }
    }
    
    print_debug(`从ThreatMiner响应中提取到 ${subdomains.len()} 个子域名`);
    return subdomains;
}

// 查询ThreatMiner API
fn query_threatminer(domain, timeout, proxy_url) {
    print_info(`开始查询ThreatMiner，目标域名: ${domain}`);
    
    // ThreatMiner API URL
    let api_url = "https://api.threatminer.org/v2/domain.php";
    
    // 构建请求参数
    let request_params = #{
        url: `${api_url}?q=${domain}&rt=5`,
        method: "GET",
        headers: #{
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
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
        print_error(`ThreatMiner查询出错: ${response.error}`);
        return [];
    }
    
    // 检查是否有响应体
    if response.body == () {
        print_warn("ThreatMiner响应没有包含有效内容");
        return [];
    }
    
    // 提取子域名
    let subdomains = extract_domains_from_response(response.body, domain);
    
    print_info(`ThreatMiner查询完成，发现 ${subdomains.len()} 个子域名`);
    return subdomains;
}

// 主执行函数
fn analyze(request_json) {
    // 解析输入请求
    let request = json_parse(request_json);
    let target = request.target;
    
    // 检查参数
    let has_params = request.params != ();
    let params;
    if has_params {
        params = request.params;
    } else {
        params = #{};
    }
    
    // 打印调试信息
    print_info(`开始分析目标: ${target}`);
    
    // 获取参数，或使用默认值
    let timeout = 30;
    if params.timeout != () {
        timeout = params.timeout;
    }
        let proxy_url = "";
    if params.proxy_url != () {
        proxy_url = params.proxy_url;
    }
        
    // 打印参数信息
    print_debug(`使用参数: timeout=${timeout}, proxy_url=${proxy_url}`);
    
    // 验证目标域名
    if target.trim().trim()== "" {
        return json_stringify(#{
            success: false,
            message: "目标域名不能为空",
            data: #{
                source: "ThreatMiner",
                target_domain: target,
                found_count: 0
            },
            raw_output: "目标域名不能为空",
            found_domains: []
        });
    }
    
    // 查询ThreatMiner
    let subdomains = query_threatminer(target, timeout, proxy_url);
    
    // 准备结果
    let result = #{
        success: subdomains.len() > 0,
        message: `ThreatMiner查询完成，发现 ${subdomains.len()} 个子域名`,
        data: #{
            source: "ThreatMiner",
            target_domain: target,
            found_count: subdomains.len()
        },
        raw_output: json_stringify(#{
            query: target,
            found_domains: subdomains
        }),
        found_domains: subdomains
    };
    
    return json_stringify(result);
} 