//风险有哪些
//1.未授权访问
//2.网页文件泄露
//3.弱口令
//3.网页和JS中的敏感信息泄露检测

// use log::error;
// use protobuf::well_known_types::Api;
// use regex::Regex;
// use reqwest::Client;

use log::error;
use reqwest::Client;
use serde::{Deserialize, Serialize};
// use spider::{features::chrome_common::RequestInterceptConfiguration, website::Website};
use sqlx::{query_as, query_scalar, FromRow, SqlitePool};
use std::{error::Error, sync::Arc};
use tokio::sync::{mpsc, Mutex, Semaphore};
use url::Url;

// use tokio::sync::{mpsc, Mutex, Semaphore};
// use url::Url;
//扫描方式
//nuclei
//目录扫描和漏洞扫描
//口令暴破
// use swc_common::{input::SourceFileInput, FileName, SourceMap};
// use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, Syntax};
// use swc_ecma_visit::{Visit, VisitWith};
// use swc_ecmascript::ast::{CallExpr, Callee, EsVersion, Expr, ImportDecl, Lit, MemberProp};

// use crate::{global::config::AppConfig, internal::html::extract_js_from_html};

use crate::{global::config::CoreConfig, internal::rsubdomain::handle};

use super::asm_task::INNERASK_MODULE;

// struct MethodVisitor<'a> {
//     source_code: &'a str,
//     current_url: &'a str,
//     results: HashMap<String, ApiInfo>,
//     js_paths: Vec<String>,
// }

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ApiInfo {
    pub id: Option<i32>,
    pub task_id: Option<i32>,
    pub method: Option<String>,
    pub uri: String,
    pub url: String,
    pub get_response: String,
    pub post_response: String,
    pub ufrom: String,
    pub update_at: i64,
    pub http_status: i64,
    pub handle_status: i64,
    pub get_body_length: i64,
    pub post_body_length: i64,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ApiTypeInfo {
    pub uri: String,
    pub count: i64,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn process_apis(handle_status: i32, api_ids: Vec<u32>) -> Result<String, String> {
    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => return Err("Global variable not initialized".into()),
    };
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    for api_id in api_ids {
        sqlx::query("UPDATE api SET handle_status = ? WHERE id = ?")
            .bind(&handle_status)
            .bind(&api_id)
            .execute(&*pool_clone)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok("Success".into())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_apis(
    page: i32,
    pagesize: i32,
    dtype: String,
    filter: String,
    query: String,
    handle_status: Vec<String>,
    http_status: Vec<String>,
) -> Result<serde_json::Value, String> {
    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => {
            error!("Global variable not initialized");
            return Err("Global variable not initialized".into());
        }
    };
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    let allowed_filters = ["ufrom", "uri", "http_status", "task_id"]; // 允许的列名
    if !allowed_filters.contains(&filter.as_str()) {
        return Err("Invalid filter".into());
    }

    let domain_pattern = format!("%{}%", query);

    let offset = (page - 1) * pagesize;
    let result = match dtype.as_str() {
        "atype" => {
            let sql = format!("SELECT uri, COUNT(*) as count FROM api WHERE {} LIKE ?  GROUP BY uri ORDER BY count DESC LIMIT ? OFFSET ?", filter);
            let apinfo: Vec<ApiTypeInfo> = query_as(&sql)
                .persistent(true)
                .bind(&domain_pattern)
                .bind(pagesize as i32)
                .bind(offset as i32)
                .fetch_all(&*pool_clone)
                .await
                .map_err(|e| {
                    error!("Failed to fetch APIs: {}", e);
                    e.to_string()
                })?;
            let count_sql = format!(
                "SELECT COUNT(DISTINCT uri) FROM api WHERE {} LIKE ?",
                filter
            );
            let total_count: i64 = query_scalar(&count_sql)
                .bind(&domain_pattern)
                .fetch_one(&*pool_clone)
                .await
                .unwrap_or(0);
            
            serde_json::json!({
                "list": apinfo,
                "total": total_count
            })
        }
        _ => {
            let sql = format!(
                "SELECT * FROM api WHERE {} LIKE ? AND handle_status IN ({}) AND http_status IN ({}) ORDER BY update_at DESC LIMIT ? OFFSET ? ",
                filter, handle_status.join(","), http_status.join(",")
            );
            let apinfo: Vec<ApiInfo> = query_as(&sql)
                .bind(&domain_pattern)
                .bind(pagesize as i32)
                .bind(offset as i32)
                .fetch_all(&*pool_clone)
                .await
                .map_err(|e| {
                    error!("Failed to fetch APIs: {}", e);
                    e.to_string()
                })?;

            let count_sql = format!("SELECT count(*) FROM api WHERE {} LIKE ? AND handle_status IN ({}) AND http_status IN ({})", filter, handle_status.join(","), http_status.join(","));
            let total_count: i64 = query_scalar(&count_sql)
                .bind(&domain_pattern)
                .fetch_one(&*pool_clone)
                .await
                .unwrap_or(0);

            serde_json::json!({
                "list": apinfo,
                "total": total_count
            })
        }
    };

    Ok(result)
}

pub async fn collection_api(task_id: &i32, apis: &Vec<ApiInfo>) -> Result<(), Box<dyn Error>> {
    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => return Err("Global variable not initialized".into()),
    };
    let write_conn: Arc<SqlitePool> = Arc::clone(&task_module.write_conn);
    // 开始事务
    let mut tx: sqlx::Transaction<'_, sqlx::Sqlite> = write_conn.begin().await?;

    for api in apis {
        sqlx::query(
            "INSERT INTO api (task_id,method, uri,ufrom,update_at) VALUES (?, ?,?,?,?) ON CONFLICT DO NOTHING",
        )
        .bind(&task_id)
        .bind(&api.method)
        .bind(&api.uri)
        .bind(&api.ufrom)
        .bind(&api.update_at)
        .execute(&mut *tx) // 在事务中执行
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn scan_api(task_id: &i32) -> Result<(), Box<dyn Error>> {
    //获取当前扫描任务的所有API信息
    //提取ufrom的网站前缀
    //组件API和网站，使用多线程进行API扫描，分别使用GET和POST进行扫描
    //如果响应值为200，则认为扫描成功，否则认为扫描失败，并且保存响应信息
    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => return Err("Global variable not initialized".into()),
    };
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    // 获取全局的http client
    let client = match CoreConfig::global()?.http_client.clone() {
        Some(client) => client,
        None => Client::new(),
    };

    //添加线程限制，限制线程从全局配置中获取
    let semaphore = Arc::new(Semaphore::new(match CoreConfig::global()?.thread_num.clone() {
        Some(num) => num as usize,
        None => 10,
    }));

    let mut apis: Vec<ApiInfo> = query_as("SELECT * FROM api WHERE task_id = ? and url is null")
        .bind(task_id)
        .fetch_all(&*pool_clone)
        .await
        .map_err(|e| {
            error!("Failed to fetch APIs: {}", e);
            e.to_string()
        })?;

    // 提取ufrom的网站前缀
    // let mut websites = Vec::new();
    for api in &mut apis {
        if api.ufrom.contains("http") {
            let url = Url::parse(&api.ufrom).unwrap();
            let host = url.origin().ascii_serialization();
            let uri = api.uri.as_str();
            let url = format!("{}{}", host, uri);
            api.url = url;
        }
    }

    let (tx, mut rx) = mpsc::channel(10000);
    let mut tasks = Vec::new();

    // 使用多线程进行API扫描
    for api in apis {
        let mut api_clone = api.clone();
        let client_clone = client.clone();
        let tx_clone = tx.clone();
        let semaphore_clone = semaphore.clone();
        tasks.push(tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await;
            // GET请求
            // println!("scan api: {}", api_clone.url);
            match client_clone.get(&api_clone.url)
                .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)...")
                .header("Referer", "https://www.mgtv.com/")
                .header("Accept", "application/json")
                .send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        api_clone.http_status = response.status().as_u16() as i64;
                        api_clone.get_body_length = response.content_length().unwrap_or(0) as i64;

                        let response_text = response.text().await.unwrap_or("--".to_string());
                        //保存任何响应体,只保存最大10KB

                        let response_text = if response_text.len() > 10240 {
                            response_text[0..10240].to_string()
                        } else {
                            response_text
                        };

                        api_clone.get_response = response_text;
                        if let Err(e) = tx_clone.send(api_clone.clone()).await {
                            // error!("Failed to send  result: {}", e);
                        }
                        
                        // if let Ok(json_data) =
                        //     serde_json::from_str::<serde_json::Value>(&response_text)
                        // {
                        //     if json_data.is_object() {
                        //         api_clone.get_body_length = response_text.len() as i64;
                        //         api_clone.get_response = json_data.to_string();
                        //         if let Err(e) = tx_clone.send(api_clone.clone()).await {
                        //             error!("Failed to send  result: {}", e);
                        //         }
                        //     }
                        // }
                    }
                }
                Err(e) => {
                    // error!("Failed to send GET request: {:#?}", e);
                }
            }

            match client_clone.post(&api_clone.url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)...")
            .header("Referer", "https://www.mgtv.com/")
            .header("Accept", "application/json")
            .send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        api_clone.http_status = response.status().as_u16() as i64;
                        api_clone.post_body_length = response.content_length().unwrap_or(0) as i64;

                        let response_text = response.text().await.unwrap_or("--".to_string());
                        //保存任何响应体,只保存最大10KB

                        let response_text = if response_text.len() > 10240 {
                            response_text[0..10240].to_string()
                        } else {
                            response_text
                        };

                        api_clone.post_response = response_text;
                        if let Err(e) = tx_clone.send(api_clone.clone()).await {
                            // error!("Failed to send  result: {}", e);
                        }
                    }
                }
                Err(e) => {
                    // error!("Failed to send POST request: {:#?}", e);
                }
            }
        }));
    }

    // 显式丢弃发送者，这样接收者就知道没有更多消息了
    drop(tx);

    let mut rapis = Vec::new();

    // 同时等待channel接收消息和所有任务完成
    // let mut tasks_iter = tasks.into_iter();
    // let mut current_task = tasks_iter.next();

    // 接收消息
    while let Some(api) = rx.recv().await {
        rapis.push(api);
    }

    // 更新数据库
    let mut tx: sqlx::Transaction<'_, sqlx::Sqlite> = pool_clone.begin().await
    .map_err(|e| {
        error!("Failed to begin transaction: {}", e);
        e.to_string()
    })?;
    for api in rapis.iter() {
         sqlx::query(
            "UPDATE api SET http_status = ?,url = ?,get_response = ?,post_response = ?,get_body_length = ?,post_body_length = ?,update_at = ? WHERE id = ?",
        )
        .bind(&api.http_status)
        .bind(&api.url)
        .bind(&api.get_response)
        .bind(&api.post_response)
        .bind(&api.get_body_length)
        .bind(&api.post_body_length)
        .bind(&api.update_at)
        .bind(&api.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error!("Failed to update API: {}", e);
            e.to_string()
        })?;
    }

    tx.commit().await
    .map_err(|e| {
        error!("Failed to commit transaction: {}", e);
        e.to_string()
    })?;
    Ok(())
}
