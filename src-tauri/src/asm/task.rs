use tokio::task::JoinHandle;
use rand::Rng;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::proto::rr::RecordType;
use trust_dns_resolver::TokioAsyncResolver;

use crate::asm::domain::Domain;
use crate::asm::enterprise::Enterprise;
use crate::asm::rootdomain::RootDomain;
use crate::config::config::AppConfig;
use crate::dns_collect::dns_collection_by_api;
use crate::utils;
use std::thread;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task;

use super::ips::IPs;

// 任务结构体
#[derive(Clone)]
pub struct Task {
    pub id: isize,
    pub name: String,
    pub task_status: bool, //任务状态
    pub running_status: String,
}

// 任务模块
#[derive(Clone)]
pub struct TaskModule {
    pub tasks: Arc<RwLock<HashMap<isize, Task>>>,
    pub conn: Arc<Mutex<Connection>>,
}

impl TaskModule {
    // 从数据库中初始化任务
    async fn init(&self) {
        let mut xtasks = Vec::new();
        // // 模拟从数据库中读取任务数据
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM Enterprise").unwrap();
        let enterprise_iter = stmt
            .query_map([], |row| {
                Ok(Enterprise {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    monitor_status: row.get(2)?,
                    next_runtime: row.get(3)?,
                    running_status: row.get(4)?,
                })
            })
            .unwrap();

        for enterprise in enterprise_iter {
            match enterprise {
                Ok(ent) => {
                    if ent.monitor_status {
                        xtasks.push(Task {
                            id: ent.id as isize,
                            name: ent.name,
                            task_status: ent.monitor_status,
                            running_status: "wait".to_string(),
                        });
                    }
                }
                Err(_) => (),
            }
        }

        // 将任务数据存储到任务模块中
        let mut tasks_map = self.tasks.write().await;
        for task in xtasks {
            tasks_map.insert(task.id, task);
        }

        // 启动任务执行线程
        for (task_id, task) in tasks_map.iter() {
            if task.task_status {
                let task_id_clone = *task_id;
                let self_clone = Arc::new(self.clone());

                task::spawn(async move {
                    // 生成一个 24 小时内的时间间隔 N
                    // let n = rand::thread_rng().gen_range(0..24 * 3600);
                    let n = rand::thread_rng().gen_range(20..40);

                    //更新下次运行的时间
                    let sleep_time = Duration::from_secs(n);
                    println!("run task:{} sleep time:{:?}", task_id_clone, sleep_time);

                    loop {
                        // 等待时间间隔 N
                        tokio::time::sleep(sleep_time).await;
                        // 执行任务
                        self_clone.run_task(task_id_clone).await;
                    }
                });
            }
        }
        println!("task_map done");
    }

    // 执行任务
    async fn run_task(&self, task_id: isize) {
        // Scope the mutex guard so it's dropped before any await points
        let root_domains = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT * FROM RootDomain where enterprise_id = ?")
                .unwrap();
            let root_domain_iter = stmt
                .query_map([&task_id], |row| {
                    Ok(RootDomain {
                        id: row.get(0)?,
                        domain: row.get(1)?,
                        enterprise_id: row.get(2)?,
                        enterprise_name: row.get(3)?,
                        create_at: row.get(4)?,
                        update_at: row.get(5)?,
                        count: 0,
                    })
                })
                .unwrap();

            let mut domains = Vec::new();
            for rt in root_domain_iter {
                if let Ok(rdt) = rt {
                    domains.push(rdt.domain);
                }
            }
            domains
        }; // MutexGuard is dropped here

        
        //获取域名信息 一个是内置的接口，二是暴破，三是通过插件模块进行获取
        {
            let mut tasks = self.tasks.write().await;
            let mut task = tasks.get(&task_id).unwrap().clone();
            task.running_status = "collection domain".to_string();
            tasks.insert(task_id, task);
        }

        //1.接口获取域名信息
        let mut result_domain = Vec::new();
        for domain in root_domains {
            println!("[*] domain collection:{}", domain);
            match dns_collection_by_api(domain.as_str()).await {
                Ok(domains) => {
                    result_domain.extend(domains);
                }
                Err(e) => {
                    println!("Error collecting DNS for domain {}: {:?}", domain, e);
                }
            }
        }
        // println!("{:?}",result_domain);
        //暴力破解
        let config = AppConfig::global();
        if config.dns_collection_brute_status {}

        //插件获取
        if config.dns_collection_plugin_status {}


        //定义一个IP数组来保存域名解析的IP地址
        //获取域名对象的DNS记录
        // let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default()).unwrap();
        let mut handle_list = Vec::<JoinHandle<()>>::new();
        for dm in result_domain.clone() {
            handle_list.push(tokio::spawn(async move {
                let mut all_domain = Vec::<Domain>::new();
                let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default()).unwrap();

                let mut td = Domain {
                    id: None,
                    enterprise_id: task_id,
                    domain: dm.clone().to_string(),
                    aaa: None,
                    cname: None,
                    mx: None,
                    ns: None,
                    create_at: 0,   
                    update_at: 0,
                };
                // 查询 A 记录（IPv4 地址）
                let mut record = Vec::new();
                let a_records = resolver.lookup_ip(dm.clone()).await;
                if let Ok(records) = a_records {
                    for ip in records {
                        record.push(ip.to_string());
                    }
                }
                if record.len() > 0{
                    td.aaa = Some(record);
                }
    
                // 查询 CNAME 记录
                record = Vec::new();
                let cname_records = resolver.lookup(dm.clone(), RecordType::CNAME).await;
                if let Ok(records) = cname_records {
                    for ip in records {
                        record.push(ip.to_string());
                    }
                }
                if record.len() > 0{
                    td.cname = Some(record);
                }
    
                // 查询 MX 记录（邮件服务器）
                record = Vec::new();
                let mx_records = resolver.lookup(dm.clone(), RecordType::MX).await;
                if let Ok(records) = mx_records {
                    for ip in records {
                        record.push(ip.to_string());
                    }
                }
                if record.len() > 0{
                    td.mx = Some(record);
                }
               
                // 查询 TXT 记录
                record = Vec::new();
                let ns_records = resolver.lookup(dm.clone(), RecordType::NS).await;
                if let Ok(records) = ns_records {
                    for ip in records {
                        record.push(ip.to_string());
                    }
                }
                if record.len() > 0{
                    td.ns = Some(record);
                }
    
                all_domain.push(td);
                // 把拿到的域名写到数据库里
                let db_path = utils::file::get_db_path();
                let conn = Connection::open(db_path).unwrap();
                for domain in &all_domain {
                    let now: i64 = chrono::Local::now().timestamp();
    
                    let aaa_json = domain.aaa.as_ref().map(|v| serde_json::to_string(v).unwrap());
                    let cname_json = domain.cname.as_ref().map(|v| serde_json::to_string(v).unwrap());
                    let ns_json = domain.ns.as_ref().map(|v| serde_json::to_string(v).unwrap());
                    let mx_json = domain.mx.as_ref().map(|v| serde_json::to_string(v).unwrap());
                    match conn.execute("INSERT INTO Domain (enterprise_id,domain,aaa,cname,mx,ns,create_at, update_at) VALUES (?1, ?2, ?3, ?4 ,?5 ,?6 ,?7 ,?8)",params![
                        &task_id,
                        domain.domain,
                        aaa_json,    
                        cname_json,  
                        mx_json,    
                        ns_json,    
                        now,    
                        now,    
                        ]){
                    Ok(_) => (),
                    Err(_) => ()
                    }
                }
            }));
            
        }

        for handle in  handle_list {
            let _ =tokio::join!(handle);
        }

        {
            let mut tasks = self.tasks.write().await;
            let mut task = tasks.get(&task_id).unwrap().clone();
            task.running_status = "collection ip".to_string();
            tasks.insert(task_id, task);
        }


        //获取IP地址///////??//??//??//??//??//??//??//??
        handle_list = Vec::<JoinHandle<()>>::new();
        for dm in result_domain.clone() {
            handle_list.push(tokio::spawn(async move {
                let mut ip_list: Vec<IPs> = Vec::<IPs>::new();
                let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default()).unwrap();
                let a_records = resolver.lookup_ip(dm.clone()).await;
                if let Ok(records) = a_records {
                    let now: i64 = chrono::Local::now().timestamp();
                    if records.iter().count() ==  1{
                        if let Some(ip) = records.iter().next() {
                            let ip_string = ip.to_string();
                            ip_list.push(IPs { id: None, enterprise_id: task_id.clone(), ip_addr: Some(ip_string),domain:Some(dm.clone()),port_count:None, create_at: now, update_at: now });
                        } 
                    }
                }

                for ip in &ip_list{
                    let db_path = utils::file::get_db_path();
                    let conn = Connection::open(db_path).unwrap();
                    match conn.execute("INSERT INTO IPs (enterprise_id,ip_addr,domain,port_count,create_at, update_at) VALUES (?1, ?2, ?3, ?4,?5,?6)",params![
                        &task_id,
                        ip.ip_addr,
                        ip.domain,
                        0,
                        ip.create_at,
                        ip.update_at,
                        ]){
                    Ok(_) => (),
                    Err(_) => ()
                    }
                }
            }));
        }

        for handle in  handle_list {
            let _ =tokio::join!(handle);
        }



        {
            let mut tasks = self.tasks.write().await;
            let mut task = tasks.get(&task_id).unwrap().clone();
            task.running_status = "scan port".to_string();
            tasks.insert(task_id, task);
        }
        //扫描端口并且识别服务


        {
            let mut tasks = self.tasks.write().await;
            let mut task = tasks.get(&task_id).unwrap().clone();
            task.running_status = "scan website".to_string();
            tasks.insert(task_id, task);
        }
        //扫描网站信息


        {
            let mut tasks = self.tasks.write().await;
            let mut task = tasks.get(&task_id).unwrap().clone();
            task.running_status = "scan risk".to_string();
            tasks.insert(task_id, task);
        }
        //扫描安全风险信息



        {
            let mut tasks = self.tasks.write().await;
            let mut task = tasks.get(&task_id).unwrap().clone();
            task.running_status = "wait".to_string();
            tasks.insert(task_id, task);
        }
    }

    // 查询任务状态
    pub fn query_task_status(&self, task_id: isize) -> String {
        match self.tasks.try_read(){
            Ok(tasks) => {
                if let Some(task) = tasks.get(&task_id) {
                    return task.running_status.clone();
                }else{
                    "Task not found".to_string()
                }
            }
            Err(_) => {
                "".to_string()
            }
        }

    }

    // 动态添加任务
    pub async fn add_task(self: Arc<Self>, task: Task) {
        let task_clone = task.clone();
        let mut tasks_map = self.tasks.write().await;
        tasks_map.insert(task.id, task);
        // 启动任务执行线程
        let task_id = task_clone.id;
        let self_clone = Arc::clone(&self);
        // let self_clone = self.clone();

        task::spawn(async move {
            // 生成一个 24 小时内的时间间隔 N
            // let n = rand::thread_rng().gen_range(0..24 * 3600);
            let n = rand::thread_rng().gen_range(20..40);

            let sleep_time = Duration::from_secs(n);

            loop {
                // 等待时间间隔 N
                thread::sleep(sleep_time);

                // 执行任务
                self_clone.run_task(task_id).await;
            }
        });
    }

    // 启动任务模块
    pub async fn start(&self) {
        self.init().await;
    }
}

// pub async fn asm_init() {
//     let db_path = utils::file::get_db_path();

//     let task_module = Arc::new(TaskModule {
//         tasks: Arc::new(RwLock::new(HashMap::new())),
//         conn: Arc::new(Mutex::new(Connection::open(db_path).unwrap())),
//     });

//     task_module.start().await;

//     // 查询任务状态
//     // task_module.query_task_status(1).await;

//     // 动态添加任务
//     // let new_task = Task {
//     //     id: 3,
//     //     name: "任务3".to_string(),
//     //     interval: 10800, // 3小时
//     //     last_run: Instant::now(),
//     // };
//     // task_module.add_task(new_task).await;
// }
