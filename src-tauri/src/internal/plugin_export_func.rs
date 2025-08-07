use chrono::Local;
use log::{debug, error, info, warn};
use regex::Regex;
use rhai::{Dynamic, Engine};
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, time::Duration};

pub fn set_plugin_export_func(engine: &mut Engine) {
    // 注册内置函数
    engine.register_fn("base64_encode", base64_encode);
    engine.register_fn("base64_decode", base64_decode);
    engine.register_fn("json_parse", json_parse);
    engine.register_fn("json_stringify", json_stringify);
    engine.register_fn("http_request", http_request);
    engine.register_fn("print_debug", print_debug);
    engine.register_fn("print_info", print_info);
    engine.register_fn("print_warn", print_warn);
    engine.register_fn("print_error", print_error);
    engine.register_fn("is_map", is_map);
    engine.register_fn("is_string", is_string);

    engine.register_fn("parse_json", parse_json);

    engine.register_fn("is_match", |text: &str, pattern: &str| -> bool {
        Regex::new(pattern).unwrap().is_match(text)
    });

    engine.register_fn("url_encode", |text: &str| -> String {
        url::form_urlencoded::byte_serialize(text.as_bytes())
            .collect::<Vec<_>>()
            .concat()
    });

    //生成一个now 生成当前时间的方法给到rhai脚本使用
    engine.register_fn("now", || -> String {
        let now = Local::now();
        now.format("%Y-%m-%d %H:%M:%S").to_string()
    });

    //注册一个rand 生成随机数的方法给到rhai脚本使用
    engine.register_fn("rand", || -> f64 {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen::<f64>() // 生成0到1之间的随机浮点数
    });

    // 注册正则提取函数
    engine.register_fn("regex_matches", |text: &str, pattern: &str| -> Dynamic {
        let re = Regex::new(pattern).unwrap();
        let matches: rhai::Array = re
            .find_iter(text)
            .map(|m| m.as_str().to_string().into())
            .collect();
        Dynamic::from(matches)
    });
}

/// Base64编码
fn base64_encode(text: &str) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, text)
}

/// Base64解码
fn base64_decode(text: &str) -> String {
    match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, text) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => "Invalid UTF-8 in decoded Base64".to_string(),
        },
        Err(_) => "Invalid Base64 string".to_string(),
    }
}

/// JSON解析
fn json_parse(json: &str) -> rhai::Dynamic {
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(value) => value_to_dynamic(value),
        Err(_) => rhai::Dynamic::UNIT,
    }
}

fn parse_json(json: &str) -> rhai::Dynamic {
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(value) => value_to_dynamic(value),
        Err(_) => rhai::Dynamic::UNIT,
    }
}

/// 将JSON值转换为Rhai动态值
pub fn value_to_dynamic(value: serde_json::Value) -> rhai::Dynamic {
    match value {
        serde_json::Value::Null => rhai::Dynamic::UNIT,
        serde_json::Value::Bool(b) => b.into(),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into()
            } else if let Some(f) = n.as_f64() {
                f.into()
            } else {
                rhai::Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => s.into(),
        serde_json::Value::Array(arr) => {
            let mut rhai_array = rhai::Array::new();
            for val in arr {
                rhai_array.push(value_to_dynamic(val));
            }
            rhai_array.into()
        }
        serde_json::Value::Object(obj) => {
            let mut rhai_map = rhai::Map::new();
            for (key, val) in obj {
                rhai_map.insert(key.into(), value_to_dynamic(val));
            }
            rhai_map.into()
        }
    }
}

/// JSON序列化
fn json_stringify(value: rhai::Dynamic) -> String {
    let json_value = dynamic_to_json_value(value);
    serde_json::to_string(&json_value).unwrap_or_else(|_| "{}".to_string())
}

/// 将Rhai动态值转换为JSON值
pub fn dynamic_to_json_value(value: rhai::Dynamic) -> serde_json::Value {
    if value.type_name() == "()" {
        return serde_json::Value::Null;
    }

    if let Ok(b) = value.clone().as_bool() {
        return serde_json::Value::Bool(b);
    }

    if let Ok(s) = value.clone().into_string() {
        return serde_json::Value::String(s);
    }

    if let Ok(i) = value.clone().as_int() {
        return serde_json::Value::Number(serde_json::Number::from(i));
    }

    if let Ok(f) = value.clone().as_float() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return serde_json::Value::Number(n);
        }
        return serde_json::Value::Null;
    }

    if value.type_name().contains("Array") {
        if let Ok(arr) = value.clone().into_array() {
            let values: Vec<serde_json::Value> =
                arr.into_iter().map(|v| dynamic_to_json_value(v)).collect();
            return serde_json::Value::Array(values);
        }
    }

    if value.type_name().contains("Map") {
        let map_result = value.try_cast::<rhai::Map>();
        if let Some(map) = map_result {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                obj.insert(k.to_string(), dynamic_to_json_value(v));
            }
            return serde_json::Value::Object(obj);
        }
    }

    serde_json::Value::Null
}

/// 调试日志
fn print_debug(message: &str) -> () {
    debug!("[PLUGIN] {}", message);
}

/// 信息日志
fn print_info(message: &str) -> () {
    info!("[PLUGIN] {}", message);
}

/// 警告日志
fn print_warn(message: &str) -> () {
    warn!("[PLUGIN] {}", message);
}

/// 错误日志
fn print_error(message: &str) -> () {
    error!("[PLUGIN] {}", message);
}

/// 检查是否为Map类型
fn is_map(value: rhai::Dynamic) -> bool {
    value.is_map()
}

/// 检查是否为字符串类型
fn is_string(value: rhai::Dynamic) -> bool {
    value.is_string()
}

/// HTTP请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestParams {
    pub url: String,
    pub method: String,
    pub headers: Option<HashMap<String, String>>,
    pub params: Option<HashMap<String, String>>,
    pub body: Option<String>,
    pub timeout: Option<u32>,
    pub proxy_url: Option<String>,
    pub follow_redirects: Option<bool>,
    pub max_redirects: Option<u32>,
}

/// HTTP响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// HTTP请求函数
fn http_request(json_params: &str) -> String {
    // 解析请求参数
    let params: HttpRequestParams = match serde_json::from_str(json_params) {
        Ok(p) => p,
        Err(e) => {
            let error = format!("{{\"error\": \"Invalid request parameters: {}\"}}", e);
            error!("{}", error);
            return error;
        }
    };
    // println!("params: {:?}", params);
    // 创建HTTP客户端
    let client_builder = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(params.timeout.unwrap_or(30) as u64))
        .danger_accept_invalid_certs(true);

    // 设置代理
    let client_builder = if let Some(proxy_url) = &params.proxy_url {
        if proxy_url.is_empty() {
            client_builder
        } else {
            match reqwest::Proxy::all(proxy_url) {
                Ok(proxy) => client_builder.proxy(proxy),
                Err(e) => {
                    let error = format!("{{\"error\": \"Invalid proxy URL: {}\"}}", e);
                    return error;
                }
            }
        }
    } else {
        client_builder
    };

    // 设置重定向策略
    let client_builder = if let Some(follow_redirects) = params.follow_redirects {
        if follow_redirects {
            let max = params.max_redirects.unwrap_or(10);
            client_builder.redirect(reqwest::redirect::Policy::limited(max as usize))
        } else {
            client_builder.redirect(reqwest::redirect::Policy::none())
        }
    } else {
        client_builder
    };

    // 创建客户端
    let client = match client_builder.build() {
        Ok(client) => client,
        Err(e) => {
            let error = format!("{{\"error\": \"Failed to create HTTP client: {}\"}}", e);
            return error;
        }
    };

    // 执行请求
    process_request(client, params)
}

/// 处理HTTP请求
fn process_request(client: reqwest::blocking::Client, params: HttpRequestParams) -> String {
    let method = match params.method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "HEAD" => reqwest::Method::HEAD,
        "OPTIONS" => reqwest::Method::OPTIONS,
        "PATCH" => reqwest::Method::PATCH,
        _ => reqwest::Method::GET,
    };

    // 创建请求
    let mut request_builder = client.request(method, &params.url);

    // 添加URL参数
    if let Some(query_params) = params.params {
        request_builder = request_builder.query(&query_params);
    }

    // 添加请求头
    if let Some(headers) = params.headers {
        for (name, value) in headers {
            request_builder = request_builder.header(name, value);
        }
    }

    // 添加请求体
    if let Some(body) = params.body {
        request_builder = request_builder.body(body);
    }

    // 执行请求
    match request_builder.send() {
        Ok(response) => {
            let status = response.status().as_u16();

            // 获取响应头
            let mut headers = HashMap::new();
            let mut cookies = Vec::new();

            for (key, value) in response.headers() {
                if let Ok(v) = value.to_str() {
                    let key_str = key.as_str().to_string();
                    if key_str.to_lowercase() == "set-cookie" {
                        cookies.push(v.to_string());
                    } else {
                        headers.insert(key_str, v.to_string());
                    }
                }
            }

            // 如果有多个Set-Cookie，将它们合并为一个数组
            if !cookies.is_empty() {
                headers.insert(
                    "Set-Cookie".to_string(),
                    serde_json::to_string(&cookies).unwrap_or_default(),
                );
            }

            // 获取响应体
            match response.text() {
                Ok(body) => {
                    let response = HttpResponse {
                        status_code: status,
                        headers,
                        body,
                    };

                    match serde_json::to_string(&response) {
                        Ok(json) => json,
                        Err(e) => format!("{{\"error\": \"Failed to serialize response: {}\"}}", e),
                    }
                }
                Err(e) => format!("{{\"error\": \"Failed to get response body: {}\"}}", e),
            }
        }
        Err(e) => format!("{{\"error\": \"Request failed: {}\"}}", e),
    }
}
