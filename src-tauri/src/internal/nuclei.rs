//

use std::collections::HashSet;
use std::io::Write;
use std::process::Command;
use std::{fs::File, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NucleiResult {
    template: String,
    host: String,
    info: VulnerabilityInfo,
    // 其他字段根据实际输出添加
}

#[derive(Debug, Serialize, Deserialize)]
struct VulnerabilityInfo {
    name: String,
    severity: String,
    description: String,
}

pub struct NucleiRunner {
    pub name: String,
    pub plugins: HashSet<PathBuf>,
    pub condition: Vec<String>,
    pub targets: HashSet<String>,
}

impl NucleiRunner {
    pub fn new(name: String) -> Self {
        Self {
            name,
            plugins: HashSet::new(),
            condition: Vec::new(),
            targets: HashSet::new(),
        }
    }

    pub fn batch_scan_nuclei(&self, targets: &[&str], template: &str) -> Result<String, String> {
        let temp_file = "targets.txt";
        let mut file = File::create(temp_file).map_err(|e| format!("创建临时文件失败: {}", e))?;

        // 将目标写入文件（每行一个）
        for target in targets {
            writeln!(file, "{}", target).map_err(|e| format!("写入文件失败: {}", e))?;
        }

        // 执行命令
        let output = Command::new("nuclei")
            .args(&[
                "-l", temp_file, // 从文件读取目标
                "-t", template, "-json",
            ])
            .output()
            .map_err(|e| format!("执行命令失败: {}", e))?;

        // 清理临时文件（可选）
        let _ = std::fs::remove_file(temp_file);

        // 处理输出（同上）
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn single_scan_nuclei(&self, target: &str, template: &str) -> Result<String, String> {
        let output = Command::new("nuclei")
            .args(&[
                "-u", target, "-t", template, "-json", // 输出为 JSON 格式
            ])
            .output()
            .map_err(|e| format!("执行命令失败: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    pub fn parse_results(&self, json_output: &str) -> Result<Vec<NucleiResult>, serde_json::Error> {
        let results: Vec<NucleiResult> = serde_json::from_str(json_output)?;
        Ok(results)
    }
}
