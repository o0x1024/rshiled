use std::sync::Arc;

use super::scan_task::ScanTask;
use crate::asm::asm_task::INNERASK_MODULE;
use chrono::prelude::*;
use log::error;
use sqlx::{query, query_as, query_scalar};

#[derive(Debug, serde::Serialize, sqlx::FromRow, Clone)]
pub struct RootDomain {
    pub id: i32,
    pub domain: String,
    pub task_name: String,
    pub task_id: i32,
    #[sqlx(skip)]
    pub count: i32,
    pub create_at: i64,
    pub update_at: i64,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct EntInfo {
    pub task_id: i32,
    pub task_name: String,
    pub count: i32,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn add_root_domain(task_id: i32, root_domain: Vec<String>) -> Result<String, String> {
    println!("{:}  {:?}", task_id, root_domain);
    let now = Utc::now();
    let timestamp = now.timestamp();

    let task_module = INNERASK_MODULE
        .get()
        .expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    for domain in root_domain {
        let task_name: String = query_scalar("SELECT name FROM scan_task where id = ? ")
            .bind(task_id)
            .fetch_one(&*pool_clone)
            .await
            .unwrap();

        match query("INSERT INTO rootdomain (domain,task_id,task_name,create_at,update_at) VALUES (?1,?2,?3,?4,?5) ON CONFLICT DO NOTHING")
        .bind(domain)
        .bind(task_id)
        .bind(task_name)
        .bind(timestamp)
        .bind(timestamp)
        .execute(&*pool_clone)
        .await{
            Ok(_) => (),
            Err(e) => error!("Error inserting root domain: {}", e),
        }
    }

    Ok("sucess".into())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_root_domains(
    page: usize,
    pagesize: usize,
    search_key: String,
    filter: String,
) -> Result<serde_json::Value, String> {
    let task_module = INNERASK_MODULE
        .get()
        .expect("Global variable not initialized");
    let tauri_conn = Arc::clone(&task_module.tauri_conn);

    let allowed_filters = ["task_id", "task_name"]; // 允许的列名
    if !allowed_filters.contains(&filter.as_str()) {
        return Err("Invalid filter".into());
    }

    let sql: String;
    match filter.as_str() {
        "task_id" => {
            sql = format!("SELECT * FROM rootdomain WHERE task_id LIKE ? ORDER BY create_at DESC limit ? OFFSET ? ",)
        }
        "task_name" => {
            sql = format!("SELECT * FROM rootdomain WHERE task_name LIKE ? ORDER BY create_at DESC limit ? OFFSET ? ",)
        }
        _ => {
            return Err("Invalid filter".into());
        }
    }
    let offset: usize = (page - 1) * pagesize;
    let mut rootdomains = sqlx::query_as::<_, RootDomain>(&sql);
    let domain_pattern = format!("%{}", search_key);
    rootdomains = rootdomains
        .bind(domain_pattern)
        .bind(pagesize as i32)
        .bind(offset as i32);

    let mut rootdomains = rootdomains
        .fetch_all(&*tauri_conn)
        .await
        .map_err(|e| format!("Failed to fetch root domains: {}", e))?;

    for rt in &mut rootdomains {
        let pattern = format!("%{}", rt.domain);

        let count: i32 = query_scalar("SELECT count(*) FROM domain WHERE domain LIKE ?")
            .bind(&pattern)
            .fetch_one(&*tauri_conn)
            .await
            .map_err(|e| format!("Failed to fetch domain count: {}", e))?;

        rt.count = count;
    }

    let total_count: i64 = if search_key.is_empty() {
        sqlx::query_scalar("SELECT COUNT(*) FROM rootdomain")
            .fetch_one(&*tauri_conn)
            .await
            .map_err(|e| format!("Failed to fetch total count: {}", e))?
    } else {
        sqlx::query_scalar("SELECT COUNT(*) FROM rootdomain WHERE task_name = ?")
            .bind(&search_key)
            .fetch_one(&*tauri_conn)
            .await
            .map_err(|e| format!("Failed to fetch total count: {}", e))?
    };

    Ok(serde_json::json!({
        "list": rootdomains,
        "total": total_count
    }))

    // 返回企业列表
}

#[tauri::command]
pub async fn get_ent_domain(page: usize, pagesize: usize) -> Result<serde_json::Value, String> {
    let mut icp_list = vec![];
    let task_module = INNERASK_MODULE
        .get()
        .expect("Global variable not initialized");
    let tauri_conn = Arc::clone(&task_module.tauri_conn);

    let offset: usize = (page - 1) * pagesize;

    let tasks = query_as::<_, ScanTask>("SELECT * FROM scan_task limit ? OFFSET ?")
        .bind(pagesize as i32)
        .bind(offset as i32)
        .fetch_all(&*tauri_conn)
        .await
        .unwrap();

    for task in tasks.iter() {
        let mut eif = EntInfo {
            task_id: task.id,
            task_name: task.name.clone(),
            count: 0,
        };
        let count = query_scalar("SELECT COUNT(*) FROM domain WHERE task_id = ?")
            .bind(task.id)
            .fetch_one(&*tauri_conn)
            .await
            .unwrap();
        eif.count = count;
        icp_list.push(eif);
    }

    let total_count: i64 = query_scalar("SELECT count(*) FROM scan_task")
        .fetch_one(&*tauri_conn)
        .await
        .unwrap();

    Ok(serde_json::json!({
        "list": tasks,
        "total": total_count
    }))
}

#[tauri::command]
pub async fn del_rootdomain_by_id(did: usize) {
    let task_module = INNERASK_MODULE
        .get()
        .expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    query("DELETE FROM rootdomain WHERE id=?;")
        .bind(did as i32)
        .execute(&*pool_clone)
        .await
        .unwrap();
}
