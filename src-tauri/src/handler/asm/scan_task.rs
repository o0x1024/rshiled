use std::sync::Arc;

use super::{asm_task::INNERASK_MODULE, innertask::InnerTask};
use log::error;
use sqlx::{query, query_as, query_scalar, FromRow};
#[derive(Debug, serde::Serialize, serde::Deserialize, FromRow)]
pub struct ScanTask {
    pub id: i32,
    pub name: String,
    pub monitor_status: u8,
    pub running_status: String,
    #[sqlx(skip)]
    pub domain_count: i32,
    #[sqlx(skip)]
    pub rootdomain_count: i32,
    #[sqlx(skip)]
    pub website_count: i32,
    #[sqlx(skip)]
    pub api_count: i32,
    #[sqlx(skip)]
    pub webcomp_count: i32,
    #[sqlx(skip)]
    pub risk_count: i32,
    #[sqlx(skip)]
    pub ips_count: i32,
    #[sqlx(skip)]
    pub port_count: i32,
    pub next_run_time: i32,
    pub last_run_time: i32,
}



#[tauri::command(rename_all = "snake_case")]
pub async fn get_task_list(page: i32, pagesize: i32) -> Result<serde_json::Value, String> {
    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => {
            error!("Global variable not initialized");
            return Err("Global variable not initialized".into());
        }
    };
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    let offset = (page - 1) * pagesize;
    let mut tasks = query_as::<_, ScanTask>(
        "SELECT id, name, monitor_status, running_status, next_run_time, last_run_time 
         FROM scan_task 
         LIMIT ? OFFSET ?",
    )
    .bind(pagesize as i32)
    .bind(offset as i32)
    .fetch_all(&*pool_clone)
    .await
    .unwrap();

    let now_24 = chrono::Utc::now().timestamp() - 86400;

    for task in &mut tasks {

        //计算24小时内发现的网站数量
        let query_website = format!(
            "SELECT COUNT(*) as count FROM website WHERE task_id = ? AND create_at >= ?"
        );
        let website_count: i64 = query_scalar(&query_website)
            .bind(task.id)
            .bind(&now_24) // 24小时前的时间戳
            .fetch_one(&*pool_clone)
            .await
            .unwrap_or(0);

        //计算24小时内发现的API数量
        let query_api = format!(
            "SELECT COUNT(*) as count FROM api WHERE task_id = ? AND update_at >= ? AND http_status = 200 and handle_status = 0"
        );
        let api_count: i64 = query_scalar(&query_api)
            .bind(task.id)
            .bind(&now_24)
            .fetch_one(&*pool_clone)
            .await
            .unwrap_or(0);

        //计算24小时内发现的组件数量
        let query_webcomp = format!(
            "SELECT COUNT(*) as count FROM webcomp WHERE task_id = ? AND create_at >= ?"
        );
        let webcomp_count: i64 = query_scalar(&query_webcomp)
            .bind(task.id)
            .bind(&now_24)
            .fetch_one(&*pool_clone)
            .await
            .unwrap_or(0);

        //计算24小时内发现的风险数量
        let query_risk = format!(
            "SELECT COUNT(*) as count FROM risk WHERE task_id = ? AND update_at >= ? AND risk_status = 0"
        );
        let risk_count: i64 = query_scalar(&query_risk)
            .bind(task.id)
            .bind(&now_24)
            .fetch_one(&*pool_clone)
            .await
            .unwrap_or(0);

        //计算24小时内发现的域名数量
        let query_domain = format!(
            "SELECT COUNT(*) as count FROM domain WHERE task_id = ? AND create_at >= ?"
        );
        let domain_count: i64 = query_scalar(&query_domain)
            .bind(task.id)
            .bind(&now_24)
            .fetch_one(&*pool_clone)
            .await
            .unwrap_or(0);

        //计算24小时内发现的根域名数量
        let query_rootdomain = format!(
            "SELECT COUNT(*) as count FROM rootdomain WHERE task_id = ? AND create_at >= ?"
        );
        let rootdomain_count: i64 = query_scalar(&query_rootdomain)
            .bind(task.id)
            .bind(&now_24)
            .fetch_one(&*pool_clone)
            .await
            .unwrap_or(0);

        //计算24小时内发现的IP数量
        let query_ips = format!(
            "SELECT COUNT(*) as count FROM IPs WHERE task_id = ? AND create_at >= ?"
        );
        let ips_count: i64 = query_scalar(&query_ips)
            .bind(task.id)
            .bind(&now_24)
            .fetch_one(&*pool_clone)
            .await
            .unwrap_or(0);

        //计算24小时内发现的端口数量
        let query_port = format!(
            "SELECT COUNT(*) as count FROM port WHERE task_id = ? AND create_at >= ?"
        );
        let port_count: i64 = query_scalar(&query_port)
            .bind(task.id)
            .bind(&now_24)
            .fetch_one(&*pool_clone)
            .await
            .unwrap_or(0);

        task.website_count = website_count as i32;
        task.api_count = api_count as i32;
        task.webcomp_count = webcomp_count as i32;
        task.risk_count = risk_count as i32;
        task.domain_count = domain_count as i32;
        task.rootdomain_count = rootdomain_count as i32;
        task.ips_count = ips_count as i32;
        task.port_count = port_count as i32;
    }

    let total_count: i64 = query_scalar("SELECT count(*) FROM scan_task")
        .fetch_one(&*pool_clone)
        .await
        .unwrap_or(0);

    Ok(serde_json::json!({
        "list": tasks,
        "total": total_count
    }))
}

#[tauri::command]
pub async fn get_asm_task_list() -> Result<Vec<ScanTask>, String> {
    // 调用get_task_list函数获取第一页数据
    let result = get_task_list(1, 100).await?;
    
    // 从JSON结果中提取任务列表
    if let Some(list) = result.get("list") {
        if let Some(task_list) = list.as_array() {
            let tasks: Vec<ScanTask> = task_list
                .iter()
                .filter_map(|task| serde_json::from_value(task.clone()).ok())
                .collect();
            return Ok(tasks);
        }
    }
    
    // 如果没有找到任务列表，返回空数组
    Ok(Vec::new())
}


#[tauri::command(rename_all = "snake_case")]
pub async fn save_next_run_time(id: i32, next_run_time: i32) -> Result<String, String> {
    let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    match query("UPDATE scan_task SET next_run_time = ? WHERE id = ?")
        .bind(next_run_time)
        .bind(id)
        .execute(&*pool_clone)
        .await
    {
        Ok(_) => Ok("success".to_string()),
        Err(err) => Err(err.to_string()),
    }
}




#[tauri::command(rename_all = "snake_case")]
pub async fn add_task(task_name: String) -> Result<String, String> {
    let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    let task_module_clone = Arc::new(task_module);
    match query(r#"INSERT INTO scan_task (name,monitor_status) VALUES (?,?) ON CONFLICT DO NOTHING"#)
        .bind(task_name)
        .bind(1)
        .execute(&*pool_clone)
        .await
    {
        Ok(row) => {
            let lid: i64 = row.last_insert_rowid();
            task_module_clone
                .add_task(InnerTask {
                    id: lid as i32,
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
pub async fn switch_task_status(eid: i32, status: i32) -> Result<String, String> {
    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => {
            error!("Global variable not initialized");
            return Err("Global variable not initialized".into());
        }
    };
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    match query("UPDATE scan_task SET monitor_status = ?  WHERE id = ?")
        .bind(status)
        .bind(eid)
        .execute(&*pool_clone)
        .await
    {
        Ok(row) => {
            if status == 1 {
                let task_module =
                    Arc::new(INNERASK_MODULE.get().expect("Global variable not initialized"));
                task_module
                    .add_task(InnerTask {
                        id: eid as i32,
                        task_status: 1,
                        running_status: "wait".to_string(),
                        name: row.last_insert_rowid().to_string(),
                    })
                    .await;
            }
            Ok("success".to_string())
        }
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn del_task_by_id(eid: i32) -> Result<String, String> {
    let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    query("DELETE FROM scan_task WHERE id = ?")
        .bind(eid)
        .execute(&*pool_clone)
        .await
        .unwrap();
    query("DELETE FROM domain WHERE task_id = ?")
        .bind(eid)
        .execute(&*pool_clone)
        .await
        .unwrap();
    query("DELETE FROM rootdomain WHERE task_id = ?")
        .bind(eid)
        .execute(&*pool_clone)
        .await
        .unwrap();
    query("DELETE FROM website WHERE task_id = ?")
        .bind(eid)
        .execute(&*pool_clone)
        .await
        .unwrap();
    query("DELETE FROM ips WHERE task_id = ?")
        .bind(eid)
        .execute(&*pool_clone)
        .await
        .unwrap();
    query("DELETE FROM port WHERE task_id = ?")
        .bind(eid)
        .execute(&*pool_clone)
        .await
        .unwrap();
    query("DELETE FROM api WHERE task_id = ?")
        .bind(eid)
        .execute(&*pool_clone)
        .await
        .unwrap();
    query("DELETE FROM webcomp WHERE task_id = ?")
        .bind(eid)
        .execute(&*pool_clone)
        .await
        .unwrap();

    let task_module = Arc::new(
        INNERASK_MODULE
            .get()
            .expect("Global variable not initialized")
            .clone(),
    );
    task_module.del_task(eid as i32).await;
    Ok("success".to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn run_scan(eid: i32) {
    let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");

    
    let handler = tokio::spawn(async move { task_module.run_task(eid,1).await });

    match handler.await {
        Ok(result) => result,
        Err(_) => (),
    };
}

#[tauri::command(rename_all = "snake_case")]
pub async fn run_scan_by_type(eid: i32, scan_type: String) {
    let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");
    
    let handler = tokio::spawn(async move { 
        task_module.run_task_by_type(eid, 1, scan_type).await 
    });

    match handler.await {
        Ok(result) => result,
        Err(_) => (),
    };
}
