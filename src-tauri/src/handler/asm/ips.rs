use std::sync::Arc;

use log::error;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query_as, query_scalar};

use super::asm_task::INNERASK_MODULE;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct IPs {
    pub id: Option<i32>,
    pub task_id: i32,
    pub ip_addr: Option<String>,
    pub domain_id: Option<i32>,
    #[sqlx(skip)]
    pub domain: Option<String>,
    #[sqlx(skip)]
    pub port_count: Option<u8>,
    pub create_at: i64,
    pub update_at: i64,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct IPWithDomain {
    pub id: Option<i32>,
    pub task_id: i32,
    pub ip_addr: Option<String>,
    pub domain_id: Option<i32>,
    pub domain: Option<String>,
    pub create_at: i64,
    pub update_at: i64,
}

impl From<IPWithDomain> for IPs {
    fn from(ip_with_domain: IPWithDomain) -> Self {
        IPs {
            id: ip_with_domain.id,
            task_id: ip_with_domain.task_id,
            ip_addr: ip_with_domain.ip_addr,
            domain_id: ip_with_domain.domain_id,
            domain: ip_with_domain.domain,
            port_count: None,
            create_at: ip_with_domain.create_at,
            update_at: ip_with_domain.update_at,
        }
    }
}

#[tauri::command]
pub async fn get_ips(
    page: i32,
    pagesize: i32,
    dtype: String,
    query: String,
    filter: String,
) -> Result<serde_json::Value, String> {
    let allowed_filters = [
        "task_id", "domain", "A", "CNAME", "NS", "MX", "provider", "location", "ip_addr",
    ]; // 允许的列名
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

    let pool_clone = Arc::clone(&task_module.tauri_conn);
    let domain_pattern = format!("%{}%", query);
    let offset = (page - 1) * pagesize;

    let result = {
        match dtype.as_str() {
            "ip_segment" => {
                let sql = format!("SELECT ips.id, ips.task_id, ips.ip_addr, ips.domain_id, domain.domain, ips.create_at, ips.update_at FROM ips LEFT JOIN domain ON ips.domain_id = domain.id WHERE ips.{} LIKE ? LIMIT ? OFFSET ?", filter);
                let ipinfo: Vec<IPWithDomain> = query_as(&sql)
                    .persistent(true)
                    .bind(&domain_pattern)
                    .bind(pagesize as i32)
                    .bind(offset as i32)
                    .fetch_all(&*pool_clone)
                    .await
                    .map_err(|e| {
                        error!("Failed to fetch ips: {}", e);
                        e.to_string()
                    })?;
                let count_sql = format!(
                    "SELECT COUNT(*) FROM ips WHERE {} LIKE ?",
                    filter
                );
                let total_count: i64 = query_scalar(&count_sql)
                    .bind(&domain_pattern)
                    .fetch_one(&*pool_clone)
                    .await
                    .unwrap_or(0);

                serde_json::json!({
                    "list": ipinfo,
                    "total": total_count
                })
            }
            "provider" => {
                let sql = format!("SELECT ips.id, ips.task_id, ips.ip_addr, ips.domain_id, domain.domain, ips.create_at, ips.update_at FROM ips LEFT JOIN domain ON ips.domain_id = domain.id WHERE ips.{} LIKE ? LIMIT ? OFFSET ?", filter);
                let ipinfo: Vec<IPWithDomain> = query_as(&sql)
                    .persistent(true)
                    .bind(&domain_pattern)
                    .bind(pagesize as i32)
                    .bind(offset as i32)
                    .fetch_all(&*pool_clone)
                    .await
                    .map_err(|e| {
                        error!("Failed to fetch ips: {}", e);
                        e.to_string()
                    })?;
                let count_sql = format!(
                    "SELECT COUNT(*) FROM ips WHERE {} LIKE ?",
                    filter
                );
                let total_count: i64 = query_scalar(&count_sql)
                    .bind(&domain_pattern)
                    .fetch_one(&*pool_clone)
                    .await
                    .unwrap_or(0);

                serde_json::json!({
                    "list": ipinfo,
                    "total": total_count
                })
            }
            "location" => {
                let sql = format!("SELECT ips.id, ips.task_id, ips.ip_addr, ips.domain_id, domain.domain, ips.create_at, ips.update_at FROM ips LEFT JOIN domain ON ips.domain_id = domain.id WHERE ips.{} LIKE ? LIMIT ? OFFSET ?", filter);
                let ipinfo: Vec<IPWithDomain> = query_as(&sql)
                    .persistent(true)
                    .bind(&domain_pattern)
                    .bind(pagesize as i32)
                    .bind(offset as i32)
                    .fetch_all(&*pool_clone)
                    .await
                    .map_err(|e| {
                        error!("Failed to fetch ips: {}", e);
                        e.to_string()
                    })?;
                let count_sql = format!(
                    "SELECT COUNT(*) FROM ips WHERE {} LIKE ?",
                    filter
                );
                let total_count: i64 = query_scalar(&count_sql)
                    .bind(&domain_pattern)
                    .fetch_one(&*pool_clone)
                    .await
                    .unwrap_or(0);

                serde_json::json!({
                    "list": ipinfo,
                    "total": total_count
                })
            }

            _ => {
                let sql = format!("SELECT ips.id, ips.task_id, ips.ip_addr, ips.domain_id, domain.domain, ips.create_at, ips.update_at FROM ips LEFT JOIN domain ON ips.domain_id = domain.id WHERE ips.{} LIKE ?  LIMIT ? OFFSET ?", filter);
                let ipinfo: Vec<IPWithDomain> = query_as(&sql)
                    .persistent(true)
                    .bind(&domain_pattern)
                    .bind(pagesize as i32)
                    .bind(offset as i32)
                    .fetch_all(&*pool_clone)
                    .await
                    .map_err(|e| {
                        error!("Failed to fetch ips: {}", e);
                        e.to_string()
                    })?;
                let count_sql = format!(
                    "SELECT COUNT(*) FROM ips WHERE {} LIKE ?",
                    filter
                );
                let total_count: i64 = query_scalar(&count_sql)
                    .bind(&domain_pattern)
                    .fetch_one(&*pool_clone)
                    .await
                    .unwrap_or(0);

                serde_json::json!({
                    "list": ipinfo,
                    "total": total_count
                })
            }
        }
    };

    Ok(result)
}
