pub mod html;
pub mod javascript;

use anyhow::Result;
use std::collections::HashMap;

pub use html::HtmlAstAnalyzer;
pub use javascript::JsAstAnalyzer;

/// AST分析器特征
pub trait AstAnalyzer {
    /// 分析AST结构
    fn analyze(&self, content: &str) -> Result<AstAnalysisResult>;
    
    /// 检测AST结构是否被破坏
    fn detect_injection(&self, original: &AstAnalysisResult, modified: &AstAnalysisResult) -> Result<InjectionResult>;
}

/// AST分析结果
#[derive(Debug, Clone)]
pub struct AstAnalysisResult {
    /// AST节点数量
    pub node_count: usize,
    /// AST深度
    pub depth: usize,
    /// AST结构哈希
    pub structure_hash: String,
    /// 节点类型统计
    pub node_types: HashMap<String, usize>,
    /// 是否有语法错误
    pub has_syntax_error: bool,
    /// 语法错误信息
    pub syntax_error: Option<String>,
    /// 危险节点
    pub dangerous_nodes: Vec<DangerousNode>,
}

/// 危险节点
#[derive(Debug, Clone)]
pub struct DangerousNode {
    /// 节点类型
    pub node_type: String,
    /// 节点内容
    pub content: String,
    /// 节点位置
    pub location: Option<NodeLocation>,
    /// 危险级别
    pub risk_level: RiskLevel,
    /// 危险原因
    pub reason: String,
}

/// 节点位置
#[derive(Debug, Clone)]
pub struct NodeLocation {
    /// 起始行
    pub start_line: usize,
    /// 起始列
    pub start_column: usize,
    /// 结束行
    pub end_line: usize,
    /// 结束列
    pub end_column: usize,
}

/// 风险级别
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    /// 低风险
    Low,
    /// 中风险
    Medium,
    /// 高风险
    High,
    /// 严重风险
    Critical,
}

/// 注入结果
#[derive(Debug, Clone)]
pub struct InjectionResult {
    /// 是否检测到注入
    pub detected: bool,
    /// 注入类型
    pub injection_type: Option<String>,
    /// 风险级别
    pub risk_level: Option<RiskLevel>,
    /// 注入点
    pub injection_point: Option<String>,
    /// 注入内容
    pub injection_content: Option<String>,
    /// 注入位置
    pub location: Option<NodeLocation>,
    /// 详细信息
    pub details: Option<String>,
}

/// 创建HTML AST分析器
pub fn create_html_analyzer() -> impl AstAnalyzer {
    html::HtmlAstAnalyzer::new()
}

/// 创建JavaScript AST分析器
pub fn create_js_analyzer() -> impl AstAnalyzer {
    javascript::JsAstAnalyzer::new()
} 