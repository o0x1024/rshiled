use rusqlite::Connection;

use crate::utils;




#[derive(Debug, serde::Serialize,Clone)]
pub struct WebSite{
    pub id:Option<isize>,
    pub enterprise_id: isize,
    pub url:String,     //网站URL
    pub favicon:Option<String>,   //图标的hash
    pub title:Option<String>,    //网站的标题
    pub headers:Option<String>,    //请求响应的头
    pub finger:Option<Vec<String>>,    //网站指纹
    pub screenshot:Option<String>,     //网站的截图
    pub tags:Option<Vec<String>>,
    pub ssl_info:Option<String>,      //网站证书信息
    pub create_at: i64,    //创建时间
    pub update_at: i64,   //最近一个访问或者更新时间
}




#[tauri::command(rename_all = "snake_case")]
pub async fn get_websites(
    page: isize,
    pagesize: isize,
) -> Result<serde_json::Value, String> {
    let mut website_list = vec![];
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();
    let mut stmt = conn.prepare("SELECT id,enterprise_id,url,favicon,title,headers,finger,screenshot,tags ,ssl_info ,create_at,update_at FROM website limit ?,?").unwrap();
    let website_iter = stmt
        .query_map(
            [(page - 1) * pagesize, pagesize],
            |row: &rusqlite::Row<'_>| {
                Ok(WebSite {
                    id: row.get(0)?,
                    enterprise_id: row.get(1)?,
                    url: row.get(2)?,
                    favicon: row.get(3)?,
                    title: row.get(4)?,
                    headers: row.get(5)?,
                    finger:row.get::<_, Option<String>>(6)?.and_then(|s| serde_json::from_str(&s).ok()),
                    screenshot: row.get(7)?,
                    tags: row.get::<_, Option<String>>(8)?.and_then(|s| serde_json::from_str(&s).ok()),
                    ssl_info: row.get(9)?,
                    create_at: row.get(10)?,
                    update_at: row.get(11)?,

                })
            },
        )
        .unwrap();

    for website in website_iter {
        //使用task_module,获取所有任务状态添加到enterprise中
        match website {
            Ok(ent) => {
                // let status = TASK_MODULE.query_task_status(ent.id);
                // ent.running_status = status;
                website_list.push(ent);
            }
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        }
    }
    let count_sql = "SELECT count(*) FROM website";
    let mut count_stmt = conn.prepare(count_sql).unwrap();
    let total_count: isize = count_stmt.query_row([], |row| row.get(0)).unwrap(); // 获取总记录数

    Ok(serde_json::json!({
        "list": website_list,
        "total": total_count
    }))
}