use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use tauri::State;
use tokio::sync::Mutex;
use serde_json::Value;
use crate::handler::scan::plugin::{ScanPluginManager, PluginContext, PluginResult};

/// 共享状态
pub struct ScanPluginManagerState {
    pub inner: Arc<Mutex<ScanPluginManager>>,
}

impl ScanPluginManagerState {
    pub fn new(manager: ScanPluginManager) -> Self {
        Self {
            inner: Arc::new(Mutex::new(manager)),
        }
    }
}

/// 初始化扫描插件管理器
pub async fn init_scan_plugin_manager(plugin_dir: PathBuf) -> Result<ScanPluginManagerState> {
    let manager = ScanPluginManager::new(plugin_dir)?;
    manager.load_all_plugins().await?;
    Ok(ScanPluginManagerState::new(manager))
}

/// 列出所有扫描插件
#[tauri::command]
pub async fn list_scan_plugins(state: State<'_, ScanPluginManagerState>) -> Result<Vec<Value>, String> {
    let manager = state.inner.lock().await;
    
    let plugins = manager.get_all_plugins().await;
    
    let plugin_infos = plugins.into_iter().map(|(id, plugin)| {
        // 如果插件有元数据，使用元数据中的信息
        if let Some(manifest) = plugin.manifest() {
            serde_json::json!({
                "id": id,
                "name": manifest.name,
                "author": manifest.author,
                "type": manifest.type_,
                "version": manifest.version,
                "description": manifest.description,
                "severity": manifest.severity,
                "references": manifest.references,
                "parameters": manifest.parameters,
                "script": plugin.script(),
            })
        } else {
            // 否则使用基本信息
            serde_json::json!({
                "id": id,
                "name": id,
                "author": "Unknown",
                "type": "unknown",
                "version": "1.0.0",
                "description": "No description available",
                "script": plugin.script(),
            })
        }
    }).collect();
    
    Ok(plugin_infos)
}

/// 获取单个扫描插件详情
#[tauri::command]
pub async fn get_scan_plugin(
    plugin_id: String,
    state: State<'_, ScanPluginManagerState>
) -> Result<Value, String> {
    let manager = state.inner.lock().await;
    
    let plugin = manager.get_plugin(&plugin_id).await
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;
    
    // 如果插件有元数据，使用元数据中的信息
    if let Some(manifest) = plugin.manifest() {
        Ok(serde_json::json!({
            "id": plugin_id,
            "name": manifest.name,
            "author": manifest.author,
            "type": manifest.type_,
            "version": manifest.version,
            "description": manifest.description,
            "severity": manifest.severity,
            "references": manifest.references,
            "parameters": manifest.parameters,
            "script": plugin.script(),
        }))
    } else {
        // 否则使用基本信息
        Ok(serde_json::json!({
            "id": plugin_id,
            "name": plugin_id,
            "author": "Unknown",
            "type": "unknown",
            "version": "1.0.0",
            "description": "No description available",
            "script": plugin.script(),
        }))
    }
}

/// 加载所有扫描插件
#[tauri::command]
pub async fn reload_scan_plugins(state: State<'_, ScanPluginManagerState>) -> Result<Value, String> {
    let manager = state.inner.lock().await;
    
    manager.load_all_plugins().await
        .map_err(|e| format!("Failed to reload plugins: {}", e))?;
    
    let plugins = manager.get_all_plugins().await;
    
    let plugin_infos = plugins.into_iter().map(|(id, plugin)| {
        // 如果插件有元数据，使用元数据中的信息
        if let Some(manifest) = plugin.manifest() {
            serde_json::json!({
                "id": id,
                "name": manifest.name,
                "author": manifest.author,
                "type": manifest.type_,
                "version": manifest.version,
                "description": manifest.description,
                "severity": manifest.severity,
                "references": manifest.references,
                "parameters": manifest.parameters,
            })
        } else {
            // 否则使用基本信息
            serde_json::json!({
                "id": id,
                "name": id,
                "author": "Unknown",
                "type": "unknown",
                "version": "1.0.0",
                "description": "No description available",
            })
        }
    }).collect::<Vec<Value>>();
    
    Ok(serde_json::json!({
        "status": "success",
        "plugins": plugin_infos,
    }))
}

/// 验证扫描插件脚本
#[tauri::command]
pub async fn validate_scan_plugin(
    script: String,
    state: State<'_, ScanPluginManagerState>
) -> Result<Value, String> {
    let manager = state.inner.lock().await;
    
    match manager.validate_plugin(&script).await {
        Ok(valid) => {
            if valid {
                Ok(serde_json::json!({
                    "valid": true,
                    "message": "插件脚本验证成功"
                }))
            } else {
                Ok(serde_json::json!({
                    "valid": false,
                    "message": "插件脚本必须包含analyze函数"
                }))
            }
        },
        Err(e) => {
            Ok(serde_json::json!({
                "valid": false,
                "message": format!("验证失败: {}", e)
            }))
        }
    }
}

/// 上传扫描插件（直接通过内容）
#[tauri::command]
pub async fn upload_scan_plugin_content(
    filename: String,
    content: String,
    state: State<'_, ScanPluginManagerState>
) -> Result<Value, String> {
    let mut manager = state.inner.lock().await;
    
    // 先验证脚本
    match manager.validate_plugin(&content).await {
        Ok(valid) => {
            if !valid {
                return Ok(serde_json::json!({
                    "status": "error",
                    "message": "插件脚本验证失败，必须包含analyze函数"
                }));
            }
        },
        Err(e) => {
            return Ok(serde_json::json!({
                "status": "error",
                "message": format!("脚本验证失败: {}", e)
            }));
        }
    }
    
    // 获取插件目录
    let plugin_dir = manager.plugin_dir().to_path_buf();
    
    // 确保文件名以.rhai结尾
    let filename = if !filename.ends_with(".rhai") {
        format!("{}.rhai", filename)
    } else {
        filename
    };
    
    // 构造完整的插件路径
    let dest_path = plugin_dir.join(&filename);
    
    // 写入文件内容
    std::fs::write(&dest_path, &content)
        .map_err(|e| format!("Failed to write plugin file: {}", e))?;
    
    // 加载插件
    let plugin = manager.load_plugin(&dest_path).await
        .map_err(|e| format!("Failed to load plugin: {}", e))?;
    
    // 将插件添加到管理器
    manager.add_plugin(plugin.clone()).await
        .map_err(|e| format!("Failed to add plugin: {}", e))?;
    
    // 返回结果，包含元数据信息
    if let Some(manifest) = plugin.manifest() {
        Ok(serde_json::json!({
            "status": "success",
            "message": "Plugin uploaded successfully",
            "plugin": {
                "id": filename.trim_end_matches(".rhai"),
                "name": manifest.name,
                "author": manifest.author,
                "type": manifest.type_,
                "version": manifest.version,
                "description": manifest.description,
            }
        }))
    } else {
        Ok(serde_json::json!({
            "status": "success",
            "message": "Plugin uploaded successfully",
            "plugin": {
                "id": filename.trim_end_matches(".rhai"),
                "name": filename.trim_end_matches(".rhai"),
            }
        }))
    }
}

/// 删除扫描插件
#[tauri::command]
pub async fn delete_scan_plugin(
    plugin_id: String,
    state: State<'_, ScanPluginManagerState>
) -> Result<bool, String> {
    let mut manager = state.inner.lock().await;
    
    // 检查插件是否存在
    let plugin = manager.get_plugin(&plugin_id).await
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;
    
    // 删除文件
    std::fs::remove_file(plugin.path())
        .map_err(|e| format!("Failed to delete plugin file: {}", e))?;
    
    // 从管理器中移除插件
    manager.remove_plugin(&plugin_id).await;
    
    Ok(true)
}

/// 更新扫描插件
#[tauri::command]
pub async fn update_scan_plugin(
    plugin_id: String,
    script: String,
    state: State<'_, ScanPluginManagerState>
) -> Result<Value, String> {
    let manager_state = state.inner.lock().await;
    
    // 先验证脚本
    match manager_state.validate_plugin(&script).await {
        Ok(valid) => {
            if !valid {
                return Ok(serde_json::json!({
                    "status": "error",
                    "message": "插件脚本验证失败，必须包含analyze函数"
                }));
            }
        },
        Err(e) => {
            return Ok(serde_json::json!({
                "status": "error",
                "message": format!("脚本验证失败: {}", e)
            }));
        }
    }
    
    // 获取插件
    let plugin = manager_state.get_plugin(&plugin_id).await
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;
    
    // 获取插件文件路径
    let plugin_path = plugin.path().to_path_buf();
    
    // 把更新内容写入到文件
    std::fs::write(&plugin_path, &script)
        .map_err(|e| format!("Failed to write plugin file: {}", e))?;
    
    // 重新加载插件
    let updated_plugin = manager_state.load_plugin(&plugin_path).await
        .map_err(|e| format!("Failed to reload plugin: {}", e))?;
    
    // 返回更新后的插件信息，包含元数据
    if let Some(manifest) = updated_plugin.manifest() {
        Ok(serde_json::json!({
            "id": plugin_id,
            "name": manifest.name,
            "author": manifest.author,
            "type": manifest.type_,
            "version": manifest.version,
            "description": manifest.description,
            "severity": manifest.severity,
            "references": manifest.references,
            "parameters": manifest.parameters,
            "script": updated_plugin.script(),
        }))
    } else {
        Ok(serde_json::json!({
            "id": plugin_id,
            "name": plugin_id,
            "author": "Unknown",
            "type": "unknown",
            "version": "1.0.0",
            "description": "No description available",
            "script": updated_plugin.script(),
        }))
    }
}

/// 执行扫描插件
#[tauri::command]
pub async fn execute_scan_plugin(
    plugin_id: String,
    target: String,
    params: Option<serde_json::Map<String, Value>>,
    state: State<'_, ScanPluginManagerState>
) -> Result<PluginResult, String> {
    let manager = state.inner.lock().await;
    
    let context = PluginContext {
        target,
        params: params.unwrap_or_default(),
    };
    
    manager.execute_plugin(&plugin_id, context).await
        .map_err(|e| format!("Failed to execute plugin: {}", e))
} 