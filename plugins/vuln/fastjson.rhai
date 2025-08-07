// FastJSON Deserialization Vulnerability Detector
// Author: Security Expert
// Version: 1.0
// Description: Detects FastJSON deserialization vulnerabilities (CVE-2017-18349 and others)
// References: ["CVE-2017-18349", "CVE-2019-14489", "CVE-2020-14040"]
// Type: vuln
// Severity: high

// Plugin metadata - the type field will be used as the namespace
// Plugin ID will be "vuln:fastjson_deserialization_detector"
fn get_manifest() {
    let manifest = #{
        name: "fastjson反序列化",
        author: "Security Expert",
        version: "1.0",
        description: "Detects potential FastJSON deserialization vulnerabilities based on URL patterns",
        rtype: "deserialization",
        severity:"high",
        references: ["CVE-2016-4437", "CVE-2016-9244", "CVE-2016-1000333"],
        params: [
            #{
                name: "Check endpoints",
                key: "check_all",
                type: "boolean",
                required: false,
                
                description: "Check all common FastJSON endpoints"
            },
            #{
                name: "ASD",
                key: "ASD",
                type: "string",
                required: true,
                default_value: "ASDSD",
                description: "ASD"
            }
        ],
        result_fields: [
            #{
                name: "Potential Vulnerability",
                key: "vulnerable",
                type: "boolean",
                description: "Whether the target might be vulnerable to FastJSON deserialization"
            },
            #{
                name: "Risk Level",
                key: "risk_level",
                type: "string",
                description: "Assessed risk level based on URL patterns"
            },
            #{
                name: "Recommendations",
                key: "recommendations",
                type: "string",
                description: "Recommendations for further testing"
            }
        ]
    };
    
    // Convert to JSON string
    manifest.to_json()
}

// Main analysis function
fn analyze(ctx) {
    // Parse context
    let target = "";
    let params = #{};
    
    if is_string(ctx) {
        // Try to parse JSON string
        let parsed = parse_json(ctx);
        if is_map(parsed) {
            target = parsed.target;
            if parsed.params != () && is_map(parsed.params) {
                params = parsed.params;
            }
        }
    } else if is_map(ctx) {
        target = ctx.target;
        if ctx.custom_params != () && is_map(ctx.custom_params) {
            params = ctx.custom_params;
        }
    }
    
    let check_all = true;
    if params.check_all != () {
        check_all = params.check_all;
    }
    
    // print_info(`Analyzing ${target} for potential FastJSON deserialization vulnerabilities...`);
    
    // Results tracking
    let potential_vulnerable = false;
    let risk_level = "Low";
    let recommendations = [];
    
    // Common FastJSON API endpoints and patterns
    let fastjson_patterns = [
        "/api/json",
        "/json/api",
        "/api/v1/json",
        "/rest/api",
        "/api/rest",
        "/data/json",
        "/json/data",
        "/api/fastjson",
        "/fastjson",
        "/json"
    ];
    
    // Common FastJSON frameworks and technologies
    let fastjson_technologies = [
        "springboot",
        "spring",
        "fastjson",
        "alibaba",
        "java",
        "jsonapi"
    ];
    
    // Check if URL contains any of the FastJSON patterns
    let target_lower = target.to_lower();
    
    // Check for FastJSON patterns in URL
    for pattern in fastjson_patterns {
        if target_lower.contains(pattern) {
            print_info(`Detected potential FastJSON endpoint pattern: ${pattern}`);
            potential_vulnerable = true;
            risk_level = "Medium";
            recommendations.push(`The URL contains a pattern (${pattern}) commonly associated with JSON APIs`);
            break;
        }
    }
    
    // Check for FastJSON technologies in URL
    for tech in fastjson_technologies {
        if target_lower.contains(tech) {
            print_info(`Detected potential FastJSON technology indicator: ${tech}`);
            if !potential_vulnerable {
                potential_vulnerable = true;
            }
            
            if risk_level == "Low" {
                risk_level = "Medium";
            }
            
            recommendations.push(`The URL contains a technology indicator (${tech}) that may use FastJSON`);
            break;
        }
    }
    
    // Check for Java-specific endpoints which often use FastJSON
    if target_lower.contains(".do") || target_lower.contains(".action") || target_lower.contains(".jsp") {
        // print_info("Detected Java-specific endpoint which may use FastJSON");
        if !potential_vulnerable {
            potential_vulnerable = true;
        }
        
        if risk_level == "Low" {
            risk_level = "Medium";
        }
        
        recommendations.push("The URL contains Java-specific endpoints which often use FastJSON for deserialization");
    }
    
    // Common recommendations for all potential vulnerabilities
    if potential_vulnerable {
        recommendations.push("Consider manual testing for FastJSON deserialization vulnerabilities");
        recommendations.push("Check application for FastJSON library version (recommended: 1.2.70+)");
        recommendations.push("Test with autotype-specific payloads if manual testing is permitted");
    } else {
        recommendations.push("No common FastJSON patterns detected in URL");
        recommendations.push("URL does not appear to be a typical FastJSON endpoint");
    }
    
    return create_result(potential_vulnerable, risk_level, recommendations.join("\n")).to_json();
}

// Helper function to create the result
fn create_result(vulnerable, risk_level, recommendations) {
    #{
        vulnerable: vulnerable,
        risk_level: risk_level,
        recommendations: recommendations
    }
}

// Parse JSON helper function
fn parse_json(json_str) {
    try {
        let parsed = JSON::parse(json_str);
        parsed
    } catch {
        #{}
    }
}

// Helper for getting absolute value
fn abs(x) {
    if x < 0 {
        return -x;
    }
    return x;
} 