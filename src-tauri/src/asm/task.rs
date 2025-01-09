use rand::Rng;
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::asm::enterprise::Enterprise;
use crate::asm::rootdomain::RootDomain;
use crate::config::config::AppConfig;
use crate::dns_collect::dns_collection_by_api;
use crate::utils;
use std::thread;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task;

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
                    icp_no: row.get(2)?,
                    monitor_status: row.get(3)?,
                    next_runtime: row.get(4)?,
                    running_status: row.get(5)?,
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
                    let n = rand::thread_rng().gen_range(0..24 * 3600);
                    // let n = rand::thread_rng().gen_range(10..20);

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
            println!("[*] domain collection:{}",domain);
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
        if config.dns_collection_brute_status{

        }

        //插件获取
        if config.dns_collection_plugin_status{
        }


        //获取域名对象的DNS记录
        // 把拿到的域名写到数据库里
        let conn = self.conn.lock().unwrap();
        for dm in result_domain {
            let now = chrono::Local::now().timestamp();
            match conn.execute("INSERT INTO Domain (enterprise_id,domain,create_at, update_at) VALUES (?1, ?2, ?3, ?4)",(
                &task_id,
                &dm,
                &now,    // create_at
                &now,    // update_at
            )){
                Ok(_) => (),
                Err(_) => ()
            }
        }

        //获取IP地址

        //扫描端口并且识别服务

        //扫描网站信息

        //扫描安全风险信息
    }

    // 查询任务状态
    pub async fn query_task_status(&self, task_id: isize) -> String {
        let tasks = self.tasks.read().await;
        let task = tasks.get(&task_id).unwrap().clone();
        task.running_status
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
            let n = rand::thread_rng().gen_range(0..24 * 3600);
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

pub async fn asm_init() {
    let db_path = utils::file::get_db_path();

    let task_module = Arc::new(TaskModule {
        tasks: Arc::new(RwLock::new(HashMap::new())),
        conn: Arc::new(Mutex::new(Connection::open(db_path).unwrap())),
    });

    task_module.start().await;

    // 查询任务状态
    // task_module.query_task_status(1).await;

    // 动态添加任务
    // let new_task = Task {
    //     id: 3,
    //     name: "任务3".to_string(),
    //     interval: 10800, // 3小时
    //     last_run: Instant::now(),
    // };
    // task_module.add_task(new_task).await;
}
