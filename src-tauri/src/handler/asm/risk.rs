//风险有哪些
//1.未授权访问
//2.网页文件泄露
//3.弱口令
//3.网页和JS中的敏感信息泄露检测

use log::{error, info};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use spider::{features::chrome_common::RequestInterceptConfiguration, website::Website};
use sqlx::{query, query_as, query_scalar, FromRow, SqlitePool};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    error::Error,
    sync::Arc,
    time::Duration,
};
use tokio::{
    fs::File,
    io::{self, AsyncBufReadExt},
    sync::{mpsc, Mutex, Semaphore},
};
use url::Url;
//扫描方式
//nuclei
//目录扫描和漏洞扫描
//口令暴破
use oxc_allocator::Allocator;
use oxc_ast::ast::{ImportDeclaration, StringLiteral};
use oxc_ast_visit::{walk, Visit};
use oxc_parser::Parser;
use oxc_span::SourceType;
use tauri::Manager;

use super::{asm_task::INNERASK_MODULE, WebSite};
use crate::{asm::api::ApiInfo, scan::scanners::XssScanner};
use crate::{
    global::config::CoreConfig,
    handler::{asm::api::collection_api, setting::scan::CRegex},
    internal::html::extract_js_from_html,
};

pub struct RiskScanner {
    all_domain: Vec<String>,
    // api_result: Vec<ApiInfo>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub(crate) struct Risk {
    pub id: Option<i32>,
    pub task_id: Option<i32>,
    pub risk_name: String,
    pub risk_type: String,
    pub risk_desc: String,
    pub risk_level: String,
    pub risk_detail: String,
    pub risk_status: u8,
    pub response: String,
    pub ufrom: String,
    pub update_at: i64,
}

#[derive(Clone, Debug)]
pub struct RiskInfo {
    pub info: String,
    pub ufrom: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, FromRow)]
pub struct RiskTypeInfo {
    pub risk_type: String,
    pub count: i64,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_risks(
    page: i32,
    pagesize: i32,
    dtype: String,
    filter: String,
    query: String,
    risk_status: Vec<String>,
) -> Result<serde_json::Value, String> {
    if page <= 0 || pagesize <= 0 {
        return Err("Invalid page or pagesize".into());
    }

    let allowed_filters = ["task_id", "risk_name", "risk_level", "risk_status"]; // 允许的列名
    if !allowed_filters.contains(&filter.as_str()) {
        return Err("Invalid filter".into());
    }

    let task_module = INNERASK_MODULE
        .get()
        .ok_or("Global variable not initialized")?;
    let tauri_conn = Arc::clone(&task_module.tauri_conn);

    let domain_pattern = format!("%{}%", query);
    let offset = (page - 1) * pagesize;
    let result = match dtype.as_str() {
        "risk_type" => {
            let sql = format!("SELECT risk_type, COUNT(*) as count FROM risk WHERE {} LIKE ?  GROUP BY risk_type ORDER BY count DESC LIMIT ? OFFSET ?", filter);
            let riskinfo: Vec<RiskTypeInfo> = query_as(&sql)
                .persistent(true)
                .bind(&domain_pattern)
                .bind(pagesize as i32)
                .bind(offset as i32)
                .fetch_all(&*tauri_conn)
                .await
                .map_err(|e| {
                    error!("Failed to fetch APIs: {}", e);
                    e.to_string()
                })?;
            let count_sql = format!(
                "SELECT COUNT(DISTINCT risk_type) FROM risk WHERE {} LIKE ?",
                filter
            );
            let total_count: i64 = query_scalar(&count_sql)
                .bind(&domain_pattern)
                .fetch_one(&*tauri_conn)
                .await
                .unwrap_or(0);

            serde_json::json!({
                "list": riskinfo,
                "total": total_count
            })
        }
        _ => {
            let sql = format!(
                "SELECT * FROM risk WHERE {} LIKE ? AND risk_status IN ({}) ORDER BY update_at DESC LIMIT ? OFFSET ? ",
                filter, risk_status.join(",")
            );
            let riskinfo: Vec<Risk> = query_as(&sql)
                .bind(&domain_pattern)
                .bind(pagesize as i32)
                .bind(offset as i32)
                .fetch_all(&*tauri_conn)
                .await
                .map_err(|e| {
                    error!("Failed to fetch APIs: {}", e);
                    e.to_string()
                })?;

            let count_sql = format!(
                "SELECT count(*) FROM risk WHERE {} LIKE ? AND risk_status IN ({})",
                filter,
                risk_status.join(",")
            );
            let total_count: i64 = query_scalar(&count_sql)
                .bind(&domain_pattern)
                .fetch_one(&*tauri_conn)
                .await
                .unwrap_or(0);

            serde_json::json!({
                "list": riskinfo,
                "total": total_count
            })
        }
    };

    Ok(result)
}

pub async fn risk_scan(task_id: &i32) -> Result<(), String> {
    let websites = Arc::new({
        let task_module = INNERASK_MODULE
            .get()
            .ok_or("Global variable not initialized")?;
        let tauri_conn = Arc::clone(&task_module.tauri_conn);
        query_scalar::<_, String>("SELECT url FROM website WHERE task_id = ?")
            .bind(task_id)
            .fetch_all(&*tauri_conn)
            .await
            .unwrap_or(vec![])
    });

    let risk_scanner = Arc::new(Mutex::new(RiskScanner::new()));
    let mut handles = vec![];
    //扫描文件泄露
    let website_clone = websites.clone();
    let task_id_clone = task_id.clone();

    let risk_scanner_clone = risk_scanner.clone();

    info!("开始风险扫描");
    handles.push(tokio::spawn(async move {
        let mut risk_scanner = risk_scanner.lock().await;
        match risk_scanner
            .check_sensitive_info_leakage(&task_id_clone, &website_clone)
            .await
        {
            Ok(_) => (),
            Err(e) => {
                error!("{}", e);
            }
        }
    }));

    //扫描敏感目录信息
    // let website_clone2 = websites.clone();
    // let task_id_clone2 = task_id.clone();

    // handles.push(tokio::spawn(async move {
    //     let risk_scanner = risk_scanner_clone.lock().await;
    //     match risk_scanner
    //         .check_file_leakage(&task_id_clone2, &website_clone2)
    //         .await
    //     {
    //         Ok(_) => (),
    //         Err(e) => {
    //             error!("{}", e);
    //         }
    //     }
    // }));

    futures::future::join_all(handles).await;

    Ok(())
}

impl RiskScanner {
    pub fn new() -> Self {
        let _ = CoreConfig::global().unwrap();

        Self {
            all_domain: vec![],
            // risk_result: vec![],
            // api_result: vec![],
            // subdomain_dict: config.subdomain_dict.clone().unwrap_or(vec![]),
        }
    }
}

impl RiskScanner {
    pub async fn check_file_leakage(
        &self,
        task_id: &i32,
        urls: &Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        let sentive_file = Arc::new(Mutex::new(Vec::new()));
        let mut tasks = vec![];
        let client = match CoreConfig::global().unwrap().http_client.clone() {
            Some(client) => client,
            None => Client::new(),
        };
        let (file_dict, thread_num) = match CoreConfig::global() {
            Ok(ac) => {
                if ac.file_dict.is_some() && ac.thread_num.is_some() {
                    (
                        ac.file_dict.clone().unwrap_or("".to_string()),
                        ac.thread_num.unwrap_or(0),
                    )
                } else {
                    ("".to_string(), 10)
                }
            }
            Err(_) => ("".to_string(), 10),
        };

        let files = match File::open(&file_dict).await {
            Ok(file) => {
                let reader = io::BufReader::new(file);
                let mut files = Vec::new();
                let mut lines = reader.lines();
                while let Some(line) = lines.next_line().await? {
                    files.push(line);
                }
                files
            }
            Err(e) => {
                error!("Error opening file: {}", e);
                vec![]
            }
        };

        let semaphore = Arc::new(Semaphore::new(thread_num as usize));
        for url in urls.iter() {
            for file in files.iter() {
                let url_clone = url.clone() + file.as_str();

                let client_clone = client.clone();
                let sem_clone = semaphore.clone();
                let sentive_file_clone = sentive_file.clone();

                tasks.push(tokio::spawn(async move {
                    let permit = match sem_clone.clone().acquire_owned().await {
                        Ok(permit) => permit,
                        Err(_) => return,
                    };

                    match client_clone.get(url_clone.clone()).send().await {
                        Ok(res) => {
                            let status = res.status();
                            let html = res.text().await.unwrap_or("".to_string());
                            if status.is_success() {
                                let mut sentive_file_clone = sentive_file_clone.lock().await;
                                sentive_file_clone.push(Risk {
                                    id: None,
                                    task_id: None,
                                    risk_name: "敏感信息泄露".to_string(),
                                    risk_type: "文件泄露".to_string(),
                                    risk_desc: "敏感信息泄露".to_string(),
                                    risk_level: "高".to_string(),
                                    risk_detail: "敏感信息泄露".to_string(),
                                    risk_status: 0,
                                    response: html,
                                    ufrom: url_clone.clone(),
                                    update_at: 0,
                                });
                            }
                        }
                        Err(_) => {
                            // error!("Failed to get {}: {}", url, e);
                        }
                    }
                    drop(permit);
                }));
            }
        }
        futures::future::join_all(tasks).await;

        let task_module = match INNERASK_MODULE.get() {
            Some(tm) => tm,
            None => {
                error!("Global variable not initialized");
                return Err("Global variable not initialized".into());
            }
        };
        let write_conn: Arc<SqlitePool> = Arc::clone(&task_module.write_conn);
        let mut tx: sqlx::Transaction<'_, sqlx::Sqlite> = write_conn.begin().await?;
        let sentive_file = sentive_file.lock().await;
        let now = chrono::Local::now().timestamp();
        for sfc in sentive_file.iter() {
            sqlx::query(
                "INSERT INTO risk (task_id,risk_name,risk_type, risk_desc,risk_level,risk_detail,risk_status,ufrom,update_at) VALUES (?, ?,?,?,?,?,?,?,?) ON CONFLICT DO NOTHING",
            )
            .bind(&task_id)
            .bind(&sfc.risk_name)
            .bind(&sfc.risk_type)
            .bind(&sfc.risk_desc)
            .bind(&sfc.risk_level)
            .bind(&sfc.risk_detail)
            .bind(&sfc.risk_status)
            .bind(&sfc.ufrom)
            .bind(&now)
            .execute(&mut *tx) // 在事务中执行
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    // 2. 弱口令检测
    pub fn check_weak_password(&self, _login_url: &str, _username: &str) -> bool {
        // 实现弱口令检测逻辑
        // 返回true表示存在弱口令风险
        false
    }

    // 4. 网页和JS中的敏感信息泄露检测
    pub async fn check_sensitive_info_leakage(
        &mut self,
        task_id: &i32,
        urls: &Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        let (regexes, root_domains) = {
            let task_module = match INNERASK_MODULE.get() {
                Some(tm) => tm,
                None => {
                    error!("Global variable not initialized");
                    return Err("db error".into());
                }
            };
            let write_conn: Arc<SqlitePool> = Arc::clone(&task_module.write_conn);
            //对正则进行预编译
            let root_domains = query_scalar::<_, String>(
                "SELECT domain 
                 FROM rootdomain 
                 WHERE task_id=?",
            )
            .bind(task_id)
            .fetch_all(&*write_conn)
            .await?;


            //对正则进行预编译
            let cregexs = query_as::<_, CRegex>(
                "SELECT * 
                 FROM cregex 
                 WHERE rtype='RISK' AND status=1",
            )
            .fetch_all(&*write_conn)
            .await?;
    
            // 用所有正则模式进行匹配
            let regexes: Vec<(Regex, String)> = cregexs
                .into_iter()
                .map(|pat| (Regex::new(&pat.regex).unwrap(), pat.name))
                .collect();

            (Arc::new(regexes), Arc::new(root_domains))
        };

        let risk_infos: Arc<Mutex<Vec<Risk>>> = Arc::new(Mutex::new(Vec::new()));
        let api_info: Arc<Mutex<Vec<ApiInfo>>> = Arc::new(Mutex::new(Vec::new()));
        let mut tasks = vec![];

        let thread_num = match CoreConfig::global() {
            Ok(ac) => match ac.thread_num {
                Some(num) => num,
                None => 10,
            },
            Err(_) => 10,
        };
        let semaphore = Arc::new(Semaphore::new(thread_num as usize));

        for url in urls.clone() {
            let sem_clone = semaphore.clone();
            let risk_infos_clone = risk_infos.clone();
            let api_info_clone = api_info.clone();
            let root_domains_clone = root_domains.clone();
            let regexes_clone = regexes.clone();
            tasks.push(tokio::spawn(async move {
                let permit = sem_clone.clone().acquire_owned().await.unwrap();
                match Self::crawl_website(url, &root_domains_clone, &regexes_clone).await {
                    Ok(results) => {
                        let mut ric = risk_infos_clone.lock().await;
                        let mut aic = api_info_clone.lock().await;

                        ric.extend(results.0);
                        aic.extend(results.1);
                    }
                    Err(e) => error!("Crawl failed for {}", e),
                };
                drop(permit);
            }));
        }
        futures::future::join_all(tasks).await;

        let api_result: tokio::sync::MutexGuard<'_, Vec<ApiInfo>> = api_info.lock().await;
        //API的信息交由API相关方法处理
        match collection_api(task_id, &*api_result).await {
            Ok(_) => (),
            Err(_) => {
                error!("API 信息入库失败");
            }
        };

        
        //保存匹配的结果
        // let mut sensitive_infos = vec![];
        // let lit_infos = lit_info.lock().await;
        // for si in lit_infos.iter() {
        //     for (regex, name) in &regexes {
        //         if regex.is_match(si.info.as_str()) {
        //             sensitive_infos.push(Risk {
        //                 id: None,
        //                 task_id: Some(*task_id),
        //                 risk_name: name.into(),
        //                 risk_type: "sensitive_info".to_string(),
        //                 risk_desc: "敏感信息泄露".to_string(),
        //                 risk_level: "medium".to_string(),
        //                 risk_detail: si.info.clone(),
        //                 risk_status: 0,
        //                 response: "".to_string(),
        //                 ufrom: si.ufrom.clone(),
        //                 update_at: chrono::Local::now().timestamp(),
        //             });
        //         }
        //     }
        // }
        // println!("sensitive_infos length:{}",sensitive_infos.len());
        let task_module = match INNERASK_MODULE.get() {
            Some(tm) => tm,
            None => return Err("Global variable not initialized".into()),
        };
        let write_conn: Arc<SqlitePool> = Arc::clone(&task_module.write_conn);
        let mut tx = write_conn.begin().await?;
        let risk_infos = risk_infos.lock().await;
        for sis in risk_infos.iter() {
            sqlx::query(
                "INSERT INTO risk (task_id,risk_name,risk_type, risk_desc,risk_level,risk_detail,risk_status,update_at,ufrom) VALUES (?, ?, ?,?,?,?,?,?,?) ON CONFLICT DO NOTHING",
            )
            .bind(&task_id)
            .bind(&sis.risk_name)
            .bind(&sis.risk_type)
            .bind(&sis.risk_desc)
            .bind(&sis.risk_level)
            .bind(&sis.risk_detail)
            .bind(&sis.risk_status)
            .bind(&sis.update_at)
            .bind(&sis.ufrom)
            .execute(&mut *tx) // 在事务中执行
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    //抓取所有的字面量
    async fn crawl_website(
        url: String,
        root_domains: &Vec<String>,
        regexes: &Vec<(Regex, String)>,
    ) -> Result<(Vec<Risk>, Vec<ApiInfo>), String> {
        let client = match CoreConfig::global()?.http_client.clone() {
            Some(client) => client,
            None => Client::new(),
        };
        let proxy = CoreConfig::global()?.proxy.clone();
        let timeout = CoreConfig::global()?.http_timeout.clone();
        let mut website= match Website::new(&url)
        .with_user_agent(Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36"))
        .with_redirect_limit(3)
        .with_limit(100)
        .with_stealth(true)
        .with_chrome_intercept(RequestInterceptConfiguration::new(true))
        .with_full_resources(true)
        .with_request_timeout(Some(Duration::from_secs(timeout.unwrap_or(5))))
        .with_return_page_links(true)
        .with_danger_accept_invalid_certs(true)
        .build(){
            Ok(website) => website,
            Err(e) => return Err(format!("Failed to build website: {}", e)),
        };
        if !proxy.is_none() {
            if let Some(proxy) = proxy {
                website.with_proxies(Some(vec![proxy]));
            };
        }

        let (tx, mut rx) = mpsc::channel(16);
        let mut rx2 = website.subscribe(16).unwrap();

        tokio::spawn(async move {
            while let Ok(page) = rx2.recv().await {
                let _ = tx.send(page.get_url().to_string()).await;
            }
        });

        website.crawl_smart().await;
        website.unsubscribe();

        let mut initial_urls = Vec::new();
        while let Some(url) = rx.recv().await {
            initial_urls.push(url);
        }

        let mut queue = VecDeque::new();
        let mut processed = HashSet::new();

        ////////////////////    把抓到的  link和js 去重后都放到队列中//////////////////////////
        let mut jshash = HashSet::new();
        let links = website.get_all_links_visited().await;
        for link in links {

            jshash.insert(link.to_string());
        }

        //图片，css，视频，音频，文档，压缩包，软件，其他，字体文件
        let static_extensions: HashSet<&str> = [
            "jpg", "jpeg", "png", "gif", "bmp", "ico", "webp", "svg", "css", "ttf", "woff", "woff2", "eot", "otf", // 与静态网页相关的文件
        ]
        .into_iter()
        .collect();

        //过滤掉静态资源文件后缀
        jshash = jshash.into_iter().filter(|url| !static_extensions.contains(url.rsplit('.').next().unwrap())).collect();

        // 初始化队列
        for url in initial_urls {
            jshash.insert(url.to_string());
        }
        //判断jshash中的url是否包含root_domains中的域名，包含则保留，否则删除
        //创建一个新的数组用来保存
        let mut jshash_new = Vec::new();
        let jshash_clone = jshash.clone();
        for url in jshash_clone.iter() {
            if root_domains.iter().any(|domain| url.contains(domain)) {
                jshash_new.push(url.clone());
            }
        }
        for url in jshash {
            queue.push_back(url);
        }
        ////////////////////    把抓到的  link和js 去重后都放到队列中//////////////////////////
        let mut risk_infos = Vec::new();
        let mut api_result = Vec::new();

        let xss_scanner = Arc::new(XssScanner::new(Arc::new(crate::core::config::AppConfig::default())));
        // 处理队列
        while let Some(current_url) = queue.pop_front() {
            if processed.contains(&current_url) {
                continue;
            }
            processed.insert(current_url.clone());


            //如上内容，应该归类到敏感信息泄露中
            if current_url.contains(".pdf") || current_url.contains(".doc") || current_url.contains(".docx") || current_url.contains(".xls") || current_url.contains(".xlsx") || current_url.contains(".ppt") || current_url.contains(".pptx") {
                risk_infos.push(Risk {
                    id: None,
                    task_id: None,
                    risk_name: "敏感信息泄露".to_string(),
                    risk_type: "sensitive_info".to_string(),
                    risk_desc: "敏感信息泄露".to_string(),
                    risk_level: "medium".to_string(),
                    risk_detail: current_url.clone(),
                    risk_status: 0,
                    response: "".to_string(),
                    ufrom: current_url.clone(),
                    update_at: chrono::Local::now().timestamp(),
                });
            }

            // 异步执行XSS扫描
            // let xss_scanner_clone = xss_scanner.clone();
            // let current_url_clone = current_url.clone();
            // tokio::spawn(async move {
            //     let xss_results = xss_scanner_clone.scan_get_url(&vec![current_url_clone.clone()]).await;
            //     if !xss_results.is_empty() {
            //         println!("xss_results:{:?}", xss_results);
            //     }
            // });

            if let Ok(resp) = client.get(current_url.clone()).send().await {
                let source_code: String;
                if current_url.ends_with(".js") {
                    source_code = resp.text().await.unwrap_or("".to_string());
                } else {
                    let html = resp.text().await.unwrap_or("".to_string());
                    source_code = extract_js_from_html(html.as_str());
                }
                let allocator = Allocator::default();
                let source_type = SourceType::from_path("Counter.tsx").unwrap();

                let ret = Parser::new(&allocator, &source_code, source_type).parse();

                let mut visitor = MethodVisitor {
                    root_domains: root_domains.clone(),
                    url_list: HashSet::new(),
                    api_regex: Regex::new(r#"^/[a-zA-Z0-9_-]+(?:/[a-zA-Z0-9_]+)*/?$"#).unwrap(),
                    current_url: &current_url,
                    api_result: vec![],
                    all_domain: vec![],
                    risk_results: Vec::new(),
                    risk_regexs: regexes.clone(),
                    js_paths: Vec::new(),
                };
                visitor.visit_program(&ret.program);

                risk_infos.extend(visitor.risk_results.clone());
                api_result.extend(visitor.api_result.clone());
                // 处理新发现的JS路径
                for js_path in visitor.js_paths {
                    if js_path.ends_with(".js") && !processed.contains(&js_path) {
                        queue.push_back(js_path);
                    }
                }

                if visitor.url_list.len() > 0 {
                    // println!("{:?}", visitor.url_list);
                }
                // let xss_results = xss_scanner.scan_get_url(&visitor.url_list).await;
                // if !xss_results.is_empty() {
                //     println!("xss_results:{:?}", xss_results);
                // }
            }
        }
        Ok((risk_infos, api_result))
    }
}

struct MethodVisitor<'a> {
    root_domains:Vec<String>,
    url_list: HashSet<String>,
    current_url: &'a str,
    risk_results: Vec<Risk>,
    api_result: Vec<ApiInfo>,
    js_paths: Vec<String>,
    all_domain: Vec<String>,
    api_regex: Regex,
    risk_regexs: Vec<(Regex, String)>,
}

impl<'a> MethodVisitor<'a> {
    fn resolve_url(&self, relative: &str) -> Option<String> {
        let base_url = Url::parse(self.current_url).ok()?;
        let resolved_url = base_url.join(relative).ok()?;
        Some(resolved_url.to_string())
    }
}

impl<'a> Visit<'a> for MethodVisitor<'a> {
    fn visit_string_literal(&mut self, s: &StringLiteral<'a>) {
        //正则风险扫描
        for (regex, name) in &self.risk_regexs {
            if regex.is_match(&s.value) {
                self.risk_results.push(Risk {
                    id: None,
                    task_id: None,
                    risk_name: name.clone(),
                    risk_type: "sensitive_info".to_string(),
                    risk_desc: "敏感信息泄露".to_string(),
                    risk_level: "medium".to_string(),
                    risk_detail: s.value.to_string(),
                    ufrom: self.current_url.to_string(),
                    risk_status: 0,
                    response: "".to_string(),
                    update_at: chrono::Local::now().timestamp(),
                });
            }

            //匹配字面量的中API
            if self.api_regex.is_match(&s.value) && s.value.starts_with("/") && s.value.len() > 3 {
                if self.root_domains.iter().any(|domain| self.current_url.to_string().contains(domain)) {
                    self.api_result.push(ApiInfo {
                        id: None,
                        url: "".to_string(),
                        get_response: "".to_string(),
                        post_response: "".to_string(),
                        task_id: None,
                        method: None,
                        uri: s.value.to_string(),
                        ufrom: self.current_url.to_string(),
                        http_status: 0,
                        handle_status: 0,
                        get_body_length: 0,
                        post_body_length: 0,
                        update_at: chrono::Local::now().timestamp(),
                    });
                }
            }
            //提取所有的url
            if let Ok(url) = Url::parse(&s.value) {
                match url.scheme() {
                    "http" | "https" => {
                        //去掉静态资源文件
                        let static_extensions: HashSet<&str> = [
                            "jpg", "jpeg", "png", "gif", "bmp", "ico", "webp", "svg", "css", "ttf", "woff", "woff2", "eot", "otf","js", // 与静态网页相关的文件
                        ]
                        .into_iter()
                        .collect();
                        if !static_extensions.contains(url.to_string().rsplit('.').next().unwrap()) {
                            if self.root_domains.iter().any(|domain| url.to_string().to_lowercase().contains(domain)) {
                                self.url_list.insert(url.to_string());
                            }
                        }


                        //提取可能是ssrf的URL
                        let ssrf_train = vec!["image=","url=","file=","target=","source=","link=","src=","share=","sourceUrl=","imageUrl=","domain=","img=","host=","path=","resourceUrl="];
                        if ssrf_train.iter().any(|train| url.to_string().to_lowercase().contains(train))&& self.root_domains.iter().any(|domain| url.to_string().to_lowercase().contains(domain)) {
                            self.risk_results.push(Risk {
                                id: None,
                                task_id: None,
                                risk_name: "SSRF".to_string(),
                                risk_type: "ssrf".to_string(),
                                risk_desc: "SSRF".to_string(),
                                risk_level: "medium".to_string(),
                                risk_detail: url.to_string(),
                                ufrom: self.current_url.to_string(),
                                risk_status: 0,
                                response: "".to_string(),
                                update_at: chrono::Local::now().timestamp(),
                            });
                        }
                    }   
                    _ => (),
                }
            }
        }
        walk::walk_string_literal(self, s);
    }

    fn visit_import_declaration(&mut self, node: &ImportDeclaration<'a>) {
        let src = node.source.value.to_string();
        if let Some(abs_url) = self.resolve_url(&src) {
            self.js_paths.push(abs_url);
        }
        walk::walk_import_declaration(self, node)
    }
}

// 添加新函数: 使用插件扫描风险
pub async fn scan_risk_by_plugin(task_id: &i32, websites: &Vec<String>) {
    // 获取全局AppHandle
    let app_handle = match crate::APP_HANDLE.get() {
        Some(handle) => handle.clone(),
        None => {
            log::error!("全局APP_HANDLE未初始化");
            return;
        }
    };

    let mut handle_list = Vec::new();

    for site in websites.clone() {
        let tid = task_id.clone();
        let _website = site.clone();
        let app_handle_clone = app_handle.clone();

        let handle = tokio::spawn(async move {
            // 获取ASM插件管理器
            let state = app_handle_clone
                .state::<crate::handler::asm::plugin_commands::AsmPluginManagerState>();
            let manager = state.inner.lock().await;

            // 查找所有风险扫描插件
            let risk_plugins = manager
                .get_all_plugins()
                .await
                .into_iter()
                .filter(|p| p.plugin_type == "risk_scanning")
                .collect::<Vec<_>>();

            // 执行每个风险扫描插件
            for plugin in risk_plugins {
                log::info!("Executing risk scanning plugin: {}", plugin.name);

                // 创建插件上下文
                let context = crate::handler::asm::plugin::AsmPluginContext {
                    task_id: tid,
                    target: site.clone(),
                    targets: Some(vec![site.clone()]),
                    custom_params: None,
                };

                // 执行插件
                match manager
                    .execute_plugin(&plugin.name, "risk_scanning", context)
                    .await
                {
                    Ok(result) => {
                        if result.success {
                            log::info!(
                                "Risk scanning plugin {} executed successfully",
                                plugin.name
                            );

                            // 处理发现的风险
                            if let Some(found_risks) = result.found_risks {
                                if !found_risks.is_empty() {
                                    log::info!(
                                        "Plugin {} found {} risks",
                                        plugin.name,
                                        found_risks.len()
                                    );

                                    // 获取数据库连接
                                    let task_module = match super::asm_task::INNERASK_MODULE.get() {
                                        Some(tm) => tm,
                                        None => {
                                            log::error!("Global variable not initialized");
                                            return;
                                        }
                                    };
                                    let write_conn = Arc::clone(&task_module.write_conn);

                                    // 开始事务
                                    match write_conn.begin().await {
                                        Ok(mut tx) => {
                                            let mut success = true;

                                            for risk_value in found_risks {
                                                if let Some(risk_obj) = risk_value.as_object() {
                                                    // 提取风险信息
                                                    let risk_name = risk_obj
                                                        .get("name")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("未知风险")
                                                        .to_string();

                                                    let risk_type = risk_obj
                                                        .get("type")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("未知类型")
                                                        .to_string();

                                                    let risk_desc = risk_obj
                                                        .get("description")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("无描述")
                                                        .to_string();

                                                    let risk_level = risk_obj
                                                        .get("level")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("low")
                                                        .to_string();

                                                    let risk_detail = risk_obj
                                                        .get("detail")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("无详情")
                                                        .to_string();

                                                    let now_timestamp =
                                                        chrono::Local::now().timestamp();

                                                    // 添加到数据库
                                                    match sqlx::query(
                                                        "INSERT INTO risk (task_id, risk_name, risk_type, risk_desc, risk_level, risk_detail, risk_status, ufrom, update_at) 
                                                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT DO NOTHING"
                                                    )
                                                    .bind(tid)
                                                    .bind(&risk_name)
                                                    .bind(&risk_type)
                                                    .bind(&risk_desc)
                                                    .bind(&risk_level)
                                                    .bind(&risk_detail)
                                                    .bind(1) // 风险状态，1表示有效
                                                    .bind(&site) // 来源
                                                    .bind(now_timestamp)
                                                    .execute(&mut *tx)
                                                    .await {
                                                        Ok(_) => {
                                                            log::info!("Added risk to database: {}", risk_name);
                                                        }
                                                        Err(e) => {
                                                            log::error!("Failed to add risk to database: {}", e);
                                                            success = false;
                                                            break;
                                                        }
                                                    }
                                                }
                                            }

                                            // 提交或回滚事务
                                            if success {
                                                if let Err(e) = tx.commit().await {
                                                    log::error!(
                                                        "Failed to commit risk transaction: {}",
                                                        e
                                                    );
                                                }
                                            } else {
                                                if let Err(e) = tx.rollback().await {
                                                    log::error!(
                                                        "Failed to rollback risk transaction: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            log::error!("Failed to begin transaction: {}", e);
                                        }
                                    }
                                }
                            }
                        } else {
                            log::warn!(
                                "Risk scanning plugin {} execution failed: {}",
                                plugin.name,
                                result.message
                            );
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to execute risk scanning plugin {}: {}",
                            plugin.name,
                            e
                        );
                    }
                }
            }
        });

        handle_list.push(handle);
    }

    for handle in handle_list {
        let _ = tokio::join!(handle);
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn process_risks(risk_status: i32, risk_ids: Vec<u32>) -> Result<String, String> {
    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => return Err("Global variable not initialized".into()),
    };
    let pool_clone = Arc::clone(&task_module.tauri_conn);

    for risk_id in risk_ids {
        sqlx::query("UPDATE risk SET risk_status = ? WHERE id = ?")
            .bind(&risk_status)
            .bind(&risk_id)
            .execute(&*pool_clone)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok("Success".into())
}
