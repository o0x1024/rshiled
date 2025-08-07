// src-tauri/src/handler/scan/api.rs
//
// THIS FILE IS DEPRECATED AND ITS CONTENTS HAVE BEEN MIGRATED.
//
// All substantive logic, Tauri command definitions (`#[tauri::command]`),
// and associated data structures (structs like `ActiveScanConfig`, `ScanConfig`,
// `Vulnerability`, `ScannerStatus`, etc.) previously in this file have been 
// refactored and moved into new, more specific modules within the 
// `src-tauri/src/handler/scan/` directory.
//
// Specifically:
// - Data Structures: `src-tauri/src/handler/scan/common/types.rs`
// - Tauri Command Entry Points: `src-tauri/src/handler/scan/api_commands.rs`
// - Logic for Active Scans: `src-tauri/src/handler/scan/active/`
// - Logic for Passive Scans: `src-tauri/src/handler/scan/passive/`
// - Logic for Scan Results: `src-tauri/src/handler/scan/results/`
// - Logic for Scan Status: `src-tauri/src/handler/scan/status/`
// - Logic for Certificate Utilities: `src-tauri/src/handler/scan/cert_utils/`
//
// The `invoke_handler` in `lib.rs` has been updated to call commands from
// `scan::api_commands::*` instead of `scan::api::*`.
//
// This file (api.rs) should ideally be deleted. If it is, ensure that
// `pub mod api;` is also removed from `src-tauri/src/handler/scan/mod.rs`.
//
// No active Rust code should remain in this file.
// If there are any utility functions that were missed during migration and are still needed,
// they should be moved to an appropriate new module (e.g., a `utils.rs` module within `scan` or `common`).
