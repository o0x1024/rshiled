use std::path::Path;
use std::fs;
use anyhow::Result;

/// Create directory if it doesn't exist
pub fn ensure_dir_exists<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Generate a random string
pub fn generate_random_string(length: usize) -> String {
    use rand::{thread_rng, Rng};
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .collect();
    let mut rng = thread_rng();
    (0..length)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

/// Format file size in human-readable format
pub fn format_file_size(size: u64) -> String {
    const UNIT: f64 = 1024.0;
    if size < UNIT as u64 {
        return format!("{} B", size);
    }
    let size_f = size as f64;
    let exp = (size_f.ln() / UNIT.ln()) as i32;
    let pre = "KMGTPE".chars().nth(exp as usize - 1).unwrap();
    format!("{:.1} {}B", size_f / UNIT.powi(exp), pre)
}
