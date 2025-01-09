// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod utils;


use rshield_lib::{asm::asm_task::asm_init, config::config::AppConfig, database};

#[tokio::main]
async fn main() {
    // 初始化配置
    AppConfig::init();

    if utils::file::is_first_run() {
        //初始化数据库
        database::init_db();
    }

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
