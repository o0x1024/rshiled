use anyhow::Result;
use chrono::{DateTime, Utc};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// 生成随机字符串
pub fn generate_random_string(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// 确保目录存在
pub fn ensure_dir_exists<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// 保存JSON数据到文件
pub fn save_json<T, P>(data: &T, path: P) -> Result<()>
where
    T: Serialize,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    
    // 确保父目录存在
    if let Some(parent) = path.parent() {
        ensure_dir_exists(parent)?;
    }
    
    // 序列化数据
    let json = serde_json::to_string_pretty(data)?;
    
    // 写入文件
    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;
    
    Ok(())
}

/// 从文件加载JSON数据
pub fn load_json<T, P>(path: P) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    
    // 读取文件
    let json = fs::read_to_string(path)?;
    
    // 反序列化数据
    let data = serde_json::from_str(&json)?;
    
    Ok(data)
}

/// 格式化日期时间
pub fn format_datetime(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 格式化文件大小
pub fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if size < KB {
        format!("{} B", size)
    } else if size < MB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else if size < GB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else {
        format!("{:.2} GB", size as f64 / GB as f64)
    }
}

/// 获取文件扩展名
pub fn get_file_extension<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

/// 获取MIME类型
pub fn get_mime_type<P: AsRef<Path>>(path: P) -> String {
    let ext = get_file_extension(path).unwrap_or_default();
    
    match ext.as_str() {
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" => "text/plain",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// 检查字符串是否为URL
pub fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// 检查字符串是否为IP地址
pub fn is_ip_address(s: &str) -> bool {
    s.split('.')
        .filter_map(|octet| octet.parse::<u8>().ok())
        .count() == 4
}

/// 检查字符串是否为域名
pub fn is_domain(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() >= 2 && !is_ip_address(s)
}

/// 检查字符串是否为邮箱地址
pub fn is_email(s: &str) -> bool {
    let parts: Vec<&str> = s.split('@').collect();
    parts.len() == 2 && !parts[0].is_empty() && is_domain(parts[1])
}

/// 检查字符串是否为Base64编码
pub fn is_base64(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() || s.len() % 4 != 0 {
        return false;
    }
    
    s.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='
    })
}

/// 检查字符串是否为十六进制编码
pub fn is_hex(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() || s.len() % 2 != 0 {
        return false;
    }
    
    s.chars().all(|c| c.is_ascii_hexdigit())
}

/// 检查字符串是否为JSON
pub fn is_json(s: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(s).is_ok()
}

/// 检查字符串是否为XML
pub fn is_xml(s: &str) -> bool {
    s.trim().starts_with("<?xml") || s.trim().starts_with("<")
}

/// 检查字符串是否为HTML
pub fn is_html(s: &str) -> bool {
    let s = s.trim().to_lowercase();
    s.starts_with("<!doctype html") || s.starts_with("<html")
}

/// 检查字符串是否为JavaScript
pub fn is_javascript(s: &str) -> bool {
    let s = s.trim();
    s.contains("function ") || s.contains("var ") || s.contains("let ") || s.contains("const ")
}

/// 检查字符串是否为CSS
pub fn is_css(s: &str) -> bool {
    let s = s.trim();
    s.contains("{") && s.contains("}") && (s.contains("margin") || s.contains("padding") || s.contains("color"))
}

/// 检查字符串是否为SQL
pub fn is_sql(s: &str) -> bool {
    let s = s.trim().to_uppercase();
    s.starts_with("SELECT ") || s.starts_with("INSERT ") || s.starts_with("UPDATE ") || s.starts_with("DELETE ")
} 