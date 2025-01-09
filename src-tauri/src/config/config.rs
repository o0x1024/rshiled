use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::OnceLock};
use tauri::State;

use crate::utils;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub dns_collection_brute_status: bool,
    pub dns_collection_plugin_status: bool,
}

//GLOBAL_CONFIG 可以全局访问么
static GLOBAL_CONFIG: OnceLock<AppConfig> = OnceLock::new();

impl AppConfig {
    pub fn global() -> &'static AppConfig {
        GLOBAL_CONFIG.get().expect("Config not initialized")
    }

    pub fn init() {
        let db_path = utils::file::get_db_path();
        let conn = Connection::open(db_path).unwrap();

        let mut stmt = conn
            .prepare("SELECT dns_collection_brute_status,dns_collection_plugin_status FROM Config")
            .unwrap();

        
        let myconfig = stmt
            .query_row([], |row: &rusqlite::Row<'_>| {
                Ok(AppConfig {
                    dns_collection_brute_status: row.get(0)?,
                    dns_collection_plugin_status: row.get(1)?,
                })
            })
            .unwrap();
        let _ = GLOBAL_CONFIG.set(myconfig);
    }
}

#[tauri::command]
pub fn get_config(state: State<'_, AppConfig>) -> AppConfig {
    state.inner().clone()
}
