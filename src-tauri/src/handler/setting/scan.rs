use std::sync::Arc;

use chrono::Utc;
use sqlx::{query, query_as, query_scalar};

use crate::asm::asm_task::INNERASK_MODULE;
#[derive(Debug, serde::Serialize,serde::Deserialize, sqlx::FromRow)]
pub struct CRegex {
    pub id: i32,
    pub name: String,    //正则名称
    pub regex: String, //正则内容
    pub rtype: String,   //正则类型
    pub status: u8,    //正则状态
    pub create_at: i32,  //创建时间
    pub update_at: i32,  //更新时间ß
}

#[tauri::command]
pub async fn get_regexs(page: i32, pagesize: i32) -> Result<serde_json::Value, String> {
    let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.read_conn);

    let offset = (page - 1) * pagesize;
    let regexs = query_as::<_, CRegex>(
        "SELECT * 
         FROM cregex 
         LIMIT ? OFFSET ?",
    )
    .bind(pagesize as i32)
    .bind(offset as i32)
    .fetch_all(&*pool_clone)
    .await
    .unwrap();

    let total_count: i64 = query_scalar("SELECT count(*) FROM cregex")
        .fetch_one(&*pool_clone)
        .await
        .unwrap_or(0);

    Ok(serde_json::json!({
        "list": regexs,
        "total": total_count
    }))
}

#[tauri::command]
pub async fn add_regex(name: String,regex:String,rtype:String) -> Result<String, String> {
    let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.tauri_conn);
    let now = Utc::now();
    let now = now.timestamp();

    match query(r#"INSERT INTO cregex (name,regex,rtype,create_at,update_at) VALUES (?,?,?,?,?) ON CONFLICT DO NOTHING"#)
        .bind(&name)
        .bind(&regex)
        .bind(&rtype)
        .bind(&now)
        .bind(&now)
        .execute(&*pool_clone)
        .await
    {
        Ok(_) => Ok("success".to_string()),
        Err(err) => Err(err.to_string()),
    }
}


#[tauri::command(rename_all = "snake_case")]
pub async fn update_regex(id:i32,status:bool,name: String,regex:String,rtype:String) -> Result<String, String> {
    if name.is_empty() || regex.is_empty() {
        return Err("param null".to_string());
    }
    let now = Utc::now();
    let now = now.timestamp();
    let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.tauri_conn);
    match query(r#"UPDATE cregex SET name=?,regex=?,rtype=?,status=?,update_at=? WHERE id=?"#)
        .bind(&name)
        .bind(&regex)
        .bind(&rtype)
        .bind(&status)
        .bind(&now)
        .bind(&id)
        .execute(&*pool_clone)
        .await
    {
        Ok(_) => Ok("success".to_string()),
        Err(err) => Err(err.to_string()),
    }
}


#[tauri::command(rename_all = "snake_case")]
pub async fn switch_regex_status(cid: i32, status: i32) -> Result<String, String> {
    let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.write_conn);

    match query("UPDATE cregex SET status = ?  WHERE id = ?")
        .bind(status)
        .bind(cid)
        .execute(&*pool_clone)
        .await
    {
        Ok(_) => Ok("success".to_string()),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn del_regex_by_id(cid: i32)->Result<String,String> {
    let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.write_conn);

    match query("DELETE FROM cregex WHERE id = ?")
        .bind(cid)
        .execute(&*pool_clone)
        .await    {
            Ok(_) => Ok("success".to_string()),
            Err(err) => Err(err.to_string()),
        }
    
}
