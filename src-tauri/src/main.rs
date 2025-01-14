// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod utils;
pub mod plugin;

use std::path::PathBuf;

use log::*;
use rshield_lib::{asm::asm_task::asm_init, config::config::AppConfig, database};
use utils::{log_client::Client, logger::{Config, Logger}};

#[tokio::main]
async fn main() {
    // 初始化配置
    let mut client = Client::new(true);

    let logger = Logger::new(Config {
        max_size: 1024 * 1024 * 5,
        path: PathBuf::from("./rshiled.log"),
        #[cfg(not(feature = "debug"))]
        file_level: LevelFilter::Info,
        #[cfg(feature = "debug")]
        file_level: LevelFilter::Debug,
        remote_level: LevelFilter::Error,
        max_backups: 10,
        compress: true,
        client: Some(client.clone()),
    });
    set_boxed_logger(Box::new(logger)).unwrap();

    if utils::file::is_first_run() {
        //初始化数据库
        database::init_db();
    }

    AppConfig::init();

    // Use spawn_blocking instead of spawn
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            asm_init().await;
            loop {}
        });
    });

    rshield_lib::run();
    // handle.join().unwrap()
}
