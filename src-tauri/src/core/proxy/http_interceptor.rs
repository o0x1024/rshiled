use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;
use tauri::Emitter;
use uuid::Uuid;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{Duration, timeout};
use serde::{Deserialize, Serialize};
use crate::core::proxy::store::{RequestStore, InterceptedRequest};

// 请求控制命令类型
enum RequestControlCommand {
    Forward {
        req_id: String,
        method: Option<String>,
        url: Option<String>,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
        response_tx: oneshot::Sender<Result<(), String>>,
    },
    Drop {
        req_id: String,
        response_tx: oneshot::Sender<Result<(), String>>,
    },
}

// 响应控制命令类型
enum ResponseControlCommand {
    Forward {
        req_id: String,
        status: Option<u16>,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
        response_tx: oneshot::Sender<Result<(), String>>,
    },
    Drop {
        req_id: String,
        response_tx: oneshot::Sender<Result<(), String>>,
    },
}

// 公开的请求控制接口
pub enum RequestInterceptControl {
    Forward {
        req_id: String,
        method: Option<String>,
        url: Option<String>,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
        response_tx: oneshot::Sender<Result<(), String>>,
    },
    Drop {
        req_id: String,
        response_tx: oneshot::Sender<Result<(), String>>,
    },
}

// 公开的响应控制接口
pub enum ResponseInterceptControl {
    Forward {
        req_id: String,
        status: Option<u16>,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
        response_tx: oneshot::Sender<Result<(), String>>,
    },
    Drop {
        req_id: String,
        response_tx: oneshot::Sender<Result<(), String>>,
    },
}

// 增加响应拦截请求结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterceptedResponse {
    pub id: String,
    pub request_id: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

// 请求控制结果类型
type RequestControlResult = Result<(Option<String>, Option<String>, Option<HashMap<String, String>>, Option<String>), String>;

// 响应控制结果类型
type ResponseControlResult = Result<(Option<u16>, Option<HashMap<String, String>>, Option<String>), String>;

// HTTP拦截器
pub struct HttpInterceptor {
    app: AppHandle,
    store: Arc<RequestStore>,
    // 请求通道
    request_tx: mpsc::Sender<RequestControlCommand>,
    // 响应通道
    response_tx: mpsc::Sender<ResponseControlCommand>,
    // 活跃请求和响应映射 - 使用 Arc 共享
    active_requests: Arc<Mutex<HashMap<String, oneshot::Sender<RequestControlResult>>>>,
    active_responses: Arc<Mutex<HashMap<String, oneshot::Sender<ResponseControlResult>>>>,
}

impl Clone for HttpInterceptor {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            store: self.store.clone(),
            request_tx: self.request_tx.clone(),
            response_tx: self.response_tx.clone(),
            // 克隆 Arc 以共享底层的 Mutex 和 HashMap
            active_requests: Arc::clone(&self.active_requests),
            active_responses: Arc::clone(&self.active_responses),
        }
    }
}

impl HttpInterceptor {
    pub fn new(app: AppHandle, store: Arc<RequestStore>) -> Self {
        let (request_tx, mut request_rx) = mpsc::channel::<RequestControlCommand>(100);
        let (response_tx, mut response_rx) = mpsc::channel::<ResponseControlCommand>(100);
        
        // 初始化共享的活跃请求/响应映射
        let active_requests = Arc::new(Mutex::new(HashMap::new()));
        let active_responses = Arc::new(Mutex::new(HashMap::new()));
        
        // 创建拦截器实例
        let interceptor = Self {
            app,
            store,
            request_tx,
            response_tx,
            // 移动 Arc 的所有权
            active_requests: Arc::clone(&active_requests),
            active_responses: Arc::clone(&active_responses),
        };

        // 启动请求处理循环
        let req_active_requests = Arc::clone(&active_requests); // 克隆 Arc 用于后台任务
        let req_store = Arc::clone(&interceptor.store); // 克隆 Arc 用于后台任务
        tokio::spawn(async move {
            while let Some(cmd) = request_rx.recv().await {
                match cmd {
                    RequestControlCommand::Forward { req_id, method, url, headers, body, response_tx } => {
                        // 查找等待处理的请求
                        let request_complete = {
                            let mut active_requests = req_active_requests.lock().await; // 使用克隆的 Arc
                            if let Some(tx) = active_requests.remove(&req_id) {
                                // 发送转发决定
                                println!("处理请求转发: ID={}", req_id);
                                let _ = tx.send(Ok((method, url, headers, body)));
                                // 通知发送方操作成功
                                let _ = response_tx.send(Ok(()));
                                true
                            } else {
                                println!("找不到待处理的请求: ID={}", req_id);
                                // 通知发送方操作失败
                                let _ = response_tx.send(Err(format!("请求ID不存在: {}", req_id)));
                                false
                            }
                        };
                        
                        if request_complete {
                            // 从拦截列表中删除请求
                            req_store.remove_intercepted(&req_id).await; // 使用克隆的 Arc
                        }
                    },
                    RequestControlCommand::Drop { req_id, response_tx } => {
                        // 查找等待处理的请求
                        let request_complete = {
                            let mut active_requests = req_active_requests.lock().await; // 使用克隆的 Arc
                            if let Some(tx) = active_requests.remove(&req_id) {
                                // 发送丢弃决定
                                println!("处理请求丢弃: ID={}", req_id);
                                let _ = tx.send(Err("Request dropped by user".to_string()));
                                // 通知发送方操作成功
                                let _ = response_tx.send(Ok(()));
                                true
                            } else {
                                println!("找不到待处理的请求: ID={}", req_id);
                                // 通知发送方操作失败
                                let _ = response_tx.send(Err(format!("请求ID不存在: {}", req_id)));
                                false
                            }
                        };
                        
                        if request_complete {
                            // 从拦截列表中删除请求
                            req_store.remove_intercepted(&req_id).await; // 使用克隆的 Arc
                        }
                    }
                }
            }
        });
        
        // 启动响应处理循环
        let resp_active_responses = Arc::clone(&active_responses); // 克隆 Arc 用于后台任务
        tokio::spawn(async move {
            while let Some(cmd) = response_rx.recv().await {
                match cmd {
                    ResponseControlCommand::Forward { req_id, status, headers, body, response_tx } => {
                        // 查找等待处理的响应
                        let response_complete = {
                            let mut active_responses = resp_active_responses.lock().await; // 使用克隆的 Arc
                            if let Some(tx) = active_responses.remove(&req_id) {
                                // 发送转发决定
                                println!("处理响应转发: ID={}", req_id);
                                let _ = tx.send(Ok((status, headers, body)));
                                // 通知发送方操作成功
                                let _ = response_tx.send(Ok(()));
                                true
                            } else {
                                println!("找不到待处理的响应: ID={}", req_id);
                                // 通知发送方操作失败
                                let _ = response_tx.send(Err(format!("响应ID不存在: {}", req_id)));
                                false
                            }
                        };
                    },
                    ResponseControlCommand::Drop { req_id, response_tx } => {
                        // 查找等待处理的响应
                        let response_complete = {
                            let mut active_responses = resp_active_responses.lock().await; // 使用克隆的 Arc
                            if let Some(tx) = active_responses.remove(&req_id) {
                                // 发送丢弃决定
                                println!("处理响应丢弃: ID={}", req_id);
                                let _ = tx.send(Err("Response dropped by user".to_string()));
                                // 通知发送方操作成功
                                let _ = response_tx.send(Ok(()));
                                true
                            } else {
                                println!("找不到待处理的响应: ID={}", req_id);
                                // 通知发送方操作失败
                                let _ = response_tx.send(Err(format!("响应ID不存在: {}", req_id)));
                                false
                            }
                        };
                    }
                }
            }
        });
        
        interceptor
    }
    
    // 获取请求控制发送器
    pub fn get_request_sender(&self) -> mpsc::Sender<RequestInterceptControl> {
        let request_tx = self.request_tx.clone();
        let (tx, mut rx) = mpsc::channel(100);
        
        // 创建一个转换器，将公开的控制命令转换为内部命令
        let converter = async move {
            while let Some(control) = rx.recv().await {
                match control {
                    RequestInterceptControl::Forward { req_id, method, url, headers, body, response_tx } => {
                        let _ = request_tx.send(RequestControlCommand::Forward { 
                            req_id, method, url, headers, body, response_tx 
                        }).await;
                    },
                    RequestInterceptControl::Drop { req_id, response_tx } => {
                        let _ = request_tx.send(RequestControlCommand::Drop { 
                            req_id, response_tx 
                        }).await;
                    }
                }
            }
        };
        
        // 启动转换器
        tokio::spawn(converter);
        
        tx
    }
    
    // 获取响应控制发送器
    pub fn get_response_sender(&self) -> mpsc::Sender<ResponseInterceptControl> {
        let internal_response_tx = self.response_tx.clone(); // 克隆内部命令通道发送器
        let (tx, mut rx) = mpsc::channel(100); // 公开接口通道
        
        // 创建一个转换器，将公开的控制命令转换为内部命令
        let converter = async move {
            while let Some(control) = rx.recv().await {
                match control {
                    ResponseInterceptControl::Forward { req_id, status, headers, body, response_tx } => {
                        // 发送内部命令到正确的通道
                        if let Err(e) = internal_response_tx.send(ResponseControlCommand::Forward { 
                            req_id, status, headers, body, response_tx 
                        }).await {
                            eprintln!("发送内部响应控制命令失败: {}", e);
                        }
                    },
                    ResponseInterceptControl::Drop { req_id, response_tx } => {
                        // 发送内部命令到正确的通道
                        if let Err(e) = internal_response_tx.send(ResponseControlCommand::Drop { 
                            req_id, response_tx 
                        }).await {
                            eprintln!("发送内部响应丢弃命令失败: {}", e);
                        }
                    }
                }
            }
        };
        
        // 启动转换器
        tokio::spawn(converter);
        
        tx // 返回公开接口通道发送器
    }
    
    // 获取所有活跃的拦截请求ID
    pub async fn get_active_request_ids(&self) -> Vec<String> {
        let active_requests = self.active_requests.lock().await;
        active_requests.keys().cloned().collect()
    }
    
    // 获取所有活跃的拦截响应ID
    pub async fn get_active_response_ids(&self) -> Vec<String> {
        let active_responses = self.active_responses.lock().await;
        active_responses.keys().cloned().collect()
    }
    
    // 处理拦截的请求
    pub async fn intercept_request(
        &self,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: String,
    ) -> Result<(Option<String>, Option<String>, Option<HashMap<String, String>>, Option<String>), String> {
        // 创建拦截请求ID
        let req_id = Uuid::new_v4().to_string();
        
        // 创建拦截请求对象并克隆用于存储
        let intercepted_req = InterceptedRequest {
            id: req_id.clone(),
            method: method.to_string(),
            url: url.to_string(),
            headers: headers.clone(),
            body: body.clone(),
        };
        
        println!("拦截请求: {} {} (ID: {})", method, url, req_id);
        
        // 创建用于等待控制结果的通道
        let (result_tx, result_rx) = oneshot::channel();
        
        // 记录活跃拦截请求
        {
            let mut active_requests = self.active_requests.lock().await;
            active_requests.insert(req_id.clone(), result_tx);
        }
        
        // 存储拦截的请求
        self.store.add_intercepted(intercepted_req.clone()).await;
        
        // 发送拦截事件到前端
        self.app.emit("proxy-request-intercepted", intercepted_req)
            .map_err(|e| format!("Failed to emit event: {}", e))?;
        
        // 等待控制结果
        let result = match result_rx.await {
            Ok(r) => r,
            Err(e) => {
                println!("等待控制结果失败: {}", e);
                // 此处可以返回默认行为（如自动转发）
                Ok((None, None, None, None))
            }
        };
        
        result
    }

    // 拦截响应
    pub async fn intercept_response(
        &self,
        req_id: &str,
        status: u16,
        headers: HashMap<String, String>,
        body: String,
    ) -> Result<(Option<u16>, Option<HashMap<String, String>>, Option<String>), String> {
        // 创建拦截响应对象，使用响应自己的ID，不再依赖请求ID
        // 但保留关联请求ID以便在历史记录中关联
        let response_id = Uuid::new_v4().to_string();
        
        let intercepted_res = InterceptedResponse {
            id: response_id.clone(),  // 使用新生成的响应ID
            request_id: req_id.to_string(), // 保存关联的请求ID
            status,
            headers: headers.clone(),
            body: body.clone(),
        };
        
        println!("拦截响应: 状态码={}, 响应ID={}, 关联请求ID={}", status, response_id, req_id);
        
        // 创建用于等待控制结果的通道
        let (result_tx, result_rx) = oneshot::channel();
        
        // 记录活跃拦截响应，使用响应自己的ID
        {
            let mut active_responses = self.active_responses.lock().await;
            active_responses.insert(response_id.clone(), result_tx);
        }
        
        // 发送拦截事件到前端
        self.app.emit("proxy-response-intercepted", intercepted_res)
            .map_err(|e| format!("Failed to emit response intercept event: {}", e))?;
        
        // 等待控制结果
        let result = match result_rx.await {
            Ok(r) => r,
            Err(e) => {
                println!("等待响应控制结果失败: {}", e);
                // 此处可以返回默认行为（如自动转发）
                Ok((None, None, None))
            }
        };
        
        result
    }
} 