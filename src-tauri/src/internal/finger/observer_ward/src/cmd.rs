use crate::cli::Mode;
use crate::cli::{default_config, ObserverWardConfig};
use crate::helper::Helper;
use crate::ObserverWard;
use console::Emoji;
use engine::execute::ClusterType;
use engine::template::cluster::cluster_templates;
use log::warn;
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::mpsc::channel;
use std::thread;

use crate::MatchedResult;

pub fn finger_run_batch(
    targets: &Vec<String>,
    cl: &ClusterType,
) -> Option<Vec<BTreeMap<String, MatchedResult>>> {
    let openssl_cfg = default_config().join("openssl.cnf");
    if !openssl_cfg.exists() {
        if let Ok(mut f) = std::fs::File::create(&openssl_cfg) {
            f.write_all(include_bytes!("openssl.cnf"))
                .unwrap_or_default();
        };
    }
    if openssl_cfg.is_file() {
        std::env::set_var("OPENSSL_CONF", openssl_cfg);
    }
    let mut config = ObserverWardConfig::default();

    // config.target = targets.iter()
    // .filter(|&x|  *x == "hr.mgtv.com")
    // .cloned()
    // .collect();
    config.target = targets.clone();
    config.mode = Some(Mode::HTTP);

    let cl = cl.clone();
    let (tx, rx) = channel();
    thread::spawn(move || {
        ObserverWard::new(&config, cl).execute(tx);
    });
    let mut mapvec = Vec::<BTreeMap<String, MatchedResult>>::new();

    for result in rx {
        if result.is_empty() {
            continue;
        }
        mapvec.push(result);
    }
    Some(mapvec)
}

pub fn finger_run_single(target: String) -> Option<Vec<BTreeMap<String, MatchedResult>>> {
    let openssl_cfg = default_config().join("openssl.cnf");
    if !openssl_cfg.exists() {
        if let Ok(mut f) = std::fs::File::create(&openssl_cfg) {
            f.write_all(include_bytes!("openssl.cnf"))
                .unwrap_or_default();
        };
    }
    if openssl_cfg.is_file() {
        std::env::set_var("OPENSSL_CONF", openssl_cfg);
    }
    let mut config = ObserverWardConfig::default();

    config.target.push(target);

    let helper = Helper::new(&config);
    // helper.run();
    let mut templates = config.templates();
    if templates.is_empty() {
        warn!(
            "{}unable to find fingerprint, automatically update fingerprint",
            Emoji("⚠️", "")
        );
        helper.update_fingerprint();
        templates = config.templates();
    }
    let cl = cluster_templates(&templates);

    let (tx, rx) = channel();
    thread::spawn(move || {
        ObserverWard::new(&config, cl).execute(tx);
    });
    let mut mapvec = Vec::<BTreeMap<String, MatchedResult>>::new();
    for result in rx {
        mapvec.push(result);
    }
    Some(mapvec)
}
