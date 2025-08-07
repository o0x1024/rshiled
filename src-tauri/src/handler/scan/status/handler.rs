use tauri::State;
use crate::state::ScannerState;
use crate::handler::scan::common::types::ScannerStatus;

pub async fn handle_get_scan_status(
    state: State<'_, ScannerState>,
) -> Result<ScannerStatus, String> {
    println!("Status handler: handle_get_scan_status called");
    
    // 直接获取ScannerState中存储的状态
    let status = state.status.lock().await.clone();
    Ok(status)
} 