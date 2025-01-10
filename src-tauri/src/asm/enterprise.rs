use crate::utils;
use deno_core::futures::executor::Enter;
use rusqlite::Connection;

use super::{asm_task::TASK_MODULE, task::{Task, TaskModule}};

#[derive(Debug, serde::Serialize)]
pub struct Enterprise {
    pub id: isize,
    pub name: String,
    pub monitor_status: bool,
    pub next_runtime: isize,
    pub running_status: String,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_enterprise_list(    
    page: isize,
    pagesize: isize
) -> Result<serde_json::Value, String> {
    let mut enterprise_list = vec![];
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();
    let mut stmt = conn.prepare("SELECT id,name,monitor_status,next_runtime,running_status FROM Enterprise limit ?,?").unwrap();
    let enterprise_iter = stmt
        .query_map([ (page - 1) * pagesize, pagesize], |row: &rusqlite::Row<'_>| {
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
        //使用task_module,获取所有任务状态添加到enterprise中
        match enterprise {
            Ok(mut ent) => {
                let status = TASK_MODULE.query_task_status(ent.id);
                ent.running_status = status;
                enterprise_list.push(ent);
            }
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        }
    }
    let count_sql = "SELECT count(*) FROM Enterprise";
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
			"INSERT INTO Enterprise (name,monitor_status) VALUES (?1,?2)",
			(enterprise_name,1),
		){
            Ok(_) => {
                TASK_MODULE.clone().add_task(Task{
                    id: conn.last_insert_rowid() as isize,
                    task_status: true,
                    running_status:"wait".to_string(),
                    name:"1".to_string()
                }).await;
                Ok("success".to_string())
            },
            Err(err) =>Err(err.to_string())
        }
}