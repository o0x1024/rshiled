use log::*;
use rand::Rng;
use sqlx::{query, query_as, query_scalar, Pool, Sqlite};
use std::collections::HashMap;
use std::sync::Arc;

use crate::asm::rootdomain::RootDomain;
use crate::asm::scan_task::ScanTask;
use crate::global::config::CoreConfig;
use crate::internal::dns_collect::dns_collection_by_api;
use crate::internal::rsubdomain::rsubdomain::domain_brute_by_rsubdomain;
use std::thread;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task;

use super::asm_task::INNERASK_MODULE;
use super::domain;
use super::internal::{fetch_domain_by_plugin, fetch_website, resolver_dns, resolver_ip};
use super::port::{self, scan_ports_by_plugin};
use super::risk::{self, scan_risk_by_plugin};
use super::web_comp;
use super::{api, fetch_finger, port_scan_by_nmap};

// 任务结构体
#[derive(Clone)]
pub struct InnerTask {
    pub id: i32,
    pub name: String,
    pub task_status: u8, //任务状态
    pub running_status: String,
}

// 任务模块
#[derive(Clone)]
pub struct InnerTaskModule {
    pub tasks: Arc<RwLock<HashMap<i32, InnerTask>>>,
    pub write_conn: Arc<Pool<Sqlite>>,
    pub read_conn: Arc<Pool<Sqlite>>,
    pub tauri_conn: Arc<Pool<Sqlite>>,
}

impl InnerTaskModule {
    // 从数据库中初始化任务
    async fn init(&self) {
        //     let pool: sqlx::Pool<sqlx::Sqlite> =
        // INNERASK_MODULE.conn.lock().unwrap().as_ref().unwrap().clone();
        // let pool =INNERASK_MODULE.conn.lock().await.unwrap();

        let task_module = INNERASK_MODULE
            .get()
            .expect("Global variable not initialized");
        let mut xtasks = Vec::new();

        let tasks = {
            let task_module = INNERASK_MODULE
                .get()
                .expect("Global variable not initialized");
            let pool_clone = Arc::clone(&task_module.write_conn);
            query_as::<_, ScanTask>("SELECT * FROM scan_task WHERE monitor_status=?")
                .bind(1)
                .fetch_all(&*pool_clone)
                .await
                .unwrap()
        };

        for task in tasks {
            xtasks.push(InnerTask {
                id: task.id as i32,
                name: task.name,
                task_status: task.monitor_status,
                running_status: "wait".to_string(),
            });
        }

        let mut tks = HashMap::<i32, InnerTask>::new();
        // 将任务数据存储到任务模块中
        {
            let mut tasks_map = self.tasks.write().await;
            for task in xtasks {
                tasks_map.insert(task.id.clone(), task.clone());
                tks.insert(task.id, task);
            }
        }

        // 启动任务执行线程
        for (task_id, _) in tks.iter().clone() {
            let task_id_clone = *task_id;
            let self_clone = Arc::new(self.clone());
            let pool_clone = Arc::clone(&task_module.write_conn);
            task::spawn(async move {
                let (sleep_time, monitor_status) = {
                    let pool_clone = Arc::clone(&self_clone.read_conn);
                    let sleep_time = query_scalar("SELECT next_run_time FROM scan_task WHERE id=?")
                        .bind(task_id_clone)
                        .fetch_one(&*pool_clone)
                        .await
                        .unwrap_or(600);

                    let monitor_status =
                        query_scalar("SELECT monitor_status FROM scan_task WHERE id=?")
                            .bind(task_id_clone)
                            .fetch_one(&*pool_clone)
                            .await
                            .unwrap();
                    (sleep_time, monitor_status)
                };

                loop {
                    // println!("{:?}",sleep_time);
                    // 等待时间间隔 N
                    tokio::time::sleep(Duration::from_secs(sleep_time)).await;
                    // 执行任务
                    self_clone.run_task(task_id_clone, monitor_status).await;
                }
            });
        }
    }

    // 执行任务
    pub async fn run_task(&self, task_id: i32, monitor_status: i64) {
        let now: i64 = chrono::Local::now().timestamp();
        if monitor_status == 1 {
            let root_domains = {
                let task_module = INNERASK_MODULE
                    .get()
                    .expect("Global variable not initialized");
                let pool_clone = Arc::clone(&task_module.read_conn);

                query_as::<_, RootDomain>("SELECT * FROM rootdomain where task_id = ?")
                    .bind(task_id)
                    .fetch_all(&*pool_clone)
                    .await
                    .unwrap()
            };

            //获取域名信息 一个是内置的接口，二是暴破，三是通过插件模块进行获取
            update_task_status(&task_id, "collection domain", None).await;

            //1.接口获取域名信息
            let task_module = INNERASK_MODULE.get().ok_or("全局变量未初始化").unwrap();

            // 从数据库读取所有域名 - 在await前完成所有读取操作
            let domains: Vec<String> = {
                let read_conn = Arc::clone(&task_module.read_conn);
                query_scalar::<_, String>("SELECT domain FROM domain")
                    .fetch_all(&*read_conn)
                    .await
                    .unwrap()
            };


            //暴力破解
            let config = CoreConfig::global().unwrap();
            if config.dns_collection_brute_status {
                let rst_result_list = domain_brute_by_rsubdomain(
                    &root_domains
                        .iter()
                        .map(|x| x.domain.clone())
                        .collect::<Vec<String>>(),
                    &domains,
                    config.subdomain_level.unwrap_or(3) as usize,
                    config.is_buildin,
                    config
                        .subdomain_dict
                        .unwrap_or("assets/subnames.txt".to_string()),
                )
                .await;

                // 处理结果
                if let Ok(dns_records) = rst_result_list {
                    let task_module = INNERASK_MODULE
                        .get()
                        .expect("Global variable not initialized");
                    let pool_clone = Arc::clone(&task_module.write_conn);

                    let mut tx = pool_clone.begin().await.unwrap();
                    let now: i64 = chrono::Local::now().timestamp();

                    for domain in dns_records.iter() {
                        let mut aaa_json: Option<String> = None;
                        let mut cname_json: Option<String> = None;
                        let mut mx_json: Option<String> = None;
                        let mut ns_json: Option<String> = None;
                        let mut txt_json: Option<String> = None;

                        match domain.record_type.as_str() {
                            "A" => {
                                aaa_json = Some(domain.record_value.clone());
                            }
                            "CNAME" => {
                                cname_json = Some(domain.record_value.clone());
                            }
                            "MX" => {
                                mx_json = Some(domain.record_value.clone());
                            }
                            "NS" => {
                                ns_json = Some(domain.record_value.clone());
                            }
                            "TXT" => {
                                txt_json = Some(domain.record_value.clone());
                            }
                            _ => {}
                        }
                        if let Err(e) = query(
                            "INSERT INTO domain (task_id,domain,aaa,cname,mx,ns,create_at,update_at,ufrom) 
                            VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)"
                        )
                        .bind(&task_id)
                        .bind(&domain.domain)
                        .bind(&aaa_json)
                        .bind(&cname_json)
                        .bind(&mx_json)
                        .bind(&ns_json)
                        .bind(&now)
                        .bind(&now)
                        .bind("rsubdomain")
                        .execute(&mut *tx)
                        .await {
                            // warn!("Failed to insert domain {}: {}", domain.domain, e);
                        }
                    }

                    tx.commit().await.unwrap();
                }
            }
            //插件获取
            if config.dns_collection_plugin_status {
                fetch_domain_by_plugin(
                    &task_id,
                    root_domains
                        .iter()
                        .map(|x| x.domain.clone())
                        .collect::<Vec<String>>(),
                )
                .await;
            }

            //获取当前任务所有域名信息
            let result_domain: Vec<String> = {
                let read_conn = Arc::clone(&task_module.read_conn);
                query_scalar::<_, String>("SELECT domain FROM domain where task_id=?")
                    .bind(&task_id)
                    .fetch_all(&*read_conn)
                    .await
                    .unwrap()
            };

            //获取域名对象的DNS记录
            let _ = resolver_dns(&task_id, &result_domain).await.map_err(|e| {
                error!("Error collecting DNS for task {}: {:?}", task_id, e);
            });
            // //获取IP地址
            update_task_status(&task_id, "collection ip", None).await;
            let _ = resolver_ip(&task_id).await.map_err(|e| {
                error!("Error collecting IP for task {}: {:?}", task_id, e);
            });

            //扫描端口并且识别服务,返回http和https的URL和端口
            update_task_status(&task_id, "collection port", None).await;
            // 内置端口扫描
            let _ = port_scan_by_nmap(&task_id).await;
            // 插件端口扫描
            if config.port_scan_plugin_status {
                scan_ports_by_plugin(&task_id).await;
            }

            //扫描网站信息
            update_task_status(&task_id, "collection website", None).await;
            let http_server = Vec::new();
            let website = fetch_website(
                &task_id,
                &http_server,
                &root_domains
                    .iter()
                    .map(|x| x.domain.clone())
                    .collect::<Vec<String>>(),
            )
            .await
            .unwrap();

            // //获取指纹信息
            update_task_status(&task_id, "collection finger", None).await;
            fetch_finger(&task_id, &result_domain).await;
            // 插件获取指纹信息
            if config.fingerprint_plugin_status && !website.is_empty() {
                web_comp::scan_fingerprint_by_plugin(&task_id, &website).await;
            }

            //扫描安全风险信息
            update_task_status(&task_id, "scan risk", None).await;
            // 原始风险扫描
            let _ = risk::risk_scan(&task_id).await;
            //插件风险扫描
            if config.risk_scan_plugin_status {
                scan_risk_by_plugin(&task_id, &website).await;
            }
            // 扫描API信息
            update_task_status(&task_id, "scan api", None).await;
            // 扫描API信息
            if let Err(e) = api::scan_api(&task_id).await {
                error!("Failed to scan api: {}", e);
            }
            // 任务完成，更新任务状态
            update_task_status(&task_id, "wait", Some(now)).await;
        }
    }

    // 执行特定类型的扫描任务
    pub async fn run_task_by_type(&self, task_id: i32, monitor_status: i64, scan_type: String) {
        if monitor_status != 1 {
            return;
        }

        let now: i64 = chrono::Local::now().timestamp();

        let root_domains = {
            let task_module = INNERASK_MODULE
                .get()
                .expect("Global variable not initialized");
            let pool_clone = Arc::clone(&task_module.read_conn);

            query_as::<_, RootDomain>("SELECT * FROM rootdomain where task_id = ?")
                .bind(task_id)
                .fetch_all(&*pool_clone)
                .await
                .unwrap()
        };

        let config = CoreConfig::global().unwrap();

        match scan_type.as_str() {
            "domain" => {
                // 域名扫描
                update_task_status(&task_id, "collection domain", None).await;

                if config.dns_collection_brute_status {
                    let task_module = INNERASK_MODULE.get().ok_or("全局变量未初始化").unwrap();

                    // 从数据库读取所有域名 - 在await前完成所有读取操作
                    let domains: Vec<String> = {
                        let read_conn = Arc::clone(&task_module.read_conn);
                        query_scalar::<_, String>("SELECT domain FROM domain")
                            .fetch_all(&*read_conn)
                            .await
                            .unwrap()
                    };

                    let rst_result_list = domain_brute_by_rsubdomain(
                        &root_domains
                            .iter()
                            .map(|x| x.domain.clone())
                            .collect::<Vec<String>>(),
                        &domains,
                        config.subdomain_level.unwrap_or(3) as usize,
                        config.is_buildin,
                        config
                            .subdomain_dict
                            .unwrap_or("assets/subnames.txt".to_string()),
                    )
                    .await
                    .unwrap();

                    println!("rst_result_list length: {:?}", rst_result_list.len());
                    let task_module = INNERASK_MODULE
                        .get()
                        .expect("Global variable not initialized");
                    let pool_clone = Arc::clone(&task_module.write_conn);
                    let mut tx = pool_clone.begin().await.unwrap();
                    let now: i64 = chrono::Local::now().timestamp();

                    for domain in rst_result_list.iter() {
                        // 存储域名信息的代码，与run_task中相同
                        let mut aaa_json: Option<String> = None;
                        let mut cname_json: Option<String> = None;
                        let mut mx_json: Option<String> = None;
                        let mut ns_json: Option<String> = None;
                        let mut txt_json: Option<String> = None;

                        match domain.record_type.as_str() {
                            "A" => {
                                aaa_json = Some(domain.record_value.clone());
                            }
                            "CNAME" => {
                                cname_json = Some(domain.record_value.clone());
                            }
                            "MX" => {
                                mx_json = Some(domain.record_value.clone());
                            }
                            "NS" => {
                                ns_json = Some(domain.record_value.clone());
                            }
                            "TXT" => {
                                txt_json = Some(domain.record_value.clone());
                            }
                            _ => {}
                        }
                        if let Err(e) = query(
                                "INSERT INTO domain (task_id,domain,aaa,cname,mx,ns,create_at,update_at,ufrom) 
                                VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)"
                            )
                            .bind(&task_id)
                            .bind(&domain.domain)
                            .bind(&aaa_json)
                            .bind(&cname_json)
                            .bind(&mx_json)
                            .bind(&ns_json)
                            .bind(&now)
                            .bind(&now)
                            .bind("rsubdomain")
                            .execute(&mut *tx)
                            .await {
                                // warn!("Failed to insert domain {}: {}", domain.domain, e);
                            }
                    }
                    tx.commit().await.unwrap();
                }

                if config.dns_collection_plugin_status {
                    fetch_domain_by_plugin(
                        &task_id,
                        root_domains
                            .iter()
                            .map(|x| x.domain.clone())
                            .collect::<Vec<String>>(),
                    )
                    .await;
                }

                let task_module = INNERASK_MODULE.get().ok_or("全局变量未初始化").unwrap();

                let task_all_domains: Vec<String> = {
                    let read_conn = Arc::clone(&task_module.read_conn);
                    query_scalar::<_, String>("SELECT domain FROM domain where task_id = ?")
                        .bind(&task_id)
                        .fetch_all(&*read_conn)
                        .await
                        .unwrap()
                };

                let _ = resolver_dns(&task_id, &task_all_domains)
                    .await
                    .map_err(|e| {
                        error!("Error collecting DNS for task {}: {:?}", task_id, e);
                    });
            }
            "ip" => {
                // IP地址扫描
                update_task_status(&task_id, "collection ip", None).await;
                let _ = resolver_ip(&task_id).await.map_err(|e| {
                    error!("Error collecting IP for task {}: {:?}", task_id, e);
                });
            }
            "port" => {
                // 端口扫描
                update_task_status(&task_id, "collection port", None).await;

               if let Err(e)= port_scan_by_nmap(&task_id).await{
                    error!("Error collecting port for task {}: {:?}", task_id, e);
                }
                // 通过插件扫描端口
                if config.port_scan_plugin_status {
                    scan_ports_by_plugin(&task_id).await;
                }
            }
            "website" => {
                // 网站信息扫描
                update_task_status(&task_id, "collection website", None).await;
                let http_server = Vec::new();
                let _ = fetch_website(
                    &task_id,
                    &http_server,
                    &root_domains
                        .iter()
                        .map(|x| x.domain.clone())
                        .collect::<Vec<String>>(),
                )
                .await;
            }
            "api" => {
                // API扫描
                update_task_status(&task_id, "scan api", None).await;
                let _ = api::scan_api(&task_id).await;
            }
            "webcomp" => {
                // Web组件扫描
                update_task_status(&task_id, "collection finger", None).await;

                // 获取网站列表
                let http_server = Vec::new();
                let website = fetch_website(
                    &task_id,
                    &http_server,
                    &root_domains
                        .iter()
                        .map(|x| x.domain.clone())
                        .collect::<Vec<String>>(),
                )
                .await
                .unwrap();

                // 插件获取指纹信息
                if config.fingerprint_plugin_status && !website.is_empty() {
                    web_comp::scan_fingerprint_by_plugin(&task_id, &website).await;
                }
            }
            "risk" => {
                // 安全风险扫描
                update_task_status(&task_id, "scan risk", None).await;
                // 原始风险扫描
                let _ =risk::risk_scan(&task_id).await;

                // 插件风险扫描
                // if config.risk_scan_plugin_status {
                //     risk::scan_risk_by_plugin(&task_id, &website).await;
                // }
            }
            "all" | _ => {
                // 全部扫描，直接调用完整的run_task方法
                self.run_task(task_id, monitor_status).await;
                return;
            }
        }

        // 任务完成，更新任务状态
        update_task_status(&task_id, "wait", Some(now)).await;
    }

    // 查询任务状态
    pub fn query_task_status(&self, task_id: i32) -> String {
        match self.tasks.try_read() {
            Ok(tasks) => {
                if let Some(task) = tasks.get(&task_id) {
                    return task.running_status.clone();
                } else {
                    "Task not found".to_string()
                }
            }
            Err(_) => "".to_string(),
        }
    }

    // 动态添加任务
    pub async fn add_task(&self, task: InnerTask) {
        let task_clone = task.clone();
        let mut tasks_map = self.tasks.write().await;
        tasks_map.insert(task.id, task);
        // 启动任务执行线程
        let task_id = task_clone.id;

        task::spawn(async move {
            // 生成一个 24 小时内的时间间隔 N
            // let n = rand::thread_rng().gen_range(20..40);
            // let sleep_time = Duration::from_secs(n);
            let self_task = Arc::new(
                INNERASK_MODULE
                    .get()
                    .expect("Global variable not initialized"),
            );
            let (sleep_time, monitor_status) = {
                let pool_clone = Arc::clone(&self_task.read_conn);
                let sleep_time = query_scalar("SELECT next_run_time FROM scan_task WHERE id=?")
                    .bind(task_id)
                    .fetch_one(&*pool_clone)
                    .await
                    .unwrap();

                let monitor_status =
                    query_scalar("SELECT monitor_status FROM scan_task WHERE id=?")
                        .bind(task_id)
                        .fetch_one(&*pool_clone)
                        .await
                        .unwrap();
                (sleep_time, monitor_status)
            };

            loop {
                // 等待时间间隔 Nw
                thread::sleep(Duration::from_secs(sleep_time));

                // 执行任务
                self_task.run_task(task_id, monitor_status).await;
            }
        });
    }

    pub async fn del_task(self: Arc<Self>, task_id: i32) {
        let mut tasks_map = self.tasks.write().await;
        tasks_map.remove(&task_id);
    }

    // 启动任务模块
    pub async fn start(&self) {
        self.init().await;
    }
}

async fn update_task_status(task_id: &i32, status: &str, last_run_time: Option<i64>) {
    let task_module = INNERASK_MODULE
        .get()
        .expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.write_conn);

    match last_run_time {
        Some(time) => {
            query("UPDATE scan_task SET running_status = ?1,last_run_time=?2  WHERE id = ?3")
                .persistent(true)
                .bind(status)
                .bind(time)
                .bind(task_id)
                .execute(&*pool_clone)
                .await
                .unwrap();
            // conn.execute(sql, (status, time, task_id)).unwrap();
        }
        None => {
            query("UPDATE scan_task SET running_status = ?1 WHERE id = ?2")
                .persistent(true)
                .bind(status)
                .bind(task_id)
                .execute(&*pool_clone)
                .await
                .unwrap();
        }
    }
}
