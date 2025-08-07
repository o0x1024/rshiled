use crate::core::config::AppConfig;
use crate::handler::scan::engine::result::ScanResult;
use crate::handler::scan::proxy::{HttpRequest, HttpResponse};
use crate::handler::scan::scanners::{
    PluginManager, Scanner, ScannerType, ScannerTypeEnum
};
use log::{error, info};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Semaphore};
use std::collections::VecDeque;
use chrono::Utc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// 扫描任务
#[derive(Clone)]
struct ScanTask {
    id: String,
    request: HttpRequest,
    response: HttpResponse,
    _timestamp: i64,
}

/// 扫描状态
#[derive(Clone)]
pub struct ScanStatus {
    pub is_running: bool,
    pub task_count: usize,
    pub completed_count: usize,
    pub error_count: usize,
    pub last_scan_time: Option<i64>,
}

/// 扫描管理器
pub struct ScanManager {
    /// 配置
    config: Arc<AppConfig>,
    /// 扫描器列表
    scanners: Vec<ScannerType>,
    /// 插件管理器
    plugin_manager: Arc<Mutex<PluginManager>>,
    /// 任务队列
    task_queue: Arc<Mutex<VecDeque<ScanTask>>>,
    /// 并发控制
    concurrency_limiter: Arc<Semaphore>,
    /// 扫描状态
    status: Arc<Mutex<ScanStatus>>,
    /// 结果发送通道
    result_tx: mpsc::Sender<ScanResult>,
    /// 停止标志
    stop_flag: Arc<AtomicBool>,
}

impl ScanManager {
    /// 创建新的扫描管理器
    pub async fn new(config: Arc<AppConfig>, result_tx: mpsc::Sender<ScanResult>) -> Self {
        let mut scanners = Vec::new();
        
        // 初始化内置扫描器
        if config.rules.vulnerabilities.xss.enabled {
            scanners.push(ScannerType { scanner_type: ScannerTypeEnum::Xss });
        }
        
        if config.rules.vulnerabilities.sql_injection.enabled {
            scanners.push(ScannerType { scanner_type: ScannerTypeEnum::SqlInjection });
        }
        
        if config.rules.vulnerabilities.rce.enabled {
            scanners.push(ScannerType { scanner_type: ScannerTypeEnum::Rce });
        }
        
        // 初始化插件管理器
        let plugin_manager = PluginManager::new();
        // 从数据库加载插件，而不是文件系统
        if let Err(e) = plugin_manager.load_plugins_from_db().await {
            error!("Failed to load plugins from database: {}", e);
        }
        
        // 初始化扫描状态
        let status = ScanStatus {
            is_running: false,
            task_count: 0,
            completed_count: 0,
            error_count: 0,
            last_scan_time: None,
        };
        
        Self {
            config: config.clone(),
            scanners,
            plugin_manager: Arc::new(Mutex::new(plugin_manager)),
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            concurrency_limiter: Arc::new(Semaphore::new(config.scanner.concurrency)),
            status: Arc::new(Mutex::new(status)),
            result_tx,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }
    
    /// 添加扫描任务
    pub async fn add_task(&self, request: HttpRequest, response: HttpResponse) {
        let task = ScanTask {
            id: uuid::Uuid::new_v4().to_string(),
            request,
            response,
            _timestamp: Utc::now().timestamp(),
        };
        
        let mut queue = self.task_queue.lock().await;
        queue.push_back(task);
        
        let mut status = self.status.lock().await;
        status.task_count += 1;
    }
    
    /// 获取扫描状态
    pub async fn get_status(&self) -> ScanStatus {
        self.status.lock().await.clone()
    }
    
    /// 启动扫描管理器
    pub async fn start(self, mut rx: mpsc::Receiver<(HttpRequest, HttpResponse)>) {
        info!("扫描管理器已启动");
        
        // 重置停止标志
        self.stop_flag.store(false, Ordering::SeqCst);
        
        // 更新运行状态
        {
            let mut status = self.status.lock().await;
            status.is_running = true;
        }
        
        // 启动任务处理循环
        let _task_processor = {
            let manager = self.clone();
            tokio::spawn(async move {
                while let Some((request, response)) = rx.recv().await {
                    if manager.stop_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    manager.add_task(request, response).await;
                }
            })
        };
        
        // 启动扫描工作器
        let mut workers = Vec::new();
        for _ in 0..self.config.scanner.concurrency {
            let worker = self.spawn_worker();
            workers.push(worker);
        }
        
        // 等待所有工作器完成或收到停止信号
        loop {
            if self.stop_flag.load(Ordering::SeqCst) {
                break;
            }
            
            let mut all_done = true;
            for worker in &workers {
                if !worker.is_finished() {
                    all_done = false;
                    break;
                }
            }
            
            if all_done {
                break;
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        // 更新运行状态
        {
            let mut status = self.status.lock().await;
            status.is_running = false;
            status.last_scan_time = Some(Utc::now().timestamp());
        }
        
        info!("扫描管理器已停止");
    }
    
    /// 停止扫描管理器
    pub async fn stop(&self) {
        info!("正在停止扫描管理器...");
        self.stop_flag.store(true, Ordering::SeqCst);
        
        let mut status = self.status.lock().await;
        status.is_running = false;
    }
    
    /// 清空任务队列
    pub async fn clear_queue(&self) {
        let mut queue = self.task_queue.lock().await;
        queue.clear();
        
        let mut status = self.status.lock().await;
        status.task_count = 0;
        status.completed_count = 0;
        status.error_count = 0;
    }
    
    /// 重新加载插件
    pub async fn reload_plugins(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("正在重新加载插件...");
        let plugin_manager = self.plugin_manager.lock().await;
        plugin_manager.load_plugins_from_db().await?;
        info!("插件重新加载完成");
        Ok(())
    }
    
    // 创建扫描工作器
    fn spawn_worker(&self) -> tokio::task::JoinHandle<()> {
        let manager = self.clone();
        
        tokio::spawn(async move {
            loop {
                // 获取信号量许可
                let _permit = manager.concurrency_limiter.acquire().await.unwrap();
                
                // 获取任务
                let task = {
                    let mut queue = manager.task_queue.lock().await;
                    queue.pop_front()
                };
                
                match task {
                    Some(task) => {
                        // 处理任务
                        if let Err(e) = manager.process_task(task).await {
                            error!("Task processing error: {}", e);
                            let mut status = manager.status.lock().await;
                            status.error_count += 1;
                        }
                        
                        // 更新完成计数
                        let mut status = manager.status.lock().await;
                        status.completed_count += 1;
                    }
                    None => {
                        // 队列为空，检查是否应该继续运行
                        let status = manager.status.lock().await;
                        if !status.is_running {
                            break;
                        }
                        // 等待一段时间后继续
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
            }
        })
    }
    
    /// 处理单个扫描任务
    async fn process_task(&self, task: ScanTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // info!("处理任务 {}: {} {}", task.id, task.request.method, task.request.url);
        
        // 运行所有扫描器
        for scanner in &self.scanners {
            let results = scanner.scan(&task.request, &task.response).await;
            for result in results {
                if let Err(e) = self.result_tx.send(result).await {
                    error!("发送扫描结果失败: {}", e);
                }
            }
        }
        
        // 运行插件扫描器
        let plugin_manager = self.plugin_manager.lock().await;
        let plugin_results = plugin_manager.scan(&task.request, &task.response).await;
        
        // 发送扫描结果
        for scan_result in plugin_results {
            if let Err(e) = self.result_tx.send(scan_result).await {
                error!("发送扫描结果失败: {}", e);
            }
        }
        
        Ok(())
    }
}

impl Clone for ScanManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            scanners: self.scanners.clone(),
            plugin_manager: self.plugin_manager.clone(),
            task_queue: self.task_queue.clone(),
            concurrency_limiter: self.concurrency_limiter.clone(),
            status: self.status.clone(),
            result_tx: self.result_tx.clone(),
            stop_flag: self.stop_flag.clone(),
        }
    }
}
