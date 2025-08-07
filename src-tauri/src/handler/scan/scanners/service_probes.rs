use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use log::{debug, warn, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NmapVersionInfo {
    pub product_capture: Option<String>,    // p/
    pub version_capture: Option<String>,    // v/
    pub info_capture: Option<String>,       // i/
    pub hostname_capture: Option<String>,   // h/
    pub ostype_capture: Option<String>,     // o/
    pub devicetype_capture: Option<String>, // d/
}

#[derive(Debug, Clone)]
pub struct NmapMatch {
    pub service: String,
    pub pattern_raw: String,
    pub pattern_compiled: Regex,
    pub version_info: NmapVersionInfo,
    pub soft_match: bool, // Differentiate between 'match' and 'softmatch'
}

#[derive(Debug, Clone)]
pub struct NmapProbe {
    pub protocol: String, // "TCP" or "UDP"
    pub name: String,
    pub data: Vec<u8>, // Raw bytes to send
    pub ports: Option<String>,
    pub sslports: Option<String>,
    pub totalwaitms: Option<u32>,
    pub tcpwrappedms: Option<u32>,
    pub rarity: Option<u8>,
    pub fallback: Option<String>,
    pub matches: Vec<NmapMatch>,
}

#[derive(Debug, Clone)]
pub struct NmapServiceProbes {
    pub probes: Vec<NmapProbe>,
    pub exclude_directive: Option<String>, // Example: "Exclude T:12345,U:54321"
}

impl NmapServiceProbes {
    pub fn new() -> Self {
        NmapServiceProbes {
            probes: Vec::new(),
            exclude_directive: None,
        }
    }

    // This is a complex function and will be implemented step-by-step.
    // For now, it's a placeholder.
    pub fn load_from_file(file_path: &str) -> Result<Self, String> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(format!("Nmap service probes file not found: {}", file_path));
        }

        let file = File::open(path).map_err(|e| format!("Failed to open probes file: {}", e))?;
        let reader = BufReader::new(file);

        let mut probes_data = NmapServiceProbes::new();
        let mut current_probe: Option<NmapProbe> = None;

        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l.trim().to_string(),
                Err(e) => {
                    error!("Error reading line from probes file: {}", e);
                    continue;
                }
            };

            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if line.starts_with("Exclude ") {
                probes_data.exclude_directive = Some(line.strip_prefix("Exclude ").unwrap_or("").to_string());
            } else if line.starts_with("Probe ") {
                if let Some(probe) = current_probe.take() {
                    probes_data.probes.push(probe);
                }
                current_probe = Self::parse_probe_line(&line);
            } else if let Some(ref mut probe) = current_probe {
                if line.starts_with("match ") || line.starts_with("softmatch ") {
                    match Self::parse_match_line(&line) {
                        Ok(nmap_match) => probe.matches.push(nmap_match),
                        Err(e) => warn!("Failed to parse match line '{}': {}", line, e),
                    }
                } else if line.starts_with("ports ") {
                    probe.ports = Some(line.strip_prefix("ports ").unwrap_or("").to_string());
                } else if line.starts_with("sslports ") {
                    probe.sslports = Some(line.strip_prefix("sslports ").unwrap_or("").to_string());
                } else if line.starts_with("totalwaitms ") {
                    probe.totalwaitms = line.strip_prefix("totalwaitms ").and_then(|s| s.parse().ok());
                } else if line.starts_with("tcpwrappedms ") {
                    probe.tcpwrappedms = line.strip_prefix("tcpwrappedms ").and_then(|s| s.parse().ok());
                } else if line.starts_with("rarity ") {
                    probe.rarity = line.strip_prefix("rarity ").and_then(|s| s.parse().ok());
                } else if line.starts_with("fallback ") {
                    probe.fallback = Some(line.strip_prefix("fallback ").unwrap_or("").to_string());
                }
            }
        }
        if let Some(probe) = current_probe.take() {
            probes_data.probes.push(probe);
        }
        
        debug!("Loaded {} Nmap service probes.", probes_data.probes.len());
        Ok(probes_data)
    }

    fn parse_probe_line(line: &str) -> Option<NmapProbe> {
        // Example: Probe TCP NULL q||
        // Example: Probe TCP GetRequest q|GET / HTTP/1.0\r\n\r\n|
        let parts: Vec<&str> = line.splitn(4, ' ').collect();
        if parts.len() < 4 {
            warn!("Invalid Probe line format: {}", line);
            return None;
        }

        let protocol = parts[1].to_string();
        let name = parts[2].to_string();
        let data_str = parts[3];

        if !data_str.starts_with("q|") || !data_str.ends_with('|') {
            warn!("Invalid probe data format: {}", data_str);
            return None;
        }
        
        let actual_data_str = &data_str[2..data_str.len()-1];
        let data_bytes = Self::parse_nmap_probe_string(actual_data_str);


        Some(NmapProbe {
            protocol,
            name,
            data: data_bytes,
            ports: None,
            sslports: None,
            totalwaitms: None,
            tcpwrappedms: None,
            rarity: None,
            fallback: None,
            matches: Vec::new(),
        })
    }
    
    // Parses Nmap's q|| string format, handling \xHH, \0, \n, \r, \t, \\
    fn parse_nmap_probe_string(s: &str) -> Vec<u8> {
        let mut result = Vec::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('x') => {
                        let h1 = chars.next().unwrap_or('0');
                        let h2 = chars.next().unwrap_or('0');
                        if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                            result.push(byte);
                        } else {
                            result.push(b'x'); // literal 'x' if not valid hex
                            if h1 != '0' { result.push(h1 as u8); } // should push as char bytes
                            if h2 != '0' { result.push(h2 as u8); }
                        }
                    }
                    Some('0') => result.push(0),
                    Some('n') => result.push(b'\n'),
                    Some('r') => result.push(b'\r'),
                    Some('t') => result.push(b'\t'),
                    Some('\\') => result.push(b'\\'),
                    Some(other) => {
                        result.push(b'\\');
                        result.push(other as u8);
                    }
                    None => result.push(b'\\'),
                }
            } else {
                result.push(c as u8);
            }
        }
        result
    }


    fn parse_match_line(line: &str) -> Result<NmapMatch, String> {
        // Example: match http m|^HTTP/1\.[01] \d\d\d .*\r\nServer: ([^\r\n]+)| p/Apache httpd/ v/$1/
        // Example: softmatch ftp m/^220.*Microsoft Ftp Service/ i/logs disabled/
        let soft_match = line.starts_with("softmatch ");
        let relevant_part = if soft_match {
            line.strip_prefix("softmatch ").unwrap_or("")
        } else {
            line.strip_prefix("match ").unwrap_or("")
        };

        let parts: Vec<&str> = relevant_part.splitn(3, ' ').collect();
        if parts.len() < 2 {
            return Err(format!("Invalid match line format: {}", line));
        }
        let service = parts[0].to_string();
        let pattern_part = parts[1];

        let version_info_str = if parts.len() > 2 { Some(parts[2]) } else { None };

        if !pattern_part.starts_with("m|") && !pattern_part.starts_with("m/") && !pattern_part.starts_with("m#") {
             return Err(format!("Invalid match pattern format: {}", pattern_part));
        }
        
        let delimiter = pattern_part.chars().nth(1).unwrap(); // |, /, #
        let pattern_end_index = pattern_part.rfind(delimiter);

        if pattern_end_index.is_none() || pattern_end_index.unwrap() <= 1 {
            return Err(format!("Unterminated match pattern or invalid format: {}", pattern_part));
        }
        
        let pattern_raw = pattern_part[2..pattern_end_index.unwrap()].to_string();
        let mut flags = "";
        if pattern_end_index.unwrap() < pattern_part.len() -1 {
            flags = &pattern_part[pattern_end_index.unwrap()+1..];
        }

        let mut final_pattern = String::new();
        if flags.contains('i') { // case-insensitive
            final_pattern.push_str("(?i)");
        }
        if flags.contains('s') { // dot matches newline
           final_pattern.push_str("(?s)");
        }
        final_pattern.push_str(&pattern_raw);


        let pattern_compiled = Regex::new(&final_pattern)
            .map_err(|e| format!("Failed to compile regex '{}': {}", final_pattern, e))?;

        let version_info = Self::parse_version_info(version_info_str);
        
        Ok(NmapMatch {
            service,
            pattern_raw,
            pattern_compiled,
            version_info,
            soft_match,
        })
    }

    fn parse_version_info(s: Option<&str>) -> NmapVersionInfo {
        let mut vi = NmapVersionInfo {
            product_capture: None,
            version_capture: None,
            info_capture: None,
            hostname_capture: None,
            ostype_capture: None,
            devicetype_capture: None,
        };

        if let Some(info_str) = s {
            let mut current_field: Option<char> = None;
            let mut current_value = String::new();
            let mut delimiter: Option<char> = None;

            for c in info_str.chars() {
                if current_field.is_none() {
                    if c.is_alphabetic() {
                        current_field = Some(c);
                    }
                } else if delimiter.is_none() {
                    delimiter = Some(c);
                } else {
                    if c == delimiter.unwrap() {
                        // End of current value
                        match current_field {
                            Some('p') => vi.product_capture = Some(current_value.clone()),
                            Some('v') => vi.version_capture = Some(current_value.clone()),
                            Some('i') => vi.info_capture = Some(current_value.clone()),
                            Some('h') => vi.hostname_capture = Some(current_value.clone()),
                            Some('o') => vi.ostype_capture = Some(current_value.clone()),
                            Some('d') => vi.devicetype_capture = Some(current_value.clone()),
                            _ => {}
                        }
                        current_value.clear();
                        current_field = None;
                        delimiter = None;
                    } else {
                        current_value.push(c);
                    }
                }
            }
        }
        vi
    }
}

// Helper struct for port_scanner.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPortDetail {
    pub port: u16,
    pub service_name: Option<String>,
    pub version: Option<String>,
    pub product: Option<String>,
    pub extrainfo: Option<String>,
    pub hostname: Option<String>,
    pub ostype: Option<String>,
    pub devicetype: Option<String>,
    pub is_ssl: bool,
    pub banner: Option<String>, // The raw banner received
} 