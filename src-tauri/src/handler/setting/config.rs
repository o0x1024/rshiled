use crate::{asm::asm_task::INNERASK_MODULE, global::config::CoreConfig};
use log::{error, info};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

#[tauri::command]
pub async fn get_asm_config() -> Result<serde_json::Value, String> {
    let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");
    let pool = &*task_module.read_conn;

    let config = query_as::<_, CoreConfig>(
        r#"
        SELECT 
            dns_collection_brute_status,
            is_buildin,
            dns_collection_plugin_status,
            port_scan_plugin_status,
            fingerprint_plugin_status,
            risk_scan_plugin_status,
            proxy,
            user_agent,
            http_headers,
            http_timeout,
            thread_num,
            subdomain_dict,
            file_dict,
            subdomain_level
        FROM config 
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let result = serde_json::json!({
        "dns_collection_brute_status": config.dns_collection_brute_status,
        "is_buildin": config.is_buildin,
        "dns_collection_plugin_status": config.dns_collection_plugin_status,
        "proxy": config.proxy,
        "user_agent": config.user_agent,
        "http_headers": config.http_headers,
        "http_timeout": config.http_timeout,
        "thread_num": config.thread_num,
        "subdomain_dict": config.subdomain_dict,
        "file_dict": config.file_dict,
        "subdomain_level": config.subdomain_level
    });
    Ok(result)
}

#[tauri::command]
pub async fn update_asm_config(config: serde_json::Value) -> Result<SuccessResponse, String> {
    // 从全局变量获取数据库连接
    let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");
    let pool = &*task_module.tauri_conn;

    // 从JSON值中提取配置
    let dns_collection_brute_status = config.get("dns_collection_brute_status").and_then(|v| v.as_bool()).unwrap_or(false);
    let dns_collection_plugin_status = config.get("dns_collection_plugin_status").and_then(|v| v.as_bool()).unwrap_or(false);
    let is_buildin = config.get("is_buildin").and_then(|v| v.as_bool()).unwrap_or(false);
    let port_scan_plugin_status = config.get("port_scan_plugin_status").and_then(|v| v.as_bool()).unwrap_or(true);
    let fingerprint_plugin_status = config.get("fingerprint_plugin_status").and_then(|v| v.as_bool()).unwrap_or(true);
    let risk_scan_plugin_status = config.get("risk_scan_plugin_status").and_then(|v| v.as_bool()).unwrap_or(true);
    let proxy = config.get("proxy").and_then(|v| v.as_str().map(|s| s.to_owned()));
    let user_agent = config.get("user_agent").and_then(|v| v.as_str().map(|s| s.to_owned()));
    let http_timeout = config["http_timeout"].as_u64().map(|v| v as i64);
    let thread_num = config["thread_num"].as_u64().map(|v| v as i64);

    let subdomain_dict = config.get("subdomain_dict").and_then(|v| v.as_str().map(|s| s.to_owned()));
    let file_dict = config.get("file_dict").and_then(|v| v.as_str().map(|s| s.to_owned()));
    let subdomain_level = config.get("subdomain_level").and_then(|v| v.as_u64().map(|v| v as i64));
    // 处理HTTP头
    let http_headers_json = match config["http_headers"].is_array() {
        true => {
            let headers = config["http_headers"].as_array().unwrap();
            let mut header_array = Vec::new();
            for header in headers {
                if header.is_array() && header.as_array().unwrap().len() == 2 {
                    let h = header.as_array().unwrap();
                    let key = h[0].as_str().unwrap_or("").to_string();
                    let value = h[1].as_str().unwrap_or("").to_string();
                    if !key.is_empty() && !value.is_empty() {
                        header_array.push((key, value));
                    }
                }
            }
            if header_array.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&header_array).unwrap_or_default())
            }
        }
        false => None,
    };

    // 更新配置到数据库
    match query(
        r#"
        UPDATE config SET 
            dns_collection_brute_status = ?,
            subdomain_dict = ?,
            file_dict = ?,
            is_buildin = ?,
            dns_collection_plugin_status = ?,
            port_scan_plugin_status = ?,
            fingerprint_plugin_status = ?,
            risk_scan_plugin_status = ?,
            proxy = ?,
            user_agent = ?,
            http_headers = ?,
            http_timeout = ?,
            thread_num = ?,
            subdomain_level = ?
        "#,
    )
    .bind(dns_collection_brute_status)
    .bind(subdomain_dict)
    .bind(file_dict)
    .bind(is_buildin)
    .bind(dns_collection_plugin_status)
    .bind(port_scan_plugin_status)
    .bind(fingerprint_plugin_status)
    .bind(risk_scan_plugin_status)
    .bind(&proxy)
    .bind(&user_agent)
    .bind(&http_headers_json)
    .bind(http_timeout)
    .bind(thread_num)
    .bind(subdomain_level)
    .execute(pool)
    .await {
        Ok(_) => {
            // 直接更新全局AppConfig对象，而不仅是重新初始化
            let updated_config = CoreConfig {
                dns_collection_brute_status,
                is_buildin,
                dns_collection_plugin_status,
                port_scan_plugin_status,
                fingerprint_plugin_status,
                risk_scan_plugin_status,
                file_dict: None,  // 这些会被update_global保留
                subdomain_dict: None, // 这些会被update_global保留
                subdomain_level: subdomain_level.map(|v| v as u8),
                http_client: None, // 这会被update_global重新创建
                proxy,
                user_agent,
                http_headers: match &http_headers_json {
                    Some(json) => serde_json::from_str(json).ok(),
                    None => None,
                },
                http_timeout: http_timeout.map(|v| v as u64),
                thread_num: thread_num.map(|v| v as u64),
            };
            
            match CoreConfig::update_global(updated_config) {
                Ok(_) => {
                    info!("ASM config updated successfully both in DB and global object");
                    Ok(SuccessResponse {
                        success: true,
                        message: "Configuration updated successfully".to_string(),
                    })
                },
                Err(e) => {
                    error!("Failed to update global AppConfig: {}", e);
                    // 尝试重新初始化AppConfig作为回退方案
                    match CoreConfig::init().await {
                        Ok(_) => {
                            info!("ASM config reinitialized as fallback");
                            Ok(SuccessResponse {
                                success: true,
                                message: "Configuration updated but required reinitialization".to_string(),
                            })
                        },
                        Err(e) => {
                            error!("Failed to reinitialize AppConfig: {:?}", e);
                            Err(format!("Failed to reinitialize AppConfig: {:?}", e))
                        }
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to update ASM config: {:?}", e);
            Err(format!("Failed to update ASM config: {:?}", e))
        }
    }
}
