// src-tauri/src/handler/scan/results/handler.rs
use std::fs::File;
use std::io::Write;
use tauri::State;
use chrono::Local;
use log::{info, error};
use crate::handler::scan::common::types::{SuccessResponse, Vulnerability};
use crate::state::ScannerState;

pub async fn handle_get_scan_vulnerabilities(
    state: State<'_, ScannerState>,
) -> Result<Vec<Vulnerability>, String> {
    println!("Results handler: handle_get_scan_vulnerabilities called");
    
    // 获取当前漏洞列表
    let vulnerabilities = state.vulnerabilities.lock().await.clone();
    Ok(vulnerabilities)
}

pub async fn handle_clear_scan_vulnerabilities(
    state: State<'_, ScannerState>,
) -> Result<SuccessResponse, String> {
    println!("Results handler: handle_clear_scan_vulnerabilities called");
    
    // 清空漏洞列表
    {
        let mut vulnerabilities = state.vulnerabilities.lock().await;
        vulnerabilities.clear();
    }
    
    // 更新状态
    state.update_status(|status| {
        status.vulnerability_count = 0;
    }).await;
    
    Ok(SuccessResponse {
        success: true,
        message: "漏洞列表已清空".to_string(),
    })
}

pub async fn handle_export_scan_vulnerabilities(
    state: State<'_, ScannerState>,
    path: String,
) -> Result<SuccessResponse, String> {
    println!("Results handler: handle_export_scan_vulnerabilities called");
    
    let vulnerabilities = state.vulnerabilities.lock().await.clone();
    
    if vulnerabilities.is_empty() {
        return Ok(SuccessResponse {
            success: false,
            message: "没有可导出的漏洞".to_string(),
        });
    }
    
    // 导出为JSON文件
    let json = match serde_json::to_string_pretty(&vulnerabilities) {
        Ok(json) => json,
        Err(e) => return Err(format!("序列化漏洞数据失败: {}", e)),
    };
    
    // 创建并写入文件
    let file_path = if path.ends_with(".json") {
        path
    } else {
        format!("{}/scan_vulnerabilities_{}.json", path, Local::now().format("%Y%m%d_%H%M%S"))
    };
    
    match File::create(&file_path) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(json.as_bytes()) {
                return Err(format!("写入文件失败: {}", e));
            }
            
            info!("漏洞数据已导出至: {}", file_path);
            Ok(SuccessResponse {
                success: true,
                message: format!("已成功导出{}条漏洞数据至{}", vulnerabilities.len(), file_path),
            })
        },
        Err(e) => {
            error!("创建导出文件失败: {}", e);
            Err(format!("创建导出文件失败: {}", e))
        }
    }
} 