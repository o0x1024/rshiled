use crate::asm::enterprise::Enterprise;
use crate::config::config::AppConfig;
use rusqlite::Connection;
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn init_db() {
    println!("Initializing database");

    let home_dir = env::var_os("HOME")
        .and_then(|dir| dir.into_string().ok())
        .unwrap_or_else(|| "/".to_string());

    // 创建 .shiled 目录
    let shield_dir = PathBuf::from(home_dir).join(".rshiled");
    fs::create_dir_all(&shield_dir).expect("Failed to create .rshiled directory");

    // 创建 test.db 文件
    let rshiledb = shield_dir.join("rshiled.db");
    println!("{:?}", rshiledb);

    let conn = Connection::open(rshiledb).unwrap();


    conn.execute(
        "CREATE TABLE IF NOT EXISTS Config (
            dns_collection_brute_status INTEGER NOT NULL DEFAULT 0,
            dns_collection_plugin_status INTEGER NOT NULL DEFAULT 0
        );",
        (), // empty list of parameters.
    )
    .unwrap();


    conn.execute(
        "CREATE TABLE  if not exists enterprise (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            name  TEXT NOT NULL,
			monitor_status INTEGER NOT NULL DEFAULT 1,
            running_status TEXT DEFAULT 'wait',
            next_run_time INTEGER DEFAULT 1693526400,
            last_run_time INTEGER  DEFAULT 1693526400
        )",
        (), // empty list of parameters.
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE  if not exists domain (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,  
            enterprise_id INTEGER,  
            domain  TEXT NOT NULL,
            aaa  TEXT,
			cname TEXT,
            ns TEXT,
            mx TEXT,
            create_at INTEGER,
            update_at INTEGER,
            UNIQUE (domain)
        )",
        (), // empty list of parameters.
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE  if not exists rootdomain (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            domain  TEXT NOT NULL ,
            enterprise_id  INTEGER NOT NULL,
            enterprise_name  TEXT,
			create_at INTEGER,
            update_at INTEGER,
            UNIQUE (domain)
        )",
        (), // empty list of parameters.
    )
    .unwrap();

    // pub id:Option<isize>,
    // pub enterprise_id: isize,
    // pub url:String,     //网站URL
    // pub favicon:Option<String>,   //图标的hash
    // pub title:Option<String>,    //网站的标题
    // pub headers:Option<String>,    //请求响应的头
    // pub finger:Option<Vec<String>>,    //网站指纹
    // pub screenshot:Option<String>,     //网站的截图
    // pub tags:Option<Vec<String>>,
    // pub ssl_info:Option<String>,      //网站证书信息
    // pub create_at: i64,    //创建时间
    // pub update_at: i64,   //最近一个访问或者更新时间


    conn.execute(
        "CREATE TABLE  if not exists port (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            ips_id INTEGER NOT NULL,
            enterprise_id  INTEGER NOT NULL,
            port TEXT,
            create_at INTEGER,                                      
            update_at INTEGER,
        )",
        (), // empty list of parameters.
    )
    .unwrap();



    conn.execute(
        "CREATE TABLE  if not exists website (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            url  TEXT NOT NULL ,
            enterprise_id  INTEGER NOT NULL,
            favicon  TEXT,
			title TEXT,
            headers TEXT,
            finger TEXT,
            screenshot TEXT,
            tags TEXT,
            ssl_info TEXT,
            create_at INTEGER,                                      
            update_at INTEGER,
            UNIQUE (url)
        )",
        (), // empty list of parameters.
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS plugins (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            description TEXT,
            author TEXT,
            plugin_type TEXT NOT NULL,
            input TEXT,
            output TEXT,
            status INTEGER NOT NULL DEFAULT 1, 
            script TEXT NOT NULL,
            create_at INTEGER NOT NULL,         
            update_at INTEGER NOT NULL
        );",
        (), // empty list of parameters.
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE  if not exists IPs (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            enterprise_id  INTEGER NOT NULL,
            ip_addr  TEXT,
            domain TEXT,
            port_count INTEGER,
			create_at INTEGER,
            update_at INTEGER,
            UNIQUE (ip_addr)
        )",
        (), // empty list of parameters.
    )
    .unwrap();

    // CREATE INDEX idx_plugins_name ON plugins(name);
    // CREATE INDEX idx_plugins_type ON plugins(plugin_type);
    // CREATE INDEX idx_plugins_status ON plugins(status);

    let me = Enterprise {
        id: 1,
        name: "芒果TV".to_string(),
        monitor_status: 1,
        running_status: "wait".to_string(),
        next_run_time: 132156464,
        last_run_time: 132156464,
    };

    conn.execute(
        "INSERT INTO Enterprise (id, name,monitor_status,running_status,next_run_time,last_run_time) VALUES (?1, ?2, ?3, ?4,?5 ,?6)",
        (&me.id,&me.name,&me.monitor_status,&me.running_status,&me.next_run_time,&me.last_run_time),
    ).unwrap();



    //插入初始化配置
    let conf = AppConfig {
        dns_collection_brute_status: false,
        dns_collection_plugin_status: false,
    };
    conn.execute(
        "INSERT INTO Config (dns_collection_brute_status, dns_collection_plugin_status) VALUES (?1, ?2)",
        (&conf.dns_collection_brute_status,&conf.dns_collection_plugin_status),
    ).unwrap();
}
