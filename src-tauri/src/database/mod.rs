use crate::asm::scan_task::ScanTask;
use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::Sqlite;
use std::env;
use std::fs;
use std::path::PathBuf;
use dirs;

use sqlx::query;

pub async fn init_db() {
    println!("Initializing database");

    // 使用dirs库跨平台获取用户主目录
    let home_dir = dirs::home_dir()
        .unwrap_or_else(|| {
            // 尝试使用环境变量作为备选方案
            let fallback_dir = if cfg!(windows) {
                env::var_os("USERPROFILE")
                    .and_then(|dir| dir.into_string().ok())
                    .unwrap_or_else(|| ".".to_string())
            } else {
                env::var_os("HOME")
                    .and_then(|dir| dir.into_string().ok())
                    .unwrap_or_else(|| ".".to_string())
            };
            PathBuf::from(fallback_dir)
        });
        
    // 创建应用目录，注意Windows系统通常不使用点号开头
    let dir_name = ".rshiled" ;
    let shield_dir = home_dir.join(dir_name);
    
    // 使用更健壮的错误处理方式创建目录
    match fs::create_dir_all(&shield_dir) {
        Ok(_) => {},
        Err(e) => {
            eprintln!("无法创建应用数据目录: {}", e);
            // 在出错时仍然要继续，使用当前目录作为备选
            // 但记录错误而不是直接panic
        }
    };

    // 创建 test.db 文件
    let rshiledb = shield_dir.join("rshiled.db");

    let db_path = "sqlite:".to_string() + rshiledb.to_str().unwrap();

    match Sqlite::create_database(&db_path).await {
        Ok(_) => (),
        Err(error) => panic!("error: {}", error),
    }

    let pool = {
        SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_path)
            .await
            .unwrap()
    };

    // let pool = SqlitePool::connect(&db_path).await.unwrap();

    query(
        r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS config (
            dns_collection_brute_status INTEGER NOT NULL DEFAULT 0,
            dns_collection_plugin_status INTEGER NOT NULL DEFAULT 0,
            subdomain_dict TEXT,
            file_dict TEXT,
            is_buildin INTEGER NOT NULL DEFAULT 1,
            proxy TEXT,
            user_agent TEXT,
            http_headers TEXT,
            http_timeout INTEGER NOT NULL DEFAULT 5,
            thread_num INTEGER DEFAULT (10), 
            port_scan_plugin_status INTEGER, 
            fingerprint_plugin_status INTEGER, 
            risk_scan_plugin_status INTEGER
        );
        COMMIT;
        "#
    )
    .execute(&pool)
    .await
    .unwrap();



    // 创建 Task 表
    query(
        r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS scan_task (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            name  TEXT NOT NULL,
            monitor_status INTEGER NOT NULL DEFAULT 1,
            running_status TEXT DEFAULT 'wait',
            next_run_time INTEGER DEFAULT 1693526400,
            last_run_time INTEGER  DEFAULT 1693526400,
            UNIQUE(id,name)
        );
        CREATE UNIQUE INDEX scan_task_id_IDX ON scan_task (id,name);
        COMMIT;
        "#
    )
    .execute(&pool)
    .await
    .unwrap();

    // 创建 Domain 表
    sqlx::query(
        r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS domain (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,  
            task_id INTEGER,  
            domain  TEXT NOT NULL,
            aaa  TEXT,
            cname TEXT,
            ns TEXT,
            mx TEXT,
            ufrom TEXT,
            create_at INTEGER,
            update_at INTEGER,
            UNIQUE (id,domain)
        );
        CREATE INDEX idx_domain_create_at ON domain(create_at DESC);
        COMMIT;
        "#
    )
    .execute(&pool)
    .await
    .unwrap();


    sqlx::query(
        r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS api (
            id  INTEGER PRIMARY KEY AUTOINCREMENT,  
            task_id INTEGER,
            http_status INTEGER DEFAULT 0,
            handle_status INTEGER DEFAULT 0,
            get_body_length INTEGER DEFAULT 0,
            post_body_length INTEGER DEFAULT 0,
            method TEXT,  
            uri  TEXT NOT NULL,
            ufrom TEXT,
            update_at INTEGER,
            url TEXT,
            get_response TEXT,
            post_response TEXT,
            UNIQUE (id)
        );
        CREATE INDEX idx_api_update_at ON api(update_at DESC);
        CREATE UNIQUE INDEX api_uri_IDX ON api (uri,ufrom);
        COMMIT;
        "#
    )
    .execute(&pool)
    .await
    .unwrap();

    // 创建 RootDomain 表
    sqlx::query(
        r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS rootdomain (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            domain  TEXT NOT NULL ,
            task_id  INTEGER NOT NULL,
            task_name  TEXT,
            create_at INTEGER,
            update_at INTEGER,
            UNIQUE (id,domain)
        );
        CREATE UNIQUE INDEX rootdomain_domain_IDX ON rootdomain (domain,task_id);
        COMMIT;
        "#
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS risk (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER,
            risk_name TEXT,
            risk_type TEXT,
            risk_desc TEXT,
            risk_level TEXT,
            risk_detail TEXT,
            risk_status INTEGER,
            response TEXT,
            ufrom TEXT,
            update_at INTEGER,
        );
        CREATE UNIQUE INDEX risk_task_id_IDX ON risk (task_id,risk_detail);
        COMMIT;
        "#
    )
    .execute(&pool)
    .await
    .unwrap();


    query("CREATE UNIQUE INDEX risk_task_id_IDX ON risk (task_id,risk_detail);")
    .execute(&pool)
    .await
    .unwrap();

    // 创建 Port 表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS port (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            ips_id INTEGER NOT NULL,
            task_id  INTEGER NOT NULL,
            port TEXT,
            create_at INTEGER,                                      
            update_at INTEGER,
        );
        CREATE UNIQUE INDEX port_ips_id_IDX ON port (ips_id,port);

        "#
    )
    .execute(&pool)
    .await
    .unwrap();

    // 创建 Website 表
    sqlx::query(
        r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS website (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            url  TEXT NOT NULL ,
            base_url TEXT,
            task_id  INTEGER NOT NULL,
            favicon  TEXT,
            title TEXT,
            status_code INTEGER,
            headers TEXT,
            finger TEXT,
            screenshot TEXT,
            tags TEXT,
            ssl_info TEXT,
            create_at INTEGER,                                      
            update_at INTEGER,
            UNIQUE (url)
        );
        CREATE INDEX idx_website_create_at ON website(create_at DESC);
        COMMIT;
        "#
    )
    .execute(&pool)
    .await
    .unwrap();

    


    // 创建 Plugins 表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS plugins (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            version INTEGER NOT NULL,
            description TEXT,
            author TEXT,
            plugin_type TEXT NOT NULL,
            input TEXT,
            output TEXT,
            status INTEGER NOT NULL DEFAULT 1, 
            script TEXT NOT NULL,
            create_at INTEGER NOT NULL,         
            update_at INTEGER NOT NULL,
        );
        "#
    )
    .execute(&pool)
    .await
    .unwrap();

    // 创建 IPs 表
    sqlx::query(
        r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS ips (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id  INTEGER NOT NULL,
            ip_addr  TEXT,
            domain_id INTEGER,
            port_count INTEGER,
            create_at INTEGER,
            update_at INTEGER,
            UNIQUE (ip_addr)
        );
        CREATE INDEX idx_ips_create_at ON ips(create_at DESC);
        CREATE UNIQUE INDEX ips_ip_addr_IDX ON ips (ip_addr,domain_id);
        COMMIT;
        "#
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS cregex (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            name  TEXT,
            regex TEXT,
            rtype INTEGER,
            status INTEGER,
            create_at INTEGER,
            update_at INTEGER,
        );
        CREATE UNIQUE INDEX cregex_name_IDX ON cregex (name);
        COMMIT;
        "#
    )
    .execute(&pool)
    .await
    .unwrap();

    

    sqlx::query(
        r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS webcomp (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id  INTEGER NOT NULL,
            website  TEXT,
            comp_name TEXT,
            ctype TEXT,
            create_at INTEGER,
            update_at INTEGER,
        );
        CREATE UNIQUE INDEX webcomp_comp_name_IDX ON webcomp (comp_name, website);
        COMMIT;
        "#
    )
    .execute(&pool)
    .await
    .unwrap();





    // 插入 Task 数据
    let me = ScanTask {
        id: 1,
        name: "example.com".to_string(),
        monitor_status: 1,
        running_status: "wait".to_string(),
        domain_count:0,
        rootdomain_count:0,
        website_count:0,
        api_count:0,
        webcomp_count:0,
        risk_count:0,
        ips_count:0,
        port_count:0,
        next_run_time: 132156464,
        last_run_time: 132156464,
    };

    query(
        r#"
        INSERT INTO scan_task (id, name, monitor_status, running_status, next_run_time, last_run_time)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT DO NOTHING
        "#)
    .bind(me.id)
    .bind(me.name)
    .bind(me.monitor_status)
    .bind(me.running_status)
    .bind(me.next_run_time)
    .bind(me.last_run_time)
    .execute(&pool)
    .await.unwrap();

    // 插入 Config 数据
    // let conf = AppConfig {
    //     asm_config: AsmConfig{
    //         dns_collection_brute_status: false,
    //         dns_collection_plugin_status: false,
    //     },

    // };

    query(
        r#"
        INSERT INTO config (dns_collection_brute_status, dns_collection_plugin_status)
        VALUES (?1, ?2) ON CONFLICT DO NOTHING
        "#,
    )
    .bind(false)
    .bind(false)
    .execute(&pool)
    .await.unwrap();


    

    

}



// query:
// query 方法用于执行 SQL 查询，并返回一个结果集（Row 或 Rows）。
// 适用于不需要将结果映射到特定结构体的情况。
// 例如，执行一个简单的查询并手动处理结果。

// query_as:
// query_as 方法用于执行 SQL 查询，并将结果映射到指定的结构体或元组。
// 适用于需要将查询结果直接转换为 Rust 类型的情况。
// 例如，将查询结果映射到一个结构体。

// query_scalar:
// query_scalar 方法用于执行 SQL 查询，并返回单个标量值（如单个列的值）。
// 适用于只需要查询一个值的情况，例如获取某个计数或总和。
// 例如，查询用户的数量。

