use crate::utils;
use tokio::join;
use rusqlite::Connection;

use super::{asm_task::TASK_MODULE, task::{self, Task}};

#[derive(Debug, serde::Serialize)]
pub struct Enterprise {
    pub id: isize,
    pub name: String,
    pub monitor_status: u8,
    pub running_status: String,
    pub next_run_time: isize,
    pub last_run_time: isize,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_enterprise_list(
    page: isize,
    pagesize: isize,
) -> Result<serde_json::Value, String> {
    let mut enterprise_list = vec![];
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();
    let mut stmt = conn.prepare("SELECT id,name,monitor_status,running_status,next_run_time,last_run_time  FROM enterprise limit ?,?").unwrap();
    let enterprise_iter = stmt
        .query_map(
            [(page - 1) * pagesize, pagesize],
            |row: &rusqlite::Row<'_>| {
                Ok(Enterprise {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    monitor_status: row.get(2)?,
                    running_status: row.get(3)?,
                    next_run_time: row.get(4)?,
                    last_run_time: row.get(5)?,
                })
            },
        )
        .unwrap();

    for enterprise in enterprise_iter {
        //使用task_module,获取所有任务状态添加到enterprise中
        match enterprise {
            Ok(ent) => {
                // let status = TASK_MODULE.query_task_status(ent.id);
                // ent.running_status = status;
                enterprise_list.push(ent);
            }
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        }
    }
    let count_sql = "SELECT count(*) FROM enterprise";
    let mut count_stmt = conn.prepare(count_sql).unwrap();
    let total_count: isize = count_stmt.query_row([], |row| row.get(0)).unwrap(); // 获取总记录数

    Ok(serde_json::json!({
        "list": enterprise_list,
        "total": total_count
    }))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn add_enterprise(enterprise_name: String) -> Result<String, String> {
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();

    match conn.execute(
        "INSERT INTO enterprise (name,monitor_status) VALUES (?1,?2)",
        (enterprise_name, 1),
    ) {
        Ok(_) => {
            TASK_MODULE
                .clone()
                .add_task(Task {
                    id: conn.last_insert_rowid() as isize,
                    task_status: 1,
                    running_status: "wait".to_string(),
                    name: "1".to_string(),
                })
                .await;
            Ok("success".to_string())
        }
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn switch_task_status(eid: isize, status: isize) -> Result<String, String> {
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();

    match conn.execute(
        "UPDATE enterprise SET monitor_status = ?1  WHERE id = ?2",
        (status, eid),
    ) {
        Ok(_) => Ok("success".to_string()),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn del_enterprise_by_id(eid: isize) {

    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();
    conn.execute("DELETE FROM enterprise WHERE id = ?1", (eid,)).unwrap();
    conn.execute("DELETE FROM domain WHERE enterprise_id = ?1", (eid,)).unwrap();
    conn.execute("DELETE FROM rootdomain WHERE enterprise_id = ?1", (eid,)).unwrap();
    conn.execute("DELETE FROM website WHERE enterprise_id = ?1", (eid,)).unwrap();
    conn.execute("DELETE FROM IPs WHERE enterprise_id = ?1", (eid,)).unwrap();
    conn.execute("DELETE FROM port WHERE enterprise_id = ?1", (eid,)).unwrap();


}

#[tauri::command(rename_all = "snake_case")]
pub async fn run_scan(eid: isize) {


    let handler = tokio::spawn(async move{
        TASK_MODULE.run_task(eid).await
    });

    match handler.await {
        Ok(result) => result, 
        Err(_) => ()
    };
}

