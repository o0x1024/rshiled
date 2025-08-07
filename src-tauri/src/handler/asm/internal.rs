use base64::{engine::general_purpose, Engine as _};
use engine::execute::ClusterType;
use engine::template::cluster::cluster_templates;
use headless_chrome::protocol::cdp::Page;
use once_cell::sync::Lazy;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::Client;
use scraper::{Html, Selector};
use sqlx::{query, query_as, query_scalar, SqlitePool};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;
use tokio::sync::{Mutex, Semaphore};

use tokio::task::JoinHandle;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    proto::rr::RecordType,
    TokioAsyncResolver,
};

use super::asm_task::INNERASK_MODULE;
use super::domain::Domain;
use super::ips::IPs;
use super::website::WebSite;
use crate::global::config::CoreConfig;
use crate::internal::finger::get_teamplate;
use log::*;

use tokio_rustls::{TlsConnector, rustls::{ClientConfig, RootCertStore}};
use rustls_pemfile;
use webpki_roots;
use tokio::net::TcpStream as TokioTcpStream;
use x509_parser::prelude::*;

use headless_chrome::{Browser, LaunchOptions};
use std::net::TcpStream;

static GLOBAL_CLT: Lazy<Mutex<ClusterType>> = Lazy::new(|| Mutex::new(get_cl()));

fn get_cl() -> ClusterType {
    let templates = get_teamplate();
    if templates.is_empty() {
        warn!("unable to find fingerprint, automatically update fingerprint");
    }
    let cl = cluster_templates(&templates);
    cl
}

pub async fn resolver_ip(task_id: &i32) -> Result<(), String> {
    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => {
            error!("Global variable not initialized");
            return Err("Global variable not initialized".into());
        }
    };
    let read_conn = Arc::clone(&task_module.read_conn);

    let domains: Vec<Domain> = query_as("SELECT * FROM domain WHERE task_id = ?")
        .bind(&task_id)
        .fetch_all(&*read_conn)
        .await
        .map_err(|e| e.to_string())?;

    let task_id = task_id.clone();
    let mut handle_list = Vec::<JoinHandle<()>>::new();
    let ip_list: Arc<Mutex<Vec<IPs>>> = Arc::new(Mutex::new(Vec::<IPs>::new()));
    for domain in domains {
        let ip_list_clone = ip_list.clone();
        handle_list.push(tokio::spawn(async move {
            let resolver =
                TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())
                    .unwrap();
            let a_records = resolver.lookup_ip(domain.domain.clone()).await;
            if let Ok(records) = a_records {
                let now: i64 = chrono::Local::now().timestamp();
                if records.iter().count() == 1 {
                    if let Some(ip) = records.iter().next() {
                        let ip_string = ip.to_string();
                        let mut ip_list = ip_list_clone.lock().await;
                        ip_list.push(IPs {
                            id: None,
                            task_id: task_id.clone(),
                            ip_addr: Some(ip_string),
                            domain: None,
                            domain_id: Some(domain.id.unwrap_or_default()),
                            port_count: None,
                            create_at: now,
                            update_at: now,
                        });
                    }
                }
            }
        }));
    }

    for handle in handle_list {
        let _ = tokio::join!(handle);
    }

    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => return Err("Global variable not initialized".into()),
    };
    let write_conn: Arc<SqlitePool> = Arc::clone(&task_module.write_conn);
    let mut tx = write_conn.begin().await.map_err(|e| e.to_string())?;
    for ip in ip_list.lock().await.iter() {
        sqlx::query(
            "INSERT INTO ips (task_id,ip_addr,domain_id,port_count,create_at, update_at) VALUES (?1, ?2, ?3, ?4,?5,?6) ON CONFLICT DO NOTHING"
        )
        .bind(&task_id)
        .bind(&ip.ip_addr)
        .bind(&ip.domain_id)
        .bind(0)
        .bind(&ip.create_at)
        .bind(&ip.update_at)
        .execute(&mut *tx) // 在事务中执行
        .await.map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn resolver_dns(task_id: &i32, result_domain: &Vec<String>) -> Result<(), String> {
    let task_id = task_id.clone();
    let all_domain = Arc::new(Mutex::new(Vec::<Domain>::new()));

    // 创建一个信号量来控制并发数量，这里设置为 50 个并发
    let semaphore = Arc::new(Semaphore::new(50));
    let mut handle_list = Vec::<JoinHandle<()>>::new();

    for dm in result_domain.clone() {
        let all_domain_clone = all_domain.clone();
        let semaphore_clone = semaphore.clone();

        handle_list.push(tokio::spawn(async move {
            // 获取信号量许可
            let _permit = semaphore_clone.acquire().await.unwrap();

            let resolver =
                TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())
                    .unwrap();

            let mut td = Domain {
                id: None,
                task_id: task_id,
                ufrom: None,
                domain: dm.clone(),
                aaa: None,
                cname: None,
                mx: None,
                ns: None,
                create_at: 0,
                update_at: 0,
            };

            // 并发执行所有 DNS 查询
            let (a_records, cname_records, mx_records, ns_records) = tokio::join!(
                resolver.lookup_ip(dm.clone()),
                resolver.lookup(dm.clone(), RecordType::CNAME),
                resolver.lookup(dm.clone(), RecordType::MX),
                resolver.lookup(dm.clone(), RecordType::NS)
            );

            // 处理 A 记录
            if let Ok(records) = a_records {
                let record: Vec<String> = records.iter().map(|ip| ip.to_string()).collect();
                if !record.is_empty() {
                    td.aaa = Some(record.join(","));
                }
            }

            // 处理 CNAME 记录
            if let Ok(records) = cname_records {
                let record: Vec<String> = records.iter().map(|r| r.to_string()).collect();
                if !record.is_empty() {
                    td.cname = Some(record.join(","));
                }
            }

            // 处理 MX 记录
            if let Ok(records) = mx_records {
                let record: Vec<String> = records.iter().map(|r| r.to_string()).collect();
                if !record.is_empty() {
                    td.mx = Some(record.join(","));
                }
            }

            // 处理 NS 记录
            if let Ok(records) = ns_records {
                let record: Vec<String> = records.iter().map(|r| r.to_string()).collect();
                if !record.is_empty() {
                    td.ns = Some(record.join(","));
                }
            }

            let mut all_domain_clone = all_domain_clone.lock().await;
            all_domain_clone.push(td);
        }));
    }

    // 等待所有任务完成
    for handle in handle_list {
        let _ = handle.await;
    }

    // 批量写入数据库
    let task_module = INNERASK_MODULE
        .get()
        .expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.write_conn);

    let mut tx = pool_clone.begin().await.map_err(|e| e.to_string())?;
    let now: i64 = chrono::Local::now().timestamp();

    for domain in all_domain.lock().await.iter() {
        let aaa_json = domain
            .aaa
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap());
        let cname_json = domain
            .cname
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap());
        let ns_json = domain
            .ns
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap());
        let mx_json = domain
            .mx
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap());

        if let Err(e) = query(
            "INSERT INTO domain (task_id,domain,aaa,cname,mx,ns,create_at,update_at,ufrom) 
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        )
        .bind(&task_id)
        .bind(&domain.domain)
        .bind(&aaa_json)
        .bind(&cname_json)
        .bind(&mx_json)
        .bind(&ns_json)
        .bind(&now)
        .bind(&now)
        .bind(&domain.ufrom)
        .execute(&mut *tx)
        .await
        {
            // warn!("Failed to insert domain {}: {}", domain.domain, e);
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn fetch_finger(task_id: &i32, target_domains: &Vec<String>) {
    let cl = { GLOBAL_CLT.lock().await.clone() };
    match observer_ward::cmd::finger_run_batch(&target_domains, &cl) {
        Some(finger) => {
            let task_module = INNERASK_MODULE
                .get()
                .expect("Global variable not initialized");
            let pool_clone = Arc::clone(&task_module.write_conn);

            // 收集所有待插入的数据
            let mut webcomp_data = Vec::new();
            let now = chrono::Local::now().timestamp();

            // 解析指纹数据并收集到一个集合中
            for mr in finger {
                for (k, mf) in &mr {
                    for comp in mf.names() {
                        webcomp_data.push((k.clone(), comp.clone()));
                    }
                }
            }

            if webcomp_data.is_empty() {
                return;
            }

            // 开始批量插入处理
            let batch_size = 100; // 每批100条数据

            // 开启事务
            match pool_clone.begin().await {
                Ok(mut tx) => {
                    let mut success = true;

                    // 分批处理数据
                    for chunk in webcomp_data.chunks(batch_size) {
                        // 使用SQL批量插入
                        let mut query_builder = sqlx::QueryBuilder::new(
                            "INSERT INTO webcomp (task_id, website, comp_name, update_at) ",
                        );

                        // 添加所有值
                        query_builder.push_values(chunk, |mut b, (website, comp_name)| {
                            b.push_bind(task_id)
                                .push_bind(website)
                                .push_bind(comp_name)
                                .push_bind(now);
                        });

                        // 添加冲突处理
                        query_builder.push(" ON CONFLICT DO NOTHING");

                        // 执行批量插入
                        match query_builder.build().execute(&mut *tx).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("Failed to insert web components batch: {}", e);
                                success = false;
                                break;
                            }
                        }
                    }

                    // 提交或回滚事务
                    if success {
                        if let Err(e) = tx.commit().await {
                            error!("Failed to commit web components transaction: {}", e);
                        } else {
                            debug!(
                                "Successfully processed {} web components",
                                webcomp_data.len()
                            );
                        }
                    } else {
                        if let Err(e) = tx.rollback().await {
                            error!("Failed to rollback web components transaction: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to begin transaction: {}", e);
                }
            }
        }
        None => {
            debug!("No fingerprint data found");
        }
    };
}

pub async fn fetch_website(
    task_id: &i32,
    _http_service_list: &Vec<String>,
    root_domains: &Vec<String>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let result_domain = {
        let task_module = match INNERASK_MODULE.get() {
            Some(tm) => tm,
            None => {
                error!("Global variable not initialized");
                return Err("Global variable not initialized".into());
            }
        };
        let read_conn = Arc::clone(&task_module.read_conn);
        let result_domain =
            query_scalar::<_, String>("SELECT domain FROM domain WHERE task_id = ?")
                .bind(task_id)
                .fetch_all(&*read_conn)
                .await
                .unwrap();
        result_domain
    };

    let website_list = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut handle_list = Vec::<JoinHandle<()>>::new();
    let browser = Arc::new(Browser::new(
        LaunchOptions::default_builder()
            .headless(true)
            .disable_default_args(true)
            .devtools(false)
            .sandbox(false)
            .enable_logging(false)
            .window_size(Some((1440, 1080)))
            .ignore_certificate_errors(true)
            .build()?,
    )?);
    let client = match CoreConfig::global()?.http_client.clone() {
        Some(client) => client,
        None => Client::builder().timeout(Duration::from_secs(5)).build().unwrap(),
    };
    let urls = Arc::new({
        let task_module = INNERASK_MODULE
            .get()
            .expect("Global variable not initialized");
        let pool_clone = Arc::clone(&task_module.write_conn);
        query_scalar::<_, String>("SELECT url FROM website WHERE task_id = ?")
            .bind(&task_id)
            .fetch_all(&*pool_clone)
            .await?
    });
    let thread_num = match CoreConfig::global() {
        Ok(ac) => match ac.thread_num {
            Some(num) => num,
            None => 10,
        },
        Err(_) => 10,
    };
    let sem = Arc::new(Semaphore::new(thread_num as usize));

    // 批量处理队列
    let websites_to_update = Arc::new(Mutex::new(Vec::new()));
    let websites_to_insert = Arc::new(Mutex::new(Vec::new()));
    let batch_size = 50; // 批处理大小

    // 输出耗时
    for target in result_domain {
        let urls_clone = Arc::clone(&urls);
        let website_list_clone = Arc::clone(&website_list);
        let websites_to_update_clone = Arc::clone(&websites_to_update);
        let websites_to_insert_clone = Arc::clone(&websites_to_insert);
        let browser_clone = Arc::clone(&browser);
        let client_clone = client.clone();
        let tar = target.clone();
        let task_id_clone = task_id.clone();
        let sem = sem.clone();
        let root_domains_clone = root_domains.clone();
        handle_list.push(tokio::spawn(async move {
            match check_website(
                &sem,
                &tar,
                task_id_clone,
                client_clone,
                &browser_clone,
                &root_domains_clone,
            )
            .await
            {
                Some(websites) => {
                    let now: i64 = chrono::Local::now().timestamp();
                    for mut website in websites {
                        website_list_clone.lock().await.push(website.url.clone());
                        website.update_at = now; // 确保时间戳一致

                        if urls_clone.contains(&website.url) {
                            // 添加到更新队列
                            websites_to_update_clone.lock().await.push(website);
                        } else {
                            // 添加到插入队列
                            websites_to_insert_clone.lock().await.push(website);
                        }
                    }
                }
                None => (),
            }
        }));
    }

    // 等待所有任务完成
    for handle in handle_list {
        let _ = tokio::join!(handle);
    }

    // 批量处理数据库操作
    let task_module = INNERASK_MODULE
        .get()
        .expect("Global variable not initialized");
    let pool_clone = Arc::clone(&task_module.write_conn);

    // 获取锁定的数据
    let update_websites = websites_to_update.lock().await.to_vec();
    let insert_websites = websites_to_insert.lock().await.to_vec();

    // 批量处理更新
    if !update_websites.is_empty() {
        // 使用事务处理更新
        let mut tx = pool_clone.begin().await?;

        for chunk in update_websites.chunks(batch_size) {
            for website in chunk {
                query("UPDATE website SET favicon = ?, title = ?, headers = ?, ssl_info = ?, update_at = ?, screenshot = ?, status_code = ? WHERE url = ?")
                    .bind(&website.favicon)
                    .bind(&website.title)
                    .bind(&website.headers)
                    .bind(&website.ssl_info)
                    .bind(&website.update_at)
                    .bind(&website.screenshot)
                    .bind(&website.status_code)
                    .bind(&website.url)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        error!("Error updating website: {:?}", e);
                        e
                    })?;
            }
        }

        // 提交事务
        tx.commit().await?;
    }

    // 批量处理插入
    if !insert_websites.is_empty() {
        // 使用事务处理插入
        let mut tx = pool_clone.begin().await?;

        for chunk in insert_websites.chunks(batch_size) {
            for website in chunk {
                query("INSERT INTO website (url, task_id, favicon, title, status_code, headers, screenshot, tags, ssl_info, create_at, update_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT DO NOTHING")
                    .bind(&website.url)
                    .bind(task_id)
                    .bind(&website.favicon)
                    .bind(&website.title)
                    .bind(&website.status_code)
                    .bind(&website.headers)
                    .bind(&website.screenshot)
                    .bind("")
                    .bind(&website.ssl_info)
                    .bind(&website.create_at)
                    .bind(&website.update_at)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        error!("Error inserting website: {:?}", e);
                        e
                    })?;
            }
        }

        // 提交事务
        tx.commit().await?;
    }

    // 返回 website_list 的内容
    let result = website_list.lock().await.to_vec();
    Ok(result)
}

pub fn get_screenshot(
    url: &String,
    browser: &Browser,
) -> Result<String, Box<dyn std::error::Error>> {
    let tab = browser.new_tab()?;
    tab.navigate_to(url)?.wait_until_navigated()?;
    let png_data = tab.capture_screenshot(
        Page::CaptureScreenshotFormatOption::Jpeg,
        Some(45),
        None,
        true,
    )?;

    // 将截图保存为Base64

    let base64_image = general_purpose::STANDARD.encode(&png_data);
    tab.close(true)?;
    Ok(base64_image)
}

async fn check_website(
    sem: &Semaphore,
    domain: &String,
    task_id: i32,
    client: Client,
    browser: &Browser,
    root_domains: &Vec<String>,
) -> Option<Vec<WebSite>> {
    let permit = sem.acquire().await.unwrap();

    let now: i64 = chrono::Local::now().timestamp();
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let mut wss = Vec::<WebSite>::new();

    let http_url = format!("http://{}/", domain);
    let https_url = format!("https://{}/", domain);

    let http_response = match client
        .get(http_url.clone())
        .headers(headers.clone())
        .send()
        .await
    {
        Ok(res) => Some(res),
        Err(_) => None,
    };

    let https_response = match client.get(https_url.clone()).headers(headers).send().await {
        Ok(res) => Some(res),
        Err(_) => None,
    };

    let (http_res, https_res) = match (http_response, https_response) {
        (Some(http_res), Some(https_res)) => {
            if http_res.content_length() == https_res.content_length() {
                (None, Some(https_res))
            } else {
                (Some(http_res), Some(https_res))
            }
        }
        (None, Some(https_res)) => (None, Some(https_res)),
        (Some(http_res), None) => (Some(http_res), None),
        (None, None) => (None, None),
    };

    if let Some(response) = http_res {
        if response.status().is_success() {
            let mut ws = WebSite::new();
            ws.task_id = task_id;
            ws.url = response.url().to_string();
            ws.create_at = now;
            ws.update_at = now;
            ws.status_code = Some(response.status().as_u16() as i32);
            let headers_clone = response.headers().clone();
            let html_content = response.text().await.unwrap_or_default();
            
            // 提取标题
            let title = {
                let document = Html::parse_document(&html_content);
                let title_selector = Selector::parse("title").unwrap();
                if let Some(title_element) = document.select(&title_selector).next() {
                    let title = title_element.text().collect::<String>();
                    Some(title.trim().to_string())
                } else {
                    None
                }
            }; // document 在这里被自动 drop
            
            let headers = headers_to_string(&headers_clone);
            let cert_info = get_ssl_info(&task_id, http_url.as_str(),&root_domains).await.unwrap_or("".into());
            
            ws.title = title;
            ws.headers = Some(headers);
            ws.ssl_info = Some(cert_info);
            ws.screenshot = match get_screenshot(&http_url, browser) {
                Ok(sc) => Some(sc),
                Err(err) => {
                    // error!("{:?}", err);
                    println!("{:?}", err);
                    None
                }
            };
            wss.push(ws);
        }
    }

    if let Some(response) = https_res {
        if response.status().is_success() {
            let mut ws = WebSite::new();
            ws.task_id = task_id;
            ws.url = response.url().to_string();
            ws.create_at = now;
            ws.update_at = now;
            ws.status_code = Some(response.status().as_u16() as i32);
            let headers_clone = response.headers().clone();
            let headers = headers_to_string(&headers_clone);
            
            let html_content = response.text().await.unwrap_or_default();
            
            // 提取标题
            let title = {
                let document = Html::parse_document(&html_content);
                let title_selector = Selector::parse("title").unwrap();
                if let Some(title_element) = document.select(&title_selector).next() {
                    let title = title_element.text().collect::<String>();
                    Some(title.trim().to_string())
                } else {
                    None
                }
            }; // document 在这里被自动 drop
            
            let cert_info = get_ssl_info(&task_id, https_url.as_str(),&root_domains).await.unwrap_or("".into());

            ws.title = title;
            ws.headers = Some(headers);
            ws.ssl_info = Some(cert_info);
            ws.screenshot = match get_screenshot(&https_url, browser) {
                Ok(sc) => Some(sc),
                Err(_) => {
                    None
                }
            };
            wss.push(ws);
        }
    }
    drop(permit);
    Some(wss)
}

async fn get_ssl_info(
    task_id: &i32,
    url_str: &str,
    root_domains: &Vec<String>,
) -> Result<String, Box<dyn Error>> {
    if !url_str.contains("https://") {
        return Err("URL 必须以 'https://' 开头".into());
    }
    let url = url::Url::parse(url_str).expect("Failed to parse URL");
    // 提取域名和端口
    let domain = url.host_str().expect("Failed to get domain");
    let port = url.port_or_known_default().expect("Failed to get port");

    // 创建 rustls 配置
    let mut root_cert_store = tokio_rustls::rustls::RootCertStore::empty();
    for cert in webpki_roots::TLS_SERVER_ROOTS {
        root_cert_store.add(&tokio_rustls::rustls::Certificate(cert.subject_public_key_info.to_vec())).unwrap();
    }
    
    let config = tokio_rustls::rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();
    
    let connector = TlsConnector::from(Arc::new(config));
    let domain_name = tokio_rustls::rustls::ServerName::try_from(domain)
        .map_err(|_| "Invalid domain name")?;

    // 建立 TCP 连接
    let stream = tokio::net::TcpStream::connect(format!("{}:{}", domain, port)).await?;
    
    // 创建 TLS 连接
    let tls_stream = connector.connect(domain_name, stream).await?;
    
    // 获取证书链
    let (_, connection) = tls_stream.into_inner();
    let peer_certificates = connection.peer_certificates();
    
    if let Some(cert_der) = peer_certificates.and_then(|certs| certs.first()) {
        // 解析证书
        let (_, cert) = X509Certificate::from_der(cert_der.as_ref())
            .map_err(|e| format!("Failed to parse certificate: {}", e))?;
        
        // 提取证书中的域名并存储到数据库
        let cert_domains = extract_domains_from_cert_rustls(&cert);
        if !cert_domains.is_empty() {
            let task_id_clone = task_id.clone();
            let root_domains_clone = root_domains.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    save_cert_domains_to_db(&task_id_clone, &cert_domains, &root_domains_clone).await
                {
                    error!("Failed to save certificate domains to DB: {}", e);
                }
            });
        }
        
        // 返回证书的文本表示
        Ok(format!("{:?}", cert))
    } else {
        Err("No certificate found".into())
    }
}

// 提取证书中的所有域名 (使用 x509_parser)
fn extract_domains_from_cert_rustls(cert: &X509Certificate) -> Vec<String> {
    let mut domains = Vec::new();

    // 尝试从主题中获取 Common Name
    let subject = cert.subject();
    for rdn in subject.iter() {
        for attr in rdn.iter() {
            if let Ok(cn) = attr.attr_value().as_str() {
                if !domains.contains(&cn.to_string()) {
                    domains.push(cn.to_string());
                }
            }
        }
    }

    // 尝试从 Subject Alternative Names 扩展中获取域名
    let extensions = cert.extensions();
    for ext in extensions {
        if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
            for name in &san.general_names {
                if let GeneralName::DNSName(dns_name) = name {
                    let domain = dns_name.to_string();
                    if !domains.contains(&domain) {
                        domains.push(domain);
                    }
                }
            }
        }
    }

    domains
}

// 将从证书中提取的域名保存到数据库
async fn save_cert_domains_to_db(
    task_id: &i32,
    domains: &[String],
    root_domains: &Vec<String>,
) -> Result<(), Box<dyn Error>> {
    if domains.is_empty() {
        return Ok(());
    }

    let task_module = INNERASK_MODULE
        .get()
        .ok_or("Global variable not initialized")?;
    let write_conn = Arc::clone(&task_module.write_conn);

    // 开启事务
    let mut tx = write_conn.begin().await?;
    let now = chrono::Local::now().timestamp();

    let mut other_domain = vec![];
    for domain in domains {
        // 跳过IP地址格式的域名
        if domain.parse::<std::net::IpAddr>().is_ok() {
            continue;
        }

        // 跳过通配符域名，或者也可以选择处理它们（去掉*.)
        let domain_to_insert = if domain.starts_with("*.") {
            domain[2..].to_string()
        } else {
            domain.clone()
        };

        // 检查是否为有效域名格式
        if !is_valid_domain_format(&domain_to_insert) {
            continue;
        }

        // 检查是否在rootdomain范围内
        let is_subdomain_of_root =
            match check_domain_in_rootdomains(&domain_to_insert, &root_domains).await {
                Ok(result) => result,
                Err(e) => {
                    log::warn!("Failed to check if domain is within rootdomain: {}", e);
                    false
                }
            };

        if !is_subdomain_of_root {
            other_domain.push(domain_to_insert);
            continue; // 不在根域名范围内，跳过
        }

        sqlx::query(
            "INSERT INTO domain (task_id, domain, ufrom, create_at, update_at) 
             VALUES (?, ?, ?, ?, ?) 
             ON CONFLICT(domain) DO NOTHING",
        )
        .bind(task_id)
        .bind(&domain_to_insert)
        .bind("ssl_cert") // 来源标记为证书
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }

    // 提交事务
    tx.commit().await?;
    if !other_domain.is_empty() {
        info!("other_domain: {:?}", other_domain);
    }
    Ok(())
}

// 检查是否为有效域名格式的辅助函数
fn is_valid_domain_format(domain: &str) -> bool {
    // 域名基本格式验证：不含空格，有至少一个点，不以点开头或结尾
    if domain.contains(' ')
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
    {
        return false;
    }

    // 检查是否只包含合法字符（字母、数字、连字符、点）
    let is_valid_chars = domain
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');

    // 检查每个部分的长度和格式
    let parts: Vec<&str> = domain.split('.').collect();
    let is_valid_parts = parts.iter().all(|part| {
        !part.is_empty() && part.len() <= 63 && !part.starts_with('-') && !part.ends_with('-')
    });

    is_valid_chars && is_valid_parts
}

// 检查域名是否在rootdomain范围内的异步函数
async fn check_domain_in_rootdomains<'a>(
    domain: &str,
    root_domains: &Vec<String>,
) -> Result<bool, sqlx::Error> {
    // 获取该任务下所有的根域名

    if root_domains.is_empty() {
        // 如果没有根域名记录，保守起见返回true（或者根据业务需求调整为false）
        return Ok(true);
    }

    // 检查域名是否为根域名的子域名
    for root_domain in root_domains {
        if domain == root_domain || domain.ends_with(&format!(".{}", root_domain)) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn headers_to_string(headers: &reqwest::header::HeaderMap) -> String {
    let mut result = String::new();

    // 遍历每个头部字段
    for (key, value) in headers {
        // 将键和值转换为字符串
        let key_str = key.as_str();
        let value_str = value.to_str().unwrap_or("<invalid UTF-8>");

        // 格式化并添加到结果字符串中
        result.push_str(&format!("{}: {}\n", key_str, value_str));
    }

    result
}

pub async fn fetch_domain_by_plugin(task_id: &i32, result_domains: Vec<String>) {
    let mut handle_list = Vec::new();

    // 获取全局AppHandle
    let app_handle = match crate::APP_HANDLE.get() {
        Some(handle) => handle.clone(),
        None => {
            log::error!("全局APP_HANDLE未初始化");
            return;
        }
    };

    // 创建共享的域名收集器，避免每个域名都单独创建事务
    let domain_collector = Arc::new(Mutex::new(Vec::<(String, String)>::new())); // (域名, 插件名)
    let batch_size = 50; // 批处理大小

    for domain in result_domains {
        let tid = task_id.clone();
        let root_domain = domain.clone();
        let app_handle_clone = app_handle.clone();
        let domain_collector_clone = Arc::clone(&domain_collector);

        let handle = tokio::spawn(async move {
            // 获取ASM插件管理器
            let state = app_handle_clone
                .state::<crate::handler::asm::plugin_commands::AsmPluginManagerState>();
            let manager = state.inner.lock().await;

            // 查找所有域名发现插件
            let domain_plugins = manager
                .get_all_plugins()
                .await
                .into_iter()
                .filter(|p| p.plugin_type == "domain_discovery")
                .collect::<Vec<_>>();

            // 执行每个域名发现插件
            for plugin in domain_plugins {
                log::info!("Executing domain discovery plugin: {}", plugin.name);

                // 创建插件上下文
                let context = crate::handler::asm::plugin::AsmPluginContext {
                    task_id: tid,
                    target: root_domain.clone(),
                    targets: Some(vec![root_domain.clone()]),
                    custom_params: None,
                };

                // 执行插件
                match manager
                    .execute_plugin(&plugin.name, "domain_discovery", context)
                    .await
                {
                    Ok(result) => {
                        if result.success {
                            log::info!(
                                "Domain discovery plugin {} executed successfully",
                                plugin.name
                            );

                            // 处理发现的域名
                            if let Some(found_domains) = result.found_domains {
                                if !found_domains.is_empty() {
                                    log::info!(
                                        "Plugin {} found {} domains",
                                        plugin.name,
                                        found_domains.len()
                                    );

                                    // 将发现的域名添加到收集器中，而不是立即写入数据库
                                    let mut collector = domain_collector_clone.lock().await;
                                    for found_domain in found_domains {
                                        collector.push((found_domain, plugin.name.clone()));
                                    }

                                    // 如果收集的域名数量超过批处理大小，则立即处理一批
                                    if collector.len() >= batch_size {
                                        let domains_to_process =
                                            collector.drain(..).collect::<Vec<_>>();
                                        drop(collector); // 释放锁，让其他任务可以继续添加

                                        // 处理批量域名
                                        if let Err(e) =
                                            process_collected_domains(tid, domains_to_process).await
                                        {
                                            log::error!("Failed to process domain batch: {}", e);
                                        }
                                    }
                                }
                            }
                        } else {
                            log::warn!(
                                "Domain discovery plugin {} execution failed: {}",
                                plugin.name,
                                result.message
                            );
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to execute domain discovery plugin {}: {}",
                            plugin.name,
                            e
                        );
                    }
                }
            }
        });

        handle_list.push(handle);
    }

    // 等待所有异步任务完成
    for handle in handle_list {
        let _ = tokio::join!(handle);
    }

    // 处理剩余的域名
    let remaining_domains = domain_collector.lock().await.drain(..).collect::<Vec<_>>();
    if !remaining_domains.is_empty() {
        if let Err(e) = process_collected_domains(*task_id, remaining_domains).await {
            log::error!("Failed to process remaining domains: {}", e);
        }
    }
}

// 批量处理收集到的域名
async fn process_collected_domains(
    task_id: i32,
    domains: Vec<(String, String)>,
) -> Result<(), Box<dyn std::error::Error>> {
    if domains.is_empty() {
        return Ok(());
    }

    // 获取数据库连接
    let task_module = super::asm_task::INNERASK_MODULE
        .get()
        .ok_or("Global variable not initialized")?;
    let write_conn = Arc::clone(&task_module.write_conn);

    // 开始事务
    let mut tx = write_conn.begin().await?;
    let now = chrono::Local::now().timestamp();

    // 预处理唯一域名，避免重复
    let mut unique_domains = std::collections::HashMap::new();
    for (domain, plugin_name) in domains {
        unique_domains.insert(domain.clone(), plugin_name);
    }

    // 批量构建插入语句 - 正确的方式
    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO domain (task_id, domain, ufrom, create_at, update_at) ", // 注意这里不包含VALUES
    );

    // 添加VALUES部分
    query_builder.push_values(unique_domains.iter(), |mut b, (domain, plugin_name)| {
        b.push_bind(task_id)
            .push_bind(domain)
            .push_bind(plugin_name)
            .push_bind(now)
            .push_bind(now);
    });

    // 添加ON CONFLICT子句
    query_builder.push(" ON CONFLICT(domain) DO NOTHING");

    // 执行查询
    query_builder.build().execute(&mut *tx).await?;

    // 提交事务
    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod test {

    #[tokio::test]
    async fn testssl() -> Result<(), Box<dyn std::error::Error>> {
        // get_screenshot(&"https://www.baidu.com".to_string());

        Ok(())
    }
}
