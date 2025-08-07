use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::Mutex;
use serde_json::Value;
use rhai::{Engine, AST, Scope};
use log::{info, error};

use crate::internal::plugin_export_func::set_plugin_export_func;

/// 扫描插件管理器
pub struct ScanPluginManager {
    plugin_dir: PathBuf,
    plugins: Arc<Mutex<std::collections::HashMap<String, ScanPlugin>>>,
    engine: Arc<Engine>,
}

/// 插件元数据
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub author: String,
    pub type_: String,
    pub version: String,
    pub description: String,
    pub severity: Option<String>,
    pub references: Option<Vec<String>>,
    pub parameters: Option<Vec<PluginParameter>>,
}

/// 插件参数
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginParameter {
    pub key: String,
    pub name: String,
    pub type_: String,
    pub required: bool,
    pub description: String,
    pub default: Option<Value>,
}

/// 扫描插件
#[derive(Debug, Clone)]
pub struct ScanPlugin {
    script: String,
    ast: Arc<AST>,
    path: PathBuf,
    manifest: Option<PluginManifest>,
}

/// 插件执行上下文
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginContext {
    pub target: String,
    #[serde(default)]
    pub params: serde_json::Map<String, Value>,
}

/// 插件执行结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginResult {
    pub status: String,
    pub message: Option<String>,
    pub raw_output: Option<String>,
    pub data: Option<Value>,
}

impl ScanPlugin {
    /// 创建新插件
    pub fn new(script: String, ast: AST, path: PathBuf) -> Self {
        Self {
            script,
            ast: Arc::new(ast),
            path,
            manifest: None,
        }
    }

    /// 创建新插件并解析元数据
    pub fn new_with_manifest(script: String, ast: AST, path: PathBuf, engine: &Engine) -> Self {
        let plugin = Self {
            script,
            ast: Arc::new(ast),
            path,
            manifest: None,
        };
        
        // 尝试解析元数据
        plugin.parse_manifest(engine);
        plugin
    }

    /// 解析插件元数据
    pub fn parse_manifest(&self, engine: &Engine) -> Option<PluginManifest> {
        // 创建作用域
        let mut scope = Scope::new();
        
        // 尝试调用get_manifest函数
        match engine.call_fn::<Value>(&mut scope, &self.ast, "get_manifest", ()) {
            Ok(manifest_value) => {
                // 尝试将返回值转换为PluginManifest
                match serde_json::from_value::<PluginManifest>(manifest_value) {
                    Ok(mut manifest) => {
                        // 修复type字段 (JSON中无法使用type作为字段名，转换时使用type_)
                        manifest.type_ = manifest.type_.clone();
                        
                        // 设置manifest并返回
                        Some(manifest)
                    },
                    Err(e) => {
                        error!("解析插件元数据失败: {}", e);
                        None
                    }
                }
            },
            Err(e) => {
                error!("获取插件元数据失败: {}", e);
                None
            }
        }
    }

    /// 获取脚本内容
    pub fn script(&self) -> &str {
        &self.script
    }

    /// 获取插件路径
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// 获取插件元数据
    pub fn manifest(&self) -> Option<&PluginManifest> {
        self.manifest.as_ref()
    }
}

impl ScanPluginManager {
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

     
        
        Ok(Self {
            plugin_dir,
            plugins: Arc::new(Mutex::new(std::collections::HashMap::new())),
            engine: Arc::new(engine),
        })
    }

    /// 获取插件目录
    pub fn plugin_dir(&self) -> &PathBuf {
        &self.plugin_dir
    }

    /// 加载所有插件
    pub async fn load_all_plugins(&self) -> Result<()> {
        info!("正在加载扫描插件...");

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
                        let plugin_id = path.file_stem().unwrap().to_string_lossy().to_string();
                        info!("已加载插件: {}", plugin_id);
                        plugins.insert(plugin_id, plugin);
                    }
                    Err(e) => {
                        error!("加载插件失败 {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!("已加载{}个扫描插件", plugins.len());
        Ok(())
    }

    /// 加载单个插件
    pub async fn load_plugin(&self, path: &std::path::Path) -> Result<ScanPlugin> {
        // 读取脚本文件内容
        let script = std::fs::read_to_string(path)?;

        // 编译脚本到AST
        let ast = self.engine.compile(&script)?;

        let plugin = ScanPlugin::new_with_manifest(script, ast, path.to_path_buf(), &self.engine);
        Ok(plugin)
    }

    /// 验证插件脚本
    pub async fn validate_plugin(&self, script: &str) -> Result<bool> {
        // 尝试编译脚本
        match self.engine.compile(script) {
            Ok(ast) => {
                // 创建作用域
                let mut scope = Scope::new();
                
                // 尝试检查get_manifest函数
                let _has_manifest = match self.engine.call_fn::<Value>(&mut scope, &ast, "get_manifest", ()) {
                    Ok(_) => true,
                    Err(_) => false,
                };
                
                // 尝试检查analyze函数
                let has_analyze = match self.engine.call_fn::<String>(&mut scope, &ast, "analyze", ("{}",)) {
                    Ok(_) => true,
                    Err(_) => false,
                };
                
                // 插件至少应该有analyze函数
                Ok(has_analyze)
            },
            Err(e) => {
                error!("脚本编译失败: {}", e);
                Err(anyhow::anyhow!("脚本编译失败: {}", e))
            }
        }
    }

    /// 执行插件
    pub async fn execute_plugin(
        &self,
        plugin_id: &str,
        context: PluginContext,
    ) -> Result<PluginResult> {
        // 获取插件
        let plugins = self.plugins.lock().await;
        let plugin = plugins.get(plugin_id).ok_or_else(|| {
            anyhow::anyhow!("Plugin not found: {}", plugin_id)
        })?;

        // 转换请求为JSON字符串
        let request_json = serde_json::to_string(&context)?;

        // Clone what we need for the blocking task
        let engine = self.engine.clone();
        let ast = plugin.ast.clone();

        // 使用spawn_blocking执行Rhai引擎的阻塞操作
        let result = tokio::task::spawn_blocking(move || {
            // 创建新的作用域
            let mut scope = Scope::new();
            
            // 执行插件的analyze函数
            engine.call_fn::<String>(&mut scope, &ast, "analyze", (request_json,))
        }).await??;

        // 解析执行结果
        let result: PluginResult = serde_json::from_str(&result)?;
        Ok(result)
    }

    /// 添加插件
    pub async fn add_plugin(&mut self, plugin: ScanPlugin) -> Result<()> {
        let plugin_id = plugin.path().file_stem().unwrap().to_string_lossy().to_string();
        let mut plugins = self.plugins.lock().await;
        plugins.insert(plugin_id, plugin);
        Ok(())
    }

    /// 删除插件
    pub async fn remove_plugin(&mut self, plugin_id: &str) -> Option<ScanPlugin> {
        let mut plugins = self.plugins.lock().await;
        plugins.remove(plugin_id)
    }

    /// 获取插件
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<ScanPlugin> {
        let plugins = self.plugins.lock().await;
        plugins.get(plugin_id).cloned()
    }

    /// 获取所有插件
    pub async fn get_all_plugins(&self) -> Vec<(String, ScanPlugin)> {
        let plugins = self.plugins.lock().await;
        plugins.iter()
            .map(|(id, plugin)| (id.clone(), plugin.clone()))
            .collect()
    }

} 