use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use tauri::State;
use tokio::sync::Mutex;
use serde_json::Value;
use crate::handler::asm::plugin::{AsmPluginManager, AsmPluginContext, AsmPluginResult};
use log::{error, info, warn};
use std::fs;
use std::io::Write;
use serde::{Deserialize, Serialize};

// 插件信息结构
#[derive(Serialize)]
pub struct AsmPluginInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub plugin_type: String,
    pub severity: Option<String>,
    pub references: Option<Vec<String>>,
    pub params: Vec<AsmParamInfoDto>,
    pub result_fields: Vec<AsmResultFieldInfoDto>,
    pub script: Option<String>,
}

// 插件参数定义传输对象
#[derive(Serialize)]
pub struct AsmParamInfoDto {
    pub name: String,
    pub key: String,
    pub r#type: String,
    pub required: bool,
    pub default: Option<Value>,
    pub description: String,
}

// 结果字段定义传输对象
#[derive(Serialize)]
pub struct AsmResultFieldInfoDto {
    pub name: String,
    pub key: String,
    pub r#type: String,
    pub description: String,
}

/// 共享状态
pub struct AsmPluginManagerState {
    pub inner: Arc<Mutex<AsmPluginManager>>,
}

impl AsmPluginManagerState {
    pub fn new(manager: AsmPluginManager) -> Self {
        Self {
            inner: Arc::new(Mutex::new(manager)),
        }
    }
}

/// 初始化ASM插件管理器
pub async fn init_asm_plugin_manager(plugin_dir: PathBuf) -> Result<AsmPluginManagerState> {
    let manager = AsmPluginManager::new(plugin_dir)?;
    manager.load_all_plugins().await?;
    Ok(AsmPluginManagerState::new(manager))
}

/// 列出所有ASM插件
#[tauri::command]
pub async fn list_asm_plugins(state: State<'_, AsmPluginManagerState>) -> Result<Vec<AsmPluginInfo>, String> {
    let manager = state.inner.lock().await;
    
    let manifests = manager.get_all_plugins().await;
    
    let plugin_infos = manifests.into_iter().map(|manifest| {
        // 转换参数
        let param_dtos = manifest.params.iter().map(|param| {
            AsmParamInfoDto {
                name: param.name.clone(),
                key: param.key.clone(),
                r#type: param.r#type.clone(),
                required: param.required,
                default: param.default.clone(),
                description: param.description.clone(),
            }
        }).collect();
        
        // 转换结果字段
        let result_field_dtos = manifest.result_fields.iter().map(|field| {
            AsmResultFieldInfoDto {
                name: field.name.clone(),
                key: field.key.clone(),
                r#type: field.r#type.clone(),
                description: field.description.clone(),
            }
        }).collect();
        
        // 创建插件信息对象
        AsmPluginInfo {
            id: format!("{}:{}", manifest.plugin_type, manifest.name),
            name: manifest.name,
            description: manifest.description,
            author: manifest.author,
            version: manifest.version,
            plugin_type: manifest.plugin_type,
            severity: manifest.severity.clone(),
            references: manifest.references.clone(),
            params: param_dtos,
            result_fields: result_field_dtos,
            script: None, // 初始化不包含脚本内容，节省带宽
        }
    }).collect();
    
    Ok(plugin_infos)
}

/// 获取单个ASM插件详情（包含脚本内容）
#[tauri::command]
pub async fn get_asm_plugin(
    plugin_id: String,
    state: State<'_, AsmPluginManagerState>
) -> Result<AsmPluginInfo, String> {
    info!("Fetching ASM plugin with ID: {}", plugin_id);
    
    let manager = state.inner.lock().await;
    
    // 拆分插件ID获取类型和名称
    let parts: Vec<&str> = plugin_id.split(':').collect();
    if parts.len() != 2 {
        let err_msg = format!("Invalid plugin ID format: {}, expected format 'type:name'", plugin_id);
        error!("{}", err_msg);
        return Err(err_msg);
    }
    
    let plugin_type = parts[0];
    let plugin_name = parts[1];
    
    info!("Looking for ASM plugin type: {}, name: {}", plugin_type, plugin_name);
    
    // 获取插件
    let manifest = match manager.get_plugin_manifest(plugin_type, plugin_name).await {
        Some(m) => m,
        None => {
            let err_msg = format!("ASM Plugin manifest not found: {}:{}", plugin_type, plugin_name);
            error!("{}", err_msg);
            return Err(err_msg);
        }
    };
    
    // 获取插件脚本内容
    let plugin = match manager.get_plugin(&format!("{}:{}", plugin_type, plugin_name)).await {
        Some(p) => p,
        None => {
            let err_msg = format!("ASM Plugin not found: {}:{}", plugin_type, plugin_name);
            error!("{}", err_msg);
            return Err(err_msg);
        }
    };
    
    // 转换参数
    let param_dtos = manifest.params.iter().map(|param| {
        AsmParamInfoDto {
            name: param.name.clone(),
            key: param.key.clone(),
            r#type: param.r#type.clone(),
            required: param.required,
            default: param.default.clone(),
            description: param.description.clone(),
        }
    }).collect();
    
    // 转换结果字段
    let result_field_dtos = manifest.result_fields.iter().map(|field| {
        AsmResultFieldInfoDto {
            name: field.name.clone(),
            key: field.key.clone(),
            r#type: field.r#type.clone(),
            description: field.description.clone(),
        }
    }).collect();
    
    // 记录日志信息 - 使用引用避免移动
    info!("Successfully found ASM plugin {}:{}, script length: {}", 
        &manifest.plugin_type, &manifest.name, plugin.script().len());
    
    // 创建插件信息对象
    let plugin_info = AsmPluginInfo {
        id: format!("{}:{}", manifest.plugin_type, manifest.name),
        name: manifest.name,
        description: manifest.description,
        author: manifest.author,
        version: manifest.version,
        plugin_type: manifest.plugin_type,
        severity: manifest.severity.clone(),
        references: manifest.references.clone(),
        params: param_dtos,
        result_fields: result_field_dtos,
        script: Some(plugin.script().to_string()),
    };
    
    Ok(plugin_info)
}

/// 重新加载所有ASM插件
#[tauri::command]
pub async fn load_asm_plugins(state: State<'_, AsmPluginManagerState>) -> Result<String, String> {
    let manager = state.inner.lock().await;
    
    match manager.load_all_plugins().await {
        Ok(_) => Ok("Successfully loaded all ASM plugins".to_string()),
        Err(e) => Err(format!("Failed to load ASM plugins: {}", e)),
    }
}

/// 测试ASM插件
#[tauri::command]
pub async fn test_asm_plugin(
    plugin_id: String,
    state: State<'_, AsmPluginManagerState>
) -> Result<AsmPluginResult, String> {  
    
    let manager = state.inner.lock().await;

    // 拆分插件ID
    let parts: Vec<&str> = plugin_id.split(':').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid plugin ID format: {}, expected format 'type:name'", plugin_id));
    }

    let plugin_type = parts[0];
    let plugin_name = parts[1];


    let context = AsmPluginContext {
        task_id: 1,
        target: "example.com".to_string(),
        targets: None,
        custom_params: None,
    };
    
    // 执行插件
    match manager.execute_plugin(plugin_name, plugin_type, context).await {
        Ok(result) => {
            info!("ASM Plugin execution completed: {}:{}", plugin_type, plugin_name);
            Ok(result)
        },
        Err(e) => {
            error!("ASM Plugin execution failed: {}", e);
            Err(format!("Failed to execute ASM plugin: {}", e))
        }
    }
}

/// 执行ASM插件
#[tauri::command]
pub async fn execute_asm_plugin(
    params: serde_json::Value,
    state: State<'_, AsmPluginManagerState>
) -> Result<AsmPluginResult, String> {
    let manager = state.inner.lock().await;
    
    // 提取插件ID
    let plugin_id = params.get("plugin_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing plugin_id parameter".to_string())?;
    
    // 拆分插件ID
    let parts: Vec<&str> = plugin_id.split(':').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid plugin ID format: {}, expected format 'type:name'", plugin_id));
    }
    
    let plugin_type = parts[0];
    let plugin_name = parts[1];
    
    info!("Executing ASM plugin: {}:{}", plugin_type, plugin_name);
    
    // 提取任务ID
    let task_id = params.get("task_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing task_id parameter".to_string())?;
    
    // 提取目标
    let target = params.get("target")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing target parameter".to_string())?;
    
    // 提取可选的多目标数组
    let targets = params.get("targets")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        });
    
    // 提取自定义参数
    let custom_params = params.get("params")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        });
    
    // 创建插件上下文
    let context = AsmPluginContext {
        task_id: task_id as i32,
        target,
        targets,
        custom_params,
    };
    
    // 执行插件
    match manager.execute_plugin(plugin_name, plugin_type, context).await {
        Ok(result) => {
            info!("ASM Plugin execution completed: {}:{}", plugin_type, plugin_name);
            Ok(result)
        },
        Err(e) => {
            error!("ASM Plugin execution failed: {}", e);
            Err(format!("Failed to execute ASM plugin: {}", e))
        }
    }
}

// 插件上传请求
#[derive(Deserialize)]
pub struct UploadPluginRequest {
    pub filename: String,
    pub content: String,
}

/// 上传ASM插件内容
#[tauri::command]
pub async fn upload_asm_plugin_content(
    filename: String,
    content: String,
    state: State<'_, AsmPluginManagerState>
) -> Result<AsmPluginInfo, String> {
    info!("Uploading ASM plugin content: {}", filename);
    
    let mut manager = state.inner.lock().await;
    
    // 确保文件名有效
    if !filename.ends_with(".rhai") {
        return Err("Plugin filename must end with .rhai".to_string());
    }
    
    // 获取插件目录
    let plugin_dir = manager.plugin_dir().clone();
    
    // 创建完整的文件路径
    let file_path = plugin_dir.join(&filename);
    
    // 将内容写入文件
    let mut file = fs::File::create(&file_path)
        .map_err(|e| format!("Failed to create plugin file: {}", e))?;
    
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write plugin content: {}", e))?;
    
    // 加载插件
    let plugin = manager.load_plugin(&file_path).await
        .map_err(|e| format!("Failed to load plugin: {}", e))?;
    
    // 获取插件元数据
    let manifest = plugin.manifest().clone();
    
    // 添加到插件管理器
    manager.add_plugin(plugin).await
        .map_err(|e| format!("Failed to add plugin: {}", e))?;
    
    // 转换参数
    let param_dtos = manifest.params.iter().map(|param| {
        AsmParamInfoDto {
            name: param.name.clone(),
            key: param.key.clone(),
            r#type: param.r#type.clone(),
            required: param.required,
            default: param.default.clone(),
            description: param.description.clone(),
        }
    }).collect();
    
    // 转换结果字段
    let result_field_dtos = manifest.result_fields.iter().map(|field| {
        AsmResultFieldInfoDto {
            name: field.name.clone(),
            key: field.key.clone(),
            r#type: field.r#type.clone(),
            description: field.description.clone(),
        }
    }).collect();
    
    // 创建插件信息对象
    let plugin_info = AsmPluginInfo {
        id: format!("{}:{}", manifest.plugin_type, manifest.name),
        name: manifest.name,
        description: manifest.description,
        author: manifest.author,
        version: manifest.version,
        plugin_type: manifest.plugin_type,
        severity: manifest.severity.clone(),
        references: manifest.references.clone(),
        params: param_dtos,
        result_fields: result_field_dtos,
        script: Some(content),
    };
    
    info!("Successfully uploaded and loaded ASM plugin: {}", filename);
    
    Ok(plugin_info)
}

/// 删除ASM插件
#[tauri::command]
pub async fn delete_asm_plugin(
    plugin_name: String,
    state: State<'_, AsmPluginManagerState>
) -> Result<bool, String> {
    info!("=== 开始删除ASM插件: {} ===", plugin_name);
    
    let mut manager = state.inner.lock().await;
    
    // 检查插件目录中所有插件的信息，帮助调试
    let plugin_dir = manager.plugin_dir().clone();
    info!("插件目录: {}", plugin_dir.display());
    
    if let Ok(entries) = fs::read_dir(&plugin_dir) {
        info!("目录中的文件:");
        for entry in entries {
            if let Ok(entry) = entry {
                info!("  - {}", entry.path().display());
            }
        }
    }
    
    // 检查是否是文件名还是插件ID格式
    if plugin_name.ends_with(".rhai") {
        // 如果是文件名，尝试查找其对应的插件
        let file_path = plugin_dir.join(&plugin_name);
        info!("尝试删除文件: {}", file_path.display());
        
        // 检查文件是否存在
        if file_path.exists() {
            info!("文件存在，开始删除");
            // 删除文件
            if let Err(e) = fs::remove_file(&file_path) {
                let err_msg = format!("Failed to delete plugin file: {}", e);
                warn!("{}", err_msg);
                return Err(err_msg);
            }
            
            info!("文件删除成功，开始重新加载插件列表");
            
            // 重新加载所有插件以更新插件列表
            if let Err(e) = manager.load_all_plugins().await {
                warn!("Failed to reload plugins after deletion: {}", e);
            }
            
            info!("插件删除成功: {}", plugin_name);
            return Ok(true);
        } else {
            let err_msg = format!("Plugin file not found: {}", file_path.display());
            warn!("{}", err_msg);
            return Err(err_msg);
        }
    } else {
        // 如果是插件ID格式，正常走移除流程
        info!("尝试通过ID格式删除插件: {}", plugin_name);
        
        if let Some(plugin) = manager.remove_plugin(&plugin_name).await {
            info!("插件已从内存移除，准备删除文件: {}", plugin.path().display());
            
            // 删除文件
            if let Err(e) = fs::remove_file(plugin.path()) {
                warn!("Failed to delete plugin file: {}", e);
            } else {
                info!("插件文件删除成功");
            }
            
            info!("插件删除成功: {}", plugin_name);
            Ok(true)
        } else {
            let err_msg = format!("Plugin not found: {}", plugin_name);
            warn!("{}", err_msg);
            Err(err_msg)
        }
    }
}

/// 更新ASM插件
#[tauri::command]
pub async fn update_asm_plugin(
    plugin_id: String,
    _name: String,
    _description: String,
    script: String,
    state: State<'_, AsmPluginManagerState>
) -> Result<AsmPluginInfo, String> {
    info!("Updating ASM plugin with ID: {}", plugin_id);
    
    let mut manager = state.inner.lock().await;
    
    // 拆分插件ID
    let parts: Vec<&str> = plugin_id.split(':').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid plugin ID format: {}, expected format 'type:name'", plugin_id));
    }
    
    let plugin_type = parts[0];
    let plugin_name = parts[1];
    
    // 获取现有插件
    let plugin = match manager.get_plugin(&format!("{}:{}", plugin_type, plugin_name)).await {
        Some(p) => p,
        None => return Err(format!("Plugin not found: {}:{}", plugin_type, plugin_name)),
    };
    
    // 获取文件路径
    let file_path = plugin.path().to_path_buf();
    
    // 将内容写入文件
    let mut file = fs::File::create(&file_path)
        .map_err(|e| format!("Failed to update plugin file: {}", e))?;
    
    file.write_all(script.as_bytes())
        .map_err(|e| format!("Failed to write plugin content: {}", e))?;
    
    // 重新加载插件
    let updated_plugin = manager.load_plugin(&file_path).await
        .map_err(|e| format!("Failed to reload plugin: {}", e))?;
    
    let manifest = updated_plugin.manifest().clone();
    
    // 更新插件
    manager.add_plugin(updated_plugin).await
        .map_err(|e| format!("Failed to update plugin in manager: {}", e))?;
    
    // 转换参数
    let param_dtos = manifest.params.iter().map(|param| {
        AsmParamInfoDto {
            name: param.name.clone(),
            key: param.key.clone(),
            r#type: param.r#type.clone(),
            required: param.required,
            default: param.default.clone(),
            description: param.description.clone(),
        }
    }).collect();
    
    // 转换结果字段
    let result_field_dtos = manifest.result_fields.iter().map(|field| {
        AsmResultFieldInfoDto {
            name: field.name.clone(),
            key: field.key.clone(),
            r#type: field.r#type.clone(),
            description: field.description.clone(),
        }
    }).collect();
    
    // 创建插件信息对象
    let plugin_info = AsmPluginInfo {
        id: format!("{}:{}", manifest.plugin_type, manifest.name),
        name: manifest.name,
        description: manifest.description,
        author: manifest.author,
        version: manifest.version,
        plugin_type: manifest.plugin_type,
        severity: manifest.severity.clone(),
        references: manifest.references.clone(),
        params: param_dtos,
        result_fields: result_field_dtos,
        script: Some(script),
    };
    
    info!("Successfully updated ASM plugin: {}:{}", plugin_type, plugin_name);
    
    Ok(plugin_info)
} 