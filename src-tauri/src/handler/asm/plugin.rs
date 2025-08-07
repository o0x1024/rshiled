use anyhow::{anyhow, Result};
use log::{ error, info, warn};
use rhai::{ Engine, Scope, AST};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task;


use crate::global::config::CoreConfig;
use crate::internal::plugin_export_func::{dynamic_to_json_value, set_plugin_export_func, value_to_dynamic};

/// ASM插件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AsmPluginType {
    #[serde(rename = "domain_discovery")]
    DomainDiscovery,
    #[serde(rename = "risk_scanning")]
    RiskScanning,
    #[serde(rename = "fingerprint")]
    Fingerprint,
    #[serde(rename = "port_scanning")]
    PortScanning,
    #[serde(rename = "other")]
    Other,
}

impl std::fmt::Display for AsmPluginType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AsmPluginType::DomainDiscovery => write!(f, "domain_discovery"),
            AsmPluginType::RiskScanning => write!(f, "risk_scanning"),
            AsmPluginType::Fingerprint => write!(f, "fingerprint"),
            AsmPluginType::PortScanning => write!(f, "port_scanning"),
            AsmPluginType::Other => write!(f, "other"),
        }
    }
}

impl From<&str> for AsmPluginType {
    fn from(s: &str) -> Self {
        match s {
            "domain_discovery" => AsmPluginType::DomainDiscovery,
            "risk_scanning" => AsmPluginType::RiskScanning,
            "fingerprint" => AsmPluginType::Fingerprint,
            "port_scanning" => AsmPluginType::PortScanning,
            _ => AsmPluginType::Other,
        }
    }
}

/// ASM插件上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsmPluginContext {
    pub task_id: i32,
    pub target: String,
    pub targets: Option<Vec<String>>,
    pub custom_params: Option<HashMap<String, Value>>,
}

/// ASM插件请求结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsmPluginRequest {
    pub task_id: i32,
    pub target: String,
    pub targets: Option<Vec<String>>,
    pub params: Option<HashMap<String, Value>>,
    pub proxy_url: Option<String>,
}

/// ASM插件执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsmPluginResult {
    pub success: bool,
    pub message: String,
    pub data: Value,
    pub raw_output: String,
    pub request: Option<String>,
    pub response: Option<String>,
    pub found_domains: Option<Vec<String>>,
    pub found_risks: Option<Vec<Value>>,
    pub found_fingerprints: Option<Vec<Value>>,
    pub found_ports: Option<Vec<Value>>,
}



/// 插件参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsmParamDef {
    pub name: String,
    pub key: String,
    pub r#type: String,
    pub required: bool,
    pub default: Option<Value>,
    pub description: String,
}

/// 结果字段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsmResultFieldDef {
    pub name: String,
    pub key: String,
    pub r#type: String,
    pub description: String,
}

/// ASM插件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsmPluginManifest {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub plugin_type: String,
    pub severity: Option<String>,
    pub references: Option<Vec<String>>,
    pub params: Vec<AsmParamDef>,
    pub result_fields: Vec<AsmResultFieldDef>,
}

/// Rhai脚本插件
#[derive(Debug, Clone)]
pub struct AsmPlugin {
    manifest: AsmPluginManifest,
    script: String,
    ast: Arc<AST>,
    path: PathBuf,
}

/// ASM Rhai插件管理器
pub struct AsmPluginManager {
    plugin_dir: PathBuf,
    plugins: Arc<Mutex<HashMap<String, AsmPlugin>>>,
    engine: Arc<Engine>,
}

impl AsmPlugin {
    /// 创建新插件
    pub fn new(manifest: AsmPluginManifest, script: String, ast: AST, path: PathBuf) -> Self {
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
    pub fn manifest(&self) -> &AsmPluginManifest {
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

impl AsmPluginManager {
    /// 创建新的ASM插件管理器
    pub fn new(plugin_dir: PathBuf) -> Result<Self> {
        // 确保插件目录存在
        if !plugin_dir.exists() {
            fs::create_dir_all(&plugin_dir)?;
        }

        // 创建Rhai引擎
        let mut engine = Engine::new();

        set_plugin_export_func(&mut engine);
       
        // 设置超时和其他限制
        engine.set_max_expr_depths(128, 128);
        engine.set_max_call_levels(64);
        engine.set_optimization_level(rhai::OptimizationLevel::Simple);

        let result = Self {
            plugin_dir,
            plugins: Arc::new(Mutex::new(HashMap::new())),
            engine: Arc::new(engine),
        };

        Ok(result)
    }

    /// 获取插件目录
    pub fn plugin_dir(&self) -> &PathBuf {
        &self.plugin_dir
    }

    /// 加载所有插件
    pub async fn load_all_plugins(&self) -> Result<()> {
        let dir_entries = fs::read_dir(&self.plugin_dir)?;
        
        let mut plugins = self.plugins.lock().await;
        plugins.clear();

        for entry in dir_entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "rhai") {
                match self.load_plugin(&path).await {
                    Ok(plugin) => {
                        let plugin_key =format!("{}:{}", plugin.manifest.plugin_type, plugin.manifest.name);
                        plugins.insert(plugin_key, plugin);
                    }
                    Err(e) => {
                        error!("Failed to load plugin from {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// 加载单个插件
    pub async fn load_plugin(&self, path: &Path) -> Result<AsmPlugin> {
        let script = fs::read_to_string(path)?;
        
        // 编译脚本
        let ast = self.engine.compile(&script)?;
        
        // 提取插件元数据
        let manifest = self.extract_plugin_manifest(&ast)?;
        
        Ok(AsmPlugin::new(manifest, script, ast, path.to_path_buf()))
    }

    /// 从AST中提取插件元数据
    fn extract_plugin_manifest(&self, ast: &AST) -> Result<AsmPluginManifest> {
        let mut scope = Scope::new();
        
        // 首先执行整个脚本以确保所有函数都被定义
        let _ = self.engine.eval_ast_with_scope::<rhai::Dynamic>(&mut scope, ast)?;
        
        // 执行get_manifest函数
        let result: String = self.engine.call_fn(&mut scope, ast, "get_manifest", ())?;
        
        // 解析JSON
        let manifest = serde_json::from_str(&result)?;
                
        // 然后调用 get_manifest 函数
        // match self.engine.call_fn::<String>(&mut scope, ast, "get_manifest", ()) {
        //     Ok(manifest_json) => {
        //         match serde_json::from_str(&manifest_json) {
        //             Ok(manifest) => Ok(manifest),
        //             Err(e) => Err(anyhow!("Failed to parse manifest JSON: {}", e))
        //         }
        //     }
        //     Err(e) => Err(anyhow!("Failed to call get_manifest function: {}", e))
        // }
        Ok(manifest)

    }

    /// 执行ASM插件
    pub async fn execute_plugin(
        &self,
        plugin_name: &str,
        plugin_type: &str,
        context: AsmPluginContext,
    ) -> Result<AsmPluginResult> {
        // let plugin_key = format!("{}:{}", plugin_type, plugin_name);
        
        //获取全局代理
        let proxy_url = match CoreConfig::global() {
            Ok(config) => config.proxy.clone(),
            Err(e) => {
                error!("获取全局代理失败: {}", e);
                None
            }
        };

        let request = AsmPluginRequest {
            task_id: context.task_id,
            target: context.target,
            targets: context.targets,
            params: context.custom_params,
            proxy_url:proxy_url,
        };
        
        self.execute_plugin_impl(plugin_type, plugin_name, request).await
    }

    /// 执行插件的内部实现
    async fn execute_plugin_impl(
        &self,
        namespace: &str,
        name: &str,
        request: AsmPluginRequest,
    ) -> Result<AsmPluginResult> {
        let plugins = self.plugins.lock().await;
        let plugin_key = format!("{}:{}", namespace, name);
        
        let plugin = plugins.get(&plugin_key).ok_or_else(|| {
            anyhow!("Plugin not found: {}", plugin_key)
        })?;
        
        let engine = self.engine.clone();
        let ast = plugin.ast.clone();
        
        let request_json = serde_json::to_string(&request)?;
        let request_json_clone = request_json.clone();

        // 在后台线程中执行插件
        let result = task::spawn_blocking(move || {
            let mut scope = Scope::new();
            
            // 将请求参数传递给脚本
            scope.push("task_id", request.task_id);
            scope.push("target", request.target.clone());
            
            if let Some(targets) = request.targets {
                let targets_array = rhai::Array::from_iter(targets.iter().map(|s| s.clone().into()));
                scope.push("targets", rhai::Dynamic::from(targets_array));
            }
            
            if let Some(params) = request.params {
                let params_iter = params.iter().map(|(k, v)| {
                    (k.clone().into(), value_to_dynamic(v.clone()))
                });
                
                let params_map = rhai::Map::from_iter(params_iter);
                
                scope.push("params", rhai::Dynamic::from(params_map));
            }
            // println!("request_json_clone: {:?}", request_json_clone);
            // 执行脚本
            let _ = engine.eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &ast)?;
            match engine.call_fn::<rhai::Map>(&mut scope, &ast,"analyze", (request_json_clone,)) {
                Ok(result_map) => {
                    // 转换返回结果
                    let mut result = AsmPluginResult {
                        success: false,
                        message: String::new(),
                        data: serde_json::Value::Null,
                        raw_output: String::new(),
                        request: None,
                        response: None,
                        found_domains: None,
                        found_risks: None,
                        found_fingerprints: None,
                        found_ports: None,
                    };
                    
                    if let Some(success) = result_map.get("success") {
                        if let Ok(success_val) = success.clone().as_bool() {
                            result.success = success_val;
                        }
                    }
                    
                    if let Some(message) = result_map.get("message") {
                        if let Ok(message_str) = message.clone().into_string() {
                            result.message = message_str;
                        }
                    }
                    
                    if let Some(raw_output) = result_map.get("raw_output") {
                        if let Ok(raw_output_str) = raw_output.clone().into_string() {
                            result.raw_output = raw_output_str;
                        }
                    }
                    
                    if let Some(data) = result_map.get("data") {
                        result.data = dynamic_to_json_value(data.clone());
                    }
                    
                    if let Some(request) = result_map.get("request") {
                        if let Ok(request_str) = request.clone().into_string() {
                            result.request = Some(request_str);
                        }
                    }
                    
                    if let Some(response) = result_map.get("response") {
                        if let Ok(response_str) = response.clone().into_string() {
                            result.response = Some(response_str);
                        }
                    }
                    
                    if let Some(domains) = result_map.get("found_domains") {
                        if let Ok(array) = domains.clone().into_array() {
                            let domains: Vec<String> = array
                                .into_iter()
                                .filter_map(|v| v.into_string().ok())
                                .collect();
                            
                            if !domains.is_empty() {
                                result.found_domains = Some(domains);
                            }
                        }
                    }
                    
                    if let Some(risks) = result_map.get("found_risks") {
                        if let Ok(array) = risks.clone().into_array() {
                            let risks = array
                                .into_iter()
                                .map(|v| dynamic_to_json_value(v))
                                .collect();
                            
                            result.found_risks = Some(risks);
                        }
                    }
                    
                    if let Some(fingerprints) = result_map.get("found_fingerprints") {
                        if let Ok(array) = fingerprints.clone().into_array() {
                            let fingerprints = array
                                .into_iter()
                                .map(|v| dynamic_to_json_value(v))
                                .collect();
                            
                            result.found_fingerprints = Some(fingerprints);
                        }
                    }
                    
                    if let Some(ports) = result_map.get("found_ports") {
                        if let Ok(array) = ports.clone().into_array() {
                            let ports = array
                                .into_iter()
                                .map(|v| dynamic_to_json_value(v))
                                .collect();
                            
                            result.found_ports = Some(ports);
                        }
                    }
                    
                    Ok(result)
                },
                Err(e) => Err(anyhow!("Failed to execute plugin: {}", e)),
            }
        }).await??;
        
        Ok(result)
    }

    /// 添加插件
    pub async fn add_plugin(&mut self, plugin: AsmPlugin) -> Result<()> {
        let mut plugins = self.plugins.lock().await;
        let plugin_key = format!("{}:{}", plugin.manifest.plugin_type, plugin.manifest.name);
        plugins.insert(plugin_key, plugin);
        Ok(())
    }

    /// 删除插件
    pub async fn remove_plugin(&mut self, plugin_name: &str) -> Option<AsmPlugin> {
        let mut plugins = self.plugins.lock().await;
        
        // 1. 先检查是否直接匹配key
        if let Some(plugin) = plugins.remove(plugin_name) {
            info!("插件通过精确ID匹配删除: {}", plugin_name);
            return Some(plugin);
        }
        
        // 2. 检查是否是文件名，尝试通过文件名查找插件
        if plugin_name.ends_with(".rhai") {
            let base_name = plugin_name.trim_end_matches(".rhai");
            
            // 查找任何包含此名称的插件
            let matching_keys: Vec<String> = plugins.keys()
                .filter(|k| k.ends_with(&format!(":{}", base_name)))
                .cloned()
                .collect();
            
            if !matching_keys.is_empty() {
                let key = matching_keys[0].clone();
                info!("插件通过文件名匹配删除: {} -> {}", plugin_name, key);
                return plugins.remove(&key);
            }
        }
        
        // 3. 尝试作为插件名查找（而非完整ID）
        let matching_keys: Vec<String> = plugins.keys()
            .filter(|k| k.ends_with(&format!(":{}", plugin_name)) || k.contains(&format!(":{}", plugin_name)))
            .cloned()
            .collect();
        
        if !matching_keys.is_empty() {
            let key = matching_keys[0].clone();
            info!("插件通过名称部分匹配删除: {} -> {}", plugin_name, key);
            return plugins.remove(&key);
        }
        
        // 4. 实在找不到，遍历所有插件路径尝试匹配文件名
        let matching_plugin_key = plugins.iter()
            .find(|(_, plugin)| {
                let path = plugin.path();
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                file_name == plugin_name
            })
            .map(|(k, _)| k.clone());
        
        if let Some(key) = matching_plugin_key {
            info!("插件通过文件路径匹配删除: {} -> {}", plugin_name, key);
            return plugins.remove(&key);
        }
        
        warn!("找不到要删除的插件: {}", plugin_name);
        None
    }

    /// 获取插件
    pub async fn get_plugin(&self, plugin_name: &str) -> Option<AsmPlugin> {
        let plugins = self.plugins.lock().await;
        plugins.get(plugin_name).cloned()
    }

    /// 获取所有插件
    pub async fn get_all_plugins(&self) -> Vec<AsmPluginManifest> {
        let plugins = self.plugins.lock().await;
        plugins.values().map(|p| p.manifest.clone()).collect()
    }

    /// 获取插件元数据
    pub async fn get_plugin_manifest(&self, namespace: &str, name: &str) -> Option<AsmPluginManifest> {
        let plugin_key = format!("{}:{}", namespace, name);
        let plugins = self.plugins.lock().await;
        plugins.get(&plugin_key).map(|p| p.manifest.clone())
    }

    // 以下是工具函数



   
} 