use std::process::Command;
use std::str;
use std::sync::Arc;
use tokio::sync::Mutex;

use log::{error, info};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query, query_scalar};
use tauri::Manager;
use tokio::task::JoinHandle;

use super::asm_task::INNERASK_MODULE;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Port {
    pub id: Option<i32>,
    pub task_id: i32,
    pub ip_addr: String,
    pub port: i32,
    pub service: Option<String>,
    pub version: Option<String>,
    pub protocol: Option<String>,
    pub state: Option<String>,
    pub create_at: i64,
    pub update_at: i64,
}

fn is_nmap_installed() -> bool {
    #[cfg(target_os = "windows")]
    let check_cmd = Command::new("where").arg("nmap").output();

    #[cfg(not(target_os = "windows"))]
    let check_cmd = Command::new("which").arg("nmap").output();

    match check_cmd {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

pub async fn port_scan_by_buildin(_task_id: &i32) -> Result<(), String> {
    //判断是否安装了nmap，测试nmap是否可以正常运行

    Ok(())
}

#[derive(Default)]
struct MyState {
    s: std::sync::Mutex<String>,
    t: std::sync::Mutex<std::collections::HashMap<String, String>>,
}
// remember to call `.manage(MyState::default())`
#[tauri::command]
async fn get_ports(
    page: i32,
    pagesize: i32,
    dtype: String,
    filter: String,
    query: String,
) -> Result<serde_json::Value, String> {
    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => {
            error!("Global variable not initialized");
            return Err("Global variable not initialized".into());
        }
    };
    let read_conn = Arc::clone(&task_module.read_conn);
    let mut query_builder =
        sqlx::QueryBuilder::<sqlx::Sqlite>::new("SELECT * FROM port WHERE task_id = ?");

    let allowed_filters = ["task_id", "ip_addr", "port", "service", "protocol", "state"]; // 允许的列名
    if !allowed_filters.contains(&filter.as_str()) {
        return Err("Invalid filter".into());
    }

    let domain_pattern = format!("%{}%", query);
    match dtype.as_str() {
        "all" => {
            query_builder.push("SELECT * FROM port WHERE task_id = ?");
        }
        _ => {
            query_builder.push("SELECT * FROM port WHERE task_id = ?");
        }
    }

    query_builder.push(" LIMIT ? OFFSET ?");
    query_builder.push(" ORDER BY create_at DESC");
    query_builder.push(" OFFSET ? LIMIT ?");

    // let ports: Vec<Port> = query_builder.build().fetch_all(&*read_conn).await.map_err(|e| e.to_string())?;

    let total_count: i64 = query_scalar::<_, i64>("SELECT COUNT(*) FROM port WHERE task_id = ?")
        // .bind(task_id)
        .fetch_one(&*read_conn)
        .await
        .unwrap_or_default();

    Ok(serde_json::json!({
        "list": "ports",
        "total": total_count
    }))
}

#[tauri::command]
pub async fn port_scan_by_nmap(task_id: &i32) -> Result<(), String> {
    //判断是否安装了nmap，测试nmap是否可以正常运行
    if !is_nmap_installed() {
        return Err("nmap not found".into());
    }

    let ipaddrs = {
        let task_module = match INNERASK_MODULE.get() {
            Some(tm) => tm,
            None => {
                error!("Global variable not initialized");
                return Err("Global variable not initialized".into());
            }
        };

        let read_conn = Arc::clone(&task_module.read_conn);
        match query_scalar::<_, String>(
            "SELECT ip_addr FROM ips WHERE task_id = ? ORDER BY create_at DESC",
        )
        .bind(task_id)
        .fetch_all(&*read_conn)
        .await
        {
            Ok(ipaddrs) => ipaddrs,
            Err(e) => Vec::new(),
        }
    };

    let mut handle_list = Vec::<JoinHandle<()>>::new();
    let task_id_clone = task_id.clone();
    handle_list.push(tokio::spawn(async move {
        if let Err(e) = batch_scan_ip_port(&ipaddrs, &task_id_clone).await {
            error!("Failed to scan {:?}", e);
        }
    }));

    // 等待所有扫描任务完成
    for handle in handle_list {
        let _ = tokio::join!(handle);
    }

    info!("所有IP端口扫描任务已完成");
    Ok(())
}

async fn batch_scan_ip_port(
    ips: &Vec<String>,
    task_id: &i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut outputs = Vec::new();

    //nmap一次性扫描所有目标
    let output = Command::new("nmap")
            .args([
                "-p", "22,80,443,843,3389,8007-8011,8443,9090,8080-8091,8093,8099,5000-5004,2222,3306,1433,21,25", // 扫描所有端口
                "-sV",           // 检测服务版本
                "--open",        // 只显示开放的端口
                "-T4",           // 设置扫描速度
                &ips.join(" ")   //把ip地址拼接成字符串用空隔隔开
            ])
            .output()?;

    outputs.push( output);

    for output in outputs {
        if !output.status.success() {
            let error_msg = str::from_utf8(&output.stderr)?;
            error!("nmap扫描失败: {}", error_msg);
            return Err(format!("nmap scan failed: {}", error_msg).into());
        }
        let output_str = str::from_utf8(&output.stdout)?;
        let ports = parse_nmap_output(output_str, &task_id);
        save_ports_to_db(&ports).await?;
    }

    Ok(())
}

fn parse_nmap_output(output: &str, task_id: &i32) -> Vec<Port> {
    
    let mut ports = Vec::new();
    let now = chrono::Local::now().timestamp();

    //从结果中解析ip地址

    // 解析nmap输出
    // 示例输出行: "80/tcp   open  http    Apache httpd 2.4.41"
    for line in output.lines() {
        if line.contains("/tcp") || line.contains("/udp") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                // 提取端口号
                let port_protocol: Vec<&str> = parts[0].split('/').collect();
                if port_protocol.len() >= 2 {
                    if let Ok(port_num) = port_protocol[0].parse::<i32>() {
                        // 提取协议(tcp/udp)
                        let protocol = port_protocol[1].to_string();

                        // 提取状态(open/closed)
                        let state = parts[1].to_string();

                        // 提取服务名称
                        let service = if parts.len() > 2 {
                            Some(parts[2].to_string())
                        } else {
                            None
                        };
                        let ip_addr = if parts.len() > 3 {
                            Some(parts[3..].join(" "))
                        } else {
                            None
                        };
                        // 提取版本信息(可能跨越多个部分)
                        let version = if parts.len() > 3 {
                            Some(parts[3..].join(" "))
                        } else {
                            None
                        };

                        ports.push(Port {
                            id: None,
                            task_id: *task_id,
                            ip_addr: ip_addr.unwrap_or_default(),
                            port: port_num,
                            service,
                            version,
                            protocol: Some(protocol),
                            state: Some(state),
                            create_at: now,
                            update_at: now,
                        });
                    }
                }
            }
        }
    }

    ports
}

async fn save_ports_to_db(ports: &[Port]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if ports.is_empty() {
        return Ok(());
    }

    let task_module = INNERASK_MODULE
        .get()
        .ok_or("Global variable not initialized")?;
    let write_conn = Arc::clone(&task_module.write_conn);

    // 开始事务
    let mut tx = write_conn.begin().await?;

    for port in ports {
        // 检查端口是否已存在
        let existing =
            query_scalar::<_, i32>("SELECT COUNT(*) FROM port WHERE ip_addr = ? AND port = ?")
                .bind(&port.ip_addr)
                .bind(port.port)
                .fetch_one(&mut *tx)
                .await?;

        if existing > 0 {
            // 更新现有记录
            query(
                "UPDATE port SET service = ?, version = ?, protocol = ?, state = ?, update_at = ? WHERE ip_addr = ? AND port = ?"
            )
            .bind(&port.service)
            .bind(&port.version)
            .bind(&port.protocol)
            .bind(&port.state)
            .bind(port.update_at)
            .bind(&port.ip_addr)
            .bind(port.port)
            .execute(&mut *tx)
            .await?;
        } else {
            // 插入新记录
            query(
                "INSERT INTO port (task_id, ip_addr, port, service, version, protocol, state, create_at, update_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT DO NOTHING"
            )
            .bind(port.task_id)
            .bind(&port.ip_addr)
            .bind(port.port)
            .bind(&port.service)
            .bind(&port.version)
            .bind(&port.protocol)
            .bind(&port.state)
            .bind(port.create_at)
            .bind(port.update_at)
            .execute(&mut *tx)
            .await?;
        }

        info!("已保存端口信息: {}:{}", port.ip_addr, port.port);
    }

    // 提交事务
    tx.commit().await?;

    Ok(())
}

/// 使用插件扫描端口
pub async fn scan_ports_by_plugin(task_id: &i32) {
    // 获取全局AppHandle
    let app_handle = match crate::APP_HANDLE.get() {
        Some(handle) => handle.clone(),
        None => {
            log::error!("全局APP_HANDLE未初始化");
            return;
        }
    };
    let ips: Vec<String> = {
        let task_module = INNERASK_MODULE
            .get()
            .expect("Global variable not initialized");
        let pool_clone = Arc::clone(&task_module.read_conn);

        query_scalar("SELECT ip FROM asm_ip WHERE task_id = ?")
            .bind(task_id)
            .fetch_all(&*pool_clone)
            .await
            .unwrap_or_default()
    };

    // 创建共享的端口收集器
    let port_collector = Arc::new(Mutex::new(
        Vec::<(String, i32, String, String, String)>::new(),
    )); // (ip, port, service, protocol, version)
    let batch_size = 100; // 每批处理100条数据

    let mut handle_list = Vec::new();

    for ip in ips {
        let tid = task_id.clone();
        let ip_addr = ip.clone();
        let app_handle_clone = app_handle.clone();
        let port_collector_clone = Arc::clone(&port_collector);

        let handle = tokio::spawn(async move {
            // 获取ASM插件管理器
            let state = app_handle_clone
                .state::<crate::handler::asm::plugin_commands::AsmPluginManagerState>();
            let manager = state.inner.lock().await;

            // 查找所有端口扫描插件
            let port_plugins = manager
                .get_all_plugins()
                .await
                .into_iter()
                .filter(|p| p.plugin_type == "port_scanning")
                .collect::<Vec<_>>();

            // 执行每个端口扫描插件
            for plugin in port_plugins {
                log::info!("Executing port scanning plugin: {}", plugin.name);

                // 创建插件上下文
                let context = crate::handler::asm::plugin::AsmPluginContext {
                    task_id: tid,
                    target: ip_addr.clone(),
                    targets: Some(vec![ip_addr.clone()]),
                    custom_params: None,
                };

                // 执行插件
                match manager
                    .execute_plugin(&plugin.name, "port_scanning", context)
                    .await
                {
                    Ok(result) => {
                        if result.success {
                            log::info!(
                                "Port scanning plugin {} executed successfully",
                                plugin.name
                            );

                            // 处理发现的端口
                            if let Some(found_ports) = result.found_ports {
                                if !found_ports.is_empty() {
                                    log::info!(
                                        "Plugin {} found {} ports",
                                        plugin.name,
                                        found_ports.len()
                                    );

                                    // 收集端口数据而不是立即写入
                                    let mut collector = port_collector_clone.lock().await;

                                    for port_value in found_ports {
                                        if let Some(port_obj) = port_value.as_object() {
                                            // 提取端口信息
                                            let port_number = port_obj
                                                .get("port")
                                                .and_then(|v| v.as_u64())
                                                .unwrap_or(0)
                                                as i32;

                                            if port_number == 0 {
                                                continue;
                                            }

                                            let service = port_obj
                                                .get("service")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("unknown")
                                                .to_string();

                                            let protocol = port_obj
                                                .get("protocol")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("tcp")
                                                .to_string();

                                            let version = port_obj
                                                .get("version")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("")
                                                .to_string();

                                            // 添加到收集器
                                            collector.push((
                                                ip_addr.clone(),
                                                port_number,
                                                service,
                                                protocol,
                                                version,
                                            ));
                                        }
                                    }

                                    // 如果收集的端口数量超过批处理大小，则立即处理一批
                                    if collector.len() >= batch_size {
                                        let ports_to_process =
                                            collector.drain(..).collect::<Vec<_>>();
                                        drop(collector); // 释放锁

                                        // 处理批量端口数据
                                        if let Err(e) =
                                            process_port_batch(tid, ports_to_process).await
                                        {
                                            log::error!("Failed to process port batch: {}", e);
                                        }
                                    }
                                }
                            }
                        } else {
                            log::warn!(
                                "Port scanning plugin {} execution failed: {}",
                                plugin.name,
                                result.message
                            );
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to execute port scanning plugin {}: {}",
                            plugin.name,
                            e
                        );
                    }
                }
            }
        });

        handle_list.push(handle);
    }

    // 等待所有扫描任务完成
    for handle in handle_list {
        let _ = tokio::join!(handle);
    }

    // 处理剩余的端口数据
    let remaining_ports = port_collector.lock().await.drain(..).collect::<Vec<_>>();
    if !remaining_ports.is_empty() {
        if let Err(e) = process_port_batch(*task_id, remaining_ports).await {
            log::error!("Failed to process remaining ports: {}", e);
        }
    }
}

// 批量处理端口数据
async fn process_port_batch(
    task_id: i32,
    ports: Vec<(String, i32, String, String, String)>,
) -> Result<(), Box<dyn std::error::Error>> {
    if ports.is_empty() {
        return Ok(());
    }

    // 保存长度用于之后的日志
    let ports_count = ports.len();

    // 获取数据库连接
    let task_module = super::asm_task::INNERASK_MODULE
        .get()
        .ok_or("Global variable not initialized")?;
    let write_conn = Arc::clone(&task_module.write_conn);

    // 开始事务
    let mut tx = write_conn.begin().await?;
    let now = chrono::Local::now().timestamp();

    // 使用批量插入
    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO port (task_id, ip_addr, port, service, protocol, version, create_at, update_at) VALUES "
    );

    query_builder.push_values(ports, |mut b, (ip, port, service, protocol, version)| {
        b.push_bind(task_id)
            .push_bind(ip)
            .push_bind(port)
            .push_bind(service)
            .push_bind(protocol)
            .push_bind(version)
            .push_bind(now)
            .push_bind(now);
    });

    // 添加冲突处理
    query_builder.push(" ON CONFLICT DO NOTHING");

    // 执行查询
    query_builder.build().execute(&mut *tx).await?;

    // 提交事务
    tx.commit().await?;
    log::info!("Batch processed {} ports", ports_count);

    Ok(())
}
