use std::sync::Arc;

use sqlx::query;

use super::asm_task::INNERASK_MODULE;

#[derive(Debug, serde::Serialize, Clone, sqlx::FromRow)]
pub struct WebSite {
    pub id: Option<i32>,
    pub task_id: i32,
    pub url: String,                 //网站URL
    pub base_url: Option<String>,     //图标的hash
    pub favicon: Option<String>,     //图标的hash
    pub title: Option<String>,       //网站的标题
    pub headers: Option<String>,     //请求响应的头
    pub status_code: Option<i32>,   //响应状态码
    pub finger: Option<String>,     //网站指纹，JSON序列化后的字符串
    pub screenshot: Option<String>,  //网站的截图
    pub tags: Option<String>,       //标签，JSON序列化后的字符串
    pub ssl_info: Option<String>, //网站证书信息
    pub create_at: i64,           //创建时间
    pub update_at: i64,           //最近一个访问或者更新时间
}

impl WebSite {
    pub fn new() -> Self {
        Self {
            id: None,
            task_id: 0,
            url: "".to_string(),
            base_url:None,
            favicon: None,
            title: None,
            headers: None,
            status_code: None,
            finger: None,
            screenshot: None,
            tags: None,
            ssl_info: None,
            create_at: 0,
            update_at: 0,
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_websites(
    page: i32,
    pagesize: i32,
    _dtype: String,
    filter: String,
    query: String,
) -> Result<serde_json::Value, String> {
    if page <= 0 || pagesize <= 0 {
        return Err("Invalid page or pagesize".into());
    }

    let allowed_filters = ["task_id", "url", "base_url", "title", "status_code", "finger"]; // 允许的列名
    if !allowed_filters.contains(&filter.as_str()) {
        return Err("Invalid filter".into());
    }

    let task_module = INNERASK_MODULE
        .get()
        .ok_or("Global variable not initialized")?;
    let tauri_conn = Arc::clone(&task_module.tauri_conn);
    let offset = (page - 1) * pagesize;

    let (sql, params) = if !query.is_empty() {
        match filter.as_str() {
            "task_id" => (
                "SELECT id,task_id,url,base_url,favicon,title,status_code,headers,finger,screenshot,tags,ssl_info,create_at,update_at FROM website WHERE task_id = ? ORDER BY create_at DESC LIMIT ? OFFSET ?",
                vec![query.clone(), pagesize.to_string(), offset.to_string()],
            ),
            "url" => (
                "SELECT id,task_id,url,base_url,favicon,title,status_code,headers,finger,screenshot,tags,ssl_info,create_at,update_at FROM website WHERE url LIKE ? ORDER BY create_at DESC LIMIT ? OFFSET ?",
                vec![
                    format!("%{}%", query.clone()),
                    pagesize.to_string(),
                    offset.to_string(),
                ],
            ),
            "title" => (
                "SELECT id,task_id,url,base_url,favicon,title,status_code,headers,finger,screenshot,tags,ssl_info,create_at,update_at FROM website WHERE title LIKE ? ORDER BY create_at DESC LIMIT ? OFFSET ?",
                vec![
                    format!("%{}%", query.clone()),
                    pagesize.to_string(),
                    offset.to_string(),
                ],
            ),
            "status_code" => (
                "SELECT id,task_id,url,base_url,favicon,title,status_code,headers,finger,screenshot,tags,ssl_info,create_at,update_at FROM website WHERE status_code = ? ORDER BY create_at DESC LIMIT ? OFFSET ?",
                vec![query.clone(), pagesize.to_string(), offset.to_string()],
            ),
            "finger" => (
                "SELECT id,task_id,url,base_url,favicon,title,status_code,headers,finger,screenshot,tags,ssl_info,create_at,update_at FROM website WHERE finger LIKE ? ORDER BY create_at DESC LIMIT ? OFFSET ?",
                vec![
                    format!("%{}%", query.clone()),
                    pagesize.to_string(),
                    offset.to_string(),
                ],
            ),
            _ => (
                "SELECT id,task_id,url,base_url,favicon,title,status_code,headers,finger,screenshot,tags,ssl_info,create_at,update_at FROM website ORDER BY create_at DESC LIMIT ? OFFSET ?",
                vec![pagesize.to_string(), offset.to_string()],
            ),
        }
    } else {
        (
            "SELECT id,task_id,url,base_url,favicon,title,status_code,headers,finger,screenshot,tags,ssl_info,create_at,update_at FROM website ORDER BY create_at DESC LIMIT ? OFFSET ?",
            vec![pagesize.to_string(), offset.to_string()],
        )
    };

    // 构建查询
    let mut query_builder = sqlx::query_as::<_, WebSite>(sql);
    for param in params {
        query_builder = query_builder.bind(param);
    }

    // 执行查询
    let websites = query_builder
        .fetch_all(&*tauri_conn)
        .await
        .map_err(|e| e.to_string())?;

    // 获取总记录数
    let count_sql = if !query.is_empty() {
        match filter.as_str() {
            "task_id" => "SELECT count(*) FROM website WHERE task_id = ?",
            "url" => "SELECT count(*) FROM website WHERE url LIKE ?",
            "title" => "SELECT count(*) FROM website WHERE title LIKE ?",
            "status_code" => "SELECT count(*) FROM website WHERE status_code = ?",
            "finger" => "SELECT count(*) FROM website WHERE finger LIKE ?",
            _ => "SELECT count(*) FROM website",
        }
    } else {
        "SELECT count(*) FROM website"
    };

    let total_count: i64 = if query.is_empty() {
        sqlx::query_scalar(count_sql)
            .fetch_one(&*tauri_conn)
            .await
            .map_err(|e| e.to_string())?
    } else {
        let param = if filter.as_str() == "status_code" || filter.as_str() == "url" || filter.as_str() == "title" || filter.as_str() == "finger" {
            format!("%{}%", query)
        } else {
            query
        };
        
        sqlx::query_scalar(count_sql)
            .bind(&param)
            .fetch_one(&*tauri_conn)
            .await
            .map_err(|e| e.to_string())?
    };

    Ok(serde_json::json!({
        "list": websites,
        "total": total_count
    }))
}




#[tauri::command(rename_all = "snake_case")]
pub async fn del_website_by_id(id: i32) -> Result<String, String> {
    let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    query("DELETE FROM website WHERE id = ?")
        .bind(id)
        .execute(&*pool_clone)
        .await
        .unwrap();
   
    Ok("success".to_string())
}