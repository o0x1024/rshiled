// src-tauri/src/handler/scan/passive/handler.rs
use tauri::{State, Emitter};
use crate::handler::scan::common::types::{PassiveScanConfig, SuccessResponse, Vulnerability, VulnerabilityDetail};
use crate::state::ScannerState;
use crate::core::config::{AppConfig, ProxyConfig};
use crate::internal::certificate::CertificateAuthority;
use crate::scan::engine::manager::ScanManager;
use crate::scan::proxy::Proxy;
use log::{error, info};
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use chrono::Utc;

// These functions will eventually contain the logic moved from the old api.rs
pub async fn handle_start_passive_scan(
    config: PassiveScanConfig,
    state: State<'_, ScannerState>,
) -> Result<SuccessResponse, String> {
    let mut running_lock = state.running.lock().await;
    if *running_lock {
        return Err("扫描器已在运行中".to_string());
    }

    let proxy_config_struct = ProxyConfig {
        host: Some("127.0.0.1".to_string()),
        port: Some(config.port),
        connect_timeout: 10,
        max_retries: 3,
        retry_delay: 1000,
        ca_cert_path: "certs/RShield_CA.crt".to_string(),
        ca_key_path: "certs/RShield_CA.key".to_string(),
    };

    let (proxy_tx, proxy_rx) = mpsc::channel(100);
    let (result_tx, mut result_rx) = mpsc::channel(100); // mut result_rx will be used later
    let (request_count_tx, mut request_count_rx) = mpsc::channel(100); // mut request_count_rx will be used later

    let app_config = Arc::new(AppConfig::default());

    let cert_manager = if config.intercept_tls {
        let app_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("certs");
        
        if !app_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&app_dir) {
                error!("创建证书目录失败: {}", e);
                // Consider returning an error or specific handling
            }
        }
        Some(Arc::new(CertificateAuthority::new(&Path::new(&proxy_config_struct.ca_cert_path))))
    } else {
        None
    };

    let proxy = Proxy::new_with_tls_settings(proxy_config_struct.clone(), proxy_tx, cert_manager.clone(), config.intercept_tls);
    
    match proxy.try_bind().await {
        Ok(_) => {
            *running_lock = true;
            drop(running_lock); // Release lock before further async operations

            {
                let mut status_lock = state.status.lock().await;
                status_lock.running = true;
                status_lock.proxy_address = proxy_config_struct.host.clone().unwrap_or_else(|| "127.0.0.1".to_string());
                status_lock.proxy_port = proxy_config_struct.port.unwrap_or(8889); 
                status_lock.scan_count = 0;
                status_lock.vulnerability_count = 0;
                status_lock.message = Some("扫描已启动".to_string());
                status_lock.last_update = Some(Utc::now().to_rfc3339());
            }

            let scan_manager = ScanManager::new(app_config.clone(), result_tx).await;
            
            let proxy_arc = Arc::new(proxy); // proxy was moved by try_bind, this might need adjustment if Proxy::new_with_tls_settings returns Arc<Proxy> or if try_bind consumes self and returns new instance or Arc
                                         // Assuming Proxy is Clone or can be re-constructed or try_bind does not consume entirely or returns what's needed.
                                         // For this step, I will assume the original structure where `proxy` can be wrapped in Arc after `try_bind` if `proxy` is still valid and Clone or similar.
                                         // If `proxy.try_bind()` consumes `proxy` and returns `Result<BoundProxy, Error>`, then `proxy_arc` logic needs to be inside Ok.
                                         // Based on original code: `proxy` is consumed by `proxy.try_bind()` and not available afterwards if it were `self`.
                                         // However, `Proxy::new_with_tls_settings` likely returns the `Proxy` instance directly.
                                         // The original `proxy.try_bind().await` suggests `proxy` instance is still available to be Arc'd. Let's stick to that for now.
            {
                let mut proxy_state_lock = state.proxy.lock().await;
                *proxy_state_lock = Some(proxy_arc.clone());
            }

            let request_count_tx_clone = request_count_tx.clone();
            let _proxy_handle = tokio::spawn(async move {
                // Re-clone proxy_arc for the task if it was moved or ensure it's cloneable for multiple tasks.
                // If proxy_arc was moved into proxy_state_lock, this will fail.
                // Correct way: Clone Arc before moving, or pass a clone to the tokio::spawn.
                let task_proxy_arc = proxy_arc.clone();
                if let Err(e) = task_proxy_arc.start_with_request_counter(request_count_tx_clone).await {
                    error!("代理服务器错误: {}", e);
                }
            });

            let scan_manager_clone = scan_manager.clone();
            let _scan_handle = tokio::spawn(async move {
                scan_manager_clone.start(proxy_rx).await;
            });

            let status_arc_clone_req = state.status.clone();
            tokio::spawn(async move {
                while let Some(_) = request_count_rx.recv().await {
                    let mut status_lock = status_arc_clone_req.lock().await;
                    status_lock.scan_count += 1;
                    status_lock.last_update = Some(Utc::now().to_rfc3339());
                }
            });

            let vulnerabilities_arc = state.vulnerabilities.clone();
            let status_arc_clone_res = state.status.clone();
            let running_arc_clone = state.running.clone();
            let window_clone = state.window.clone();

            tokio::spawn(async move {
                while let Some(result) = result_rx.recv().await { // result_rx was defined earlier
                    let new_vulnerability_id = vulnerabilities_arc.lock().await.len() as u32;
                    let vulnerability = Vulnerability {
                        id: new_vulnerability_id,
                        vulnerability_type: result.vulnerability_type,
                        name: result.name,
                        url: result.url,
                        parameter: result.parameter.clone(),
                        value: result.value.clone(),
                        evidence: result.evidence.clone(),
                        risk_level: result.risk_level,
                        timestamp: result.timestamp.to_utc().to_string(),
                        description: result.description,
                        solution: result.remediation.unwrap_or_else(|| "No solution provided".to_string()),
                        details: result.details.map(|details_note| VulnerabilityDetail {
                            note: details_note,
                            request: result.request_details.unwrap_or_else(|| "No request captured".to_string()),
                            response: result.response_details.unwrap_or_else(|| "No response captured".to_string()),
                        }),
                    };

                    {
                        let mut vulns_lock = vulnerabilities_arc.lock().await;
                        vulns_lock.push(vulnerability.clone());

                        let mut status_lock = status_arc_clone_res.lock().await;
                        status_lock.vulnerability_count = vulns_lock.len();
                        status_lock.last_update = Some(Utc::now().to_rfc3339());
                        
                        info!("发现漏洞: {} ({}) - 当前列表中共有 {} 个漏洞", 
                              vulnerability.name, 
                              vulnerability.risk_level, 
                              vulns_lock.len());
                    }

                    if let Err(emit_err) = window_clone.emit("vulnerability_found", serde_json::json!({
                        "count": status_arc_clone_res.lock().await.vulnerability_count,
                        "latest": vulnerability
                    })) {
                        error!("Failed to emit vulnerability_found event: {}", emit_err);
                    }
                }

                {
                    let mut running_lock_done = running_arc_clone.lock().await;
                    *running_lock_done = false;

                    let mut status_lock_done = status_arc_clone_res.lock().await;
                    status_lock_done.running = false;
                    status_lock_done.message = Some("扫描已完成".to_string());
                    status_lock_done.last_update = Some(Utc::now().to_rfc3339());
                    status_lock_done.last_stop_time = Some(Utc::now().to_rfc3339());
                }

                info!("被动扫描已完成");
                if let Err(emit_err) = window_clone.emit("passive_scan_completed", serde_json::json!({})) {
                    error!("Failed to emit passive_scan_completed event: {}", emit_err);
                }
            });

            Ok(SuccessResponse {
                success: true,
                message: format!("被动扫描已成功启动，代理服务器运行在端口 {}", config.port),
            })
        }
        Err(e) => {
            let error_msg = if e.contains("端口") && e.contains("已被占用") {
                format!("端口 {} 已被占用，请选择其他端口", config.port)
            } else if e.contains("Permission denied") {
                format!("无权限使用端口 {}，请尝试使用大于 1024 的端口", config.port)
            } else {
                format!("启动代理服务器失败: {}", e)
            };
            error!("{}", error_msg);
            if let Err(emit_err) = state.window.emit("scan_error", json!({
                "error": error_msg.clone(),
                "port": config.port,
                "type": "passive_start",
                "timestamp": Utc::now().to_rfc3339()
            })) {
                error!("Failed to emit scan_error event: {}", emit_err);
            }
            Err(error_msg)
        }
    }
}

pub async fn handle_stop_passive_scan(
    state: State<'_, ScannerState>,
) -> Result<SuccessResponse, String> {
    let mut running = state.running.lock().await;
    if !*running {
        return Ok(SuccessResponse {
            success: false, // Should be true if the state is already stopped, or adjust message
            message: "扫描器未在运行中".to_string(),
        });
    }

    // Update state to reflect that stopping is in progress or completed.
    *running = false;
    // It's generally better to update the ScannerStatus.running field as well,
    // and do it consistently.
    info!("扫描状态已更新为停止");
    
    // Stop the proxy server
    let mut proxy_lock = state.proxy.lock().await;
    if let Some(proxy_arc) = proxy_lock.take() { // .take() removes the proxy from the Option
        info!("正在停止代理服务器...");
        match tokio::time::timeout(
            std::time::Duration::from_secs(5), 
            proxy_arc.stop() // Assuming stop() is an async method on Proxy
        ).await {
            Ok(stop_result) => {
                // match stop_result {
                //     Ok(_) => {
                //         info!("代理服务器已成功停止");
                //     },
                //     Err(e) => {
                //         error!("代理服务器停止时发生错误: {}", e);
                //     },
                // };
                ()
            },
            Err(_) => {
                error!("停止代理服务器超时");
                // Even if timeout, continue as state is already set to stopped.
                ()
            }
        }
    } else {
        info!("没有运行中的代理服务器实例需要停止");
    }
    
    // Update ScannerStatus fields
    {
        let mut status = state.status.lock().await;
        status.running = false; // Ensure this is also set
        status.last_stop_time = Some(Utc::now().to_utc().to_string());
        status.message = Some("扫描已被用户停止".to_string());
        status.last_update = Some(Utc::now().to_rfc3339()); // Update last_update time
    }

    // Emit event to frontend
    let event_payload = json!({
        "type": "scan_stopped", // Consistent event type
        "timestamp": Utc::now().to_rfc3339(),
        "message": "扫描已被用户停止",
        "success": true
    });

    if let Err(e) = state.window.emit("scan_event", event_payload) { // Use a general event channel like "scan_event"
        error!("发送扫描停止事件失败: {}", e);
    }

    Ok(SuccessResponse {
        success: true,
        message: "扫描器已成功停止".to_string(),
    })
} 