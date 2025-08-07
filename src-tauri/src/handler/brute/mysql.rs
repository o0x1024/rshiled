use crate::handler::brute::{BruteForceResult, BruteForceTask, Protocol, TaskStatus, BruteForceManager};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::time::{Duration, Instant};
use futures::future::join_all;

// MySQL暴力破解功能
pub async fn mysql_brute_force(
    task: BruteForceTask,
    manager_state: Arc<Mutex<BruteForceManager>>
) -> Vec<BruteForceResult> {
    let target = task.target.clone();
    let port = task.port;
    let task_id = task.id.unwrap_or(0);
    let num_threads = task.threads.max(1) as usize;

    let usernames = match (task.usernames.clone(), task.username_file.clone()) {
        (Some(names), _) if !names.is_empty() => names,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["root".to_string(), "admin".to_string(), "mysql".to_string()],
    };

    let passwords = match (task.passwords.clone(), task.password_file.clone()) {
        (Some(pwds), _) if !pwds.is_empty() => pwds,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["".to_string(), "password".to_string(), "123456".to_string(), "root".to_string(), "mysql".to_string()],
    };

    if usernames.is_empty() || passwords.is_empty() {
        return vec![BruteForceResult {
            task_id,
            target,
            protocol: Protocol::MySQL,
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
            let t_timeout_duration = Duration::from_secs(task.timeout);
            let t_task_id = task_id;
            let t_protocol_clone = task.protocol.clone();

            spawned_tasks.push(tokio::spawn(async move {
                let start_time = Instant::now();
                let login_attempt_result = try_mysql_login(&t_target, t_port, &t_username, &t_password, t_timeout_duration).await;
                let duration = start_time.elapsed().as_millis() as u64;
                drop(permit); // Release semaphore permit

                let success = login_attempt_result.is_ok();

                if success {
                    Some(BruteForceResult {
                        task_id: t_task_id,
                        target: t_target,
                        protocol: t_protocol_clone,
                        username: t_username,
                        password: t_password,
                        success, // Will be true
                        time_taken: duration,
                        error: None, // No error on success
                    })
                } else {
                    None // Do not return a result for failed attempts
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
                // Login attempt failed, do nothing
            }
            Err(e) => {
                eprintln!("MySQL brute force sub-task panicked: {}", e);
                // Do not add a placeholder error result
            }
        }
    }
    results
}

// 尝试MySQL登录
async fn try_mysql_login(host: &str, port: u16, username: &str, password: &str, timeout: Duration) -> Result<(), String> {
    let host_clone = host.to_string();
    let username_clone = username.to_string();
    let password_clone = password.to_string();
    
    tokio::task::spawn_blocking(move || {
        let opts = mysql::OptsBuilder::new()
            .ip_or_hostname(Some(host_clone))
            .tcp_port(port)
            .user(Some(username_clone))
            .pass(Some(password_clone))
            .tcp_connect_timeout(Some(timeout));
        
        match mysql::Conn::new(opts) {
            Ok(_conn) => {
                Ok(())
            }
            Err(e) => Err(format!("MySQL connection failed: {}", e)),
        }
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