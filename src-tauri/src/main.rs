// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
pub mod internal;

use fern::Dispatch;
use serde_json::json;
use std::env;
use std::fs::OpenOptions;

use rshield_lib::{
    database::init_db, global::config::CoreConfig, handler::asm::asm_task::asm_init,
};

use pistol::vs::vs_scan;
use pistol::Host;
use pistol::Target;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;


#[tokio::main]
async fn main() {
    if let Err(e) = setup_logger() {
        eprintln!("日志初始化失败: {}", e);
        std::process::exit(1);
    }

    if internal::file::is_first_run() {
        //初始化数据库
        init_db().await;
    }

    // Initialize ASM in background
    let handle = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        handle.block_on(async {
            asm_init().await;
            tokio::time::sleep(Duration::from_secs(2)).await;
            //初始化配置
            CoreConfig::init().await.unwrap();
        });
    });

    rshield_lib::run().await;
}

#[allow(dead_code)]
fn test_nmap() {
    let dst_addr = Ipv4Addr::from_str("175.6.15.109").unwrap();
    let host = Host::new(dst_addr.into(), Some(vec![22, 80, 443, 8080]));
    let target = Target::new(vec![host]);
    let timeout = Some(Duration::new(1, 0));
    // only_null_probe = true, only_tcp_recommended = any, only_udp_recomended = any: only try the NULL probe (for TCP)
    // only_tcp_recommended = true: only try the tcp probe recommended port
    // only_udp_recommended = true: only try the udp probe recommended port
    let (only_null_probe, only_tcp_recommended, only_udp_recommended) = (false, true, false);
    let intensity = 7; // nmap default
    let threads_num = Some(8);
    let ret = vs_scan(
        &target,
        threads_num,
        only_null_probe,
        only_tcp_recommended,
        only_udp_recommended,
        intensity,
        timeout,
    )
    .unwrap();
    println!("{}", ret);
}

pub fn setup_logger() -> Result<(), fern::InitError> {
    // 配置日志输出到控制台
    // let console_output = Dispatch::new()
    //     .format(|out, message, record| {
    //         out.finish(format_args!(
    //             "[{}] [{}] {} - {}",
    //             chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
    //             record.level(),
    //             record.target(),
    //             message
    //         ))
    //     })
    //     .chain(std::io::stdout());

    // 确保logs目录存在
    let logs_dir = "logs";
    std::fs::create_dir_all(logs_dir)?;

    let filename = format!(
        "logs/rshiled-{}.log",
        chrono::Local::now().format("%Y-%m-%d")
    );
    // 配置日志输出到文件
    let file_output = Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] [{}:{}] [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), // 时间
                record.file().unwrap_or("unknown_file"),          // 文件路径
                record.line().unwrap_or(0),                       // 行号
                record.level(),                                   // 日志级别
                message                                           // 日志消息
            ))
        })
        .filter(move |metadata| {
            // 检查日志的模块路径是否包含 "my_module"
            metadata.target().contains("rshield")
        })
        .chain(
            OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(filename)?,
        );

    // 合并配置并初始化
    Dispatch::new()
        // .chain(console_output) // 输出到控制台
        .chain(file_output) // 输出到文件
        .apply()?; // 应用配置

    Ok(())
}
