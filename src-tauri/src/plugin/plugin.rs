use chrono::Utc;
use log::{error, info};
use rusqlite::{params, Connection, Result};

use crate::utils;

#[derive(Debug, serde::Serialize, serde::Deserialize)] // Add Deserialize here
pub struct Plugin {
    pub id: isize,
    // 插件的名称
    pub name: String,
    // 插件的版本号
    pub version: String,
    // 插件的描述信息
    pub description: String,
    // 插件的作者
    pub author: String,
    // 插件的类型，用于区分不同类型的插件
    pub plugin_type: String,
    // // 插件的配置信息，以 JSON 格式存储
    // 插件的输入参数或输入数据
    pub input: String,
    // 插件的输出结果或输出数据
    pub output: String,
    // 插件的状态，表示插件是启用还是禁用
    pub status: isize,
    // 插件的脚本代码，通常用于定义插件的具体行为
    pub script: String,
    pub create_at: isize,
    pub update_at: isize,
}

#[tauri::command]
pub fn get_plugin_list(
    page: isize,
    pagesize: isize,
    ptype: &str,
    query: &str,
) -> Result<serde_json::Value, String> {
    //返回插件列表
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();
    let mut stmt = conn.prepare("SELECT id,name, version, description, author, plugin_type, input, output, status, script,create_at,update_at FROM plugins limit ?,?").unwrap();
    let plugin_iter = stmt
        .query_map(
            [(page - 1) * pagesize, pagesize],
            |row: &rusqlite::Row<'_>| {
                Ok(Plugin {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    description: row.get(3)?,
                    author: row.get(4)?,
                    plugin_type: row.get(5)?,
                    input: row.get(6)?,
                    output: row.get(7)?,
                    status: row.get::<_, isize>(8)?,
                    script: row.get(9)?,
                    create_at: row.get::<_, isize>(10)?,
                    update_at: row.get::<_, isize>(11)?,
                })
            },
        )
        .unwrap();

    let mut plugins = vec![];

    for plugin in plugin_iter {
        plugins.push(plugin.unwrap());
    }

    let count_sql = "SELECT count(*) FROM plugins";
    let mut count_stmt = conn.prepare(count_sql).unwrap();
    let total_count: isize = count_stmt.query_row([], |row| row.get(0)).unwrap(); // 获取总记录数

    Ok(serde_json::json!({
        "list": plugins,
        "total": total_count
    }))
}

#[tauri::command]
pub async fn get_plugin_from_id(pid: isize) -> Result<Plugin, String> {
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();

    let mut stmt = conn.prepare("SELECT * FROM plugins WHERE id=?").unwrap();

    let plugin = stmt
        .query_row([pid], |row: &rusqlite::Row<'_>| {
            Ok(Plugin {
                id: row.get::<_, isize>(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                description: row.get(3)?,
                author: row.get(4)?,
                plugin_type: row.get(5)?,
                input: row.get(6)?,
                output: row.get(7)?,
                status: row.get::<_, isize>(8)?,
                script: row.get(9)?,
                create_at: row.get::<_, isize>(10)?,
                update_at: row.get::<_, isize>(11)?,
            })
        })
        .unwrap();
    Ok(plugin)
}

#[tauri::command]
pub fn get_plugin_type_list() -> Vec<&'static str> {
    //返回插件列表
    let plugin_type_list = vec!["asm-domain-collection", "asm-risk-scan", "asm-finger-detect"];

    plugin_type_list
}

#[tauri::command]
pub fn new_plugin(plugin: Plugin) -> String {
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();
    let now = Utc::now();
    let timestamp = now.timestamp();

    match conn.execute(
        "INSERT INTO plugins (name, version, description, author, plugin_type, input, output, status, script, create_at,update_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            &plugin.name,
            &plugin.version,
            &plugin.description,
            &plugin.author,
            &plugin.plugin_type,
            &plugin.input,
            &plugin.output,
            plugin.status,
            &plugin.script,
            timestamp,
            timestamp,
        ],
    ){
        Ok(_) => {"新建成功".to_string()}
        Err(err) => err.to_string()
    }
}






#[tauri::command]
pub fn save_plugin(plugin: Plugin) -> String {
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();
    let now = Utc::now();
    let timestamp = now.timestamp();

    match conn.execute(
        "UPDATE plugins SET name=?1,version=?2,description=?3,author=?4,plugin_type=?5 
        , input=?6,output=?7,status=?8,script=?9,update_at=?10 WHERE id = ?11 ",
        params![
            &plugin.name,
            &plugin.version,
            &plugin.description,
            &plugin.author,
            &plugin.plugin_type,
            &plugin.input,
            &plugin.output,
            plugin.status,
            &plugin.script,
            timestamp,
            plugin.id
        ],
    ){
        Ok(_) => {"保存成功".to_string()}
        Err(err) => {
            error!("{:?}", err);
            println!("{:?}", err);
            err.to_string()
        }
    }
}


#[tauri::command]
pub fn edit_plugin(plugin: Plugin) -> String {
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();
    let now = Utc::now();
    let timestamp = now.timestamp();
    match conn.execute(
        "UPDATE plugins SET name = ?1, version = ?2, description = ?3, author = ?4, plugin_type = ?5, input = ?6, output = ?7, status = ?8, script = ?9, update_at = ?10 
         WHERE id = ?11",
        params![
            &plugin.name,
            &plugin.version,
            &plugin.description,
            &plugin.author,
            &plugin.plugin_type,
            &plugin.input,
            &plugin.output,
            plugin.status,
            &plugin.script,
            timestamp,
            plugin.id,
        ],
    ){
        Ok(_) => {"编辑成功".to_string()}
        Err(err) => err.to_string()
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn switch_plugins_status(pid: isize, status: isize) -> Result<String, String> {
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();

    match conn.execute(
        "UPDATE plugins SET status = ?1  WHERE id = ?2",
        (status, pid),
    ) {
        Ok(_) => Ok("success".to_string()),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn del_plugins_by_id(pid: isize) {
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();
    conn.execute("DELETE FROM plugins WHERE id = ?1", (pid,))
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_plugin() {}
}
