use std::fs;
use std::path::{Path, PathBuf};
use rcgen::{BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType, IsCa, SanType, PKCS_ECDSA_P256_SHA256, PKCS_RSA_SHA256};
use time::OffsetDateTime;
use tokio_rustls::rustls;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::collections::HashMap;
use chrono::{Duration, Utc};
use time::macros::datetime;

pub struct CertificateAuthority {
    cert_dir: PathBuf,
    ca_cert: Mutex<Option<Certificate>>,
    domain_certs: Mutex<HashMap<String, Certificate>>,
}

impl CertificateAuthority {
    pub fn new(cert_dir: &Path) -> Self {
        CertificateAuthority {
            cert_dir: cert_dir.to_path_buf(),
            ca_cert: Mutex::new(None),
            domain_certs: Mutex::new(HashMap::new()),
        }
    }
    
    // 确保证书目录存在
    async fn ensure_cert_dir(&self) -> Result<(), String> {
        if !self.cert_dir.exists() {
            fs::create_dir_all(&self.cert_dir)
                .map_err(|e| format!("无法创建证书目录: {}", e))?;
            println!("已创建证书目录: {:?}", self.cert_dir);
        }
        Ok(())
    }
    
    // 获取CA证书路径
    fn ca_cert_path(&self) -> PathBuf {
        self.cert_dir.join("RShield_CA.crt")
    }
    
    // 获取CA私钥路径
    fn ca_key_path(&self) -> PathBuf {
        self.cert_dir.join("RShield_CA.key")
    }
    
    // 获取域名证书路径
    fn domain_cert_path(&self, domain: &str) -> PathBuf {
        self.cert_dir.join(format!("{}.crt", domain))
    }
    
    // 获取域名私钥路径
    fn domain_key_path(&self, domain: &str) -> PathBuf {
        self.cert_dir.join(format!("{}.key", domain))
    }

    
    // 生成CA证书
    pub async fn generate_ca(&self) -> Result<(), String> {
        // 确保证书目录存在
        self.ensure_cert_dir().await?;
        
        let ca_cert_path = self.ca_cert_path();
        let ca_key_path = self.ca_key_path();
        
        // 检查CA证书文件是否已存在
        if ca_cert_path.exists() && ca_key_path.exists() {
            println!("发现现有CA证书，尝试加载...");
            
            // 读取现有的CA证书和私钥
            let cert_pem = fs::read_to_string(&ca_cert_path)
                .map_err(|e| format!("无法读取CA证书文件: {}", e))?;
            
            let key_pem = fs::read_to_string(&ca_key_path)
                .map_err(|e| format!("无法读取CA私钥文件: {}", e))?;
            
            // 从PEM文件重新构建Certificate对象
            match Certificate::from_params(
                self.create_ca_params()?
            ) {
                Ok(cert) => {
                    println!("已成功加载现有CA证书");
                    let mut ca_cert = self.ca_cert.lock().await;
                    *ca_cert = Some(cert);
                    return Ok(());
                },
                Err(e) => {
                    println!("加载现有CA证书失败: {}，将生成新证书", e);
                    // 如果加载失败，将尝试生成新证书
                }
            }
        }

        // 如果没有现有证书或加载失败，生成新证书
        self.create_new_ca_certificate().await
    }
    
    // 创建新的CA证书
    async fn create_new_ca_certificate(&self) -> Result<(), String> {
        println!("正在生成新的CA证书...");
        
        // 创建证书参数
        let params = self.create_ca_params()?;
        
        // 生成证书
        let cert = Certificate::from_params(params)
            .map_err(|e| format!("生成CA证书失败: {}", e))?;
        
        // 保存证书和私钥到文件
        let cert_pem = cert.serialize_pem()
            .map_err(|e| format!("序列化CA证书失败: {}", e))?;
            
        let key_pem = cert.serialize_private_key_pem();
        
        let ca_cert_path = self.cert_dir.join("RShield_CA.crt");
        let ca_key_path = self.cert_dir.join("RShield_CA.key");
        
        fs::write(&ca_cert_path, cert_pem.clone())
            .map_err(|e| format!("保存CA证书文件失败: {}", e))?;
            
        fs::write(&ca_key_path, key_pem)
            .map_err(|e| format!("保存CA私钥文件失败: {}", e))?;
        
        println!("已成功生成新的CA证书并保存至: {:?}", ca_cert_path);
        
        // 更新内存中的证书
        let mut ca_cert = self.ca_cert.lock().await;
        *ca_cert = Some(cert);
        
        Ok(())
    }
    
    // 创建CA证书参数
    fn create_ca_params(&self) -> Result<CertificateParams, String> {
        let mut params = CertificateParams::default();
        
        // 设置基本约束 - CA证书
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        
        // 设置证书有效期 - 10年
        let now = Utc::now();
        let not_before = OffsetDateTime::now_utc();
        let not_after = not_before.checked_add(time::Duration::days(3650)).unwrap(); // 10年
        params.not_before = not_before;
        params.not_after = not_after;
        
        // 设置证书主题
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, "RShield Local CA");
        distinguished_name.push(DnType::OrganizationName, "RShield Proxy");
        distinguished_name.push(DnType::CountryName, "CN");
        params.distinguished_name = distinguished_name;
        
        // 设置使用PKCS_ECDSA_P256_SHA256签名算法
        params.alg = &PKCS_ECDSA_P256_SHA256;
        
        Ok(params)
    }
    
    // 生成站点证书
    pub async fn generate_certificate_for_domain(&self, domain: &str) -> Result<(Vec<u8>, Vec<u8>), String> {
        // 确保CA证书已加载
        let ca_cert_guard = self.ca_cert.lock().await;
        let ca_cert = match &*ca_cert_guard {
            Some(cert) => cert,
            None => return Err("CA证书尚未生成".to_string()),
        };
        
        // 检查是否已有域名证书缓存
        let domain_cert_path = self.cert_dir.join(format!("{}.crt", domain));
        let domain_key_path = self.cert_dir.join(format!("{}.key", domain));
        
        // 如果域名证书已存在，直接返回
        if domain_cert_path.exists() && domain_key_path.exists() {
            println!("发现域名 {} 的现有证书，直接加载", domain);
            
            // 读取证书和私钥
            let mut cert_file = File::open(&domain_cert_path).await
                .map_err(|e| format!("无法打开域名证书文件: {}", e))?;
                
            let mut key_file = File::open(&domain_key_path).await
                .map_err(|e| format!("无法打开域名私钥文件: {}", e))?;
                
            let mut cert_pem = Vec::new();
            let mut key_pem = Vec::new();
            
            cert_file.read_to_end(&mut cert_pem).await
                .map_err(|e| format!("读取域名证书文件失败: {}", e))?;
                
            key_file.read_to_end(&mut key_pem).await
                .map_err(|e| format!("读取域名私钥文件失败: {}", e))?;
                
            return Ok((cert_pem, key_pem));
        }
        
        // 创建证书参数
        let mut params = CertificateParams::default();
        
        // 设置证书主题
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, domain);
        distinguished_name.push(DnType::OrganizationName, "RShield Proxy Generated");
        distinguished_name.push(DnType::CountryName, "CN");
        params.distinguished_name = distinguished_name;
        
        // 设置有效期 - 1年
        let not_before = OffsetDateTime::now_utc();
        let not_after = not_before.checked_add(time::Duration::days(365)).unwrap();
        params.not_before = not_before;
        params.not_after = not_after;
        
        // 添加域名到SAN
        params.subject_alt_names.push(SanType::DnsName(domain.to_string()));
        
        // 如果域名是IP地址形式，添加IP SAN
        if let Ok(ip) = domain.parse::<std::net::IpAddr>() {
            params.subject_alt_names.push(SanType::IpAddress(ip));
        }
        
        // 设置使用PKCS_ECDSA_P256_SHA256签名算法
        params.alg = &PKCS_ECDSA_P256_SHA256;
        
        // 生成证书
        println!("正在为域名 {} 生成证书...", domain);
        let cert = Certificate::from_params(params)
            .map_err(|e| format!("生成域名证书失败: {}", e))?;
            
        // 使用CA证书签名
        let cert_pem = cert.serialize_pem_with_signer(&ca_cert)
            .map_err(|e| format!("使用CA签名域名证书失败: {}", e))?;
            
        let key_pem = cert.serialize_private_key_pem();
        
        // 保存证书和私钥
        fs::write(&domain_cert_path, &cert_pem)
            .map_err(|e| format!("保存域名证书文件失败: {}", e))?;
            
        fs::write(&domain_key_path, &key_pem)
            .map_err(|e| format!("保存域名私钥文件失败: {}", e))?;
            
        println!("已成功为域名 {} 生成证书", domain);
        
        // 更新域名证书缓存
        let mut domain_certs = self.domain_certs.lock().await;
        domain_certs.insert(domain.to_string(), cert);
        
        Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
    }
    
    // 导出CA证书
    pub async fn export_ca(&self, path: Option<&Path>) -> Result<PathBuf, String> {
        // 确保CA证书已生成
        let ca_cert_path = self.ca_cert_path();
        if !ca_cert_path.exists() {
            return Err("CA证书尚未生成".to_string());
        }
        
        // 如果提供了路径，则将CA证书复制到该路径
        if let Some(target_path) = path {
            let mut source_file = File::open(&ca_cert_path).await
                .map_err(|e| format!("无法打开CA证书文件: {}", e))?;
            let mut target_file = File::create(target_path).await
                .map_err(|e| format!("无法创建目标CA证书文件: {}", e))?;
            
            let mut buffer = Vec::new();
            source_file.read_to_end(&mut buffer).await
                .map_err(|e| format!("无法读取CA证书: {}", e))?;
            target_file.write_all(&buffer).await
                .map_err(|e| format!("无法写入CA证书到目标路径: {}", e))?;
            
            Ok(target_path.to_path_buf())
        } else {
            Ok(ca_cert_path)
        }
    }
    
    // 获取证书目录
    pub fn get_cert_dir(&self) -> &PathBuf {
        &self.cert_dir
    }
    
    // 检查CA证书是否已经存在
    pub fn has_ca_certificate(&self) -> bool {
        let ca_cert_path = self.cert_dir.join("RShield_CA.crt");
        let ca_key_path = self.cert_dir.join("RShield_CA.key");
        ca_cert_path.exists() && ca_key_path.exists()
    }


    pub fn get_ca_cert_and_key(&self) -> Result<(Vec<u8>, Vec<u8>), String> {
        let ca_cert_path = PathBuf::from("./certs/RShield_CA.crt");
        let ca_key_path = PathBuf::from("./certs/RShield_CA.key");
        let ca_cert = fs::read(&ca_cert_path).map_err(|e| format!("无法读取CA证书: {}", e))?;
        let ca_key = fs::read(&ca_key_path).map_err(|e| format!("无法读取CA私钥: {}", e))?;
        Ok((ca_cert, ca_key))
    }
}

impl Default for CertificateAuthority {
    fn default() -> Self {
        let cert_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rshield/certs");
        Self::new(&cert_dir)
    }
}