use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Row};
use std::collections::HashMap;
use std::sync::Arc;
use chrono::Local;
use std::path::PathBuf;
use std::fs;
use tauri;


use super::asm_task::INNERASK_MODULE;
use super::{domain::Domain, ips::IPs, port::Port, risk::Risk, website::WebSite, web_comp::WebComp};

// 资产图谱相关结构
#[derive(Debug, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub category: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphLink {
    pub source: String,
    pub target: String,
    pub value: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetGraphData {
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
}

// 风险热图相关结构
#[derive(Debug, Serialize, Deserialize)]
pub struct RiskData {
    pub asset: String,
    pub risk_type: String,
    pub risk_level: String,
    pub count: i32,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskAnalysis {
    pub main_risk: String,
    pub weak_point: String,
    pub recommendation: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskStats {
    pub high_risk_count: i32,
    pub medium_risk_count: i32,
    pub low_risk_count: i32,
    pub top_high_risk_asset: String,
    pub top_medium_risk_asset: String,
    pub top_low_risk_asset: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskHeatmapData {
    pub risk_data: Vec<RiskData>,
    pub analysis: RiskAnalysis,
    pub stats: RiskStats,
}

// 合规报告相关结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ReportSection {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportStats {
    pub critical_issues: i32,
    pub high_issues: i32,
    pub medium_issues: i32,
    pub low_issues: i32,
    pub compliance_score: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportPreview {
    pub title: String,
    pub type_: String,
    pub generated_at: String,
    pub task_name: String,
    pub sections: Vec<ReportSection>,
    pub summary: String,
    pub stats: ReportStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportResult {
    pub preview: ReportPreview,
    pub file_path: String,
}

// 资产图谱的Tauri命令
#[tauri::command(rename_all = "snake_case")]
pub async fn get_asset_graph_data(task_id: u32, graph_type: String) -> Result<AssetGraphData, String> {
    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => {
            return Err("全局变量未初始化".to_string());
        }
    };
    let pool_clone = Arc::clone(&task_module.read_conn);

    // 构建查询结果
    let mut nodes = Vec::new();
    let mut links = Vec::new();
    let mut node_ids = HashMap::new();

    // 根据图谱类型获取数据
    match graph_type.as_str() {
        "domain" => {
            // 获取域名数据
            let domains: Vec<Domain> = query_as("SELECT * FROM domain WHERE task_id = ? LIMIT 100")
                .bind(&task_id)
                .fetch_all(&*pool_clone)
                .await
                .map_err(|e| e.to_string())?;

            // 添加域名节点
            for domain in &domains {
                let id = format!("domain_{}", domain.id.unwrap_or_default());
                node_ids.insert(id.clone(), nodes.len());
                nodes.push(GraphNode {
                    id: id.clone(),
                    name: domain.domain.clone(),
                    category: "domain".to_string(),
                    data: Some(serde_json::to_value(domain).unwrap_or_default()),
                });

                // 获取关联的IP
                let ips: Vec<IPs> = query_as("SELECT * FROM ips WHERE domain_id = ?")
                    .bind(domain.id)
                    .fetch_all(&*pool_clone)
                    .await
                    .map_err(|e| e.to_string())?;

                // 添加IP节点和链接
                for ip in &ips {
                    let ip_id = format!("ip_{}", ip.id.unwrap_or_default());
                    if !node_ids.contains_key(&ip_id) {
                        node_ids.insert(ip_id.clone(), nodes.len());
                        nodes.push(GraphNode {
                            id: ip_id.clone(),
                            name: ip.ip_addr.clone().unwrap_or_else(|| "未知IP".to_string()),
                            category: "ip".to_string(),
                            data: Some(serde_json::to_value(ip).unwrap_or_default()),
                        });
                    }

                    // 添加域名到IP的链接
                    links.push(GraphLink {
                        source: id.clone(),
                        target: ip_id.clone(),
                        value: Some(1),
                    });
                }
            }
        },
        "ip" => {
            // 获取IP数据
            let ips: Vec<IPs> = query_as("SELECT ips.id, ips.task_id, ips.ip_addr, ips.domain_id, ips.create_at, ips.update_at, domain.domain FROM ips LEFT JOIN domain ON ips.domain_id = domain.id LIMIT 100")
                .bind(&task_id)
                .fetch_all(&*pool_clone)
                .await
                .map_err(|e| e.to_string())?;

            // 添加IP节点
            for ip in &ips {
                let id = format!("ip_{}", ip.id.unwrap_or_default());
                node_ids.insert(id.clone(), nodes.len());
                nodes.push(GraphNode {
                    id: id.clone(),
                    name: ip.ip_addr.clone().unwrap_or_else(|| "未知IP".to_string()),
                    category: "ip".to_string(),
                    data: Some(serde_json::to_value(ip).unwrap_or_default()),
                });

                // 获取关联的端口
                let ports: Vec<Port> = query_as("SELECT * FROM port WHERE ips_id = ?")
                    .bind(ip.id)
                    .fetch_all(&*pool_clone)
                    .await
                    .map_err(|e| e.to_string())?;

                // 添加端口节点和链接
                for port in &ports {
                    let port_id = format!("port_{}_{}", ip.id.unwrap_or_default(), port.port);
                    if !node_ids.contains_key(&port_id) {
                        node_ids.insert(port_id.clone(), nodes.len());
                        nodes.push(GraphNode {
                            id: port_id.clone(),
                            name: format!("{}:{}", ip.ip_addr.clone().unwrap_or_else(|| "未知IP".to_string()), port.port),
                            category: "port".to_string(),
                            data: Some(serde_json::to_value(port).unwrap_or_default()),
                        });
                    }

                    // 添加IP到端口的链接
                    links.push(GraphLink {
                        source: id.clone(),
                        target: port_id.clone(),
                        value: Some(1),
                    });
                }
            }
        },
        "website" => {
            // 获取网站数据
            let websites: Vec<WebSite> = query_as("SELECT * FROM website WHERE task_id = ? LIMIT 100")
                .bind(&task_id)
                .fetch_all(&*pool_clone)
                .await
                .map_err(|e| e.to_string())?;

            // 添加网站节点
            for website in &websites {
                let id = format!("website_{}", website.id.unwrap_or_default());
                node_ids.insert(id.clone(), nodes.len());
                nodes.push(GraphNode {
                    id: id.clone(),
                    name: website.url.clone(),
                    category: "website".to_string(),
                    data: Some(serde_json::to_value(website).unwrap_or_default()),
                });

                // 获取关联的组件
                let components: Vec<WebComp> = query_as("SELECT * FROM webcomp WHERE website = ?")
                    .bind(&website.url)
                    .fetch_all(&*pool_clone)
                    .await
                    .map_err(|e| e.to_string())?;

                // 添加组件节点和链接
                for component in &components {
                    let comp_id = format!("component_{}", component.id.unwrap_or_default());
                    if !node_ids.contains_key(&comp_id) {
                        node_ids.insert(comp_id.clone(), nodes.len());
                        nodes.push(GraphNode {
                            id: comp_id.clone(),
                            name: component.comp_name.clone(),
                            category: "component".to_string(),
                            data: Some(serde_json::to_value(component).unwrap_or_default()),
                        });
                    }

                    // 添加网站到组件的链接
                    links.push(GraphLink {
                        source: id.clone(),
                        target: comp_id.clone(),
                        value: Some(1),
                    });
                }

                // 获取关联的风险
                let risks: Vec<Risk> = query_as("SELECT * FROM risk WHERE ufrom = ?")
                    .bind(&website.url)
                    .fetch_all(&*pool_clone)
                    .await
                    .map_err(|e| e.to_string())?;

                // 添加风险节点和链接
                for risk in &risks {
                    let risk_id = format!("risk_{}", risk.id.unwrap_or_default());
                    if !node_ids.contains_key(&risk_id) {
                        node_ids.insert(risk_id.clone(), nodes.len());
                        
                        // 使用 serde_json::to_value 获取风险数据
                        let risk_json = serde_json::to_value(risk).unwrap_or_default();
                        let risk_name = risk_json.get("risk_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("未知风险")
                            .to_string();
                        
                        nodes.push(GraphNode {
                            id: risk_id.clone(),
                            name: risk_name,
                            category: "risk".to_string(),
                            data: Some(risk_json),
                        });
                    }

                    // 添加网站到风险的链接
                    links.push(GraphLink {
                        source: id.clone(),
                        target: risk_id.clone(),
                        value: Some(1),
                    });
                }
            }
        },
        _ => {
            return Err(format!("不支持的图谱类型: {}", graph_type));
        }
    }

    Ok(AssetGraphData { nodes, links })
}

// 风险热图的Tauri命令
#[tauri::command]
pub async fn get_risk_heatmap_data(task_id: u32, _view_type: String) -> Result<RiskHeatmapData, String> {
    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => {
            return Err("全局变量未初始化".to_string());
        }
    };
    let pool_clone = Arc::clone(&task_module.read_conn);

    // 获取所有风险
    let risks: Vec<Risk> = query_as("SELECT * FROM risk WHERE task_id = ?")
        .bind(&task_id)
        .fetch_all(&*pool_clone)
        .await
        .map_err(|e| e.to_string())?;

    // 统计信息
    let mut high_risk_count = 0;
    let mut medium_risk_count = 0;
    let mut low_risk_count = 0;

    // 风险数据
    let mut risk_data = Vec::new();
    let mut risk_by_asset = HashMap::new();

    for risk in &risks {
        // 基于资产和风险类型的统计
        let key = format!("{}_{}", risk.ufrom, risk.risk_type);
        let entry = risk_by_asset.entry(key).or_insert((risk.ufrom.clone(), risk.risk_type.clone(), risk.risk_level.clone(), 0, risk.update_at));
        entry.3 += 1;

        // 风险等级统计
        match risk.risk_level.as_str() {
            "high" => high_risk_count += 1,
            "medium" => medium_risk_count += 1,
            "low" => low_risk_count += 1,
            _ => {}
        }
    }

    // 转换为风险数据列表
    for (_, (asset, risk_type, risk_level, count, timestamp)) in risk_by_asset.into_iter() {
        risk_data.push(RiskData {
            asset,
            risk_type,
            risk_level,
            count,
            timestamp: Some(timestamp),
        });
    }

    // 找出高风险和中风险资产
    let mut high_risk_assets = HashMap::new();
    let mut medium_risk_assets = HashMap::new();
    let mut low_risk_assets = HashMap::new();

    for risk in &risks {
        match risk.risk_level.as_str() {
            "high" => {
                *high_risk_assets.entry(risk.ufrom.clone()).or_insert(0) += 1;
            }
            "medium" => {
                *medium_risk_assets.entry(risk.ufrom.clone()).or_insert(0) += 1;
            }
            "low" => {
                *low_risk_assets.entry(risk.ufrom.clone()).or_insert(0) += 1;
            }
            _ => {}
        }
    }

    // 找出最高风险的资产
    let top_high_risk_asset = high_risk_assets.iter()
        .max_by_key(|(_, &count)| count)
        .map(|(asset, _)| asset.clone())
        .unwrap_or_else(|| "无".to_string());

    let top_medium_risk_asset = medium_risk_assets.iter()
        .max_by_key(|(_, &count)| count)
        .map(|(asset, _)| asset.clone())
        .unwrap_or_else(|| "无".to_string());

    let top_low_risk_asset = low_risk_assets.iter()
        .max_by_key(|(_, &count)| count)
        .map(|(asset, _)| asset.clone())
        .unwrap_or_else(|| "无".to_string());

    // 生成风险分析
    let analysis = RiskAnalysis {
        main_risk: if high_risk_count > 0 {
            format!("发现{}个高危风险，主要集中在{}上", high_risk_count, top_high_risk_asset)
        } else if medium_risk_count > 0 {
            format!("未发现高危风险，但存在{}个中危风险", medium_risk_count)
        } else {
            "未发现高危或中危风险".to_string()
        },
        weak_point: if !top_high_risk_asset.eq("无") {
            format!("资产「{}」存在最多的高危风险，建议优先处理", top_high_risk_asset)
        } else if !top_medium_risk_asset.eq("无") {
            format!("资产「{}」存在较多的中危风险，建议关注", top_medium_risk_asset)
        } else {
            "未发现明显的薄弱环节".to_string()
        },
        recommendation: if high_risk_count > 0 {
            "建议立即修复所有高危风险，并定期进行安全扫描".to_string()
        } else if medium_risk_count > 0 {
            "建议在下一次计划维护中修复中危风险".to_string()
        } else {
            "当前风险等级较低，建议继续保持良好的安全实践".to_string()
        },
    };

    let stats = RiskStats {
        high_risk_count,
        medium_risk_count,
        low_risk_count,
        top_high_risk_asset,
        top_medium_risk_asset,
        top_low_risk_asset,
    };

    Ok(RiskHeatmapData {
        risk_data,
        analysis,
        stats,
    })
}

// 打开文件的辅助命令
#[tauri::command]
pub async fn open_file(path: String) -> Result<bool, String> {
    let path = PathBuf::from(path);
    
    if !path.exists() {
        return Err("文件不存在".into());
    }
    
    // 获取AppHandle
    let _app_handle = match crate::APP_HANDLE.get() {
        Some(handle) => handle,
        None => return Err("全局APP_HANDLE未初始化".into()),
    };
    
    // 使用tauri的命令打开文件
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/c", "start", "", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    Ok(true)
}

// 合规报告的Tauri命令
#[tauri::command(rename_all = "snake_case")]
pub async fn generate_compliance_report(
    task_id: u32,
    report_type: String,
    report_title: Option<String>,
    company_info: Option<String>,
    sections: Vec<String>,
    report_format: String,
) -> Result<ReportResult, String> {
    let task_module = match INNERASK_MODULE.get() {
        Some(tm) => tm,
        None => {
            return Err("全局变量未初始化".to_string());
        }
    };
    let pool_clone = Arc::clone(&task_module.read_conn);

    // 获取任务信息
    let task_name: String = query("SELECT name FROM scan_task WHERE id = ?")
        .bind(&task_id)
        .fetch_one(&*pool_clone)
        .await
        .map(|row: sqlx::sqlite::SqliteRow| row.get::<String, _>(0))
        .map_err(|e| e.to_string())?;

    // 获取风险统计
    let (critical, high, medium, low): (i32, i32, i32, i32) = query(
        "SELECT 
            COUNT(CASE WHEN risk_level = 'critical' THEN 1 END),
            COUNT(CASE WHEN risk_level = 'high' THEN 1 END),
            COUNT(CASE WHEN risk_level = 'medium' THEN 1 END),
            COUNT(CASE WHEN risk_level = 'low' THEN 1 END)
        FROM risk WHERE task_id = ?"
    )
    .bind(&task_id)
    .fetch_one(&*pool_clone)
    .await
    .map(|row: sqlx::sqlite::SqliteRow| (
        row.get::<i32, _>(0),
        row.get::<i32, _>(1),
        row.get::<i32, _>(2),
        row.get::<i32, _>(3)
    ))
    .map_err(|e| e.to_string())?;

    // 计算合规评分
    let total_issues = critical + high + medium + low;
    let weighted_score = if total_issues > 0 {
        let weighted_issues = critical * 100 + high * 30 + medium * 10 + low * 2;
        let max_weighted = total_issues * 100;
        ((max_weighted - weighted_issues) as f64 / max_weighted as f64 * 100.0) as i32
    } else {
        100
    };

    // 生成报告章节
    let mut report_sections = Vec::new();
    
    // 根据选择的章节添加内容
    for section in &sections {
        match section.as_str() {
            "overview" => {
                report_sections.push(ReportSection {
                    title: "概述".to_string(),
                    content: format!("本报告基于对「{}」的安全评估结果生成", task_name),
                });
            }
            "assets" => {
                // 获取资产数量
                let (domains, ips, websites): (i64, i64, i64) = query(
                    "SELECT 
                        (SELECT COUNT(*) FROM domain WHERE task_id = ?), 
                        (SELECT COUNT(*) FROM ips WHERE task_id = ?),
                        (SELECT COUNT(*) FROM website WHERE task_id = ?)"
                )
                .bind(&task_id)
                .bind(&task_id)
                .bind(&task_id)
                .fetch_one(&*pool_clone)
                .await
                .map(|row: sqlx::sqlite::SqliteRow| (
                    row.get(0),
                    row.get(1),
                    row.get(2)
                ))
                .map_err(|e| e.to_string())?;

                report_sections.push(ReportSection {
                    title: "资产清单".to_string(),
                    content: format!("本次评估涉及 {} 个域名, {} 个IP地址, {} 个网站", domains, ips, websites),
                });
            }
            "vulnerabilities" => {
                report_sections.push(ReportSection {
                    title: "安全漏洞分析".to_string(),
                    content: format!(
                        "共发现 {} 个安全问题，其中严重问题 {} 个，高危问题 {} 个，中危问题 {} 个，低危问题 {} 个",
                        total_issues, critical, high, medium, low
                    ),
                });
            }
            "risks" => {
                report_sections.push(ReportSection {
                    title: "风险评估".to_string(),
                    content: "根据评估结果，系统当前的整体风险等级为：".to_string() + 
                        if weighted_score >= 90 {
                            "低风险"
                        } else if weighted_score >= 70 {
                            "中低风险"
                        } else if weighted_score >= 50 {
                            "中风险"
                        } else if weighted_score >= 30 {
                            "中高风险"
                        } else {
                            "高风险"
                        },
                });
            }
            "compliance" => {
                let compliance_text = match report_type.as_str() {
                    "djcp" => format!(
                        "根据《网络安全等级保护标准》的要求，系统当前安全状况满足约 {}% 的合规要求",
                        weighted_score
                    ),
                    "gdpr" => format!(
                        "根据《通用数据保护条例》(GDPR) 的要求，系统当前安全状况满足约 {}% 的合规要求",
                        weighted_score
                    ),
                    "iso27001" => format!(
                        "根据 ISO/IEC 27001 信息安全管理标准的要求，系统当前安全状况满足约 {}% 的合规要求",
                        weighted_score
                    ),
                    _ => format!(
                        "系统当前安全状况满足约 {}% 的合规要求",
                        weighted_score
                    ),
                };
                
                report_sections.push(ReportSection {
                    title: "合规评估".to_string(),
                    content: compliance_text,
                });
            }
            "recommendations" => {
                report_sections.push(ReportSection {
                    title: "安全建议".to_string(),
                    content: if critical > 0 {
                        "建议立即修复所有严重和高危安全问题，并在短期内解决中危问题。同时建立定期安全评估机制，确保系统持续符合安全要求。"
                    } else if high > 0 {
                        "建议尽快修复所有高危安全问题，并在计划维护期间解决中危问题。同时加强安全意识培训，提高整体安全水平。"
                    } else if medium > 0 {
                        "建议在下一次系统维护时解决所有中危问题，并定期进行安全扫描，确保系统安全状态稳定。"
                    } else {
                        "当前系统安全状况良好，建议继续保持现有的安全实践，并定期进行安全评估。"
                    }.to_string(),
                });
            }
            _ => {}
        }
    }

    // 生成标题
    let title = report_title.unwrap_or_else(|| {
        let report_type_name = match report_type.as_str() {
            "djcp" => "等级保护",
            "gdpr" => "GDPR",
            "iso27001" => "ISO 27001",
            _ => "安全合规",
        };
        format!("{} - {}合规评估报告", task_name, report_type_name)
    });

    // 生成报告时间
    let now = Local::now();
    let generated_at = now.format("%Y-%m-%d %H:%M:%S").to_string();

    // 模拟文件路径
    let report_ext = match report_format.as_str() {
        "html" => "html",
        "word" => "docx",
        _ => "pdf",
    };
    
    // 创建报告目录
    let reports_dir = std::env::temp_dir().join("rshiled_reports");
    
    if !reports_dir.exists() {
        fs::create_dir_all(&reports_dir).map_err(|e| e.to_string())?;
    }
    
    let file_name = format!("{}_{}.{}", task_name.replace(" ", "_"), now.format("%Y%m%d%H%M%S"), report_ext);
    let file_path = reports_dir.join(&file_name);
    
    // 生成报告内容
    match report_format.as_str() {
        "html" => {
            // 生成HTML报告
            let mut html_content = String::from("<!DOCTYPE html>\n<html>\n<head>\n");
            html_content.push_str(&format!("<title>{}</title>\n", title));
            html_content.push_str("<meta charset=\"UTF-8\">\n");
            html_content.push_str("<style>\n");
            html_content.push_str("body { font-family: Arial, sans-serif; margin: 40px; line-height: 1.6; }\n");
            html_content.push_str("h1 { color: #2c3e50; text-align: center; margin-bottom: 30px; }\n");
            html_content.push_str("h2 { color: #3498db; margin-top: 30px; border-bottom: 1px solid #eee; padding-bottom: 10px; }\n");
            html_content.push_str(".meta { color: #7f8c8d; font-size: 14px; margin-bottom: 30px; text-align: center; }\n");
            html_content.push_str(".summary { background-color: #f8f9fa; padding: 20px; border-radius: 5px; margin-bottom: 30px; }\n");
            html_content.push_str(".stats { display: flex; justify-content: space-around; margin: 30px 0; }\n");
            html_content.push_str(".stat-item { text-align: center; }\n");
            html_content.push_str(".stat-value { font-size: 24px; font-weight: bold; margin-bottom: 5px; }\n");
            html_content.push_str(".critical { color: #e74c3c; }\n");
            html_content.push_str(".high { color: #e67e22; }\n");
            html_content.push_str(".medium { color: #f1c40f; }\n");
            html_content.push_str(".low { color: #2ecc71; }\n");
            html_content.push_str(".score { color: #3498db; }\n");
            html_content.push_str(".footer { text-align: center; margin-top: 50px; color: #7f8c8d; font-size: 12px; }\n");
            html_content.push_str("</style>\n");
            html_content.push_str("</head>\n<body>\n");
            
            // 报告标题
            html_content.push_str(&format!("<h1>{}</h1>\n", title));
            
            // 报告元信息
            html_content.push_str("<div class=\"meta\">\n");
            html_content.push_str(&format!("生成时间：{}<br>\n", generated_at));
            html_content.push_str(&format!("任务名称：{}<br>\n", task_name));
            if let Some(info) = &company_info {
                html_content.push_str(&format!("公司信息：{}<br>\n", info));
            }
            html_content.push_str("</div>\n");
            
            // 报告摘要
            html_content.push_str("<div class=\"summary\">\n");
            html_content.push_str(&format!("<p>{}</p>\n", format!(
                "本报告基于对系统的安全评估，发现共 {} 个安全问题，合规评分为 {}%。",
                total_issues, weighted_score
            )));
            html_content.push_str("</div>\n");
            
            // 统计数据
            html_content.push_str("<div class=\"stats\">\n");
            html_content.push_str("<div class=\"stat-item\">\n");
            html_content.push_str(&format!("<div class=\"stat-value critical\">{}</div>\n", critical));
            html_content.push_str("<div>严重问题</div>\n");
            html_content.push_str("</div>\n");
            
            html_content.push_str("<div class=\"stat-item\">\n");
            html_content.push_str(&format!("<div class=\"stat-value high\">{}</div>\n", high));
            html_content.push_str("<div>高危问题</div>\n");
            html_content.push_str("</div>\n");
            
            html_content.push_str("<div class=\"stat-item\">\n");
            html_content.push_str(&format!("<div class=\"stat-value medium\">{}</div>\n", medium));
            html_content.push_str("<div>中危问题</div>\n");
            html_content.push_str("</div>\n");
            
            html_content.push_str("<div class=\"stat-item\">\n");
            html_content.push_str(&format!("<div class=\"stat-value low\">{}</div>\n", low));
            html_content.push_str("<div>低危问题</div>\n");
            html_content.push_str("</div>\n");
            
            html_content.push_str("<div class=\"stat-item\">\n");
            html_content.push_str(&format!("<div class=\"stat-value score\">{}</div>\n", weighted_score));
            html_content.push_str("<div>合规评分</div>\n");
            html_content.push_str("</div>\n");
            html_content.push_str("</div>\n");
            
            // 报告章节
            for section in &report_sections {
                html_content.push_str(&format!("<h2>{}</h2>\n", section.title));
                html_content.push_str(&format!("<p>{}</p>\n", section.content));
            }
            
            // 页脚
            html_content.push_str("<div class=\"footer\">\n");
            html_content.push_str("由 RShiled 安全合规系统生成\n");
            html_content.push_str("</div>\n");
            
            html_content.push_str("</body>\n</html>");
            
            // 写入HTML文件
            fs::write(&file_path, html_content).map_err(|e| e.to_string())?;
        },
        "word" => {
            // 简单生成一个Word XML格式文档
            let mut docx_content = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
            docx_content.push_str("<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\n");
            docx_content.push_str("<w:body>\n");
            
            // 标题
            docx_content.push_str("<w:p><w:pPr><w:jc w:val=\"center\"/><w:rPr><w:b/><w:sz w:val=\"40\"/></w:rPr></w:pPr>");
            docx_content.push_str(&format!("<w:r><w:rPr><w:b/><w:sz w:val=\"40\"/></w:rPr><w:t>{}</w:t></w:r></w:p>\n", title));
            
            // 元信息
            docx_content.push_str("<w:p><w:pPr><w:jc w:val=\"center\"/></w:pPr>");
            docx_content.push_str(&format!("<w:r><w:t>生成时间：{}</w:t></w:r></w:p>\n", generated_at));
            
            docx_content.push_str("<w:p><w:pPr><w:jc w:val=\"center\"/></w:pPr>");
            docx_content.push_str(&format!("<w:r><w:t>任务名称：{}</w:t></w:r></w:p>\n", task_name));
            
            // 摘要
            docx_content.push_str("<w:p><w:pPr><w:rPr><w:b/></w:rPr></w:pPr>");
            docx_content.push_str(&format!("<w:r><w:rPr><w:b/></w:rPr><w:t>摘要</w:t></w:r></w:p>\n"));
            
            docx_content.push_str("<w:p>");
            docx_content.push_str(&format!("<w:r><w:t>{}</w:t></w:r></w:p>\n", format!(
                "本报告基于对系统的安全评估，发现共 {} 个安全问题，合规评分为 {}%。",
                total_issues, weighted_score
            )));
            
            // 章节
            for section in &report_sections {
                docx_content.push_str("<w:p><w:pPr><w:rPr><w:b/><w:sz w:val=\"32\"/></w:rPr></w:pPr>");
                docx_content.push_str(&format!("<w:r><w:rPr><w:b/><w:sz w:val=\"32\"/></w:rPr><w:t>{}</w:t></w:r></w:p>\n", section.title));
                
                docx_content.push_str("<w:p>");
                docx_content.push_str(&format!("<w:r><w:t>{}</w:t></w:r></w:p>\n", section.content));
            }
            
            docx_content.push_str("</w:body>\n</w:document>");
            
            // 注意：这不是一个有效的.docx文件，只是一个简化的XML示例
            // 实际应用中，应使用专门的库生成Word文档
            fs::write(&file_path, docx_content).map_err(|e| e.to_string())?;
        },
        _ => { // PDF 或其他格式
            // 生成简单的文本报告作为替代
            let mut text_content = String::new();
            
            // 报告标题
            text_content.push_str(&format!("{}\n\n", title));
            
            // 元信息
            text_content.push_str(&format!("生成时间：{}\n", generated_at));
            text_content.push_str(&format!("任务名称：{}\n", task_name));
            if let Some(info) = &company_info {
                text_content.push_str(&format!("公司信息：{}\n", info));
            }
            text_content.push_str("\n");
            
            // 摘要
            text_content.push_str("摘要：\n");
            text_content.push_str(&format!("{}\n\n", format!(
                "本报告基于对系统的安全评估，发现共 {} 个安全问题，合规评分为 {}%。",
                total_issues, weighted_score
            )));
            
            // 统计数据
            text_content.push_str(&format!("严重问题: {}\n", critical));
            text_content.push_str(&format!("高危问题: {}\n", high));
            text_content.push_str(&format!("中危问题: {}\n", medium));
            text_content.push_str(&format!("低危问题: {}\n", low));
            text_content.push_str(&format!("合规评分: {}%\n\n", weighted_score));
            
            // 章节内容
            for section in &report_sections {
                text_content.push_str(&format!("## {}\n", section.title));
                text_content.push_str(&format!("{}\n\n", section.content));
            }
            
            // 注意：这不是一个真正的PDF文件，只是文本
            // 实际应用中，应使用PDF生成库
            fs::write(&file_path, text_content).map_err(|e| e.to_string())?;
        }
    }
    
    // 创建报告预览
    let preview = ReportPreview {
        title,
        type_: report_type,
        generated_at,
        task_name,
        sections: report_sections,
        summary: format!(
            "本报告基于对系统的安全评估，发现共 {} 个安全问题，合规评分为 {}%。",
            total_issues, weighted_score
        ),
        stats: ReportStats {
            critical_issues: critical,
            high_issues: high,
            medium_issues: medium,
            low_issues: low,
            compliance_score: weighted_score,
        },
    };

    Ok(ReportResult {
        preview,
        file_path: file_path.to_str().unwrap_or_default().to_string(),
    })
}
