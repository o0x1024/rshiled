use crate::handler::brute::{BruteForceResult, BruteForceTask, Protocol, TaskStatus, BruteForceManager};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::time::{Duration, Instant};
use futures::future::join_all;
use rand::Rng;
use tiberius::{Config, Client, AuthMethod};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

// MSSQL暴力破解功能
pub async fn mssql_brute_force(
    task: BruteForceTask,
    manager_state: Arc<Mutex<BruteForceManager>>
) -> Vec<BruteForceResult> {
    let target = task.target.clone();
    let port = if task.port == 0 { 1433 } else { task.port }; // 默认MSSQL端口1433
    let task_id = task.id.unwrap_or(0);
    let num_threads = task.threads.max(1) as usize;
    
    // 获取用户名和密码列表
    let usernames = match (task.usernames.clone(), task.username_file.clone()) {
        (Some(names), _) if !names.is_empty() => names,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["sa".to_string(), "admin".to_string()],
    };

    let passwords = match (task.passwords.clone(), task.password_file.clone()) {
        (Some(pwds), _) if !pwds.is_empty() => pwds,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["".to_string(), "sa".to_string(), "password".to_string(), "123456".to_string()],
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
                
                // MSSQL连接尝试
                let result = try_mssql_login(&t_target, t_port, &t_username, &t_password, t_timeout_secs).await;
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
                eprintln!("MSSQL brute force task panicked: {}", e);
            }
        }
    }

    results
}

// 尝试MSSQL登录
async fn try_mssql_login(host: &str, port: u16, username: &str, password: &str, timeout_secs: u64) -> Result<(), String> {
    // 配置MSSQL连接
    let mut config = Config::new();
    config.host(host);
    config.port(port);
    config.authentication(AuthMethod::sql_server(username, password));
    config.trust_cert(); // 开发环境可信任证书，生产环境应谨慎

    // 尝试连接，并为连接操作设置超时
    let tcp = match tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        TcpStream::connect(format!("{}:{}", host, port))
    ).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => return Err(format!("Failed to connect to TCP stream: {}", e)),
        Err(_) => return Err(format!("Timeout connecting to TCP stream after {} seconds", timeout_secs)),
    };
    
    tcp.set_nodelay(true).map_err(|e| format!("Failed to set nodelay: {}", e))?;
    
    // 尝试连接客户端，也应该考虑为这个操作设置超时
    // 注意：tiberius::Client::connect 本身可能没有显式的超时参数
    // 但底层的异步操作如果阻塞太久，会被 Tokio 运行时取消（如果任务被取消）
    // 或者如果需要更精确的控制，可以将 Client::connect 也包在 tokio::time::timeout 中
    match tokio::time::timeout(
        Duration::from_secs(timeout_secs), // 为 tiberius 客户端连接也应用超时
        Client::connect(config, tcp.compat_write())
    ).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(format!("MSSQL authentication failed: {}", e)),
        Err(_) => Err(format!("Timeout during MSSQL authentication after {} seconds", timeout_secs)),
    }
} 