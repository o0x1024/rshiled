use rusqlite::{params, Connection};

use crate::utils;

#[derive(Debug, serde::Serialize)]
pub struct Domain {
    pub id: usize,
    pub enterprise_id: usize,
    pub domain: String,
    pub aaa: Option<String>,
    pub cname: Option<String>,
    pub ns: Option<String>,
    pub mx: Option<String>,
    pub create_at: isize,
    pub update_at: isize,
}

#[tauri::command]
pub async fn get_domains(
    page: isize,
    pagesize: isize,
    dtype: String,
    query: String,
) -> Result<serde_json::Value, String> {
    let mut domain_list = vec![];
    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();

    let base_sql = match dtype.as_str() {
        "AAAA" => "SELECT id,enterprise_id,domain,aaa,cname,ns,mx,create_at,update_at FROM Domain WHERE aaa IS NOT NULL",
        "CNAME" => "SELECT id,enterprise_id,domain,aaa,cname,ns,mx,create_at,update_at FROM Domain WHERE cname IS NOT NULL", 
        "NS" => "SELECT id,enterprise_id,domain,aaa,cname,ns,mx,create_at,update_at FROM Domain WHERE ns IS NOT NULL",
        "MS" => "SELECT id,enterprise_id,domain,aaa,cname,ns,mx,create_at,update_at FROM Domain WHERE mx IS NOT NULL",
        _ => "SELECT id,enterprise_id,domain,aaa,cname,ns,mx,create_at,update_at FROM Domain"
    };

    let sql =format!("{} WHERE domain LIKE ? LIMIT ?, ?", base_sql);

    // Prepare the domain pattern if query is not empty
    let domain_pattern = format!("%{}", query);

    let mut stmt = conn.prepare(&sql).unwrap();
    let domain_iter = stmt
        .query_map(params![domain_pattern, (page - 1) * pagesize, pagesize],
            |row| {
                Ok(Domain {
                    id: row.get(0)?,
                    enterprise_id: row.get(1)?,
                    domain: row.get(2)?,
                    aaa: row.get(3)?,
                    cname: row.get(4)?,
                    ns: row.get(5)?,
                    mx: row.get(6)?,
                    create_at: row.get(7)?,
                    update_at: row.get(8)?,
                })
            },
        )
        .unwrap();

    for domain in domain_iter {
        domain_list.push(domain.unwrap());
    }

    // 获取总记录数
    let count_sql = match dtype.as_str() {
        "AAAA" => "SELECT count(*) FROM Domain WHERE aaa IS NOT NULL AND domain LIKE ? ",
        "CNAME" => "SELECT count(*) FROM Domain WHERE cname IS NOT NULL AND domain LIKE ? ",
        "NS" => "SELECT count(*) FROM Domain WHERE ns IS NOT NULL AND domain LIKE ? ",
        "MS" => "SELECT count(*) FROM Domain WHERE mx IS NOT NULL AND domain LIKE ? ",
        _ => "SELECT count(*) FROM Domain WHERE domain LIKE ? ",
    };

    let mut count_stmt = conn.prepare(count_sql).unwrap();
    let domain_pattern = format!("%{}", query);
    let total_count: isize = match count_stmt.query_row([domain_pattern], |row| row.get(0)) {
        Ok(sum) => sum,
        Err(_) => 0,
    }; // 获取总记录数

    Ok(serde_json::json!({
        "domain_list": domain_list,
        "total": total_count
    }))

    // Ok((domain_list,total_count)) // 返回企业列表
}
