use crate::scan::ast::{AstAnalysisResult, AstAnalyzer, DangerousNode, InjectionResult, RiskLevel};
use anyhow::Result;
use kuchiki::traits::*;
use kuchiki::NodeRef;
use log::debug;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

/// HTML AST分析器
pub struct HtmlAstAnalyzer {
    /// 危险标签
    dangerous_tags: HashSet<String>,
    /// 危险属性
    dangerous_attrs: HashSet<String>,
    /// 危险事件处理器
    dangerous_events: HashSet<String>,
    /// 全局监控标签列表
    watched_tags: RwLock<HashSet<String>>,
}

impl HtmlAstAnalyzer {
    /// 创建新的HTML AST分析器
    pub fn new() -> Self {
        let mut dangerous_tags = HashSet::new();
        dangerous_tags.insert("script".to_string());
        dangerous_tags.insert("iframe".to_string());
        dangerous_tags.insert("object".to_string());
        dangerous_tags.insert("embed".to_string());
        dangerous_tags.insert("applet".to_string());
        dangerous_tags.insert("base".to_string());
        dangerous_tags.insert("meta".to_string());
        
        let mut dangerous_attrs = HashSet::new();
        dangerous_attrs.insert("src".to_string());
        dangerous_attrs.insert("href".to_string());
        dangerous_attrs.insert("style".to_string());
        dangerous_attrs.insert("formaction".to_string());
        
        let mut dangerous_events = HashSet::new();
        dangerous_events.insert("onclick".to_string());
        dangerous_events.insert("onload".to_string());
        dangerous_events.insert("onerror".to_string());
        dangerous_events.insert("onmouseover".to_string());
        dangerous_events.insert("onmouseout".to_string());
        dangerous_events.insert("onmousedown".to_string());
        dangerous_events.insert("onmouseup".to_string());
        dangerous_events.insert("onkeydown".to_string());
        dangerous_events.insert("onkeyup".to_string());
        dangerous_events.insert("onkeypress".to_string());
        dangerous_events.insert("onchange".to_string());
        dangerous_events.insert("onsubmit".to_string());
        dangerous_events.insert("onfocus".to_string());
        dangerous_events.insert("onblur".to_string());
        
        // 初始化监控标签列表，默认为空
        let watched_tags = RwLock::new(HashSet::new());
        
        Self {
            dangerous_tags,
            dangerous_attrs,
            dangerous_events,
            watched_tags,
        }
    }
    
    /// 设置监控标签列表
    pub fn set_watched_tags(&self, tags: Vec<String>) {
        let mut watched = self.watched_tags.write().unwrap();
        watched.clear();
        for tag in tags {
            watched.insert(tag.to_lowercase());
        }
        // debug!("已设置监控标签列表: {:?}", *watched);
    }
    
    /// 添加监控标签
    pub fn add_watched_tag(&self, tag: &str) {
        let mut watched = self.watched_tags.write().unwrap();
        watched.insert(tag.to_lowercase());
        // debug!("已添加监控标签: {}", tag);
    }
    
    /// 清除监控标签列表
    pub fn clear_watched_tags(&self) {
        let mut watched = self.watched_tags.write().unwrap();
        watched.clear();
        debug!("已清除监控标签列表");
    }
    
    /// 获取当前监控标签列表
    pub fn get_watched_tags(&self) -> Vec<String> {
        let watched = self.watched_tags.read().unwrap();
        watched.iter().cloned().collect()
    }
    
    /// 检查标签是否在监控列表中
    fn is_tag_watched(&self, tag_name: &str) -> bool {
        let watched = self.watched_tags.read().unwrap();
        
        // 如果监控列表为空，则监控所有标签
        if watched.is_empty() {
            return true;
        }
        
        watched.contains(&tag_name.to_lowercase())
    }
    
    /// 计算AST深度
    fn calculate_depth(&self, node: &NodeRef) -> usize {
        let mut max_depth = 0;
        
        for child in node.children() {
            let child_depth = self.calculate_depth(&child);
            if child_depth > max_depth {
                max_depth = child_depth;
            }
        }
        
        max_depth + 1
    }
    
    /// 计算节点类型统计
    fn calculate_node_types(&self, node: &NodeRef, node_types: &mut HashMap<String, usize>) {
        if let Some(element) = node.as_element() {
            let name = element.name.local.to_string().to_lowercase();
            
            // 只统计监控列表中的标签
            if self.is_tag_watched(&name) {
                *node_types.entry(name).or_insert(0) += 1;
            }
        }
        
        for child in node.children() {
            self.calculate_node_types(&child, node_types);
        }
    }
    
    /// 计算结构哈希
    fn calculate_structure_hash(&self, node: &NodeRef) -> String {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash_node(node, &mut hasher);
        format!("{:x}", hasher.finish())
    }
    
    /// 哈希节点
    fn hash_node(&self, node: &NodeRef, hasher: &mut impl Hasher) {
        if let Some(element) = node.as_element() {
            let name = element.name.local.to_string().to_lowercase();
            
            // 只哈希监控列表中的标签
            if self.is_tag_watched(&name) {
                name.hash(hasher);
                
                let attrs = element.attributes.borrow();
                let mut attr_names: Vec<_> = attrs.map.keys().collect();
                attr_names.sort();
                
                for name in attr_names {
                    name.hash(hasher);
                }
            }
        } else if let Some(_text) = node.as_text() {
            "text".hash(hasher);
            // 不哈希文本内容，只哈希结构
        } else if node.as_comment().is_some() {
            "comment".hash(hasher);
        } else if node.as_doctype().is_some() {
            "doctype".hash(hasher);
        }
        
        let mut children: Vec<_> = node.children().collect();
        children.sort_by_key(|child| {
            if let Some(element) = child.as_element() {
                element.name.local.to_string()
            } else if child.as_text().is_some() {
                "text".to_string()
            } else if child.as_comment().is_some() {
                "comment".to_string()
            } else if child.as_doctype().is_some() {
                "doctype".to_string()
            } else {
                "unknown".to_string()
            }
        });
        
        for child in children {
            self.hash_node(&child, hasher);
        }
    }
    
    /// 检测危险节点
    fn detect_dangerous_nodes(&self, node: &NodeRef, dangerous_nodes: &mut Vec<DangerousNode>) {
        if let Some(element) = node.as_element() {
            let name = element.name.local.to_string().to_lowercase();
            
            // 只检测监控列表中的标签
            if !self.is_tag_watched(&name) {
                // 递归检查子节点，即使当前节点不在监控列表中
                for child in node.children() {
                    self.detect_dangerous_nodes(&child, dangerous_nodes);
                }
                return;
            }
            
            // 检查危险标签
            if self.dangerous_tags.contains(&name) {
                dangerous_nodes.push(DangerousNode {
                    node_type: format!("tag:{}", name),
                    content: format!("<{}>", name),
                    location: None, // HTML解析器不提供位置信息
                    risk_level: if name == "script" {
                        RiskLevel::Critical
                    } else {
                        RiskLevel::Medium
                    },
                    reason: format!("危险的HTML标签: {}", name),
                });
            }
            
            // 检查危险属性
            let attrs = element.attributes.borrow();
            for (attr_name, attr_value) in attrs.map.iter() {
                let attr_name_lower = attr_name.local.to_string().to_lowercase();
                
                // 检查危险属性
                if self.dangerous_attrs.contains(&attr_name_lower) {
                    let value = attr_value.value.to_string();
                    
                    // 检查javascript:协议
                    if (attr_name_lower == "src" || attr_name_lower == "href") && 
                       value.trim().to_lowercase().starts_with("javascript:") {
                        dangerous_nodes.push(DangerousNode {
                            node_type: format!("attr:{}:javascript", attr_name_lower),
                            content: format!("{}=\"{}\"", attr_name_lower, value),
                            location: None,
                            risk_level: RiskLevel::Critical,
                            reason: format!("JavaScript协议在{}属性中: {}", attr_name_lower, value),
                        });
                    } else if attr_name_lower == "style" && value.contains("expression") {
                        dangerous_nodes.push(DangerousNode {
                            node_type: format!("attr:{}:expression", attr_name_lower),
                            content: format!("{}=\"{}\"", attr_name_lower, value),
                            location: None,
                            risk_level: RiskLevel::High,
                            reason: format!("CSS表达式在style属性中: {}", value),
                        });
                    }
                }
                
                // 检查危险事件处理器
                if self.dangerous_events.contains(&attr_name_lower) {
                    dangerous_nodes.push(DangerousNode {
                        node_type: format!("event:{}", attr_name_lower),
                        content: format!("{}=\"{}\"", attr_name_lower, attr_value.value),
                        location: None,
                        risk_level: RiskLevel::High,
                        reason: format!("危险的事件处理器: {}", attr_name_lower),
                    });
                }
                
                // 检查data-*属性中的javascript:
                if attr_name_lower.starts_with("data-") {
                    let value = attr_value.value.to_string().to_lowercase();
                    if value.contains("javascript:") {
                        dangerous_nodes.push(DangerousNode {
                            node_type: format!("attr:{}:javascript", attr_name_lower),
                            content: format!("{}=\"{}\"", attr_name_lower, attr_value.value),
                            location: None,
                            risk_level: RiskLevel::Medium,
                            reason: format!("JavaScript协议在data-*属性中: {}", attr_value.value),
                        });
                    }
                }
            }
        }
        
        // 递归检查子节点
        for child in node.children() {
            self.detect_dangerous_nodes(&child, dangerous_nodes);
        }
    }
}

impl AstAnalyzer for HtmlAstAnalyzer {
    fn analyze(&self, content: &str) -> Result<AstAnalysisResult> {
        // 解析HTML
        let document = kuchiki::parse_html().one(content);
        
        // 计算节点数量
        let node_count = document.descendants().count();
        
        // 计算AST深度
        let depth = self.calculate_depth(&document);
        
        // 计算节点类型统计
        let mut node_types = HashMap::new();
        self.calculate_node_types(&document, &mut node_types);
        
        // 计算结构哈希
        let structure_hash = self.calculate_structure_hash(&document);
        
        // 检测危险节点
        let mut dangerous_nodes = Vec::new();
        self.detect_dangerous_nodes(&document, &mut dangerous_nodes);
        
        self.set_watched_tags(vec!["audio".to_string()]);
        // 输出当前监控的标签
        let watched_tags = self.get_watched_tags();
        if !watched_tags.is_empty() {
            // debug!("当前监控标签: {:?}", watched_tags);
        }
        
        Ok(AstAnalysisResult {
            node_count,
            depth,
            structure_hash,
            node_types,
            has_syntax_error: false,
            syntax_error: None,
            dangerous_nodes,
        })
    }
    
    fn detect_injection(&self, original: &AstAnalysisResult, modified: &AstAnalysisResult) -> Result<InjectionResult> {
        // 检查结构是否被破坏
        let structure_changed = original.structure_hash != modified.structure_hash;
        
        // 检查是否有新的危险节点
        let mut new_dangerous_nodes = Vec::new();
        
        for node in &modified.dangerous_nodes {
            let mut is_new = true;
            
            for orig_node in &original.dangerous_nodes {
                if node.node_type == orig_node.node_type && node.content == orig_node.content {
                    is_new = false;
                    break;
                }
            }
            
            if is_new {
                new_dangerous_nodes.push(node.clone());
            }
        }
        
        // 如果结构被破坏或有新的危险节点，则认为检测到注入
        let detected = structure_changed || !new_dangerous_nodes.is_empty();
        
        // 确定风险级别
        let risk_level = if !new_dangerous_nodes.is_empty() {
            // 使用最高风险级别
            let mut highest_risk = RiskLevel::Low;
            
            for node in &new_dangerous_nodes {
                if node.risk_level > highest_risk {
                    highest_risk = node.risk_level.clone();
                }
            }
            
            Some(highest_risk)
        } else if structure_changed {
            Some(RiskLevel::Medium)
        } else {
            None
        };
        
        // 构建详细信息
        let details = if detected {
            let mut details = String::new();
            
            if structure_changed {
                details.push_str("HTML结构被修改。\n");
                details.push_str(&format!("原始结构哈希: {}\n", original.structure_hash));
                details.push_str(&format!("修改后结构哈希: {}\n", modified.structure_hash));
            }
            
            if !new_dangerous_nodes.is_empty() {
                details.push_str("检测到新的危险节点:\n");
                
                for node in &new_dangerous_nodes {
                    details.push_str(&format!("- 类型: {}\n", node.node_type));
                    details.push_str(&format!("  内容: {}\n", node.content));
                    details.push_str(&format!("  风险级别: {:?}\n", node.risk_level));
                    details.push_str(&format!("  原因: {}\n", node.reason));
                }
            }
            
            Some(details)
        } else {
            None
        };
        
        // 构建注入内容
        let injection_content = if !new_dangerous_nodes.is_empty() {
            Some(new_dangerous_nodes[0].content.clone())
        } else {
            None
        };
        
        Ok(InjectionResult {
            detected,
            injection_type: if detected { Some("XSS".to_string()) } else { None },
            risk_level,
            injection_point: None, // HTML解析器不提供注入点信息
            injection_content,
            location: None, // HTML解析器不提供位置信息
            details,
        })
    }
} 