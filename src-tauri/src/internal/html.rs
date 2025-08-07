use scraper::{Html, Selector};

// 从 HTML 内容中提取 JavaScript 脚本
pub fn extract_js_from_html(html: &str) -> String {
    let document = Html::parse_document(html);
    let script_selector = Selector::parse("script").unwrap();

    let mut scripts = String::new();

    // 遍历所有 <script> 标签
    for element in document.select(&script_selector) {
        if let Some(_) = element.value().attr("src") {
            // 外部脚本：提取 src 属性
            // scripts.push(format!("External Script: {}", src));
        } else if let Some(script_content) = element.text().next() {
            // 内联脚本：提取标签内的文本内容
            scripts = format!("{}", script_content);
        }
    }

    scripts
}
