use std::sync::Arc;

use chrono::Utc;
use log::error;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query, query_as, query_scalar};


use super::asm_task::INNERASK_MODULE;

#[derive(Debug, Serialize, Deserialize,FromRow)]
pub struct Domain {
    pub id: Option<i32>,
    pub task_id: i32,
    pub domain: String,
    pub ufrom: Option<String>,
    pub aaa: Option<String>,
    pub cname: Option<String>,
    pub ns: Option<String>,
    pub mx: Option<String>,
    pub create_at: i32,
    pub update_at: i32,
}


#[tauri::command]
pub async fn get_domains(
    page: i32,
    pagesize: i32,
    dtype: String,
    filter: String,
    query: String,
) -> Result<serde_json::Value, String> {
    if page <= 0 || pagesize <= 0 {
        return Err("Invalid page or pagesize".into());
    }
    let allowed_filters = ["task_id", "domain", "A", "CNAME", "NS", "MX"]; // 允许的列名
    if !allowed_filters.contains(&filter.as_str()) {
        return Err("Invalid filter".into());
    }


    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => {
            error!("Global variable not initialized");
            return Err("Global variable not initialized".into());
        }
    };
    let tauri_conn = Arc::clone(&task_module.tauri_conn);

    let base_sql = match dtype.as_str() {
        "AAAA" => format!("SELECT * FROM domain WHERE aaa IS NOT NULL AND {} LIKE ? ORDER BY create_at DESC",filter),
        "CNAME" => format!("SELECT * FROM domain WHERE cname IS NOT NULL AND {} LIKE ? ORDER BY create_at DESC",filter),
        "NS" => format!("SELECT * FROM domain WHERE ns IS NOT NULL AND {} LIKE ? ORDER BY create_at DESC",filter),
        "MX" => format!("SELECT * FROM domain WHERE mx IS NOT NULL AND {} LIKE ? ORDER BY create_at DESC",filter),
        _ => format!("SELECT * FROM domain WHERE {} LIKE ? ORDER BY create_at DESC",filter),
    };


    let sql = format!("{} LIMIT ? OFFSET ?", base_sql);


    let mut domain_pattern = format!("%{}%", query);
    let offset = (page - 1) * pagesize;
    let domains = query_as::<_, Domain>(&sql)
        .bind(domain_pattern)
        .bind(pagesize as i32)
        .bind(offset as i32)
        .fetch_all(&*tauri_conn)
        .await
        .unwrap();

    // 获取总记录数
    let count_sql = match dtype.as_str() {
        "AAAA" => format!("SELECT count(*) FROM domain WHERE aaa IS NOT NULL AND {} LIKE ? ",filter),
        "CNAME" => format!("SELECT count(*) FROM domain WHERE cname IS NOT NULL AND {} LIKE ? ",filter),
        "NS" => format!("SELECT count(*) FROM domain WHERE ns IS NOT NULL AND {} LIKE ? ",filter),
        "MS" => format!("SELECT count(*) FROM domain WHERE mx IS NOT NULL AND {} LIKE ? ",filter),
        _ => format!("SELECT count(*) FROM domain WHERE {} LIKE ? ",filter),
    };

    domain_pattern = format!("%{}", query);
    let total_count: i64 = query_scalar(&count_sql)
        .bind(domain_pattern)
        .fetch_one(&*tauri_conn)
        .await
        .unwrap();

    Ok(serde_json::json!({
        "list": domains,
        "total": total_count
    }))

    // Ok((domain_list,total_count)) // 返回企业列表
}





#[tauri::command(rename_all = "snake_case")]
pub async fn add_domain(task_id: i32, domains: Vec<String>) -> Result<String, String> {


     println!("{:}  {:?}", task_id, domains);
    let now = Utc::now();
    let timestamp = now.timestamp();

    let task_module = INNERASK_MODULE
        .get()
        .expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    for domain in domains {
        let task_name: String = query_scalar("SELECT name FROM scan_task where id = ? ")
            .bind(task_id)
            .fetch_one(&*pool_clone)
            .await
            .unwrap();

        match query("INSERT INTO domain (domain,task_id,create_at,update_at) VALUES (?1,?2,?3,?4) ON CONFLICT DO NOTHING")
        .bind(domain)
        .bind(task_id)
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


#[tauri::command]
pub async fn delete_domain_by_id(id: i32) -> Result<String, String> {
    let task_module = INNERASK_MODULE
        .get()
        .expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    match query("DELETE FROM domain WHERE id = ?")
        .bind(id)
        .execute(&*pool_clone)
        .await {
            Ok(_) => Ok("success".to_string()),
            Err(e) => {
                error!("Error deleting domain: {}", e);
                Err(e.to_string())
            }
        }
}

