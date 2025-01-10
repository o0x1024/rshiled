use crate::utils;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct IPs {
    pub id: Option<isize>,
    pub enterprise_id: isize,
    pub ip_addr: Option<String>,
    pub domain: Option<String>,
    pub port_count: Option<u8>,
    pub create_at: i64,
    pub update_at: i64,
}

#[tauri::command]
pub async fn get_ips(
    page: isize,
    pagesize: isize,
    dtype: String,
    query: String,
) -> Result<serde_json::Value, String> {
    let mut ip_list = vec![];
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();

    let base_sql = match dtype.as_str() {
        "provider" => "SELECT id,enterprise_id,ip_addr,domain,create_at,update_at FROM IPs ",
        "location" => "SELECT id,enterprise_id,ip_addr,domain,reate_at,update_at FROM IPs ", 
        _ => "SELECT id,enterprise_id,ip_addr,domain,create_at,update_at FROM IPs"
    };

    let sql = format!("{} WHERE ip_addr LIKE ? LIMIT ?, ?", base_sql);

    // Prepare the domain pattern if query is not empty
    let ip_pattern = format!("%{}", query);

    let mut stmt = conn.prepare(&sql).unwrap();
    let ips_iter = stmt
        .query_map(
            params![ip_pattern, (page - 1) * pagesize, pagesize],
            |row| {
                Ok(IPs {
                    id: row.get(0)?,
                    enterprise_id: row.get(1)?,
                    ip_addr: row.get(2)?,
                    domain:row.get(3)?,
                    port_count:Some(0),
                    create_at: row.get(4)?,
                    update_at: row.get(5)?,
                })
            },
        )
        .unwrap();

    for ip in ips_iter {
        ip_list.push(ip.unwrap());
    }

    // 获取总记录数
    let count_sql = match dtype.as_str() {
        "AAAA" => "SELECT count(*) FROM IPs WHERE aaa IS NOT NULL AND ip_addr LIKE ? ",
        "CNAME" => "SELECT count(*) FROM IPs WHERE cname IS NOT NULL AND ip_addr LIKE ? ",
        "NS" => "SELECT count(*) FROM IPs WHERE ns IS NOT NULL AND ip_addr LIKE ? ",
        "MS" => "SELECT count(*) FROM IPs WHERE mx IS NOT NULL AND ip_addr LIKE ? ",
        _ => "SELECT count(*) FROM IPs WHERE ip_addr LIKE ? ",
    };

    let mut count_stmt = conn.prepare(count_sql).unwrap();
    let domain_pattern = format!("%{}", query);
    let total_count: isize = match count_stmt.query_row([domain_pattern], |row| row.get(0)) {
        Ok(sum) => sum,
        Err(_) => 0,
    }; // 获取总记录数

    Ok(serde_json::json!({
        "ip_list": ip_list,
        "total": total_count
    }))

    // Ok((domain_list,total_count)) // 返回企业列表
}
