use std::env;
use std::{fs, path::PathBuf};

pub fn is_first_run() -> bool {
    let dbpath = get_db_path(); // 替换为你要检查的文件路径

    if fs::metadata(dbpath).is_ok() {
        false
    } else {
        true
    }
}

pub fn get_data_dir() -> PathBuf {
    let home_dir = env::var_os("HOME")
        .and_then(|dir| dir.into_string().ok())
        .unwrap_or_else(|| "/".to_string());

    // 创建 .shiled 目录
    let shield_dir = PathBuf::from(home_dir).join(".rshiled");
    fs::create_dir_all(&shield_dir).expect("Failed to create .shiled directory");
    shield_dir
}

pub fn get_db_path() -> PathBuf {
    let home_dir = dirs::home_dir().unwrap_or_else(|| {
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

    // 创建 .shiled 目录
    let db_path = PathBuf::from(home_dir).join(".rshiled").join("rshiled.db");
    db_path
}
