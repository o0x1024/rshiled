use std::process::Command;
use log::{info, error};

pub fn handle_open_cert_file(path: String) -> Result<(), String> {
    println!("Cert utils handler: handle_open_cert_file called with path: {}", path);
    
    let result = open_file(&path);
    
    match result {
        Ok(_) => {
            info!("成功打开证书文件: {}", path);
            Ok(())
        },
        Err(e) => {
            error!("打开证书文件失败: {}", e);
            Err(format!("打开证书文件失败: {}", e))
        }
    }
}

// 根据操作系统打开文件
fn open_file(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/c", "start", "", path])
            .spawn()
            .map_err(|e| format!("Windows打开文件失败: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("MacOS打开文件失败: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Linux打开文件失败: {}", e))?;
    }
    
    Ok(())
} 