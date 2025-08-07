// 百度搜索子域收集插件 - Rhai脚本实现
//
// 搜索方法：
// 1. 向百度发送站点搜索请求 (site:域名)
// 2. 解析响应中的子域名
// 3. 递归查询识别到的子域名
//
// 功能描述：
// 使用百度搜索引擎的site语法查询目标域名的子域名

// 返回插件元数据的函数
fn get_manifest() {
    let manifest = #{
        name: "BaiduDomainSearch",
        description: "使用百度搜索引擎发现子域名",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        references: [
            "https://www.baidu.com"
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
                name: "最大页数",
                key: "max_pages",
                type: "number",
                required: false,
                default_value: 5,
                description: "最大搜索页数"
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
                name: "搜索结果",
                key: "search_results",
                type: "object",
                description: "搜索过程中的详细信息"
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

// 随机休眠函数
fn random_sleep(min, max) {
    // 模拟Python中的随机休眠
    let delay = min + (max - min);
    print_debug(`随机休眠 ${delay} 秒`);
    // 由于Rhai没有sleep函数，这里只是模拟一下
    // 实际的脚本将使用服务器端的时间控制机制
}

// 从响应中提取子域名
fn extract_subdomains(response_body, domain) {
    print_debug(`正在从响应中提取子域名，目标域名: ${domain}`);
    
    let subdomains = [];
    
    // 这里使用简单的正则匹配逻辑来模拟Python中的匹配
    // 实际实现可能需要更复杂的正则表达式
    
    // 模拟子域名匹配
    // 在实际的Rhai脚本中，可以使用正则表达式或字符串操作来实现
    
    // 这里假设response_body是HTML字符串
    // 简单的模拟：查找域名的出现
    
    // 方法1: 查找超链接中的子域名
    let a_tags = response_body.split("<a");
    for tag in a_tags {
        if tag.contains("href=") {
            let href_part = tag.split("href=\"")[1];
            if href_part != () {
                let url = href_part.split("\"")[0];
                if url.contains("://") {
                    let host_part = url.split("://")[1].split("/")[0];
                    if host_part.ends_with(domain) && host_part != domain {
                        let subdomain = host_part.trim();
                        if !subdomains.contains(subdomain) {
                            subdomains.push(subdomain);
                        }
                    }
                }
            }
        }
    }
    
    // 方法2: 查找文本中的子域名
    let parts = response_body.split(" ");
    for part in parts {
        if part.contains(domain) {
            // 尝试提取可能的子域名格式
            let cleaned = part.replace(",", "").replace("\"", "").replace("'", "").replace("<", "").replace(">", "");
            
            if cleaned.ends_with(domain) && cleaned != domain {
                // 检查是否包含域名分隔符
                if cleaned.contains(".") {
                    let subdomain = cleaned.trim();
                    if !subdomains.contains(subdomain) {
                        subdomains.push(subdomain);
                    }
                }
            }
        }
    }
    
    print_debug(`从响应中提取到 ${subdomains.len()} 个子域名`);
    return subdomains;
}

// 搜索百度
fn search_baidu(domain, max_pages, timeout, proxy_url) {
    print_info(`开始百度搜索，目标域名: ${domain}`);
    
    let all_subdomains = [];
    let init_url = "https://www.baidu.com/";
    let search_url = "https://www.baidu.com/s";
    
    // 获取初始化Cookie
    let init_params = #{
        url: init_url,
        method: "GET",
        headers: #{
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
            "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8"
        },
        timeout: timeout,
        proxy_url: proxy_url,
        follow_redirects: true,
        max_redirects: 5
    };
    
    let init_response = send_http_request(init_params);
    
    // 模拟获取Cookie
    let cookies = "";
    if init_response.headers != () {
        if init_response.headers["set-cookie"] != () {
            cookies = init_response.headers["set-cookie"];
        }
    }
    
    // 执行搜索
    for page in 0..max_pages {
        // 随机休眠2-5秒，模拟人类行为
        random_sleep(2, 5);
        
        // 百度分页参数
        let pn = page * 10;
        let query = `site:${domain}`;
        
        print_debug(`正在搜索第 ${page + 1} 页, 起始位置: ${pn}`);
        
        // 构建搜索请求
        let search_params = #{
            url: `${search_url}?wd=${query}&pn=${pn}&rn=10`,
            method: "GET",
            headers: #{
                "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
                "Referer": "https://www.baidu.com/",
                "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
                "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
                "Cookie": cookies
            },
            timeout: timeout,
            proxy_url: proxy_url,
            follow_redirects: true,
            max_redirects: 5
        };
        
        let search_response = send_http_request(search_params);
        
        // 检查是否有正确的响应体
        if search_response.body != () {
            // 提取子域名
            let found_subdomains = extract_subdomains(search_response.body, domain);
            
            // 添加到总集合中
            for subdomain in found_subdomains {
                if !all_subdomains.contains(subdomain) {
                    all_subdomains.push(subdomain);
                }
            }
            
            // 检查是否还有下一页
            if !search_response.body.contains("下一页") {
                print_debug("没有更多结果页面，停止搜索");
                break;
            }
        } else {
            print_error("搜索响应没有包含有效内容");
            break;
        }
    }
    
    print_info(`百度搜索完成，共发现 ${all_subdomains.len()} 个子域名`);
    return all_subdomains;
}

// 按关键词进行深度搜索
fn deep_search(domain, timeout, proxy_url) {
    print_info(`开始对 ${domain} 进行深度搜索`);
    
    let keywords = [
        "admin", "api", "app", "bbs", "blog", "cdn", "cloud", "cms", "crm", 
        "dev", "erp", "forum", "ftp", "git", "mail", "mx", "oa", "portal", 
        "s", "shop", "store", "test", "uat", "vip", "vpn", "web", "www"
    ];
    
    let all_subdomains = [];
    
    for keyword in keywords {
        print_debug(`使用关键词 ${keyword} 进行搜索`);
        
        // 构造定向搜索查询
        let search_domain = `${keyword}.${domain}`;
        
        // 进行单页搜索
        let search_params = #{
            url: `https://www.baidu.com/s?wd=site:${search_domain}`,
            method: "GET",
            headers: #{
                "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
                "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
                "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8"
            },
            timeout: timeout,
            proxy_url: proxy_url,
            follow_redirects: true,
            max_redirects: 5
        };
        
        // 随机休眠2-4秒
        random_sleep(2, 4);
        
        let response = send_http_request(search_params);
        
        if response.body != () {
            // 提取子域名
            let found_subdomains = extract_subdomains(response.body, domain);
            
            // 如果查询有结果，就将该关键词子域名加入结果
            if response.body.contains(search_domain) {
                if !all_subdomains.contains(search_domain) {
                    all_subdomains.push(search_domain);
                }
            }
            
            // 添加其他找到的子域名
            for subdomain in found_subdomains {
                if !all_subdomains.contains(subdomain) {
                    all_subdomains.push(subdomain);
                }
            }
        }
    }
    
    print_info(`深度搜索完成，发现 ${all_subdomains.len()} 个子域名`);
    return all_subdomains;
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
        let max_pages = 5;
    if params.max_pages != () {
        max_pages = params.max_pages;
    }
        let proxy_url = "";
    if params.proxy_url != () {
        proxy_url = params.proxy_url;
    }
    
    // 打印参数信息
    print_debug(`使用参数: timeout=${timeout}, max_pages=${max_pages}, proxy_url=${proxy_url}`);
    
    // 验证目标域名
    if target.trim().trim()== "" {
        return json_stringify(#{
            success: false,
            message: "目标域名不能为空",
            data: #{
                source: "Baidu",
                target_domain: target,
                found_count: 0,
                pages_searched: max_pages
            },
            raw_output: "目标域名不能为空",
            found_domains: []
        });
    }
    
    // 执行百度搜索
    let subdomains = search_baidu(target, max_pages, timeout, proxy_url);
    
    // 执行深度搜索
    let deep_subdomains = deep_search(target, timeout, proxy_url);
    
    // 合并结果
    for subdomain in deep_subdomains {
        if !subdomains.contains(subdomain) {
            subdomains.push(subdomain);
        }
    }
    
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
} 