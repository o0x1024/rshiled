// src-tauri/src/handler/scan/api_commands.rs
use tauri::{State, command};
// Ensure ScannerStatus is correctly imported if this file needs it directly.
// However, get_scan_status now returns crate::state::ScannerStatus (which itself imports from common::types)
// So this direct import might not be strictly needed here if types are correctly inferred from function signatures.
// Let's assume for now it's needed for clarity or direct use elsewhere in this file.
use super::common::types::{ActiveScanConfig, PassiveScanConfig, SuccessResponse, Vulnerability, ScannerStatus};
use crate::state::ScannerState; // For State<'_, ScannerState>

// Direct imports for handlers to potentially resolve linter issues
use crate::handler::scan::status::handler::handle_get_scan_status as get_status_handler;
use crate::handler::scan::cert_utils::handler::handle_open_cert_file as open_cert_handler;
use crate::handler::scan::active::handler::handle_start_active_scan;
use crate::handler::scan::passive::handler::{handle_start_passive_scan, handle_stop_passive_scan};
use crate::handler::scan::results::handler::{handle_get_scan_vulnerabilities, handle_clear_scan_vulnerabilities, handle_export_scan_vulnerabilities};

// Placeholder for actual logic handlers that will be in other modules
// For example, active::handler::start_active_scan_logic, etc.

#[command]
pub async fn start_active_scan(
    config: ActiveScanConfig,
    state: State<'_, ScannerState>,
) -> Result<SuccessResponse, String> {
    handle_start_active_scan(config, state).await
}

#[command]
pub async fn start_passive_scan(
    config: PassiveScanConfig,
    state: State<'_, ScannerState>,
) -> Result<SuccessResponse, String> {
    handle_start_passive_scan(config, state).await
}

#[command]
pub async fn stop_passive_scan(state: State<'_, ScannerState>) -> Result<SuccessResponse, String> {
    handle_stop_passive_scan(state).await
}

#[command]
pub async fn get_scan_status(state: State<'_, ScannerState>) -> Result<ScannerStatus, String> {
    get_status_handler(state).await
}

#[command]
pub async fn get_scan_vulnerabilities(state: State<'_, ScannerState>) -> Result<Vec<Vulnerability>, String> { 
    handle_get_scan_vulnerabilities(state).await
}

#[command]
pub async fn clear_scan_vulnerabilities(state: State<'_, ScannerState>) -> Result<SuccessResponse, String> {
    handle_clear_scan_vulnerabilities(state).await
}

#[command]
pub async fn export_scan_vulnerabilities(
    state: State<'_, ScannerState>,
    path: String,
) -> Result<SuccessResponse, String> {
    handle_export_scan_vulnerabilities(state, path).await
}

#[command]
pub fn open_cert_file(path: String) -> Result<(), String> {
    open_cert_handler(path)
} 