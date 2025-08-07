use anyhow::{anyhow, Result};
use log::{debug, error, info};
use rhai::{Engine, Scope, AST, Dynamic};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;

use crate::internal::plugin_export_func::set_plugin_export_func;
use super::vuln_commands;

/// 插件上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    pub target: String,
    pub custom_params: Option<HashMap<String, Value>>,
}

/// 插件请求结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRequest {
    pub target: String,
    pub params: Option<HashMap<String, Value>>,
}

/// 插件执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    pub success: bool,
    pub details: String,
    pub data: Value,
    pub raw_output: String,
    pub request: Option<String>,        // HTTP请求内容
    pub response: Option<String>,       // HTTP响应内容
    pub status_code: Option<u16>,       // HTTP状态码
    pub status_text: Option<String>,    // HTTP状态描述
    pub request_method: Option<String>, // HTTP请求方法
    pub request_url: Option<String>,    // 请求URL
}

/// HTTP请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestParams {
    pub url: String,
    pub method: String,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
    pub timeout: Option<u32>,
    pub proxy_url: Option<String>,
    pub follow_redirects: Option<bool>,
    pub max_redirects: Option<u32>,
}

/// HTTP响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// 插件参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub key: String,
    pub r#type: String,
    pub required: bool,
    #[serde(alias = "default_value")]
    pub default: Option<Value>,
    pub description: String,
    pub options: Option<Vec<Value>>,
}

/// 结果字段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultFieldDef {
    pub name: String,
    pub key: String,
    pub r#type: String,
    pub description: String,
}

/// 插件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub rtype: String,
    pub severity: Option<String>,
    pub references: Option<Vec<String>>,
    pub params: Vec<ParamDef>,
    pub result_fields: Vec<ResultFieldDef>,
}

/// Rhai脚本插件
#[derive(Debug, Clone)]
pub struct RhaiPlugin {
    manifest: PluginManifest,
    script: String,
    ast: Arc<AST>,
    path: PathBuf,
}

/// Rhai插件管理器
pub struct VulnRhaiPluginManager {
    plugin_dir: PathBuf,
    plugins: Arc<Mutex<HashMap<String, RhaiPlugin>>>,
    engine: Arc<Engine>,
}

impl RhaiPlugin {
    /// 创建新插件
    pub fn new(manifest: PluginManifest, script: String, ast: AST, path: PathBuf) -> Self {
        Self {
            manifest,
            script,
            ast: Arc::new(ast),
            path,
        }
    }

    /// 获取插件名称
    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    /// 获取插件描述
    pub fn description(&self) -> &str {
        &self.manifest.description
    }

    /// 获取插件元数据
    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    /// 获取脚本内容
    pub fn script(&self) -> &str {
        &self.script
    }

    /// 获取插件路径
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl VulnRhaiPluginManager {
    /// 创建新的插件管理器
    pub fn new(plugin_dir: PathBuf) -> Result<Self> {
        // 确保插件目录存在
        if !plugin_dir.exists() {
            std::fs::create_dir_all(&plugin_dir)?;
        }

        // 创建Rhai引擎
        let mut engine = Engine::new();

        set_plugin_export_func(&mut engine);
        
        // 禁用复杂度检查和增加限制
        engine.set_max_expr_depths(0, 0);  // 0 means no limit
        engine.set_max_string_size(0);  // No limit on string size
        engine.set_max_array_size(0);  // No limit on array size
        engine.set_max_map_size(0);  // No limit on map size
        engine.set_optimization_level(rhai::OptimizationLevel::Full);
        engine.set_strict_variables(false);  // Less strict variable access

        engine.register_fn("generate_shiro_rce_payload_external", vuln_commands::generate_shiro_gadget_payload);
        
        // 创建管理器
        Ok(Self {
            plugin_dir,
            plugins: Arc::new(Mutex::new(HashMap::new())),
            engine: Arc::new(engine),
        })
    }

    /// 获取插件目录
    pub fn plugin_dir(&self) -> &PathBuf {
        &self.plugin_dir
    }

    /// 加载所有插件
    pub async fn load_all_plugins(&self) -> Result<()> {
        info!("正在加载Rhai脚本插件...");

        // 清空插件列表
        let mut plugins = self.plugins.lock().await;
        plugins.clear();

        // 确保插件目录存在
        if !self.plugin_dir.exists() {
            std::fs::create_dir_all(&self.plugin_dir)?;
        }

        // 遍历插件目录
        for entry in std::fs::read_dir(&self.plugin_dir)? {
            let entry = entry?;
            let path = entry.path();

            // 仅处理.rhai文件
            if path.is_file() && path.extension().map_or(false, |ext| ext == "rhai") {
                match self.load_plugin(&path).await {
                    Ok(plugin) => {
                        let plugin_key = format!("{}:{}", plugin.manifest.rtype, plugin.manifest.name);
                        // info!("已加载插件: {}", plugin_key);
                        plugins.insert(plugin_key, plugin);
                    }
                    Err(e) => {
                        error!("加载插件失败 {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!("已加载{}个Rhai脚本插件", plugins.len());
        Ok(())
    }

    /// 加载单个Rhai脚本插件
    pub async fn load_plugin(&self, path: &Path) -> Result<RhaiPlugin> {
        // 读取脚本文件内容
        let script = std::fs::read_to_string(path)?;

        // 编译脚本到AST
        let ast = self.engine.compile(&script)?;

        // 执行插件的get_manifest函数获取元数据
        let manifest = self.extract_plugin_manifest(&ast)?;

        Ok(RhaiPlugin::new(manifest, script, ast, path.to_path_buf()))
    }

    /// 从脚本中提取插件元数据
    fn extract_plugin_manifest(&self, ast: &AST) -> Result<PluginManifest> {
        let mut scope = Scope::new();
        
        // 执行get_manifest函数
        let result: String = self.engine.call_fn(&mut scope, ast, "get_manifest", ())?;
        
        // 解析JSON
        let manifest: PluginManifest = serde_json::from_str(&result)?;
        
        Ok(manifest)
    }

    /// 执行Rhai脚本插件
    pub async fn execute_plugin(
        &self,
        plugin_name: &str,
        plugin_type: &str,
        context: PluginContext,
    ) -> Result<PluginResult> {
        let request = PluginRequest {
            target: context.target,
            params: context.custom_params,
        };

        self.execute_plugin_impl(plugin_type, plugin_name, request).await
    }

    /// 实际执行Rhai脚本插件的实现
    async fn execute_plugin_impl(
        &self,
        namespace: &str,
        name: &str,
        request: PluginRequest,
    ) -> Result<PluginResult> {
        // 获取插件
        let plugin_key = format!("{}:{}", namespace, name);
        info!("Executing plugin: {}", plugin_key);
        
        let plugins = self.plugins.lock().await;
        let plugin = plugins.get(&plugin_key).ok_or_else(|| {
            error!("Plugin not found: {}", plugin_key);
            anyhow!("插件未找到: {}", plugin_key)
        })?;

        // 转换请求为JSON字符串
        let request_json = serde_json::to_string(&request)?;
        debug!("Plugin request: {}", request_json);

        // Clone what we need for the blocking task
        let engine = self.engine.clone();
        let ast = plugin.ast.clone();
        let plugin_key_clone = plugin_key.clone();
        let request_json_clone = request_json.clone();

        // 使用spawn_blocking执行Rhai引擎的阻塞操作
        let result = task::spawn_blocking(move || {
            // 创建新的作用域
            let mut scope = Scope::new();
            
            // 执行插件的analyze函数
            info!("Calling analyze function for plugin {}", plugin_key_clone);
            engine.call_fn::<String>(&mut scope, &ast, "analyze", (request_json_clone,))
        }).await?;

        match result {
            Ok(result) => {
                
                // 解析结果
                let plugin_result: PluginResult = match serde_json::from_str(&result) {
                    Ok(res) => res,
                    Err(e) => {
                        error!("Error parsing plugin result: {}", e);
                        return Ok(PluginResult {
                            success: false,
                            details: format!("解析插件结果时出错: {}", e),
                            data: serde_json::json!({"error": format!("{}", e)}),
                            raw_output: result,
                            request: None,
                            response: None,
                            status_code: None,
                            status_text: None,
                            request_method: None,
                            request_url: None,
                        });
                    }
                };
                
                Ok(plugin_result)
            },
            Err(e) => {
                error!("Error executing plugin {}: {}", plugin_key, e);
                Ok(PluginResult {
                    success: false,
                    details: format!("执行插件时出错: {}", e),
                    data: serde_json::json!({"error": format!("{}", e)}),
                    raw_output: String::new(),
                    request: None,
                    response: None,
                    status_code: None,
                    status_text: None,
                    request_method: None,
                    request_url: None,
                })
            }
        }
    }

    /// 添加插件
    pub async fn add_plugin(&mut self, plugin: RhaiPlugin) -> Result<()> {
        let plugin_key = format!("{}:{}", plugin.manifest.rtype, plugin.manifest.name);
        let mut plugins = self.plugins.lock().await;
        plugins.insert(plugin_key, plugin);
        Ok(())
    }

    /// 移除插件
    pub async fn remove_plugin(&mut self, plugin_name: &str) -> Option<RhaiPlugin> {
        let mut plugins = self.plugins.lock().await;
        plugins.remove(plugin_name)
    }

    /// 获取插件
    pub async fn get_plugin(&self, plugin_name: &str) -> Option<RhaiPlugin> {
        let plugins = self.plugins.lock().await;
        plugins.get(plugin_name).cloned()
    }

    /// 获取所有插件
    pub async fn get_all_plugins(&self) -> Vec<PluginManifest> {
        let plugins = self.plugins.lock().await;
        plugins.values().map(|p| p.manifest.clone()).collect()
    }

    /// 获取插件元数据
    pub async fn get_plugin_manifest(&self, namespace: &str, name: &str) -> Option<PluginManifest> {
        let plugin_key = format!("{}:{}", namespace, name);
        let plugins = self.plugins.lock().await;
        plugins.get(&plugin_key).map(|p| p.manifest.clone())
    }

} 