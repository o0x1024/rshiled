use crate::asm::asm_task::INNERASK_MODULE;
use log::{error, info};
use once_cell::sync::Lazy;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, ClientBuilder, Proxy};
use serde::{Deserialize, Serialize};
use sqlx::{query_as, FromRow};
use sqlx::{sqlite::SqliteRow, Row};
use std::{
    sync::RwLock,
    time::Duration,
};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    pub dns_collection_brute_status: bool,
    pub is_buildin: bool,
    pub dns_collection_plugin_status: bool,
    pub port_scan_plugin_status: bool,
    pub fingerprint_plugin_status: bool,
    pub risk_scan_plugin_status: bool,
    pub file_dict: Option<String>,
    pub subdomain_dict: Option<String>,
    pub subdomain_level: Option<u8>,
    #[serde(skip)]
    pub http_client: Option<Client>,
    pub proxy: Option<String>,      // 改为 Option<String>
    pub user_agent: Option<String>, // 改为 Option<String>
    pub http_headers: Option<Vec<(String, String)>>,
    pub http_timeout: Option<u64>,
    pub thread_num: Option<u64>,
}

impl<'r> FromRow<'r, SqliteRow> for CoreConfig {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        let proxy = row.try_get("proxy")?;
        let user_agent = row.try_get("user_agent")?;
        let dns_collection_brute_status: bool = row.try_get::<bool, _>("dns_collection_brute_status")?;
        let is_buildin: bool = row.try_get::<bool, _>("is_buildin")?;
        let dns_collection_plugin_status: bool = row.try_get::<bool, _>("dns_collection_plugin_status")?;
        let port_scan_plugin_status: bool = row.try_get::<bool, _>("port_scan_plugin_status")?;
        let fingerprint_plugin_status: bool = row.try_get::<bool, _>("fingerprint_plugin_status")?;
        let risk_scan_plugin_status: bool = row.try_get::<bool, _>("risk_scan_plugin_status")?;
        let http_headers: Option<Vec<(String, String)>> =
            match row.try_get::<Option<String>, _>("http_headers")? {
                Some(json_str) => serde_json::from_str(&json_str).ok(),
                None => None,
            };
        let http_timeout  = row.try_get("http_timeout")?;
        let thread_num  = row.try_get::<Option<u64>, _>("thread_num")?;
        let file_dict = row.try_get::<Option<String>, _>("file_dict")?;
        let subdomain_dict = row.try_get::<Option<String>, _>("subdomain_dict")?;
        let subdomain_level = row.try_get::<Option<u8>, _>("subdomain_level")?;

        // 返回 Domain 结构体
        Ok(CoreConfig {
            dns_collection_brute_status,
            is_buildin,
            dns_collection_plugin_status,
            port_scan_plugin_status,
            fingerprint_plugin_status,
            risk_scan_plugin_status,
            file_dict,
            subdomain_dict,
            subdomain_level,
            http_client: None,
            proxy,
            user_agent,
            http_headers,
            http_timeout,
            thread_num,
        })
    }
}

//GLOBAL_CONFIG 可以全局访问么
// 将 OnceLock 改为可修改的 RwLock
static GLOBAL_CONFIG: Lazy<RwLock<Option<CoreConfig>>> = Lazy::new(|| RwLock::new(None));

impl CoreConfig {
    pub fn global() -> Result<Self, &'static str> {
        match GLOBAL_CONFIG.read() {
            Ok(guard) => {
                if let Some(config) = guard.as_ref() {
                    Ok(config.clone())
                } else {
                    Err("Config not initialized")
                }
            },
            Err(_) => Err("Failed to acquire read lock")
        }
    }

    pub async fn init() -> Result<(), Box<dyn std::error::Error>> {
        info!("Initializing AppConfig...");

        let task_module = INNERASK_MODULE.get().expect("Global variable not initialized");
        let pool = &*task_module.read_conn;

        let appconfig: CoreConfig = query_as::<_, CoreConfig>(
            r#"
            SELECT 
                dns_collection_brute_status,
                is_buildin,
                dns_collection_plugin_status,
                port_scan_plugin_status,
                fingerprint_plugin_status,
                risk_scan_plugin_status,
                proxy,
                user_agent,
                http_headers,
                http_timeout,
                thread_num,
                file_dict,
                subdomain_dict,
                subdomain_level
            FROM config 
            "#,
        )
        .fetch_one(pool)
        .await?;

        // println!("appconfig: {:?}", appconfig);

        //读取文件字典
        // let path = "file.txt";
        // match File::open(&path) {
        //     Ok(file) => {
        //         let reader = io::BufReader::new(file);
        //         for line in reader.lines() {
        //             if let Ok(line) = line {
        //                 file_dict.push(line);
        //             }
        //         }
        //     }
        //     Err(e) => {
        //         error!("Error opening file: {}", e);
        //     }
        // }

        //读取子域名字典
        // let path = "subdomain.txt";
        // match File::open(&path) {
        //     Ok(file) => {
        //         let reader = io::BufReader::new(file);
        //         for line in reader.lines() {
        //             if let Ok(line) = line {
        //                 subdomain_dict.push(line);
        //             }
        //         }
        //     }
        //     Err(e) => {
        //         println!("Error opening file: {}", e);
        //     }
        // }

        let mut client_builder = ClientBuilder::new();

        
        if !appconfig.proxy.is_none() {
            match appconfig.clone().proxy{
                Some(proxy) => {
                    if proxy != ""{
                        match Proxy::all(proxy) {
                            Ok(p) => {
                                client_builder =  client_builder.proxy(p);
                            }
                            Err(e) => {
                                eprintln!("Invalid proxy URL: {}", e);
                                return Ok(());
                            }
                        };
                    }
                },
                None => {}
            }

            // client_builder =  client_builder.proxy(proxy);
        } 

        if  !appconfig.user_agent.is_none() {
            client_builder = client_builder.user_agent(appconfig.user_agent.as_ref().unwrap());
        }

        if !appconfig.http_headers.is_none() {
            let headers = appconfig.get_headers().unwrap();
            client_builder = client_builder.default_headers(headers);
        }

        if !appconfig.http_timeout.is_none() {
            client_builder = client_builder.timeout(Duration::from_secs(appconfig.http_timeout.unwrap()));
        }

        let client: Client =  client_builder.build()?;


        // let _ = GLOBAL_CONFIG.set(AppConfig { asm_config:myconfig ,file_dict:Some(file_dict),subdomain_dict:Some(subdomain_dict)});
        let complete_config = CoreConfig {
            http_client: Some(client),
            ..appconfig
        };

        if let Ok(mut config) = GLOBAL_CONFIG.write() {
            *config = Some(complete_config);
            info!("AppConfig initialized successfully.");
        } else {
            error!("Failed to acquire write lock for global config");
        }
        
        Ok(())
    }

    pub fn get_headers(&self) -> Option<HeaderMap> {
        self.http_headers.as_ref().map(|headers| {
            let mut header_map = HeaderMap::new();
            for (key, value) in headers {
                if let Ok(header_name) = key.parse::<HeaderName>() {
                    if let Ok(header_value) = value.parse::<HeaderValue>() {
                        header_map.insert(header_name, header_value);
                    }
                }
            }
            header_map
        })
    }

    // 直接更新全局配置，不从数据库读取
    pub fn update_global(config: CoreConfig) -> Result<(), &'static str> {
        let mut updated_config = config.clone();
        
        // 保留现有的文件字典和子域名字典
        if let Ok(current) = Self::global() {
            updated_config.file_dict = current.file_dict.clone();
            updated_config.subdomain_dict = current.subdomain_dict.clone();
        }
        
        // 创建HTTP客户端
        let mut client_builder = ClientBuilder::new();
        
        if let Some(proxy) = &updated_config.proxy {
            if !proxy.is_empty() {
                if let Ok(p) = Proxy::all(proxy) {
                    client_builder = client_builder.proxy(p);
                }
            }
        }
        
        if let Some(ua) = &updated_config.user_agent {
            client_builder = client_builder.user_agent(ua);
        }
        
        if let Some(headers) = &updated_config.http_headers {
            let mut header_map = HeaderMap::new();
            for (key, value) in headers {
                if let Ok(header_name) = key.parse::<HeaderName>() {
                    if let Ok(header_value) = value.parse::<HeaderValue>() {
                        header_map.insert(header_name, header_value);
                    }
                }
            }
            client_builder = client_builder.default_headers(header_map);
        }
        
        if let Some(timeout) = updated_config.http_timeout {
            client_builder = client_builder.timeout(Duration::from_secs(timeout));
        }
        
        // 创建HTTP客户端并更新配置
        match client_builder.build() {
            Ok(client) => {
                updated_config.http_client = Some(client);
                
                // 更新全局配置
                if let Ok(mut config_guard) = GLOBAL_CONFIG.write() {
                    *config_guard = Some(updated_config);
                    Ok(())
                } else {
                    Err("Failed to acquire write lock for global config")
                }
            },
            Err(_) => Err("Failed to build HTTP client"),
        }
    }
}

#[tauri::command]
pub fn get_config(state: State<'_, CoreConfig>) -> CoreConfig {
    state.inner().clone()
}
