// src-tauri/src/handler/scan/active/handler.rs
use tauri::State;
use crate::handler::scan::common::types::{ActiveScanConfig, SuccessResponse, TargetType};
use crate::state::ScannerState;
use log::{info, error, warn};
use super::orchestrator;

pub async fn handle_start_active_scan(
    mut config: ActiveScanConfig,
    state: State<'_, ScannerState>,
) -> Result<SuccessResponse, String> {
    info!("处理主动扫描请求，扫描类型: {}", config.scan_type);
    
    // 验证目标
    if config.targets.is_empty() {
        return Err("未指定扫描目标".to_string());
    }
    
    // 处理目标，自动添加http://前缀
    config.targets = config.targets.iter().map(|target| {
        let target_type = ActiveScanConfig::identify_target_type(target);
        
        match target_type {
            TargetType::Domain => {
                if !target.starts_with("http://") && !target.starts_with("https://") {
                    return format!("http://{}", target);
                }
                target.clone()
            },
            _ => target.clone()
        }
    }).collect();

    // 验证线程数和超时设置
    if config.threads < 1 {
        config.threads = 1;
        warn!("线程数调整为最小值1");
    } else if config.threads > 100 {
        config.threads = 100;
        warn!("线程数调整为最大值100");
    }
    
    if config.timeout < 1 {
        config.timeout = 30;
        warn!("超时时间调整为默认值30秒");
    } else if config.timeout > 300 {
        config.timeout = 300;
        warn!("超时时间调整为最大值300秒");
    }

    // 验证扫描类型
    if !["full", "quick", "custom", "nuclei"].contains(&config.scan_type.as_str()) {
        return Err(format!("无效的扫描类型: {}", config.scan_type));
    }
    
    // 验证自定义选项
    if config.scan_type == "custom" && config.detailed_scan_options.is_none() {
        return Err("选择了自定义扫描但未提供详细扫描选项".to_string());
    }

    // 调用扫描协调器
    orchestrator::run_scan(config, state).await
} 