use super::enterprise::Enterprise;
use crate::utils;
use chrono::prelude::*;
use rusqlite::Connection;

#[derive(Debug, serde::Serialize)]
pub struct RootDomain {
    pub id: isize,
    pub domain: String,
    pub enterprise_name: String,
    pub enterprise_id: isize,
    pub count: isize,
    pub create_at: i64,
    pub update_at: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct EntInfo {
    pub enterprise_id: isize,
    pub enterprise_name: String,
    pub count: isize,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn add_root_domain(
    enterprise_id: isize,
    root_domain: Vec<String>,
) -> Result<String, String> {
    println!("{:}  {:?}", enterprise_id, root_domain);
    let db_path = utils::file::get_db_path();
    let now = Utc::now();
    let timestamp = now.timestamp();
    let conn = Connection::open(db_path).unwrap();

    for domain in root_domain {
        let mut stmt = conn
            .prepare("SELECT name FROM Enterprise where id = ? ")
            .unwrap();
        let enterprise_name: String = stmt.query_row([&enterprise_id], |row| row.get(0)).unwrap();

        print!("{}", enterprise_name);
        let icpdomain = RootDomain {
            id: 0,
            domain: domain,
            enterprise_name: enterprise_name,
            enterprise_id: enterprise_id,
            create_at: timestamp,
            update_at: timestamp,
            count: 0,
        };
        match conn.execute(
			"INSERT INTO RootDomain (domain,enterprise_id,enterprise_name,create_at,update_at) VALUES (?1,?2,?3,?4,?5)",
			(&icpdomain.domain, &icpdomain.enterprise_id, &icpdomain.enterprise_name, &icpdomain.create_at, &icpdomain.update_at),
		){
            Ok(_) => (),
            Err(_) =>()
        }
    }

    Ok("This worked!".into())
}

#[tauri::command]
pub async fn get_root_domains(page: usize, pagesize: usize) -> Result<serde_json::Value, String> {
    let mut icp_list = vec![];
    let db_path = utils::file::get_db_path();

    let conn = Connection::open(db_path).unwrap();
    let mut stmt = conn.prepare("SELECT * FROM RootDomain limit ?,?").unwrap();
    let root_domain_iter = stmt
        .query_map([(page - 1) * pagesize, pagesize], |row| {
            Ok(RootDomain {
                id: row.get(0)?,
                domain: row.get(1)?,
                enterprise_id: row.get(2)?,
                enterprise_name: row.get(3)?,
                create_at: row.get(4)?,
                update_at: row.get(5)?,
                count: 0,
            })
        })
        .unwrap();

    for root_domain in root_domain_iter {

        let mut rd = match root_domain {
            Ok(_rd) => _rd,
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        };

        let count_sql = "SELECT count(*) FROM Domain where domain like ?";
        let pattern = format!("%{}", rd.domain.clone());

        let mut count_stmt = conn.prepare(count_sql).unwrap();
        let total_count: isize = count_stmt.query_row([pattern], |row| row.get(0)).unwrap(); // 获取总记录数
        rd.count = total_count;

        icp_list.push(rd);
    }

    

    // 获取总记录数
    let count_sql = "SELECT count(*) FROM RootDomain";
    let mut count_stmt = conn.prepare(count_sql).unwrap();
    let total_count: isize = count_stmt.query_row([], |row| row.get(0)).unwrap(); // 获取总记录数

    Ok(serde_json::json!({
        "list": icp_list,
        "total": total_count
    }))

    // 返回企业列表
}

#[tauri::command]
pub async fn get_ent_domain(page: usize, pagesize: usize) -> Result<Vec<EntInfo>, String> {
    let mut icp_list = vec![];
    let db_path = utils::file::get_db_path();

    let conn = Connection::open(db_path).unwrap();
    let mut ent_stmt = conn.prepare("SELECT * FROM Enterprise LIMIT ?, ?").unwrap();

    let enterprise_iter = ent_stmt
        .query_map([(page - 1) * pagesize, pagesize], |row| {
            Ok(Enterprise {
                id: row.get(0)?,
                name: row.get(1)?,
                monitor_status: row.get(2)?,
                next_runtime: row.get(3)?,
                running_status: row.get(4)?,
            })
        })
        .unwrap();

    for ent in enterprise_iter {
        match ent {
            Ok(enterprise) => {
                let mut eif = EntInfo {
                    enterprise_id: enterprise.id,
                    enterprise_name: enterprise.name,
                    count: 0,
                };
                let mut count_stmt = conn
                    .prepare("SELECT COUNT(*) FROM Domain WHERE enterprise_id = ?")
                    .unwrap();
                let count: i64 = count_stmt
                    .query_row([enterprise.id], |row| row.get(0))
                    .unwrap();
                eif.count = count as isize;

                icp_list.push(eif);
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }

    Ok(icp_list) // 返回企业列表
}

#[tauri::command]
pub async fn del_rootdomain_by_id(did: usize) {
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();
    let sql = "DELETE FROM RootDomain WHERE id=?;";
    match conn.execute(sql, &[&did]) {
        Ok(_) => (),
        Err(err) => println!("err:{:}", err),
    }
}
