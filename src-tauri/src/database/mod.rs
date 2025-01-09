

use rusqlite::Connection;
use std::env;
use std::fs;
use std::path::PathBuf;
use crate::asm::enterprise::Enterprise;
use crate::config::config::AppConfig;


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
        "CREATE TABLE  if not exists Enterprise (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            name  TEXT NOT NULL,
            icp_no  TEXT,
			monitor_status INTEGER NOT NULL DEFAULT 1,,
            next_runtime INT,
            running_status INT
        )",
        (), // empty list of parameters.
    )
    .unwrap();

    conn.execute(
        "CREATE TABLE  if not exists Domain (
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
        "CREATE TABLE  if not exists RootDomain (
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
        "CREATE TABLE IF NOT EXISTS Config (
            dns_collection_brute_status INTEGER NOT NULL DEFAULT 0,
            dns_collection_plugin_statuINTEGER NOT NULL DEFAULT 0,
        );",
        (), // empty list of parameters.
    )
    .unwrap();

    // CREATE INDEX idx_plugins_name ON plugins(name);
    // CREATE INDEX idx_plugins_type ON plugins(plugin_type);
    // CREATE INDEX idx_plugins_status ON plugins(status);

    let me = Enterprise {
        id: 1,
        name: "芒果TV".to_string(),
        icp_no: "2516465456456".to_string(),
        monitor_status: true,
        next_runtime: 132156464,
        running_status: 0,
    };

    conn.execute(
        "INSERT INTO Enterprise (id, name,icp_no,monitor_status,next_runtime,running_status) VALUES (?1, ?2, ?3, ?4,?5,?6)",
        (&me.id,&me.name,&me.icp_no,&me.monitor_status,&me.next_runtime,&me.running_status),
    ).unwrap();


    //插入初始化配置
    let conf = AppConfig{
        dns_collection_brute_status:false,
        dns_collection_plugin_status:false,
    };
    conn.execute(
        "INSERT INTO Config (dns_collection_brute_status, dns_collection_plugin_status,) VALUES (?1, ?2)",
        (&conf.dns_collection_brute_status,&conf.dns_collection_plugin_status),
    ).unwrap();


}
