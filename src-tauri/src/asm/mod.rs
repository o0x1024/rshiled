pub mod domain;
pub mod enterprise;
pub mod rootdomain;
pub mod task;
pub mod ips;
pub mod internal;
pub mod website;

use lazy_static::lazy_static;

pub mod asm_task {
    use super::*;
    use std::{collections::HashMap, sync::Arc};

    use super::task::TaskModule;
    use crate::utils;
    use rusqlite::Connection;
    use std::sync::Mutex;
    use tokio::sync::RwLock;

    lazy_static! {
        pub static ref TASK_MODULE: Arc<TaskModule> = Arc::new(TaskModule {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            conn: Arc::new(Mutex::new(
                Connection::open(utils::file::get_db_path()).unwrap()
            )),
        });
    }

    pub async fn asm_init() {
        // let db_path = utils::file::get_db_path();

        // let task_module = Arc::new(TaskModule {
        //     tasks: RwLock::new(HashMap::new()),
        //     conn: Arc::new(Mutex::new(Connection::open(db_path).unwrap())),
        // });

        TASK_MODULE.start().await;
        // TASK_MODULE.query_task_status(1);

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
}
