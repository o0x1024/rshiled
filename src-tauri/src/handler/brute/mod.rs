use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task;
use std::path::PathBuf;

// 支持的协议定义
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Protocol {
    SSH,
    SMB,
    RDP,
    MySQL,
    MSSQL,
    Redis,
    PostgreSQL,
    Oracle,
    FTP,
    Telnet,
}

// 暴力破解任务配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BruteForceTask {
    pub id: Option<i64>,
    pub name: String,
    pub target: String,
    pub port: u16,
    pub protocol: Protocol,
    pub username_file: Option<String>,
    pub password_file: Option<String>,
    pub usernames: Option<Vec<String>>,
    pub passwords: Option<Vec<String>>,
    pub threads: u8,
    pub timeout: u64,
    pub created_at: Option<i64>,
    pub status: TaskStatus,
}

// 任务状态
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Stopped,
}

// 暴力破解结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BruteForceResult {
    pub task_id: i64,
    pub target: String,
    pub protocol: Protocol,
    pub username: String,
    pub password: String,
    pub success: bool,
    pub time_taken: u64,
    pub error: Option<String>,
}

// 暴力破解任务管理器
#[derive(Clone)]
pub struct BruteForceManager {
    pub tasks: Vec<BruteForceTask>,
    pub results: Vec<BruteForceResult>,
}

impl BruteForceManager {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            results: Vec::new(),
        }
    }

    // 添加任务
    pub fn add_task(&mut self, task: BruteForceTask) -> i64 {
        let id = (self.tasks.len() + 1) as i64;
        let mut task = task;
        task.id = Some(id);
        task.created_at = Some(chrono::Utc::now().timestamp());
        task.status = TaskStatus::Pending;
        self.tasks.push(task);
        id
    }

    // 获取所有任务
    pub fn get_tasks(&self) -> &Vec<BruteForceTask> {
        &self.tasks
    }

    // 获取任务结果
    pub fn get_results(&self, task_id: i64) -> Vec<BruteForceResult> {
        self.results.iter()
            .filter(|r| r.task_id == task_id)
            .cloned()
            .collect()
    }

    // 获取所有结果
    pub fn get_all_results(&self) -> &Vec<BruteForceResult> {
        &self.results
    }

    // 删除任务
    pub fn delete_task(&mut self, task_id: i64) -> bool {
        if let Some(index) = self.tasks.iter().position(|t| t.id == Some(task_id)) {
            self.tasks.remove(index);
            // 同时删除相关结果
            self.results.retain(|r| r.task_id != task_id);
            true
        } else {
            false
        }
    }

    // 启动任务的内部逻辑，被Tauri命令调用
    // 这个方法在 manager 已经被锁定的情况下调用
    pub fn start_task_internal(&mut self, task_id: i64, manager_state_arc: Arc<Mutex<BruteForceManager>>) -> Result<(), String> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == Some(task_id)) {
            if task.status == TaskStatus::Running {
                return Err("Task is already running".to_string());
            }
            
            task.status = TaskStatus::Running; // 在共享状态中标记为运行中
            let task_clone = task.clone(); // 克隆任务配置以传递给异步块
            
            // 创建执行任务
            tokio::spawn(async move {
                // manager_state_arc 被移动到这个异步块中
                // 这是 BruteForceState 持有的同一个 Arc<Mutex<BruteForceManager>>

                let results = match task_clone.protocol {
                    Protocol::SSH => {
                        // 传递 task_clone 和 manager_state_arc 以便 ssh_brute_force 可以检查实时状态
                        crate::handler::brute::ssh::ssh_brute_force(task_clone.clone(), Arc::clone(&manager_state_arc)).await
                    },
                    Protocol::MySQL => {
                        crate::handler::brute::mysql::mysql_brute_force(task_clone.clone(), Arc::clone(&manager_state_arc)).await
                    },
                    Protocol::FTP => {
                        crate::handler::brute::ftp::ftp_brute_force(task_clone.clone(), Arc::clone(&manager_state_arc)).await
                    },
                    Protocol::SMB => {
                        crate::handler::brute::smb::smb_brute_force(task_clone.clone(), Arc::clone(&manager_state_arc)).await
                    },
                    Protocol::RDP => {
                        crate::handler::brute::rdp::rdp_brute_force(task_clone.clone(), Arc::clone(&manager_state_arc)).await
                    },
                    Protocol::MSSQL => {
                        crate::handler::brute::mssql::mssql_brute_force(task_clone.clone(), Arc::clone(&manager_state_arc)).await
                    },
                    Protocol::Redis => {
                        crate::handler::brute::redis::redis_brute_force(task_clone.clone(), Arc::clone(&manager_state_arc)).await
                    },
                    Protocol::PostgreSQL => {
                        crate::handler::brute::postgresql::postgresql_brute_force(task_clone.clone(), Arc::clone(&manager_state_arc)).await
                    },
                    Protocol::Oracle => {
                        crate::handler::brute::oracle::oracle_brute_force(task_clone.clone(), Arc::clone(&manager_state_arc)).await
                    },
                    Protocol::Telnet => {
                        crate::handler::brute::telnet::telnet_brute_force(task_clone.clone(), Arc::clone(&manager_state_arc)).await
                    }
                };
                
                // 锁定共享管理器以保存结果和更新最终状态
                let mut manager_guard = manager_state_arc.lock().await;
                for result in results {
                    manager_guard.add_result(result);
                }
                
                // 在共享管理器中更新任务状态
                if let Some(final_task_state) = manager_guard.tasks.iter_mut().find(|t| t.id == Some(task_id)) {
                    // 只有当任务仍然是 Running 状态时才标记为 Completed（以防中途被 Stopped）
                    if final_task_state.status == TaskStatus::Running {
                         final_task_state.status = TaskStatus::Completed;
                    }
                }
            });
            
            Ok(())
        } else {
            Err(format!("Task with ID {} not found", task_id))
        }
    }
    
    // 停止任务
    pub fn stop_task(&mut self, task_id: i64) -> Result<(), String> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == Some(task_id)) {
            if task.status != TaskStatus::Running && task.status != TaskStatus::Pending {
                return Err(format!("Task is not in a stoppable state (current: {:?})", task.status));
            }
            task.status = TaskStatus::Stopped;
            Ok(())
        } else {
            Err(format!("Task with ID {} not found for stopping", task_id))
        }
    }
    
    // 添加结果
    pub fn add_result(&mut self, result: BruteForceResult) {
        self.results.push(result);
    }
}

// 从文件读取字典 (公共辅助函数)
pub async fn read_wordlist_file(file_path: &str) -> Result<Vec<String>, String> {
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

// 共享状态
pub struct BruteForceState {
    pub manager: Arc<Mutex<BruteForceManager>>,
}

impl BruteForceState {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(Mutex::new(BruteForceManager::new())),
        }
    }
}

// Tauri命令
// 创建新任务
#[tauri::command]
pub async fn brute_create_task(
    state: tauri::State<'_, BruteForceState>,
    task: BruteForceTask,
) -> Result<i64, String> {
    let mut manager = state.manager.lock().await;
    let id = manager.add_task(task);
    Ok(id)
}

// 获取所有任务
#[tauri::command]
pub async fn brute_get_tasks(
    state: tauri::State<'_, BruteForceState>,
) -> Result<Vec<BruteForceTask>, String> {
    let manager = state.manager.lock().await;
    Ok(manager.get_tasks().clone())
}

// 获取任务结果
#[tauri::command]
pub async fn brute_get_results(
    state: tauri::State<'_, BruteForceState>,
    task_id: i64,
) -> Result<Vec<BruteForceResult>, String> {
    let manager = state.manager.lock().await;
    Ok(manager.get_results(task_id))
}

// 删除任务
#[tauri::command]
pub async fn brute_delete_task(
    state: tauri::State<'_, BruteForceState>,
    task_id: i64,
) -> Result<bool, String> {
    let mut manager = state.manager.lock().await;
    Ok(manager.delete_task(task_id))
}

// 开始任务
#[tauri::command]
pub async fn brute_start_task(
    state: tauri::State<'_, BruteForceState>,
    task_id: i64,
) -> Result<(), String> {
    let manager_arc_clone = Arc::clone(&state.manager); // 克隆Arc，指向同一个Mutex
    let mut manager_guard = state.manager.lock().await; // 锁定共享管理器
    
    // 调用内部方法来设置状态并启动异步任务
    // 将克隆的Arc传递给它，以便生成的任务可以使用它
    manager_guard.start_task_internal(task_id, manager_arc_clone)
}

// 停止任务
#[tauri::command]
pub async fn brute_stop_task(
    state: tauri::State<'_, BruteForceState>,
    task_id: i64,
) -> Result<(), String> {
    let mut manager = state.manager.lock().await;
    manager.stop_task(task_id)
}

// 导入各个协议模块
mod ssh;
mod mysql;
mod ftp;
mod smb;
mod rdp;
mod mssql;
mod redis;
mod postgresql;
mod oracle;
mod telnet;

// 导出所有模块
pub use ssh::*;
pub use mysql::*;
pub use ftp::*;
pub use smb::*;
pub use rdp::*;
pub use mssql::*;
pub use redis::*;
pub use postgresql::*;
pub use oracle::*;
pub use telnet::*;
