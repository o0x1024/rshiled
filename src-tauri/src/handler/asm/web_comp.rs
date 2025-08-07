use sqlx::{query_as, query_scalar};
use std::sync::Arc;

// Import Manager trait
use tauri::Manager;

use super::asm_task::INNERASK_MODULE;

#[derive(Debug, serde::Serialize, Clone, sqlx::FromRow)]
pub struct WebComp {
    pub id: Option<i32>,
    pub task_id: i32,
    pub website: String,       //组件URL
    pub comp_name: String,     //组件名称
    pub ctype: Option<String>, //组件类型
    pub create_at: i64,        //创建时间
    pub update_at: i64,        //最近一个访问或者更å新时间
}

impl WebComp {
    pub fn new() -> Self {
        Self {
            id: None,
            task_id: 0,
            website: "".to_string(),
            comp_name: "".to_string(),
            ctype: None,
            create_at: 0,
            update_at: 0,
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_webcomps(
    page: i32,
    pagesize: i32,
    filter: String,
) -> Result<serde_json::Value, String> {
    let allowed_filters = ["task_id", "comp_name", "ctype"]; // 允许的列名
    if !allowed_filters.contains(&filter.as_str()) {
        return Err("Invalid filter".into());
    }

    let task_module = INNERASK_MODULE
        .get()
        .ok_or("Global variable not initialized")?;
    let tauri_conn = Arc::clone(&task_module.tauri_conn);

    let offset: i32 = (page - 1) * pagesize;
    let webcomps = query_as::<_, WebComp>(
      "SELECT id,task_id,website,comp_name,ctype,create_at,update_at FROM webcomp ORDER BY create_at DESC limit ? offset ? "
    )
    .bind(pagesize as i32)
    .bind(offset as i32)
    .fetch_all(&*tauri_conn)
    .await
    .unwrap();

    let total_count: i64 = query_scalar("SELECT count(*) FROM webcomp")
        .fetch_one(&*tauri_conn)
        .await
        .unwrap_or(0);

    Ok(serde_json::json!({
        "list": webcomps,
        "total": total_count
    }))
}

/// 使用插件扫描网站指纹
pub async fn scan_fingerprint_by_plugin(task_id: &i32, websites: &Vec<String>) {
    // 获取全局AppHandle
    let app_handle = match crate::APP_HANDLE.get() {
        Some(handle) => handle.clone(),
        None => {
            log::error!("全局APP_HANDLE未初始化");
            return;
        }
    };

    let mut handle_list = Vec::new();

    for website in websites {
        let tid = task_id.clone();
        let site = website.clone();
        let app_handle_clone = app_handle.clone();

        let handle = tokio::spawn(async move {
            // 获取ASM插件管理器
            let state = app_handle_clone
                .state::<crate::handler::asm::plugin_commands::AsmPluginManagerState>();
            let manager = state.inner.lock().await;

            // 查找所有指纹识别插件
            let fingerprint_plugins = manager
                .get_all_plugins()
                .await
                .into_iter()
                .filter(|p| p.plugin_type == "fingerprint")
                .collect::<Vec<_>>();

            // 执行每个指纹识别插件
            for plugin in fingerprint_plugins {
                log::info!("Executing fingerprint plugin: {}", plugin.name);

                // 创建插件上下文
                let context = crate::handler::asm::plugin::AsmPluginContext {
                    task_id: tid,
                    target: site.clone(),
                    targets: Some(vec![site.clone()]),
                    custom_params: None,
                };

                // 执行插件
                match manager
                    .execute_plugin(&plugin.name, "fingerprint", context)
                    .await
                {
                    Ok(result) => {
                        if result.success {
                            log::info!("Fingerprint plugin {} executed successfully", plugin.name);

                            // 处理发现的指纹
                            if let Some(found_fingerprints) = result.found_fingerprints {
                                if !found_fingerprints.is_empty() {
                                    log::info!(
                                        "Plugin {} found {} fingerprints",
                                        plugin.name,
                                        found_fingerprints.len()
                                    );

                                    // 获取数据库连接
                                    let task_module = match super::asm_task::INNERASK_MODULE.get() {
                                        Some(tm) => tm,
                                        None => {
                                            log::error!("Global variable not initialized");
                                            return;
                                        }
                                    };
                                    let write_conn = Arc::clone(&task_module.write_conn);

                                    // 开始事务
                                    let tx_result = write_conn.begin().await;
                                    match tx_result {
                                        Ok(mut tx) => {
                                            let mut success = true;

                                            for fp_value in found_fingerprints {
                                                if let Some(fp_obj) = fp_value.as_object() {
                                                    // 提取指纹信息
                                                    let name = fp_obj
                                                        .get("name")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("未知组件")
                                                        .to_string();

                                                    let version = fp_obj
                                                        .get("version")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("未知版本")
                                                        .to_string();

                                                    let category = fp_obj
                                                        .get("category")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("未知类别")
                                                        .to_string();

                                                    let _confidence = fp_obj
                                                        .get("confidence")
                                                        .and_then(|v| v.as_u64())
                                                        .unwrap_or(100)
                                                        as i32;

                                                    let now_timestamp =
                                                        chrono::Local::now().timestamp();

                                                    // 添加到数据库
                                                    match sqlx::query(
                                                        "INSERT INTO webcomp (task_id, website, comp_name, comp_version, ctype, create_at, update_at) 
                                                         VALUES (?, ?, ?, ?, ?, ?, ?) 
                                                         ON CONFLICT DO NOTHING"
                                                    )
                                                    .bind(tid)
                                                    .bind(&site)
                                                    .bind(&name)
                                                    .bind(&version)
                                                    .bind(&category)
                                                    .bind(now_timestamp)
                                                    .bind(now_timestamp)
                                                    .execute(&mut *tx)
                                                    .await {
                                                        Ok(_) => {
                                                            log::info!("Added web component to database");
                                                        }
                                                        Err(e) => {
                                                            log::error!("Failed to add web component to database: {}", e);
                                                            success = false;
                                                            break;
                                                        }
                                                    }
                                                }
                                            }

                                            // 提交或回滚事务
                                            if success {
                                                if let Err(e) = tx.commit().await {
                                                    log::error!("Failed to commit fingerprint transaction: {}", e);
                                                }
                                            } else {
                                                if let Err(e) = tx.rollback().await {
                                                    log::error!("Failed to rollback fingerprint transaction: {}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            log::error!("Failed to begin transaction: {}", e);
                                        }
                                    }
                                }
                            }
                        } else {
                            log::warn!(
                                "Fingerprint plugin {} execution failed: {}",
                                plugin.name,
                                result.message
                            );
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to execute fingerprint plugin {}: {}",
                            plugin.name,
                            e
                        );
                    }
                }
            }
        });

        handle_list.push(handle);
    }

    for handle in handle_list {
        let _ = tokio::join!(handle);
    }
}
