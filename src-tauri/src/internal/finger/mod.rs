use std::{env::current_dir, fs::File};

use engine::template::Template;
use log::error;
use observer_ward::cli::default_config;

pub fn get_teamplate() -> Vec<Template> {
    let mut templates = Vec::new();
    for path in ["web_fingerprint_v4.json", "service_fingerprint_v4.json"] {
        let fingerprint_path = current_dir().map_or(default_config().join(path), |x| {
            let p = x.join(path);
            if p.exists() {
                p
            } else {
                default_config().join(path)
            }
        });
        if let Ok(f) = std::fs::File::open(&fingerprint_path) {
            match serde_json::from_reader::<File, Vec<_>>(f) {
                Ok(t) => {
                    templates.extend(t);
                }
                Err(err) => {
                    error!(
                        "load template {} err {}",
                        fingerprint_path.to_string_lossy(),
                        err
                    );
                }
            }
        }
    }
    templates
}
