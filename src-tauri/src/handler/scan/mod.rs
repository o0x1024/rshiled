// pub mod Alltag; // Removed as it seems to be an unresolved module
pub mod api; // This will be refactored/removed eventually
pub mod ast;
pub mod config;
pub mod engine;
pub mod plugin;
pub mod plugin_commands;
pub mod proxy;
pub mod rules;
pub mod scanners;
pub mod utils;

// New modules according to the refactoring plan
pub mod common;
pub mod api_commands;
pub mod active;
pub mod passive;
pub mod results;
pub mod status;
pub mod cert_utils;

// Re-export important types - keep these if they are genuinely used by other modules outside `scan`
pub use engine::ScanResult;
pub use scanners::{Scanner, ScannerType};
pub use plugin::*;
// pub use plugin_commands::*; // Consider if this is too broad or if specific items should be re-exported

// The init_scan_system and other functions will be implemented
// as part of a later refactoring. For now, we just have the module
// structure set up.

// The commands() function has been removed.
// Commands are now directly registered in lib.rs by calling functions from scan::api_commands module.
