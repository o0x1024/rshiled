use crate::handler::brute::{BruteForceResult, BruteForceTask, Protocol, TaskStatus, BruteForceManager, read_wordlist_file};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::time::{Duration, Instant};
use suppaftp::FtpStream; // Using suppaftp
use std::net::ToSocketAddrs; // Required for parsing socket address
use futures::future::join_all;

// FTP暴力破解功能
pub async fn ftp_brute_force(
    task: BruteForceTask,
    manager_state: Arc<Mutex<BruteForceManager>>
) -> Vec<BruteForceResult> {
    let target = task.target.clone();
    let port = task.port;
    let socket_addr_template = format!("{}:{}", target, port); // Template for socket address string
    let task_id = task.id.unwrap_or(0);
    let num_threads = task.threads.max(1) as usize;

    let usernames = match (task.usernames.clone(), task.username_file.clone()) {
        (Some(names), _) if !names.is_empty() => names,
        (_, Some(file)) => read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["anonymous".to_string(), "ftp".to_string(), "user".to_string()], // Default FTP usernames
    };

    let passwords = match (task.passwords.clone(), task.password_file.clone()) {
        (Some(pwds), _) if !pwds.is_empty() => pwds,
        (_, Some(file)) => read_wordlist_file(&file).await.unwrap_or_default(),
        _ => vec!["anonymous".to_string(), "ftp".to_string(), "user".to_string(), "password".to_string()], // Default FTP passwords
    };

    if usernames.is_empty() || passwords.is_empty() {
        return vec![BruteForceResult {
            task_id,
            target,
            protocol: Protocol::FTP,
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
        // Check if task was stopped externally
        {
            let guard = manager_state.lock().await;
            if let Some(current_task_in_manager) = guard.tasks.iter().find(|t| t.id == Some(task_id)) {
                if current_task_in_manager.status == TaskStatus::Stopped {
                    break 'outer;
                }
            } else {
                break 'outer; // Task removed or not found
            }
        }

        for password in &passwords {
            // Check again if task was stopped externally
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

            let t_socket_addr_str = socket_addr_template.clone(); // Pass the string to the task
            let t_username = username.clone();
            let t_password = password.clone();
            let t_timeout_secs = task.timeout;
            let t_task_id = task_id;
            let t_target_clone = target.clone();
            let t_protocol_clone = task.protocol.clone();

            spawned_tasks.push(tokio::spawn(async move {
                let start_time = Instant::now();
                
                let username_for_blocking = t_username.clone();
                let password_for_blocking = t_password.clone();
                let socket_addr_for_blocking = t_socket_addr_str.clone();

                let login_attempt_result = tokio::task::spawn_blocking(move || {
                    let resolved_addr = match socket_addr_for_blocking.as_str().to_socket_addrs() {
                        Ok(mut addrs) => match addrs.next() {
                            Some(addr) => addr,
                            None => return Err(format!("FTP address resolution failed: no addresses for {}", socket_addr_for_blocking)),
                        },
                        Err(e) => return Err(format!("FTP address resolution failed for {}: {}", socket_addr_for_blocking, e)),
                    };

                    let mut ftp_stream = match FtpStream::connect_timeout(resolved_addr, Duration::from_secs(t_timeout_secs)) {
                        Ok(stream) => stream,
                        Err(e) => return Err(format!("FTP connect failed: {}", e)),
                    };
                    
                    match ftp_stream.login(&username_for_blocking, &password_for_blocking) {
                        Ok(_) => {
                            let _ = ftp_stream.quit();
                            Ok(())
                        }
                        Err(e) => Err(format!("FTP login failed: {}", e)),
                    }
                }).await.unwrap_or_else(|e| Err(format!("FTP sub-task panic: {}",e)));
                
                let duration = start_time.elapsed().as_millis() as u64;
                drop(permit);

                let success = login_attempt_result.is_ok();

                if success {
                    Some(BruteForceResult {
                        task_id: t_task_id,
                        target: t_target_clone,
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
                eprintln!("FTP brute force main sub-task join error (panicked?): {}", e);
                // Do not add a placeholder error result
            }
        }
    }
    results
} 