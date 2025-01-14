use log::*;
use rand::Rng;
use rusqlite::{params, Connection};
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

use super::internal::{fetch_website, resolver_dns, resolver_ip};

// 任务结构体
#[derive(Clone)]
pub struct Task {
    pub id: isize,
    pub name: String,
    pub task_status: u8, //任务状态
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
        let mut stmt = conn.prepare("SELECT * FROM enterprise").unwrap();
        let enterprise_iter = stmt
            .query_map([], |row| {
                Ok(Enterprise {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    monitor_status: row.get(2)?,
                    running_status: row.get(3)?,
                    next_run_time: row.get(4)?,
                    last_run_time: row.get(5)?,
                })
            })
            .unwrap();

        for enterprise in enterprise_iter {
            match enterprise {
                Ok(ent) => {
                    xtasks.push(Task {
                        id: ent.id as isize,
                        name: ent.name,
                        task_status: ent.monitor_status,
                        running_status: "wait".to_string(),
                    });
                }
                Err(_) => (),
            }
        }

        let mut tks = HashMap::<isize, Task>::new();
        // 将任务数据存储到任务模块中
        {
            let mut tasks_map = self.tasks.write().await;
            for task in xtasks {
                tasks_map.insert(task.id.clone(), task.clone());
                tks.insert(task.id, task);
            }
        }

        // 启动任务执行线程
        for (task_id, _) in tks.iter() {
            let task_id_clone = *task_id;
            let self_clone = Arc::new(self.clone());

            task::spawn(async move {
                // 生成一个 24 小时内的时间间隔 N
                // let n = rand::thread_rng().gen_range(0..24 * 3600);
                let n = rand::thread_rng().gen_range(10..20);

                //更新下次运行的时间
                let sleep_time = Duration::from_secs(n);

                loop {
                    // 等待时间间隔 N
                    tokio::time::sleep(sleep_time).await;
                    // 执行任务
                    self_clone.run_task(task_id_clone).await;
                }
            });
        }
    }

    // 执行任务
    pub async fn run_task(&self, task_id: isize,) {
        let db_path = utils::file::get_db_path();
        let conn = Connection::open(db_path).unwrap();
        let now: i64 = chrono::Local::now().timestamp();
        let monitor_status = match conn.query_row(
            "SELECT monitor_status FROM enterprise WHERE id=?1",
            params![task_id],
            |row| row.get(0),
        ) {
            Ok(status) => status,
            Err(_) => 0,
        };

        if monitor_status == 1 {
            let root_domains = {
                let mut stmt = conn
                    .prepare("SELECT * FROM rootdomain where enterprise_id = ?")
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

            //1.接口获取域名信息
            let mut result_domain = Vec::new();
            for domain in root_domains {
                println!("[*] domain collection:{}", domain);
                match dns_collection_by_api(domain.as_str()).await {
                    Ok(domains) => {
                        result_domain.extend(domains);
                    }
                    Err(e) => {
                        error!("Error collecting DNS for domain {}: {:?}", domain, e);
                    }
                }
            }
            //暴力破解
            let config = AppConfig::global();
            if config.dns_collection_brute_status {}

            //插件获取
            if config.dns_collection_plugin_status {}

            //定义一个IP数组来保存域名解析的IP地址
            //获取域名对象的DNS记录
            update_task_status(&task_id, "collection domain",None);
            resolver_dns(&task_id, &result_domain).await;

            //获取IP地址///////??//??//??//??//??//??//??//??

            update_task_status(&task_id, "collection ip",None);
            resolver_ip(&task_id, &result_domain).await;

            //扫描端口并且识别服务,返回http和https的URL和端口
            let http_server = Vec::new();

            //扫描网站信息
            update_task_status(&task_id, "collection website",None);
            fetch_website(&task_id, &result_domain, &http_server).await;

            //扫描安全风险信息
            update_task_status(&task_id, "scan risk",None);
            // scan_risk(&task_id, &result_domain).await;

            // 任务完成，更新任务状态
            update_task_status(&task_id, "wait", Some(now));
        }
    }

    // 查询任务状态
    pub fn query_task_status(&self, task_id: isize) -> String {
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

fn update_task_status(task_id: &isize, status: &str, last_run_time: Option<i64>) {
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();

    match last_run_time {
        Some(time) => {
            let sql = "UPDATE enterprise SET running_status = ?1,last_run_time=?2  WHERE id = ?3";
            conn.execute(sql, (status, time, task_id)).unwrap();
        }
        None => {
            let sql = "UPDATE enterprise SET running_status = ?1 WHERE id = ?2";
            conn.execute(sql, (status, task_id)).unwrap();
        }
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
