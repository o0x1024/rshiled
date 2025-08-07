pub mod core;
pub mod database;
pub mod global;
pub mod handler;
pub mod internal;
pub mod state;
pub mod tasks;
mod commands;

use crate::handler::asm::api::get_apis;
use crate::handler::asm::risk::get_risks;
use crate::handler::{asm, repeater, setting};
use cmds::*;
use std::time::Duration;
use core::proxy::*;
use handler::asm::{
    delete_domain_by_id, get_asm_task_list, process_apis, process_risks, run_scan_by_type,
    save_next_run_time, test_asm_plugin,
};
use handler::{get_asm_config, update_asm_config};
use once_cell::sync::OnceCell;
use std::path::PathBuf;
// Re-export scan module for easier access
use crate::handler::asm::plugin_commands::{
    delete_asm_plugin, execute_asm_plugin, get_asm_plugin, init_asm_plugin_manager,
    list_asm_plugins, load_asm_plugins, update_asm_plugin, upload_asm_plugin_content,
};
use crate::state::AppState;
use crate::commands::open_url;
pub use handler::scan;
use handler::vuln::vuln_commands::{
    delete_rhai_plugin, execute_rhai_plugin, get_rhai_plugin, init_vuln_plugin_manager,
    list_rhai_plugins, load_rhai_plugins, update_rhai_plugin, upload_rhai_plugin,
    upload_rhai_plugin_content,
};
use log::error;
use tauri::Manager;

// 导入扫描模块API

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};


use crate::core::proxy::{config::ProxyConfig, store::RequestRecord, ProxyState};
use std::collections::HashMap;

use crate::asm::{
    domain::{add_domain, get_domains},
    ips::get_ips,
    rootdomain::{add_root_domain, del_rootdomain_by_id, get_ent_domain, get_root_domains},
    scan_task::{add_task, del_task_by_id, get_task_list, run_scan, switch_task_status},
    website::get_websites,
};

use crate::setting::scan::{
    add_regex, del_regex_by_id, get_regexs, switch_regex_status, update_regex,
};

use crate::state::ScannerState;

#[cfg(mobile)]
mod mobile;
#[cfg(mobile)]
pub use mobile::*;

// 添加全局AppHandle实例
pub static APP_HANDLE: OnceCell<tauri::AppHandle> = OnceCell::new();

use crate::handler::scan::plugin_commands::init_scan_plugin_manager;

// 导入暴力破解模块
use crate::handler::brute::{
    BruteForceState, brute_create_task, brute_delete_task, 
    brute_get_results, brute_get_tasks, brute_start_task, brute_stop_task,
};


pub async fn run() {
    // 扫描引擎的proxy配置文件初始化
    // crate::handler::scan::config::init_config().await.expect("Failed to initialize scan config");

    // 初始化ASM插件
    let asm_plugin_dir = PathBuf::from("./plugins/asm");
    let asm_plugin_manager = match init_asm_plugin_manager(asm_plugin_dir).await {
        Ok(manager) => manager,
        Err(e) => {
            error!("初始化ASM插件管理器失败: {}", e);
            panic!("初始化ASM插件管理器失败");
        }
    };
    // 初始化Rhai插件
    let vuln_plugin_dir = PathBuf::from("./plugins/vuln");
    let vuln_plugin_manager = match init_vuln_plugin_manager(vuln_plugin_dir).await {
        Ok(manager) => manager,
        Err(e) => {
            error!("初始化Rhai插件管理器失败: {}", e);
            panic!("初始化Rhai插件管理器失败");
        }
    };

    let scan_plugin_dir = PathBuf::from("./plugins/scan");
    let scan_plugin_manager = match init_scan_plugin_manager(scan_plugin_dir).await {
        Ok(manager) => manager,
        Err(e) => {
            error!("Failed to initialize scan plugin manager: {}", e);
            panic!("Failed to initialize scan plugin manager");
        }
    };

    // 初始化暴力破解模块
    let brute_force_state = BruteForceState::new();

    // 初始化Repeater插件
    // 不再使用插件方式
    // let repeater_plugin = handler::repeater::init();

    // let menu = Menu::new()?;
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            let _ = app
                .get_webview_window("main")
                .expect("no main window")
                .set_focus();
        }))
        .manage(vuln_plugin_manager)
        .manage(asm_plugin_manager)
        .manage(scan_plugin_manager)
        .manage(ProxyState::default()) // 注册代理状态
        .manage(brute_force_state) // 注册暴力破解状态
        .plugin(tauri_plugin_websocket::init())
        .plugin(tauri_plugin_dialog::init())
        // .plugin(repeater_plugin)
        .invoke_handler(tauri::generate_handler![
            get_websites,
            get_task_list,
            get_asm_task_list,
            add_root_domain,
            add_domain,
            get_root_domains,
            del_task_by_id,
            delete_domain_by_id,
            run_scan,
            add_task,
            process_apis,
            switch_task_status,
            get_websites,
            get_ent_domain,
            del_rootdomain_by_id,
            get_domains,
            get_ips,
            get_regexs,
            update_regex,
            add_regex,
            switch_regex_status,
            del_regex_by_id,
            get_apis,
            save_next_run_time,
            run_scan_by_type,
            scan::api_commands::start_passive_scan,
            scan::api_commands::start_active_scan,
            scan::api_commands::stop_passive_scan,
            scan::api_commands::get_scan_status,
            scan::api_commands::get_scan_vulnerabilities,
            scan::api_commands::clear_scan_vulnerabilities,
            scan::api_commands::export_scan_vulnerabilities,
            get_risks,
            // asm::port_scan,
            asm::get_asset_statistics,
            asm::visualization::get_asset_graph_data,
            asm::visualization::get_risk_heatmap_data,
            asm::visualization::generate_compliance_report,
            asm::visualization::open_file,
            get_asm_config,
            update_asm_config,
            list_rhai_plugins,
            get_rhai_plugin,
            load_rhai_plugins,
            execute_rhai_plugin,
            upload_rhai_plugin,
            upload_rhai_plugin_content,
            process_risks,
            delete_rhai_plugin,
            update_rhai_plugin,
            // ASM 插件管理
            list_asm_plugins,
            get_asm_plugin,
            load_asm_plugins,
            read_log_file,
            get_log_file_path,
            execute_asm_plugin,
            upload_asm_plugin_content,
            delete_asm_plugin,
            update_asm_plugin,
            // 证书安装向导相关命令
            scan::api_commands::open_cert_file,
            // 扫描插件管理相关命令
            scan::plugin_commands::list_scan_plugins,
            scan::plugin_commands::get_scan_plugin,
            scan::plugin_commands::reload_scan_plugins,
            scan::plugin_commands::validate_scan_plugin,
            scan::plugin_commands::upload_scan_plugin_content,
            scan::plugin_commands::delete_scan_plugin,
            scan::plugin_commands::update_scan_plugin,
            scan::plugin_commands::execute_scan_plugin,
            open_url,
            // Repeater命令
            handler::repeater::repeater_send_request,
            handler::repeater::repeater_get_request_history,
            handler::repeater::repeater_delete_history_item,
            handler::repeater::repeater_get_settings,
            handler::repeater::repeater_save_settings,
            // 代理模块命令
            get_proxy_config,
            save_proxy_config,
            start_proxy,
            stop_proxy,
            get_proxy_status,
            get_proxy_intercept_request_status,
            set_proxy_intercept_request_status,
            get_proxy_history,
            clear_proxy_history,
            forward_intercepted_request,
            drop_intercepted_request,
            send_to_repeater,
            get_proxy_configs,
            save_proxy_config_with_id,
            test_asm_plugin,
            delete_proxy_config,
            start_proxy_by_id,
            stop_proxy_by_id,
            get_proxy_status_by_id,
            get_proxy_intercept_status,
            set_proxy_intercept_status,
            get_proxy_intercept_response_status,
            set_proxy_intercept_response_status,
            get_proxy_settings,
            // 添加响应拦截控制命令
            forward_intercepted_response,
            drop_intercepted_response,
            get_request_rules,
            get_response_rules,
            set_request_rules,
            set_response_rules,
            // 暴力破解模块命令
            brute_create_task,
            brute_get_tasks,
            brute_get_results,
            brute_delete_task,
            brute_start_task,
            brute_stop_task,
        ])
        .setup(|app| {
            // 存储全局AppHandle
            let _ = APP_HANDLE.set(app.handle().clone());

            // 初始化扫描器状态
            let main_window = app
                .get_webview_window("main")
                .expect("Failed to get main window");
            app.manage(ScannerState::new(main_window));

            // 初始化应用全局状态
            app.manage(AppState::new());

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show RShiled", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            TrayIconBuilder::new()
                .menu(&menu)
                // .menu_on_left_click(true)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    #[cfg(target_os = "macos")]
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        app.show().unwrap();
                    }
                    _ => {
                        println!("menu item {:?} not handled", event.id);
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                window.hide().unwrap();
                api.prevent_close();
            }

            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

mod cmds {
    #[tauri::command]
    pub fn read_log_file() -> Result<String, String> {
        let mut log_path = std::env::current_dir().map_err(|e| e.to_string())?;
        log_path.push("logs");

        // 创建日志目录如果不存在
        if !log_path.exists() {
            return Err("日志目录不存在".into());
        }

        // 获取当前日期格式化为YYYY-MM-DD
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let log_file_name = format!("rshiled-{}.log", today);
        log_path.push(&log_file_name);

        // 检查日志文件是否存在
        if !log_path.exists() {
            return Err(format!("今天的日志文件不存在: {}", log_file_name));
        }

        // 读取日志文件内容
        let log_content = std::fs::read_to_string(&log_path).map_err(|e| e.to_string())?;

        Ok(log_content)
    }

    #[tauri::command]
    pub fn get_log_file_path() -> Result<String, String> {
        let mut log_path = std::env::current_dir().map_err(|e| e.to_string())?;
        log_path.push("logs");

        // 获取当前日期格式化为YYYY-MM-DD
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let log_file_name = format!("rshiled-{}.log", today);
        log_path.push(&log_file_name);

        Ok(log_path.to_string_lossy().to_string())
    }
}

// 代理模块命令实现

/// 获取代理配置
#[tauri::command]
async fn get_proxy_config(state: tauri::State<'_, ProxyState>) -> Result<ProxyConfig, String> {
    Ok(state.get_config().await)
}

/// 保存代理配置
#[tauri::command]
async fn save_proxy_config(
    state: tauri::State<'_, ProxyState>,
    config: ProxyConfig,
) -> Result<(), String> {
    state.save_config(config).await
}

/// 启动代理
#[tauri::command]
async fn start_proxy(
    state: tauri::State<'_, ProxyState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    state.start_proxy(app_handle).await
}

/// 停止代理
#[tauri::command]
async fn stop_proxy(state: tauri::State<'_, ProxyState>) -> Result<(), String> {
    state.stop_proxy().await
}

/// 获取代理状态
#[tauri::command]
async fn get_proxy_status(state: tauri::State<'_, ProxyState>) -> Result<bool, String> {
    Ok(state.get_status().await)
}

/// 获取请求拦截状态
#[tauri::command]
async fn get_proxy_intercept_request_status(
    state: tauri::State<'_, ProxyState>,
) -> Result<bool, String> {
    Ok(state.get_intercept_request_status().await)
}

/// 设置请求拦截状态
#[tauri::command]
async fn set_proxy_intercept_request_status(
    state: tauri::State<'_, ProxyState>,
    enabled: bool,
) -> Result<(), String> {
    state.set_intercept_request_status(enabled).await
}

/// 获取代理历史记录
#[tauri::command]
async fn get_proxy_history(
    state: tauri::State<'_, ProxyState>,
) -> Result<Vec<RequestRecord>, String> {
    Ok(state.get_history().await)
}

/// 清空代理历史记录
#[tauri::command]
async fn clear_proxy_history(state: tauri::State<'_, ProxyState>) -> Result<(), String> {
    state.clear_history().await
}

/// 转发拦截的请求
#[tauri::command]
async fn forward_intercepted_request(
    state: tauri::State<'_, ProxyState>,
    request_id: String,
    method: Option<String>,
    url: Option<String>,
    headers: Option<std::collections::HashMap<String, String>>,
    body: Option<String>,
    response_headers: Option<std::collections::HashMap<String, String>>,
    response_body: Option<String>,
) -> Result<(), String> {
    let server_guard = state.server.read().await;
    match &*server_guard {
        Some(server) => {
            // 创建请求JSON字符串
            let request_data = serde_json::json!({
                "id": request_id,
                "method": method,
                "url": url,
                "headers": headers,
                "body": body
            });
            server.forward_intercepted(request_data.to_string()).await?;
            Ok(())
        }
        None => Err("Proxy server not running".to_string()),
    }
}

/// 丢弃拦截的请求
#[tauri::command]
async fn drop_intercepted_request(
    state: tauri::State<'_, ProxyState>,
    request_id: String,
) -> Result<(), String> {
    let server_guard = state.server.read().await;
    match &*server_guard {
        Some(server) => server.drop_intercepted(&request_id).await,
        None => Err("Proxy server not running".to_string()),
    }
}

#[tauri::command]
async fn send_to_repeater(request: serde_json::Value) -> Result<(), String> {
    // 从代理记录转换到Repeater历史记录
    let method = request["method"].as_str().unwrap_or("GET").to_string();
    let url = request["url"].as_str().unwrap_or("").to_string();

    // 转换请求头
    let mut headers = HashMap::new();
    if let Some(req_headers) = request["request_headers"].as_object() {
        for (key, value) in req_headers {
            if let Some(val_str) = value.as_str() {
                headers.insert(key.clone(), val_str.to_string());
            }
        }
    }

    // 获取请求体
    let body = request["request_body"].as_str().map(|s| s.to_string());

    // 使用已有的命令来创建历史记录
    handler::repeater::repeater_send_request(
        method, url, headers, body, None, // use_socket
        None, // target_host
        None, // target_port
        None, // use_https
        None, // raw_request
    )
    .await?;

    Ok(())
}

/// 获取所有代理配置
#[tauri::command]
async fn get_proxy_configs(
    state: tauri::State<'_, ProxyState>,
) -> Result<Vec<ProxyConfig>, String> {
    Ok(state.get_configs().await)
}

/// 使用指定ID保存代理配置
#[tauri::command]
async fn save_proxy_config_with_id(
    state: tauri::State<'_, ProxyState>,
    id: String,
    config: ProxyConfig,
) -> Result<(), String> {
    state.save_config_with_id(&id, config).await
}

/// 删除指定ID的代理配置
#[tauri::command]
async fn delete_proxy_config(
    state: tauri::State<'_, ProxyState>,
    id: String,
) -> Result<(), String> {
    state.delete_config(&id).await
}

/// 启动指定ID的代理
#[tauri::command]
async fn start_proxy_by_id(
    state: tauri::State<'_, ProxyState>,
    app_handle: tauri::AppHandle,
    id: String,
) -> Result<(), String> {
    state.start_proxy_by_id(&id, app_handle).await
}

/// 停止指定ID的代理
#[tauri::command]
async fn stop_proxy_by_id(state: tauri::State<'_, ProxyState>, id: String) -> Result<(), String> {
    state.stop_proxy_by_id(&id).await
}

/// 获取指定ID的代理状态
#[tauri::command]
async fn get_proxy_status_by_id(
    state: tauri::State<'_, ProxyState>,
    id: String,
) -> Result<bool, String> {
    Ok(state.get_status_by_id(&id).await)
}

/// 获取拦截响应状态
#[tauri::command]
async fn get_proxy_intercept_response_status(
    state: tauri::State<'_, ProxyState>,
) -> Result<bool, String> {
    Ok(state.get_intercept_response_status().await)
}

/// 获取拦截响应状态
#[tauri::command]
async fn get_proxy_intercept_status(state: tauri::State<'_, ProxyState>) -> Result<bool, String> {
    Ok(state.get_intercept_status().await)
}
/// 设置拦截状态
#[tauri::command]
async fn set_proxy_intercept_status(
    state: tauri::State<'_, ProxyState>,
    enabled: bool,
) -> Result<bool, String> {
    state.set_intercept_status(enabled).await?;
    Ok(true)
}

/// 设置拦截响应状态
#[tauri::command]
async fn set_proxy_intercept_response_status(
    state: tauri::State<'_, ProxyState>,
    enabled: bool,
) -> Result<bool, String> {
    state.set_intercept_response_status(enabled).await?;
    Ok(true)
}

/// 获取代理设置
#[tauri::command]
async fn get_proxy_settings(
    state: tauri::State<'_, ProxyState>,
) -> Result<serde_json::Value, String> {
    state.get_proxy_settings().await
}

/// 转发拦截的响应
#[tauri::command]
async fn forward_intercepted_response(
    state: tauri::State<'_, ProxyState>,
    responseId: String,
    status: Option<u16>,
    headers: Option<std::collections::HashMap<String, String>>,
    body: Option<String>,
) -> Result<(), String> {
    state
        .forward_intercepted_response(responseId, status, headers, body)
        .await
}

/// 丢弃拦截的响应
#[tauri::command]
async fn drop_intercepted_response(
    state: tauri::State<'_, ProxyState>,
    responseId: String,
) -> Result<(), String> {
    state.drop_intercepted_response(responseId).await
}
