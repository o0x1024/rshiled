
use std::hash::DefaultHasher;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use base64::{Engine as _, engine::general_purpose};
use headless_chrome::protocol::cdp::Page;
use std::hash::{Hash, Hasher};use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use select::document::Document;
use select::predicate::Attr;
use select::predicate::Name;

use rusqlite::{params, Connection};
use tokio::task::JoinHandle;
use trust_dns_resolver::{config::{ResolverConfig, ResolverOpts}, proto::rr::RecordType, TokioAsyncResolver};

use crate::utils;
use super::website::WebSite;
use super::{domain::Domain, ips::IPs};
use log::*;


use openssl::ssl::{Ssl, SslContext, SslMethod};

use std::net::TcpStream;
use headless_chrome::{Browser, LaunchOptions};






pub async  fn resolver_ip(task_id:&isize,result_domain:&Vec<String>){
    let task_id = task_id.clone();
    let mut handle_list = Vec::<JoinHandle<()>>::new();
            for dm in result_domain.clone() {
                handle_list.push(tokio::spawn(async move {
                    let mut ip_list: Vec<IPs> = Vec::<IPs>::new();
                    let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default()).unwrap();
                    let a_records = resolver.lookup_ip(dm.clone()).await;
                    if let Ok(records) = a_records {
                        let now: i64 = chrono::Local::now().timestamp();
                        if records.iter().count() ==  1{
                            if let Some(ip) = records.iter().next() {
                                let ip_string = ip.to_string();
                                ip_list.push(IPs { id: None, enterprise_id: task_id.clone(), ip_addr: Some(ip_string),domain:Some(dm.clone()),port_count:None, create_at: now, update_at: now });
                            } 
                        }
                    }

                    for ip in &ip_list{
                        let db_path = utils::file::get_db_path();
                        let conn = Connection::open(db_path).unwrap();
                        match conn.execute("INSERT INTO IPs (enterprise_id,ip_addr,domain,port_count,create_at, update_at) VALUES (?1, ?2, ?3, ?4,?5,?6)",params![
                            &task_id,
                            ip.ip_addr,
                            ip.domain,
                            0,
                            ip.create_at,
                            ip.update_at,
                            ]){
                        Ok(_) => (),
                        Err(_) => ()
                        }
                    }
                }));
            }

            for handle in  handle_list {
                let _ =tokio::join!(handle);
            }
}



pub async  fn resolver_dns(task_id:&isize,result_domain:&Vec<String>){
    let task_id = task_id.clone();

    let mut handle_list = Vec::<JoinHandle<()>>::new();
            for dm in result_domain.clone() {
                handle_list.push(tokio::spawn(async move {
                    let mut all_domain = Vec::<Domain>::new();
                    let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default()).unwrap();

                    let mut td = Domain {
                        id: None,
                        enterprise_id: task_id,
                        domain: dm.clone().to_string(),
                        aaa: None,
                        cname: None,
                        mx: None,
                        ns: None,
                        create_at: 0,   
                        update_at: 0,
                    };
                    // 查询 A 记录（IPv4 地址）
                    let mut record = Vec::new();
                    let a_records = resolver.lookup_ip(dm.clone()).await;
                    if let Ok(records) = a_records {
                        for ip in records {
                            record.push(ip.to_string());
                        }
                    }
                    if record.len() > 0{
                        td.aaa = Some(record);
                    }
        
                    // 查询 CNAME 记录
                    record = Vec::new();
                    let cname_records = resolver.lookup(dm.clone(), RecordType::CNAME).await;
                    if let Ok(records) = cname_records {
                        for ip in records {
                            record.push(ip.to_string());
                        }
                    }
                    if record.len() > 0{
                        td.cname = Some(record);
                    }
        
                    // 查询 MX 记录（邮件服务器）
                    record = Vec::new();
                    let mx_records = resolver.lookup(dm.clone(), RecordType::MX).await;
                    if let Ok(records) = mx_records {
                        for ip in records {
                            record.push(ip.to_string());
                        }
                    }
                    if record.len() > 0{
                        td.mx = Some(record);
                    }
                
                    // 查询 TXT 记录
                    record = Vec::new();
                    let ns_records = resolver.lookup(dm.clone(), RecordType::NS).await;
                    if let Ok(records) = ns_records {
                        for ip in records {
                            record.push(ip.to_string());
                        }
                    }
                    if record.len() > 0{
                        td.ns = Some(record);
                    }
        
                    all_domain.push(td);
                    // 把拿到的域名写到数据库里
                    let db_path = utils::file::get_db_path();
                    let conn = Connection::open(db_path).unwrap();
                    for domain in &all_domain {
                        let now: i64 = chrono::Local::now().timestamp();
        
                        let aaa_json = domain.aaa.as_ref().map(|v| serde_json::to_string(v).unwrap());
                        let cname_json = domain.cname.as_ref().map(|v| serde_json::to_string(v).unwrap());
                        let ns_json = domain.ns.as_ref().map(|v| serde_json::to_string(v).unwrap());
                        let mx_json = domain.mx.as_ref().map(|v| serde_json::to_string(v).unwrap());
                        match conn.execute("INSERT INTO domain (enterprise_id,domain,aaa,cname,mx,ns,create_at, update_at) VALUES (?1, ?2, ?3, ?4 ,?5 ,?6 ,?7 ,?8)",params![
                            &task_id,
                            domain.domain,
                            aaa_json,    
                            cname_json,  
                            mx_json,    
                            ns_json,    
                            now,    
                            now,    
                            ]){
                        Ok(_) => (),
                        Err(_) => ()
                        }
                    }
                }));
            }

            for handle in  handle_list {
                let _ =tokio::join!(handle);
            }
}



pub async fn fetch_website(task_id:&isize,result_domain:&Vec<String>,http_server:&Vec<String>){
    let website_list = Arc::new(Mutex::new(Vec::<WebSite>::new()));
    let mut handle_list = Vec::<JoinHandle<()>>::new();
    let browser = Arc::new(Browser::new(LaunchOptions::default_builder().headless(true).disable_default_args(true).devtools(false).sandbox(false).enable_logging(false).window_size(Some((1440,1080))).build().unwrap()).unwrap());
    let client = Client::builder()
    .timeout(Duration::from_secs(5)) // 设置超时时间
    .build()
    .unwrap();
    
    for dm in result_domain.iter() {
        let mut dm_clone: String = dm.clone();
        let website_list = Arc::clone(&website_list);
        let browser_clone = browser.clone();
        let client_clone = client.clone();
        dm_clone = "https://".to_string()+ &dm_clone;
        handle_list.push(tokio::spawn(async move {
            match  check_website(&dm_clone,client_clone,&browser_clone).await {
                Some(website) =>{
                    let mut websites = website_list.lock().unwrap();
                    websites.push(website);
                },
                None => ()
            }
        }));
    }
    // Wait for all tasks to complete
    let _ = futures::future::join_all(handle_list).await;

    let db_path = utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();

    for website in website_list.lock().unwrap().iter() {
        let tags_json = serde_json::to_string(&website.tags).unwrap();
        let finger_json = serde_json::to_string(&website.finger).unwrap();

        if let Err(_) = conn.execute(
            "INSERT INTO website (url, enterprise_id, favicon, title, headers, finger, screenshot, tags, ssl_info, create_at, update_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                website.url,
                task_id,
                website.favicon,
                website.title,
                website.headers,
                finger_json,
                website.screenshot,
                tags_json,
                website.ssl_info,
                website.create_at,
                website.update_at,
            ],
        ) {
            // error!("Error inserting website into database: {}", e);
            // eprintln!("Error inserting website into database: {}", e);
        }
    }
}



pub  fn get_screenshot(url:&String,browser:&Browser) -> Result<String, Box<dyn std::error::Error>> {
    // let browser = Browser::new(LaunchOptions::default_builder().headless(true).disable_default_args(true).devtools(false).sandbox(false).enable_logging(false).window_size(Some((1440,1080))).build().unwrap())?;
    
    let tab = browser.new_tab()?;
    tab.navigate_to(url)?.wait_until_navigated()?;
    let png_data = tab.capture_screenshot(Page::CaptureScreenshotFormatOption::Jpeg, Some(45),None, true).unwrap_or("n/a".into());
     
    // 将截图保存为Base64

    let base64_image  = general_purpose::STANDARD.encode(&png_data);
    tab.close(true)?;
    Ok(base64_image)
}


async fn check_website(url: &str,client:Client,browser:&Browser) -> Option<WebSite> {

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    match client.get(url).headers(headers).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let headers_clone = response.headers().clone();
                let body =match  response.text().await.ok(){
                    Some(b) => b,
                    None => "".to_string()
                };
                if body.is_empty(){return None}
                let doc = Document::from(body.as_str());

                let favicon = doc.find(Attr("rel", "icon")).next().and_then(|node| node.attr("href"));
                let mut  favicon_hash = String::new();
                if let Some(fav) = favicon {
                    favicon_hash = mmh3_hash32(fav)
                }
                let title = match doc.find(Name("title")).next(){
                    Some(node) =>{
                        node.text()
                    },
                    None => "".to_string()
                 };
                let headers = headers_to_string(&headers_clone);

                 let screenshot = match get_screenshot(&url.to_string(),browser){
                    Ok(sc) =>sc,
                    Err(err) => {
                        error!("{:?}",err);
                        err.to_string()
                    }
                };
                // let screenshot = get_screenshot(&url.to_string()).unwrap_or("n/a".into());
                let cert_info = get_ssl_info(url).unwrap_or("n/a".into());
                let now: i64 = chrono::Local::now().timestamp();
                Some(WebSite {
                            id: None,
                            enterprise_id: 0,
                            url: url.to_string(),
                            favicon: Some(favicon_hash),
                            title: Some(title),
                            headers: Some(headers),
                            finger: None,
                            screenshot: Some(screenshot),
                            // screenshot: None,
                            tags: None,
                            ssl_info: Some(cert_info),
                            create_at: now,
                            update_at: now,
                        })
            }else{
               None
            }
        }
        Err(_) => {
            None
        },
    }

}




 fn get_ssl_info(url_str: &str) -> Result<String, String> {
    let url = url::Url::parse(url_str).expect("Failed to parse URL");
    // 提取域名和端口
    let domain = url.host_str().expect("Failed to get domain");
    let port = url.port_or_known_default().expect("Failed to get port");
    // 创建 SSL 上下文
    // 创建 SSL 上下文
   let mut ctx = SslContext::builder(SslMethod::tls()).unwrap();
   ctx.set_default_verify_paths().unwrap();
   let ctx = ctx.build();

   // 建立 TCP 连接
   let stream = TcpStream::connect(format!("{}:{}", domain,port)).unwrap();

   // 创建 SSL 连接
   let  ssl = Ssl::new(&ctx).unwrap();
   let  stream = ssl.connect(stream).unwrap();

   // 获取证书
   let cert = stream.ssl().peer_certificate().ok_or("No certificate found")?;

   if let Ok(ct) = cert.to_text(){
       match String::from_utf8(ct){
           Ok(s) => Ok(s),
           Err(err) => {
            error!("{}",err);
            Err("".to_string())
        }
       }
   }else{
       Err("Failed to convert certificate to text".into()) // Convert &'static str to Box<dyn Error>
   }

}



fn mmh3_hash32(data: &str) -> String {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:x}", hasher.finish())
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


#[cfg(test)]
mod test{


    #[tokio::test]
    async fn testssl() -> Result<(), Box<dyn std::error::Error>>{
        // get_screenshot(&"https://www.baidu.com".to_string());

        Ok(())
    }

}