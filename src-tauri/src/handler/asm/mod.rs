use sqlx::sqlite::SqlitePool;
use tokio::sync::OnceCell;
use std::sync::Arc;

pub mod api;
pub mod domain;
pub mod innertask;
pub mod internal;
pub mod ips;
pub mod port;
pub mod risk;
pub mod rootdomain;
pub mod scan_task;
pub mod web_comp;
pub mod website;
pub mod plugin;
pub mod plugin_commands;
pub mod visualization;
pub mod command;


pub use api::*;
pub use domain::*;
pub use scan_task::*;
pub use internal::*;
pub use ips::*;
pub use port::*;
pub use risk::*;
pub use rootdomain::*;
pub use web_comp::*;
pub use website::*;
pub use plugin::*;
pub use plugin_commands::*;
pub use visualization::*;

// 重新导出以便Tauri命令注册
pub use port::port_scan_by_nmap;

// 添加资产统计API
#[tauri::command]
pub async fn get_asset_statistics(task_id: i32) -> Result<serde_json::Value, String> {
    let task_module = match asm_task::INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => {
            return Err("Global variable not initialized".into());
        }
    };
    let pool_clone = Arc::clone(&task_module.read_conn);

    // 合并统计查询
    let stats: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT 
            (SELECT COUNT(*) FROM domain) as domains,
            (SELECT COUNT(*) FROM ips) as ips,
            (SELECT COUNT(*) FROM port) as ports,
            (SELECT COUNT(*) FROM website) as websites"
    )
    .fetch_one(&*pool_clone)
    .await
    .unwrap_or((0, 0, 0, 0));

    // 获取漏洞总数
    let vuln_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM risk ")
        .bind(task_id)
        .fetch_one(&*pool_clone)
        .await
        .unwrap_or(0);

    // 获取漏洞风险等级分布
    let critical_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM risk WHERE  level = 'critical'")
        .bind(task_id)
        .fetch_one(&*pool_clone)
        .await
        .unwrap_or(0);

    let high_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM risk WHERE level = 'high'")
        .bind(task_id)
        .fetch_one(&*pool_clone)
        .await
        .unwrap_or(0);

    let medium_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM risk WHERE  level = 'medium'")
        .bind(task_id)
        .fetch_one(&*pool_clone)
        .await
        .unwrap_or(0);

    let low_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM risk WHERE  level = 'low'")
        .bind(task_id)
        .fetch_one(&*pool_clone)
        .await
        .unwrap_or(0);

    let info_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM risk WHERE level = 'info'")
        .bind(task_id)
        .fetch_one(&*pool_clone)
        .await
        .unwrap_or(0);

    Ok(serde_json::json!({
        "total_domains": stats.0,
        "total_ips": stats.1,
        "total_ports": stats.2,
        "total_websites": stats.3,
        "total_vulnerabilities": vuln_count,
        "risk_distribution": {
            "critical": critical_count,
            "high": high_count,
            "medium": medium_count,
            "low": low_count,
            "info": info_count
        }
    }))
}

pub mod asm_task {
    use super::*;
    use std::str::FromStr;
    use std::{collections::HashMap, sync::Arc};

    use super::innertask::InnerTaskModule;
    use crate::internal;
    use sqlx::sqlite::SqliteConnectOptions;

    use tokio::sync::RwLock;

    // lazy_static! {
    //     pub static ref INNERASK_MODULE: Arc<TaskModule> = Arc::new(TaskModule {
    //         tasks: Arc::new(RwLock::new(HashMap::new())),
    //         conn: Arc::new(Mutex::new(get_sqlite_pool().await.unwrap())),
    //     });
    // }

    pub static INNERASK_MODULE: OnceCell<InnerTaskModule> = OnceCell::const_new();

    async fn init_task_module() -> InnerTaskModule {
        let db_path = format!("sqlite:{}", internal::file::get_db_path().to_str().unwrap());
        let options = SqliteConnectOptions::from_str(db_path.as_str()).unwrap()
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal) // 开启 WAL 模式
        .create_if_missing(true).pragma("synchronous", "NORMAL"); 

        InnerTaskModule {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            write_conn: Arc::new(SqlitePool::connect_with(options.clone()).await.unwrap()),
            read_conn: Arc::new(SqlitePool::connect_with(options.clone()).await.unwrap()),
            tauri_conn: Arc::new(SqlitePool::connect_with(options).await.unwrap()),
        }
    }

    // async fn initialize_conn() -> Result<(), Box<dyn Error>> {
    //     let mut conn = INNERASK_MODULE.conn.lock().await;
    //     if conn.is_none() {
    //         let pool = get_sqlite_pool().await?;
    //         *conn = Some(pool);
    //     }
    //     Ok(())
    // }

    pub async fn asm_init() {

        let task_module = INNERASK_MODULE.get_or_init(init_task_module).await;

        // initialize_conn().await.unwrap();
        task_module.start().await;
        // INNERASK_MODULE.start().await;
        // INNERASK_MODULE.query_task_status(1);

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
