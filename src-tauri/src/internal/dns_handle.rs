use regex::Regex;
use std::collections::HashSet;

pub fn match_subdomains(domain: &str, html: &str) -> Vec<String> {
    let mut results = HashSet::new();

    let regexp = format!(
        r#"(?:>|"|'|=|,)(?:http://|https://)?(?:[a-z0-9](?:[a-z0-9\-]{{0,61}}[a-z0-9])?\.)*{}"#,
        regex::escape(domain)
    );
    let re = Regex::new(&regexp).unwrap();

    for cap in re.captures_iter(html) {
        if let Some(matched) = cap.get(0) {
            let cleaned = matched
                .as_str()
                .trim_start_matches(|c| c == '>' || c == '"' || c == '\'' || c == '=' || c == ',');
            let cleaned = cleaned
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .to_lowercase();
            results.insert(cleaned);
        }
    }

    results.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_matching() {
        let url = format!("https://api.certspotter.com/v1/issuances?domain={}&include_subdomains=true&expand=dns_names", "mgtv.com");
        // Make the HTTP request
        let response = reqwest::blocking::get(&url).unwrap();
        let html = response.text().unwrap();
        let result = match_subdomains("mgtv.com", &html);
        println!("{:?}", result);

        // if let Either::Left(set) = result {
        //     assert_eq!(set.len(), 2);
        //     assert!(set.contains("test.example.com"));
        //     assert!(set.contains("sub.example.com"));
        // }
    }
}
