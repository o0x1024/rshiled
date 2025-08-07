// NSEC记录枚举子域名收集插件 - Rhai脚本实现
//
// 收集方法：
// 1. 查询域名的NSEC记录
// 2. 通过NSEC记录枚举获取子域名
//
// 功能描述：
// 利用DNS NSEC记录链进行子域名枚举，该方法适用于开启了DNSSEC的域名

// 返回插件元数据的函数
fn get_manifest() {
    let manifest = #{
        name: "NsecDomainSearch",
        description: "通过NSEC记录枚举子域名",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        reference: "https://tools.ietf.org/html/rfc4034",
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
                description: "DNS查询超时时间"
            },
            #{
                name: "DNS服务器",
                key: "dns_server",
                type: "string",
                required: false,
                default_value: "8.8.8.8",
                description: "用于查询的DNS服务器"
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
                name: "DNS查询结果",
                key: "dns_results",
                type: "object",
                description: "DNS查询过程中的详细信息"
            }
        ]
    };

    manifest.to_json()
}

// 发送DNS查询HTTP请求函数
fn send_dns_api_request(params) {
    // 打印请求信息
    print_debug(`params: ${params}`);
    print_debug(`发送DNS API请求: ${params.url}`);
    
    // 通过HTTP请求函数发送
    let response_json = http_request(params.to_json());
    
    // 解析响应
    let response = json_parse(response_json);
    
    // 检查是否有错误
    if response.error != () {
        print_error(`DNS API请求错误: ${response.error}`);
    }
    
    return response;
}

// 查询域名的NSEC记录
fn query_nsec(domain, timeout, dns_server, proxy_url) {
    print_info(`开始查询 ${domain} 的NSEC记录`);
    
    // 使用Google DNS-over-HTTPS API
    let api_url = "https://dns.google/resolve";
    
    // 构建请求参数 - 查询不存在的随机子域名触发NSEC记录
    // 使用一个静态字符串作为前缀，足够不可能存在即可
    let random_prefix = "nsec-nonexist-probe-123456";
    let random_subdomain = `${random_prefix}.${domain}`;
    
    let request_params = #{
        url: `${api_url}?name=${random_subdomain}&type=A&cd=false&do=true`,
        method: "GET",
        headers: #{
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
            "Accept": "application/dns-json"
        },
        timeout: timeout,
        proxy_url: proxy_url,
        follow_redirects: true,
        max_redirects: 5
    };
    
    print_debug(`NSEC查询请求参数: ${request_params.to_json()}`);
    // 发送请求
    let response = send_dns_api_request(request_params);
    
    print_debug(`NSEC查询响应: ${response}`);
    // 检查是否有错误
    if response.error != () {
        print_error(`NSEC查询出错: ${response.error}`);
        return [];
    }
    
    // 检查是否有响应体
    if response.body == () {
        print_warn("NSEC查询响应没有包含有效内容");
        return [];
    }
    
    return extract_nsec_domains(response.body, domain);
}

// 从NSEC响应中提取域名
fn extract_nsec_domains(response_body, domain) {
    print_debug(`解析NSEC响应，目标域名: ${domain}`);
    
    let subdomains = [];
    let parsed = json_parse(response_body);
    
    // 检查是否有NSEC记录
    if parsed.Authority != () && is_array(parsed.Authority) {
        for auth in parsed.Authority {
            // 查找NSEC/NSEC3记录
            if (auth.type == 47 || auth.type == 50) {  // NSEC=47, NSEC3=50
                if auth.data != () {
                    // 解析NSEC数据部分，提取子域名
                    // NSEC记录格式：next-domain-name [list of types]
                    let nsec_data = auth.data;
                    let next_domain = nsec_data.split(" ")[0];
                    
                    // 如果是子域名，添加到结果
                    if next_domain.ends_with(domain) && next_domain != domain {
                        if !subdomains.contains(next_domain) {
                            subdomains.push(next_domain);
                        }
                    }
                }
            }
            
            // 有时域名信息在name字段
            if auth.name != () {
                let name = auth.name;
                if name.ends_with(domain) && name != domain {
                    if !subdomains.contains(name) {
                        subdomains.push(name);
                    }
                }
            }
        }
    }
    
    // 如果在Authority部分没找到，检查Answer部分
    if parsed.Answer != () && is_array(parsed.Answer) {
        for answer in parsed.Answer {
            if answer.name != () {
                let name = answer.name;
                if name.ends_with(domain) && name != domain {
                    if !subdomains.contains(name) {
                        subdomains.push(name);
                    }
                }
            }
        }
    }
    
    // 尝试从整个响应文本中搜索可能的子域名
    let text_response = response_body;
    let parts = text_response.split("\"");
    
    for part in parts {
        if part.ends_with(domain) && part != domain && part.contains(".") {
            let cleaned = part.trim();
            if !subdomains.contains(cleaned) {
                subdomains.push(cleaned);
            }
        }
    }
    
    print_debug(`从NSEC响应中提取到 ${subdomains.len()} 个子域名`);
    return subdomains;
}

// 递归查询NSEC记录
fn recursive_nsec_query(domain, subdomains, timeout, dns_server, proxy_url, depth) {
    // 设置默认depth值
    if depth == () {
        depth = 0;
    }
    
    if depth > 3 {  // 最多递归3次
        return subdomains;
    }
    
    let new_domains = [];
    
    for subdomain in subdomains {
        print_debug(`递归查询子域名: ${subdomain}`);
        
        // 为每个子域名进行NSEC查询
        let new_results = query_nsec(subdomain, timeout, dns_server, proxy_url);
        
        // 合并新结果
        for new_domain in new_results {
            if !subdomains.contains(new_domain) && !new_domains.contains(new_domain) {
                new_domains.push(new_domain);
            }
        }
        
        // 短暂休眠，避免请求过快
        print_debug("短暂休眠，避免请求过快");
    }
    
    if new_domains.len() > 0 {
        // 合并结果
        for new_domain in new_domains {
            if !subdomains.contains(new_domain) {
                subdomains.push(new_domain);
            }
        }
        
        // 递归查询新发现的域名
        return recursive_nsec_query(domain, subdomains, timeout, dns_server, proxy_url, depth + 1);
    }
    
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
    let dns_server = "8.8.8.8";
    if params.dns_server != () {
        dns_server = params.dns_server;
    }
    let proxy_url = "";
    if params.proxy_url != () {
        proxy_url = params.proxy_url;
    }
        
    // 打印参数信息
    print_debug(`使用参数: timeout=${timeout}, dns_server=${dns_server}, proxy_url=${proxy_url}`);
    
    // 验证目标域名
    if target == "" {
        return #{
            success: false,
            message: "目标域名不能为空",
            data: #{
                source: "NSEC",
                target_domain: "",
                found_count: 0,
                dns_server: dns_server
            },
            raw_output: "目标域名不能为空",
            found_domains: []
        };
    }
    
    // 执行NSEC记录查询
    let initial_subdomains = query_nsec(target, timeout, dns_server, proxy_url);
    
    // 递归查询
    let subdomains = recursive_nsec_query(target, initial_subdomains, timeout, dns_server, proxy_url);
    
    // 准备结果
    return #{
        success: subdomains.len() > 0,
        message: `NSEC查询完成，发现 ${subdomains.len()} 个子域名`,
        data: #{
            source: "NSEC",
            target_domain: target,
            found_count: subdomains.len(),
            dns_server: dns_server
        },
        raw_output: json_stringify(#{
            dns_server: dns_server,
            found_domains: subdomains
        }),
        found_domains: subdomains
    };
} 