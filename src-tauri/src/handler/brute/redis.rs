use crate::handler::brute::{BruteForceResult, BruteForceTask, Protocol, TaskStatus, BruteForceManager};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::time::{Duration, Instant};
use futures::future::join_all;
use rand::Rng;
use redis::AsyncCommands;

// Redis暴力破解功能
pub async fn redis_brute_force(
    task: BruteForceTask,
    manager_state: Arc<Mutex<BruteForceManager>>
) -> Vec<BruteForceResult> {
    let target = task.target.clone();
    let port = if task.port == 0 { 6379 } else { task.port }; // 默认Redis端口6379
    let task_id = task.id.unwrap_or(0);
    let num_threads = task.threads.max(1) as usize;
    
    // Redis通常只有密码没有用户名，但我们仍然保持一致的接口
    let passwords = match (task.passwords.clone(), task.password_file.clone()) {
        (Some(pwds), _) if !pwds.is_empty() => pwds,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["".to_string(), "redis".to_string(), "password".to_string(), "admin".to_string()],
    };

    if passwords.is_empty() {
        return Vec::new();
    }

    let semaphore = Arc::new(Semaphore::new(num_threads));
    let mut spawned_tasks = Vec::new();

    'outer: for password in &passwords {
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
        let t_password = password.clone();
        let t_timeout_secs = task.timeout;
        let t_task_id = task_id;
        let t_protocol_clone = task.protocol.clone();
        
        // 增加随机延迟以避免触发服务器防御机制
        let delay_ms = rand::thread_rng().gen_range(10..100);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;

        spawned_tasks.push(tokio::spawn(async move {
            let start_time = Instant::now();
            
            // Redis连接尝试
            let result = try_redis_login(&t_target, t_port, &t_password, t_timeout_secs).await;
            let duration = start_time.elapsed().as_millis() as u64;
            drop(permit);
            
            match result {
                Ok(_) => {
                    // 登录成功
                    Some(BruteForceResult {
                        task_id: t_task_id,
                        target: t_target,
                        protocol: t_protocol_clone,
                        username: "".to_string(), // Redis没有用户名概念
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
                eprintln!("Redis brute force task panicked: {}", e);
            }
        }
    }

    results
}

// 尝试Redis登录
async fn try_redis_login(host: &str, port: u16, password: &str, timeout_secs: u64) -> Result<(), String> {
    // 构建Redis连接URL, 包含连接超时
    let redis_url = if password.is_empty() {
        format!("redis://{}:{}/?connect_timeout={}s&socket_timeout={}s", host, port, timeout_secs, timeout_secs)
    } else {
        format!("redis://:{}@{}:{}/?connect_timeout={}s&socket_timeout={}s", password, host, port, timeout_secs, timeout_secs)
    };
    
    let client = redis::Client::open(redis_url)
        .map_err(|e| format!("Failed to create Redis client: {}", e))?;
    
    // 为获取连接操作本身也设置一个超时，并显式指定 con 的类型
    let mut con: redis::aio::MultiplexedConnection = match tokio::time::timeout(
        Duration::from_secs(timeout_secs), // 连接超时
        client.get_multiplexed_async_connection() // 直接获取 MultiplexedConnection
    ).await {
        Ok(Ok(conn)) => conn,
        Ok(Err(e)) => return Err(format!("Failed to connect to Redis: {}", e)),
        Err(_) => return Err(format!("Timeout connecting to Redis after {} seconds", timeout_secs)),
    };
    
    // 执行一个简单的PING命令验证连接，并为 PING 命令本身设置超时
    // 注意: redis-rs 0.23 的异步连接超时主要通过连接字符串中的 socket_timeout 控制
    // 此处的 tokio::time::timeout 额外增加了一层保障
    match tokio::time::timeout(
        Duration::from_secs(timeout_secs), 
        redis::cmd("PING").query_async::<_, String>(&mut con)
    ).await {
        Ok(Ok(pong_response)) => {
            // 验证PING返回PONG
            if pong_response.to_lowercase() == "pong" {
                Ok(())
            } else {
                Err(format!("Redis replied with unexpected response: {}", pong_response))
            }
        }
        Ok(Err(e)) => Err(format!("Redis PING command failed: {}", e)),
        Err(_) => Err(format!("Redis PING command timed out after {} seconds", timeout_secs)),
    }
} 