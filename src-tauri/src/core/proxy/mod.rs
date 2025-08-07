pub mod config;
pub mod http_interceptor;
pub mod intercept_rules;
pub mod proxy_server;
pub mod store;

use config::ProxyConfig;
use intercept_rules::{InterceptionRule, RuleManager};
use proxy_server::{ProxyServer, wait_for_port_release};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use store::RequestStore;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use std::{thread, time::Duration};
use std::net::TcpListener;

use crate::internal::certificate::CertificateAuthority;

pub struct ProxyState {
    // 代理配置映射表，键为代理ID
    pub configs: Mutex<HashMap<String, ProxyConfig>>,
    // 默认配置（向后兼容）
    pub config: Mutex<ProxyConfig>,
    // 代理服务器映射表，键为代理ID
    pub servers: Mutex<HashMap<String, ProxyServer>>,
    // 默认服务器（向后兼容）
    pub server: RwLock<Option<ProxyServer>>,
    // 请求存储
    pub store: Arc<RequestStore>,
    // 证书颁发机构
    pub cert_authority: Arc<Mutex<CertificateAuthority>>,
    // 全局拦截状态
    pub intercept_enabled: Mutex<bool>,
    // 全局拦截请求状态
    pub intercept_request_enabled: Mutex<bool>,
    // 全局拦截响应状态
    pub intercept_response_enabled: Mutex<bool>,
    pub is_running: RwLock<bool>,
    pub port: RwLock<u16>,
    pub rule_manager: Arc<RuleManager>,
}

impl Default for ProxyState {
    fn default() -> Self {
        let cert_dir = PathBuf::from("certs");
        let rules_file = "config/intercept_rules.json".to_string();

        let rule_manager = RuleManager::new(&rules_file).unwrap_or_else(|_| {
            println!("创建规则管理器失败，使用默认配置");
            RuleManager::new("config/intercept_rules.json").expect("无法创建规则管理器")
        });

        Self {
            configs: Mutex::new(HashMap::new()),
            config: Mutex::new(ProxyConfig::default()),
            servers: Mutex::new(HashMap::new()),
            server: RwLock::new(None),
            store: Arc::new(RequestStore::new()),
            cert_authority: Arc::new(Mutex::new(CertificateAuthority::new(&cert_dir))),
            intercept_enabled: Mutex::new(false),
            intercept_request_enabled: Mutex::new(true),
            intercept_response_enabled: Mutex::new(false),
            is_running: RwLock::new(false),
            port: RwLock::new(8888),
            rule_manager: Arc::new(rule_manager),
        }
    }
}

impl ProxyState {
    // 获取所有代理配置
    pub async fn get_configs(&self) -> Vec<ProxyConfig> {
        let configs = self.configs.lock().await;
        let mut result = configs.values().cloned().collect::<Vec<_>>();

        // 如果映射表为空但存在旧的单个配置，则添加旧配置
        if result.is_empty() {
            let default_config = self.config.lock().await.clone();
            // 为旧配置添加ID
            let mut config_with_id = default_config.clone();
            if config_with_id.id.is_none() {
                config_with_id.id = Some("default".to_string());
            }
            result.push(config_with_id);
        }

        result
    }

    // 获取单个代理配置
    pub async fn get_config_by_id(&self, id: &str) -> Option<ProxyConfig> {
        let configs = self.configs.lock().await;
        if let Some(config) = configs.get(id) {
            return Some(config.clone());
        }

        // 如果找不到指定ID的配置，且ID为"default"，返回默认配置
        if id == "default" {
            let default_config = self.config.lock().await.clone();
            return Some(default_config);
        }

        None
    }

    // 获取当前代理配置（向后兼容）
    pub async fn get_config(&self) -> ProxyConfig {
        self.config.lock().await.clone()
    }

    // 保存代理配置
    pub async fn save_config_with_id(
        &self,
        id: &str,
        new_config: ProxyConfig,
    ) -> Result<(), String> {
        let mut configs = self.configs.lock().await;

        // 创建带ID的配置副本
        let mut config_with_id = new_config.clone();
        config_with_id.id = Some(id.to_string());

        // 保存到配置映射表
        configs.insert(id.to_string(), config_with_id);

        // 如果是默认配置，同时更新旧配置（向后兼容）
        if id == "default" {
            let mut default_config = self.config.lock().await;
            *default_config = new_config;
        }

        Ok(())
    }

    // 保存代理配置（向后兼容）
    pub async fn save_config(&self, new_config: ProxyConfig) -> Result<(), String> {
        // 获取或生成ID
        let id = match &new_config.id {
            Some(id) => id.clone(),
            None => "default".to_string(),
        };

        self.save_config_with_id(&id, new_config).await
    }

    // 删除代理配置
    pub async fn delete_config(&self, id: &str) -> Result<(), String> {
        // 不允许删除默认配置
        if id == "default" {
            return Err("不能删除默认配置".to_string());
        }

        // 如果代理正在运行，先停止它
        if self.get_status_by_id(id).await {
            self.stop_proxy_by_id(id).await?;
        }

        // 从配置映射表中移除
        let mut configs = self.configs.lock().await;
        configs.remove(id);

        Ok(())
    }

    // 启动指定ID的代理服务
    pub async fn start_proxy_by_id(&self, id: &str, app_handle: AppHandle) -> Result<(), String> {
        // 获取配置
        let config = match self.get_config_by_id(id).await {
            Some(config) => config,
            None => return Err(format!("找不到ID为 {} 的代理配置", id)),
        };

        // 检查该代理是否已经运行
        {
            let servers = self.servers.lock().await;
            if servers.contains_key(id) {
                return Err(format!("ID为 {} 的代理服务已在运行", id));
            }
        }

        // 如果是默认代理，同时更新旧的单例代理（向后兼容）
        if id == "default" {
            return self.start_proxy(app_handle).await;
        }

        // 先尝试绑定端口，检查端口是否可用
        let addr = format!("{}:{}", config.interface, config.port);
        if let Err(e) = std::net::TcpListener::bind(addr.clone()) {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                let error_msg = format!("端口 {} 已被占用，请修改端口设置", config.port);
                // 发送错误事件到前端
                let _ = app_handle.emit(
                    "proxy-error",
                    serde_json::json!({
                        "id": id,
                        "message": error_msg
                    }),
                );
                return Err(error_msg);
            } else {
                let error_msg = format!("绑定地址 {} 失败: {}", addr, e);
                // 发送错误事件到前端
                let _ = app_handle.emit(
                    "proxy-error",
                    serde_json::json!({
                        "id": id,
                        "message": error_msg
                    }),
                );
                return Err(error_msg);
            }
        }

        // 获取拦截状态
        let intercept_enabled = *self.intercept_enabled.lock().await;
        let intercept_request_enabled = *self.intercept_request_enabled.lock().await;
        let intercept_response_enabled = *self.intercept_response_enabled.lock().await;

        println!("intercept_response_enabled: {}", intercept_response_enabled);
        // 创建代理服务器
        let server = match ProxyServer::new(
            config.clone(),
            app_handle.clone(),
            self.store.clone(),
            intercept_enabled,
            intercept_request_enabled,
            intercept_response_enabled,
        ) {
            Ok(s) => s,
            Err(e) => {
                let error_msg = format!("创建代理服务器失败: {}", e);
                app_handle
                    .emit(
                        "proxy-error",
                        serde_json::json!({
                            "id": id,
                            "message": error_msg
                        }),
                    )
                    .map_err(|e| format!("发送错误事件失败: {}", e))?;
                return Err(error_msg);
            }
        };

        match server.start().await {
            Ok(running_server) => {
                // 保存到服务器映射表
                let mut servers = self.servers.lock().await;
                servers.insert(id.to_string(), running_server);

                // 发送代理状态变化事件
                let _ = app_handle.emit(
                    "proxy-status-change",
                    serde_json::json!({
                        "id": id,
                        "status": true
                    }),
                );

                Ok(())
            }
            Err(e) => {
                // 如果启动失败，确保发送错误事件
                let _ = app_handle.emit(
                    "proxy-error",
                    serde_json::json!({
                        "id": id,
                        "message": format!("启动代理服务失败: {}", e)
                    }),
                );
                Err(format!("启动代理服务失败: {}", e))
            }
        }
    }

    // 启动代理服务（向后兼容）
    pub async fn start_proxy(&self, app_handle: AppHandle) -> Result<(), String> {
        let mut server_guard = self.server.write().await;

        // 如果服务已经运行，则先停止
        if server_guard.is_some() {
            return Err("代理服务已在运行".to_string());
        }

        let config = self.config.lock().await.clone();
        let store = Arc::clone(&self.store);
        let intercept_enabled = *self.intercept_enabled.lock().await;
        let intercept_request_enabled = *self.intercept_request_enabled.lock().await;
        let intercept_response_enabled = *self.intercept_response_enabled.lock().await;

        // 先尝试绑定端口，检查端口是否可用
        let addr = format!("{}:{}", config.interface, config.port);
        if let Err(e) = std::net::TcpListener::bind(addr.clone()) {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                let error_msg = format!("端口 {} 已被占用，请修改端口设置", config.port);
                // 发送错误事件到前端
                let _ = app_handle.emit(
                    "proxy-error",
                    serde_json::json!({
                        "message": error_msg
                    }),
                );
                return Err(error_msg);
            } else {
                let error_msg = format!("绑定地址 {} 失败: {}", addr, e);
                // 发送错误事件到前端
                let _ = app_handle.emit(
                    "proxy-error",
                    serde_json::json!({
                        "message": error_msg
                    }),
                );
                return Err(error_msg);
            }
        }

        // 创建并启动代理服务
        let server = ProxyServer::new(
            config,
            app_handle.clone(),
            store,
            intercept_enabled,
            intercept_request_enabled,
            intercept_response_enabled,
        )?;
        match server.start().await {
            Ok(running_server) => {
                // 保存到服务器实例
                *server_guard = Some(running_server.clone());

                // 发送代理状态变化事件
                let _ = app_handle.emit(
                    "proxy-status-change",
                    serde_json::json!({
                        "id": "default",
                        "status": true
                    }),
                );

                // 同时更新servers映射表
                drop(server_guard);
                {
                    let mut servers = self.servers.lock().await;
                    servers.insert("default".to_string(), running_server);
                }

                Ok(())
            }
            Err(e) => {
                // 如果启动失败，确保发送错误事件
                let _ = app_handle.emit(
                    "proxy-error",
                    serde_json::json!({
                        "message": format!("启动代理服务失败: {}", e)
                    }),
                );
                Err(format!("启动代理服务失败: {}", e))
            }
        }
    }

    // 停止指定ID的代理服务
    pub async fn stop_proxy_by_id(&self, id: &str) -> Result<(), String> {
        // 如果是默认代理，同时更新旧的单例代理（向后兼容）
        if id == "default" {
            return self.stop_proxy().await;
        }

        let mut servers = self.servers.lock().await;
        if let Some(server) = servers.remove(id) {
            // 提前获取app_handle克隆
            let app_handle = server.app.clone();
            let port = server.config().port;

            // 停止代理服务
            match server.stop().await {
                Ok(_) => {
            // 发送代理状态变化事件
            let _ = app_handle.emit(
                "proxy-status-change",
                serde_json::json!({
                    "id": id,
                    "status": false
                }),
            );

                    // 使用更长的超时时间等待端口释放
                    if !wait_for_port_release(port, 15000) {
                        // 即使端口未释放，也不立即返回错误
                        // 而是记录警告并继续
                        log::warn!("端口 {} 关闭后未能及时释放，但代理已停止", port);
                        
                        // 启动一个后台任务继续尝试等待端口释放
                        let port_clone = port;
                        tokio::spawn(async move {
                            // 再次尝试等待端口释放，使用更长的超时时间
                            if wait_for_port_release(port_clone, 30000) {
                                log::info!("端口 {} 最终已释放", port_clone);
                            } else {
                                log::error!("端口 {} 在延长等待后仍未释放", port_clone);
                            }
                        });
                        
                        // 返回成功，因为代理服务已经停止
                        return Ok(());
            }

            Ok(())
                },
                Err(e) => Err(format!("停止代理服务失败: {}", e))
            }
        } else {
            Err(format!("ID为 {} 的代理服务未在运行", id))
        }
    }

    // 停止代理服务（向后兼容）
    pub async fn stop_proxy(&self) -> Result<(), String> {
        let mut server_guard = self.server.write().await;

        match server_guard.take() {
            Some(server) => {
                // 提前获取app_handle克隆
                let app_handle = server.app.clone();
                let port = server.config().port;

                // 停止代理服务
                match server.stop().await {
                    Ok(_) => {
                // 发送代理状态变化事件
                let _ = app_handle.emit(
                    "proxy-status-change",
                    serde_json::json!({
                        "id": "default",
                        "status": false
                    }),
                );

                // 同时更新servers映射表
                let mut servers = self.servers.lock().await;
                servers.remove("default");

                        // 使用更长的超时时间等待端口释放
                        if !wait_for_port_release(port, 15000) {
                            // 即使端口未释放，也不立即返回错误
                            // 而是记录警告并继续
                            log::warn!("端口 {} 关闭后未能及时释放，但代理已停止", port);
                            
                            // 启动一个后台任务继续尝试等待端口释放
                            let port_clone = port;
                            tokio::spawn(async move {
                                // 再次尝试等待端口释放，使用更长的超时时间
                                if wait_for_port_release(port_clone, 30000) {
                                    log::info!("端口 {} 最终已释放", port_clone);
                                } else {
                                    log::error!("端口 {} 在延长等待后仍未释放", port_clone);
                                }
                            });
                            
                            // 返回成功，因为代理服务已经停止
                            return Ok(());
                }

                Ok(())
                    },
                    Err(e) => Err(format!("停止代理服务失败: {}", e))
                }
            }
            None => Err("代理服务未在运行".to_string()),
        }
    }

    // 获取指定ID的代理状态
    pub async fn get_status_by_id(&self, id: &str) -> bool {
        // 如果是默认代理，查询旧的单例代理状态（向后兼容）
        if id == "default" {
            return self.get_status().await;
        }

        let servers = self.servers.lock().await;
        servers.contains_key(id)
    }

    // 获取代理状态（向后兼容）
    pub async fn get_status(&self) -> bool {
        self.server.read().await.is_some()
    }

    // 设置请求拦截状态
    pub async fn set_intercept_request_status(&self, enabled: bool) -> Result<(), String> {
        // 如果要启用拦截，先检查是否有代理在运行
        if enabled {
            let servers = self.servers.lock().await;
            let default_running = self.server.read().await.is_some();

            if servers.is_empty() && !default_running {
                return Err("没有代理服务在运行，请先启动代理服务再开启拦截功能".to_string());
            }
        }

        let mut intercept_request_enabled = self.intercept_request_enabled.lock().await;
        *intercept_request_enabled = enabled;

        if let Some(server) = self.server.read().await.as_ref() {
            server.update_intercept_request_status(enabled).await?;
        }

        Ok(())
    }

    // 获取请求拦截状态
    pub async fn get_intercept_request_status(&self) -> bool {
        *self.intercept_request_enabled.lock().await
    }

    // 获取历史记录
    pub async fn get_history(&self) -> Vec<store::RequestRecord> {
        self.store.get_all_records().await
    }

    // 清空历史记录
    pub async fn clear_history(&self) -> Result<(), String> {
        self.store.clear().await;
        Ok(())
    }

    // 获取拦截状态
    pub async fn get_intercept_status(&self) -> bool {
        *self.intercept_enabled.lock().await
    }

    // 设置拦截状态
    pub async fn set_intercept_status(&self, enabled: bool) -> Result<(), String> {
        // 如果要启用拦截，先检查是否有代理在运行
        if enabled {
            let servers = self.servers.lock().await;
            let default_running = self.server.read().await.is_some();

            if servers.is_empty() && !default_running {
                return Err("没有代理服务在运行，请先启动代理服务再开启拦截功能".to_string());
            }
        }

        let mut intercept_enabled = self.intercept_enabled.lock().await;
        *intercept_enabled = enabled;

        if let Some(server) = self.server.read().await.as_ref() {
            server.update_intercept_status(enabled).await?;
        }

        Ok(())
    }

    // 设置拦截响应状态
    pub async fn set_intercept_response_status(&self, enabled: bool) -> Result<(), String> {
        // 如果要启用响应拦截，先检查是否有代理在运行
        if enabled {
            let servers = self.servers.lock().await;
            let default_running = self.server.read().await.is_some();

            if servers.is_empty() && !default_running {
                return Err("没有代理服务在运行，请先启动代理服务再开启响应拦截功能".to_string());
            }
        }

        let mut intercept_response_enabled = self.intercept_response_enabled.lock().await;
        *intercept_response_enabled = enabled;

        if let Some(server) = self.server.read().await.as_ref() {
            server.update_intercept_response_status(enabled).await?;
        }

        Ok(())
    }

    // 获取拦截响应状态
    pub async fn get_intercept_response_status(&self) -> bool {
        *self.intercept_response_enabled.lock().await
    }

    // 获取代理设置
    pub async fn get_proxy_settings(&self) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "intercept_enabled": self.get_intercept_status().await,
            "intercept_request_enabled": self.get_intercept_request_status().await,
            "intercept_response_enabled": self.get_intercept_response_status().await
        }))
    }

    // 转发拦截的响应
    pub async fn forward_intercepted_response(
        &self,
        responseId: String,
        status: Option<u16>,
        headers: Option<std::collections::HashMap<String, String>>,
        body: Option<String>,
    ) -> Result<(), String> {
        let server_guard = self.server.read().await;
        match &*server_guard {
            Some(server) => {
                // 创建响应控制通道
                let (response_tx, response_rx) = tokio::sync::oneshot::channel();
                
                // 构造响应控制命令
                let control = http_interceptor::ResponseInterceptControl::Forward {
                    req_id: responseId.clone(),
                    status,
                    headers,
                    body,
                    response_tx,
                };
                
                // 获取响应控制通道
                let response_control_tx = server.get_response_control_tx();
                
                // 发送控制命令
                if let Err(e) = response_control_tx.send(control).await {
                    return Err(format!("发送响应控制命令失败: {}", e));
                }
                
                // 等待结果
                match response_rx.await {
                    Ok(result) => {
                        match result {
                            Ok(_) => Ok(()),
                            Err(e) => Err(e),
                        }
                    },
                    Err(e) => Err(format!("等待响应结果失败: {}", e)),
                }
            },
            None => Err("代理服务器未运行".to_string()),
        }
    }
    
    // 丢弃拦截的响应
    pub async fn drop_intercepted_response(
        &self,
        responseId: String,
    ) -> Result<(), String> {
        let server_guard = self.server.read().await;
        match &*server_guard {
            Some(server) => {
                // 创建响应控制通道
                let (response_tx, response_rx) = tokio::sync::oneshot::channel();
                
                // 构造响应控制命令
                let control = http_interceptor::ResponseInterceptControl::Drop {
                    req_id: responseId.clone(),
                    response_tx,
                };
                
                // 获取响应控制通道
                let response_control_tx = server.get_response_control_tx();
                
                // 发送控制命令
                if let Err(e) = response_control_tx.send(control).await {
                    return Err(format!("发送响应丢弃命令失败: {}", e));
                }
                
                // 等待结果
                match response_rx.await {
                    Ok(result) => {
                        match result {
                            Ok(_) => Ok(()),
                            Err(e) => Err(e),
                        }
                    },
                    Err(e) => Err(format!("等待响应丢弃结果失败: {}", e)),
                }
            },
            None => Err("代理服务器未运行".to_string()),
        }
    }

    // 获取请求拦截规则
    pub async fn get_request_rules(&self) -> Vec<InterceptionRule> {
        self.rule_manager.get_request_rules().await
    }

    // 获取响应拦截规则
    pub async fn get_response_rules(&self) -> Vec<InterceptionRule> {
        self.rule_manager.get_response_rules().await
    }

    // 设置请求拦截规则
    pub async fn set_request_rules(&self, rules: Vec<InterceptionRule>) -> Result<(), String> {
        self.rule_manager
            .set_request_rules(rules)
            .await
            .map_err(|e| format!("保存请求拦截规则失败: {}", e))
    }

    // 设置响应拦截规则
    pub async fn set_response_rules(&self, rules: Vec<InterceptionRule>) -> Result<(), String> {
        self.rule_manager
            .set_response_rules(rules)
            .await
            .map_err(|e| format!("保存响应拦截规则失败: {}", e))
    }

    // 检查请求是否应该被拦截
    pub async fn should_intercept_request(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> bool {
        // 先检查全局开关
        if !*self.intercept_request_enabled.lock().await {
            return false;
        }

        // 然后应用规则
        self.rule_manager
            .should_intercept_request(method, url, headers)
            .await
    }

    // 检查响应是否应该被拦截
    pub async fn should_intercept_response(
        &self,
        status: u16,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> bool {
        // 先检查全局开关
        if !*self.intercept_response_enabled.lock().await {
            return false;
        }

        // 然后应用规则
        self.rule_manager
            .should_intercept_response(status, url, headers)
            .await
    }
}

#[tauri::command]
pub async fn get_request_rules(
    state: tauri::State<'_, ProxyState>,
) -> Result<Vec<InterceptionRule>, String> {
    Ok(state.get_request_rules().await)
}

#[tauri::command]
pub async fn get_response_rules(
    state: tauri::State<'_, ProxyState>,
) -> Result<Vec<InterceptionRule>, String> {
    Ok(state.get_response_rules().await)
}

#[tauri::command]
pub async fn set_request_rules(
    state: tauri::State<'_, ProxyState>,
    rules: Vec<InterceptionRule>,
) -> Result<(), String> {
    state.set_request_rules(rules).await
}

#[tauri::command]
pub async fn set_response_rules(
    state: tauri::State<'_, ProxyState>,
    rules: Vec<InterceptionRule>,
) -> Result<(), String> {
    state.set_response_rules(rules).await
}
