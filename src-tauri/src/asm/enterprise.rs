use rusqlite::Connection;

use crate::utils;

#[derive(Debug, serde::Serialize)]
pub struct Enterprise {
    pub id: isize,
    pub name: String,
    pub icp_no: String,
    pub monitor_status: bool,
    pub next_runtime: isize,
    pub running_status: isize,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_enterprise_list() -> Vec<Enterprise> {
    let mut enterprise_list = vec![];
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();
    let mut stmt = conn.prepare("SELECT * FROM Enterprise").unwrap();
    let enterprise_iter = stmt
        .query_map([], |row: &rusqlite::Row<'_>| {
            Ok(Enterprise {
                id: row.get(0)?,
                name: row.get(1)?,
                icp_no: row.get(2)?,
                monitor_status: row.get(3)?,
                next_runtime: row.get(3)?,
                running_status: row.get(4)?,
            })
        })
        .unwrap();

    for enterprise in enterprise_iter {
        enterprise_list.push(enterprise.unwrap());
    }

    enterprise_list // 返回企业列表
}

// fn add_enterprise(enterprise: Enterprise) {

// }
