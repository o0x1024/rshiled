// Sitemap子域名收集插件 - Rhai脚本实现
//
// 收集方法：
// 1. 获取站点的sitemap.xml文件
// 2. 解析文件中的URL提取子域名
//
// 功能描述：
// 通过分析网站的sitemap.xml文件发现子域名

// 返回插件元数据的函数
fn get_manifest() {
    let manifest = #{
        name: "SitemapDomainSearch",
        description: "通过网站Sitemap发现子域名",
        author: "RShield Team",
        version: "1.0.0",
        plugin_type: "domain_discovery",
        severity: "low",
        references: [
            "https://www.sitemaps.org/protocol.html"
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
                name: "Sitemap信息",
                key: "sitemap_info",
                type: "object",
                description: "Sitemap解析的详细信息"
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

// 从URL中提取主机名
fn extract_hostname(url) {
    // 处理URL格式
    let hostname = url;
    
    // 移除协议部分
    if hostname.contains("://") {
        hostname = hostname.split("://")[1];
    }
    
    // 移除路径部分
    if hostname.contains("/") {
        hostname = hostname.split("/")[0];
    }
    
    // 移除端口号
    if hostname.contains(":") {
        hostname = hostname.split(":")[0];
    }
    
    return hostname.trim();
}

// 从Sitemap内容中提取子域名
fn extract_domains_from_sitemap(sitemap_content, domain) {
    print_debug(`解析Sitemap内容，目标域名: ${domain}`);
    
    let subdomains = [];
    
    // 查找所有URL标签内容
    let url_tags = sitemap_content.split("<url>");
    
    for tag in url_tags {
        if tag.contains("<loc>") {
            let loc_parts = tag.split("<loc>");
            if loc_parts.len() > 1 {
                let url_part = loc_parts[1].split("</loc>")[0];
                let hostname = extract_hostname(url_part);
                
                // 检查是否是目标域名的子域名
                if hostname.ends_with(domain) && hostname != domain {
                    if !subdomains.contains(hostname) {
                        subdomains.push(hostname);
                    }
                }
            }
        }
    }
    
    // 查找sitemap索引中的sitemap引用
    let sitemap_tags = sitemap_content.split("<sitemap>");
    
    for tag in sitemap_tags {
        if tag.contains("<loc>") {
            let loc_parts = tag.split("<loc>");
            if loc_parts.len() > 1 {
                let url_part = loc_parts[1].split("</loc>")[0];
                let hostname = extract_hostname(url_part);
                
                // 检查是否是目标域名的子域名
                if hostname.ends_with(domain) && hostname != domain {
                    if !subdomains.contains(hostname) {
                        subdomains.push(hostname);
                    }
                }
            }
        }
    }
    
    print_debug(`从Sitemap内容中提取到 ${subdomains.len()} 个子域名`);
    return subdomains;
}

// 获取网站Sitemap
fn get_sitemap(domain, timeout, proxy_url) {
    print_info(`获取 ${domain} 的Sitemap`);
    
    // 常见的Sitemap路径
    let sitemap_paths = [
        "/sitemap.xml",
        "/sitemap_index.xml",
        "/sitemap-index.xml",
        "/sitemapindex.xml",
        "/sitemap/sitemap.xml"
    ];
    
    let all_subdomains = [];
    let processed_sitemaps = [];
    
    // 处理HTTP和HTTPS协议
    let protocols = ["https://", "http://"];
    
    for protocol in protocols {
        for path in sitemap_paths {
            let sitemap_url = `${protocol}${domain}${path}`;
            
            if processed_sitemaps.contains(sitemap_url) {
                continue;
            }
            
            print_debug(`尝试获取Sitemap: ${sitemap_url}`);
            
            // 构建请求参数
            let request_params = #{
                url: sitemap_url,
                method: "GET",
                headers: #{
                    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
                    "Accept": "application/xml,text/xml,application/xhtml+xml,text/html;q=0.9"
                },
                timeout: timeout,
                proxy_url: proxy_url,
                follow_redirects: true,
                max_redirects: 5
            };
            
            // 发送请求
            let response = send_http_request(request_params);
            processed_sitemaps.push(sitemap_url);
            
            // 检查是否有错误
            if response.error != () {
                print_debug(`获取Sitemap出错: ${response.error}`);
                continue;
            }
            
            // 检查是否有响应体
            if response.body == () {
                print_debug(`Sitemap响应没有包含有效内容: ${sitemap_url}`);
                continue;
            }
            
            // 检查响应状态码
            if response.status != () && response.status != 200 {
                print_debug(`Sitemap响应状态码非200: ${response.status}`);
                continue;
            }
            
            // 检查内容类型
            let is_xml = false;
            
            if response.headers != () {
                let content_type = response.headers["content-type"];
                if content_type != () {
                    is_xml =if  content_type.contains("xml") != () { content_type.contains("text"); }
                }
            }
            
            // 检查响应体是否包含XML内容
            if !is_xml && !response.body.contains("<urlset") && !response.body.contains("<sitemapindex") {
                print_debug(`响应内容不是有效的Sitemap XML: ${sitemap_url}`);
                continue;
            }
            
            // 提取子域名
            let found_subdomains = extract_domains_from_sitemap(response.body, domain);
            
            // 处理找到的子站点地图
            if response.body.contains("<sitemapindex") {
                print_debug("找到Sitemap索引，解析子Sitemap");
                
                let sitemap_tags = response.body.split("<sitemap>");
                
                for tag in sitemap_tags {
                    if tag.contains("<loc>") {
                        let loc_parts = tag.split("<loc>");
                        if loc_parts.len() > 1 {
                            let sub_sitemap_url = loc_parts[1].split("</loc>")[0].trim();
                            
                            if !processed_sitemaps.contains(sub_sitemap_url) {
                                print_debug(`处理子Sitemap: ${sub_sitemap_url}`);
                                
                                // 构建请求参数
                                let sub_request_params = #{
                                    url: sub_sitemap_url,
                                    method: "GET",
                                    headers: #{
                                        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
                                        "Accept": "application/xml,text/xml"
                                    },
                                    timeout: timeout,
                                    proxy_url: proxy_url,
                                    follow_redirects: true,
                                    max_redirects: 5
                                };
                                
                                let sub_response = send_http_request(sub_request_params);
                                processed_sitemaps.push(sub_sitemap_url);
                                
                                if sub_response.error == () && sub_response.body != () && sub_response.status == 200 {
                                    let sub_found_domains = extract_domains_from_sitemap(sub_response.body, domain);
                                    
                                    // 合并子站点地图中找到的子域名
                                    for subdomain in sub_found_domains {
                                        if !found_subdomains.contains(subdomain) {
                                            found_subdomains.push(subdomain);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // 合并结果
            for subdomain in found_subdomains {
                if !all_subdomains.contains(subdomain) {
                    all_subdomains.push(subdomain);
                }
            }
            
            // 如果已经找到了有效的Sitemap，可以提前退出
            if found_subdomains.len() > 0 {
                print_info(`成功获取Sitemap: ${sitemap_url}`);
                break;
            }
        }
        
        // 如果已经找到了子域名，不需要尝试另一个协议
        if all_subdomains.len() > 0 {
            break;
        }
    }
    
    print_info(`Sitemap解析完成，共发现 ${all_subdomains.len()} 个子域名`);
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
            data: #{name: "data",},
            raw_output: "目标域名不能为空",
            found_domains: []
        });
    }
    
    // 获取并解析Sitemap
    let subdomains = get_sitemap(target, timeout, proxy_url);
    
    // 准备结果
    let result = #{
        success: subdomains.len() > 0,
        message: `Sitemap解析完成，发现 ${subdomains.len()} 个子域名`,
        data: #{
            source: "Sitemap",
            target_domain: target,
            found_count: subdomains.len()
        },
        raw_output: json_stringify(#{
            target: target,
            found_domains: subdomains
        }),
        found_domains: subdomains
    };
    
    return json_stringify(result);
} 