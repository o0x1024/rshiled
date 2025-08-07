use oxc_allocator::Allocator;
use oxc_ast::ast::CallExpression;
use oxc_ast_visit::{walk, Visit};
use oxc_parser::Parser;
use oxc_span::SourceType;

#[derive(Clone)]
pub struct AstAnalyzer {}

#[derive(Default)]
pub struct DangerousPatternVisitor {
    pub dangerous_patterns: Vec<String>,
    pub current_context: Option<String>,
}

impl<'a> Visit<'a> for DangerousPatternVisitor {
    // 检查函数调用
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(member_expr) = it.callee.as_member_expression() {
            if let Some(object) = member_expr.object().get_identifier_reference() {
                // 检查危险的 DOM 操作
                if object.name == "document" {
                    if let Some(property) = member_expr.static_property_name() {
                        match property {
                            "write" | "writeln" => {
                                self.dangerous_patterns
                                    .push("document.write() usage detected".to_string());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        walk::walk_call_expression(self, it);
    }

    fn visit_member_expression(&mut self, it: &oxc_ast::ast::MemberExpression<'a>) {
        if let Some(object) = it.object().get_identifier_reference() {
            if let Some(property) = it.static_property_name() {
                match (object.name.as_str(), property) {
                    ("eval", _) => {
                        self.dangerous_patterns
                            .push("eval() usage detected".to_string());
                    }
                    ("Function", "constructor") => {
                        self.dangerous_patterns
                            .push("Function constructor usage detected".to_string());
                    }
                    _ => {}
                }
            }
        }
        walk::walk_member_expression(self, it);
    }
}

impl AstAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn analyze_js(&self, js_code: &str) -> Vec<String> {
        let allocator = Allocator::default();
        let source_type = SourceType::default();

        // 使用 oxc 解析 JavaScript 代码
        let ret = Parser::new(&allocator, &js_code, source_type).parse();

        let mut myvisit = DangerousPatternVisitor::default();
        myvisit.visit_program(&ret.program);
        myvisit.dangerous_patterns
    }

    pub fn analyze_html(&self, html_code: &str) -> Vec<String> {
        let mut dangerous_patterns = Vec::new();

        // 检查内联事件处理器
        let event_handlers = [
            "onclick",
            "onmouseover",
            "onload",
            "onerror",
            "onmouseout",
            "onkeyup",
            "onkeydown",
            "onsubmit",
            "onmouseenter",
            "onmouseleave",
        ];

        for handler in event_handlers {
            if html_code.contains(handler) {
                dangerous_patterns.push(format!("Inline event handler {} detected", handler));
            }
        }

        // 检查危险的 meta 标签
        if html_code.contains(r#"http-equiv="refresh""#) {
            dangerous_patterns.push("Meta refresh detected".to_string());
        }

        // 检查 base 标签劫持
        if html_code.contains("<base") {
            dangerous_patterns.push("Base tag manipulation detected".to_string());
        }

        dangerous_patterns
    }
}
