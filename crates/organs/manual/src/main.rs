//! Manual discovery, targeted reading, and documentation search organ for Presence Triad.
use presence_organ_sdk::{OrganArgs, OrganCompatibility, OrganResponse};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganManualEntry {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub manual_found: bool,
    pub path: Option<String>,
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualMatch {
    pub organ: String,
    pub section: String,
    pub line: usize,
    pub snippet: String,
}

fn find_workspace_root() -> PathBuf {
    if let Ok(w) = std::env::var("PRESENCE_WORKSPACE") {
        let p = PathBuf::from(w);
        if p.is_dir() {
            return p;
        }
    }
    let mut cur = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if cur.join("memory").is_dir() || cur.join("AGENTS.md").is_file() {
            return cur;
        }
        if !cur.pop() {
            break;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Extract argument by key from CLI flags or JSON args string.
fn extract_arg(args: &OrganArgs, key: &str) -> Option<String> {
    if let Some(val) = args.get(key) {
        return Some(val.to_string());
    }
    if let Some(raw_json) = args.get("args") {
        if let Ok(val) = serde_json::from_str::<Value>(raw_json) {
            if let Some(v) = val.get(key).and_then(Value::as_str) {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// Parse `## ` section headers and their contents from Markdown.
pub fn parse_manual_sections(content: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current_title = String::new();
    let mut current_body = Vec::new();

    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let body = current_body.join("\n").trim().to_string();
            if !current_title.is_empty() {
                sections.push((current_title, body));
            } else if !body.is_empty() {
                sections.push(("Header".to_string(), body));
            }
            current_title = heading.trim().to_string();
            current_body.clear();
        } else {
            current_body.push(line);
        }
    }

    let last_body = current_body.join("\n").trim().to_string();
    if !current_title.is_empty() {
        sections.push((current_title, last_body));
    } else if !last_body.is_empty() {
        sections.push(("Header".to_string(), last_body));
    }

    sections
}

/// Collect candidate directories across Presence workspace and organ repositories.
fn get_search_directories(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.push(root.join("organs"));
    dirs.push(root.join("crates/organs"));
    dirs.push(root.join("registry"));
    dirs.push(root.join("workspace/organs"));

    if let Ok(env_path) = std::env::var("PRESENCE_ORGANS_PATH") {
        dirs.push(PathBuf::from(env_path));
    }

    let mut cur: Option<&Path> = Some(root);
    for _ in 0..3 {
        if let Some(p) = cur {
            dirs.push(p.join("Dev/presence-organs/crates/organs"));
            dirs.push(p.join("Dev/presence-organs/organs"));
            dirs.push(p.join("Dev/presence-organs/registry"));
            dirs.push(p.join("Dev/presence/seed/organs"));
            dirs.push(p.join("presence-organs/crates/organs"));
            dirs.push(p.join("presence-organs/organs"));
            dirs.push(p.join("presence-organs/registry"));
            dirs.push(p.join("presence/seed/organs"));
            cur = p.parent();
        } else {
            break;
        }
    }
    dirs
}

/// Discover all installed or registered organ manuals.
pub fn discover_organs(root: &Path) -> BTreeMap<String, OrganManualEntry> {
    let mut map: BTreeMap<String, OrganManualEntry> = BTreeMap::new();
    let dirs = get_search_directories(root);

    for base in dirs {
        if !base.is_dir() {
            continue;
        }
        let entries = match std::fs::read_dir(&base) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            if name.is_empty() || name.starts_with('.') {
                continue;
            }

            let yaml_path = path.join("organ.yaml");
            let manual_path = path.join("MANUAL.md");

            let mut version = "0.3.0".to_string();
            let mut description = String::new();

            if yaml_path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&yaml_path) {
                    if let Ok(val) = serde_yaml::from_str::<Value>(&content) {
                        if let Some(v) = val.get("version").and_then(Value::as_str) {
                            version = v.to_string();
                        }
                        if let Some(d) = val.get("description").and_then(Value::as_str) {
                            description = d.to_string();
                        }
                    }
                }
            }

            let (manual_found, topics, resolved_manual_path) = if manual_path.is_file() {
                let topics = if let Ok(md) = std::fs::read_to_string(&manual_path) {
                    parse_manual_sections(&md).into_iter().map(|(t, _)| t).collect()
                } else {
                    Vec::new()
                };
                (true, topics, Some(manual_path.display().to_string()))
            } else {
                (false, Vec::new(), None)
            };

            // Prefer entries that have an actual MANUAL.md
            if !map.contains_key(&name) || (manual_found && !map[&name].manual_found) {
                map.insert(
                    name.clone(),
                    OrganManualEntry {
                        name,
                        version,
                        description,
                        manual_found,
                        path: resolved_manual_path,
                        topics,
                    },
                );
            }
        }
    }

    map
}

/// Tool: manual_list
pub fn tool_manual_list(root: &Path) -> OrganResponse {
    let organs = discover_organs(root);
    let list: Vec<OrganManualEntry> = organs.into_values().collect();
    let mut resp = OrganResponse::ok();
    resp.data.insert("total_organs".to_string(), json!(list.len()));
    resp.data.insert("organs".to_string(), json!(list));
    resp
}

/// Tool: manual_read
pub fn tool_manual_read(root: &Path, organ_name: &str, topic: Option<&str>) -> OrganResponse {
    let organs = discover_organs(root);
    let entry = match organs.get(organ_name) {
        Some(e) => e,
        None => {
            let available: Vec<String> = organs.keys().cloned().collect();
            return OrganResponse::error(format!(
                "Organ '{organ_name}' not found. Available organs: {:?}",
                available
            ));
        }
    };

    let path_str = match &entry.path {
        Some(p) => p,
        None => return OrganResponse::error(format!("Organ '{organ_name}' has no MANUAL.md")),
    };

    let content = match std::fs::read_to_string(path_str) {
        Ok(c) => c,
        Err(e) => return OrganResponse::error(format!("Failed to read manual file '{path_str}': {e}")),
    };

    let mut resp = OrganResponse::ok();
    resp.data.insert("organ".to_string(), json!(organ_name));
    resp.data.insert("version".to_string(), json!(entry.version));
    resp.data.insert("manual_path".to_string(), json!(path_str));

    if let Some(top) = topic {
        if top.eq_ignore_ascii_case("all") {
            resp.data.insert("content".to_string(), json!(content));
            return resp;
        }

        let sections = parse_manual_sections(&content);
        let top_lower = top.to_lowercase();

        // Match section by heading
        let matched = sections.iter().find(|(heading, _)| {
            let h = heading.to_lowercase();
            h == top_lower || h.contains(&top_lower) || top_lower.contains(&h)
        });

        if let Some((heading, body)) = matched {
            resp.data.insert("topic".to_string(), json!(heading));
            resp.data.insert("content".to_string(), json!(body));
        } else {
            let available_topics: Vec<String> = sections.into_iter().map(|(h, _)| h).collect();
            return OrganResponse::error(format!(
                "Topic '{top}' not found in organ '{organ_name}'. Available topics: {:?}",
                available_topics
            ));
        }
    } else {
        resp.data.insert("content".to_string(), json!(content));
    }

    resp
}

/// Tool: manual_search
pub fn tool_manual_search(root: &Path, query: &str) -> OrganResponse {
    let organs = discover_organs(root);
    let mut matches = Vec::new();
    let q_lower = query.to_lowercase();

    for (organ_name, entry) in &organs {
        let path_str = match &entry.path {
            Some(p) => p,
            None => continue,
        };

        let content = match std::fs::read_to_string(path_str) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut current_section = "Header".to_string();

        for (idx, line) in content.lines().enumerate() {
            let line_num = idx + 1;
            if let Some(heading) = line.strip_prefix("## ") {
                current_section = heading.trim().to_string();
            }

            if line.to_lowercase().contains(&q_lower) {
                matches.push(ManualMatch {
                    organ: organ_name.clone(),
                    section: current_section.clone(),
                    line: line_num,
                    snippet: line.trim().to_string(),
                });
            }
        }
    }

    let mut resp = OrganResponse::ok();
    resp.data.insert("query".to_string(), json!(query));
    resp.data.insert("total_matches".to_string(), json!(matches.len()));
    resp.data.insert("matches".to_string(), json!(matches));
    resp
}

fn main() {
    let args = OrganArgs::from_env();

    if args.has("compatibility") || args.has("compat") {
        let compat = OrganCompatibility {
            presence: Some("^0.3.0".to_string()),
            api_version: Some(1),
            platforms: None,
            features: None,
        };
        println!("{}", serde_json::to_string(&compat).unwrap());
        std::process::exit(0);
    }

    let root = find_workspace_root();
    let op = args.op().to_lowercase();

    let resp = match op.as_str() {
        "manual_list" | "list" => tool_manual_list(&root),
        "manual_read" | "read" => {
            let organ = extract_arg(&args, "organ")
                .or_else(|| extract_arg(&args, "name"))
                .unwrap_or_default();
            let topic = extract_arg(&args, "topic")
                .or_else(|| extract_arg(&args, "section"));
            if organ.is_empty() {
                OrganResponse::error("Missing required parameter: 'organ'")
            } else {
                tool_manual_read(&root, &organ, topic.as_deref())
            }
        }
        "manual_search" | "search" => {
            let query = extract_arg(&args, "query")
                .or_else(|| extract_arg(&args, "q"))
                .unwrap_or_default();
            if query.is_empty() {
                OrganResponse::error("Missing required parameter: 'query'")
            } else {
                tool_manual_search(&root, &query)
            }
        }
        other => {
            if args.has("organ") {
                let organ = extract_arg(&args, "organ").unwrap_or_default();
                let topic = extract_arg(&args, "topic");
                tool_manual_read(&root, &organ, topic.as_deref())
            } else if args.has("query") {
                let query = extract_arg(&args, "query").unwrap_or_default();
                tool_manual_search(&root, &query)
            } else if other.is_empty() || other == "help" {
                tool_manual_list(&root)
            } else {
                OrganResponse::error(format!("Unknown tool or action: {other}"))
            }
        }
    };

    resp.print_and_exit();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_unique_dir(prefix: &str) -> PathBuf {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("{prefix}_{ts}"))
    }

    const SAMPLE_MD: &str = r#"
# Manual: sample_organ

A mock sample organ for testing.
Version: 0.3.0

## Overview
This is the overview description of sample organ.

## Environment & Dependencies
- Platforms: Windows and Linux
- Runtime: bun

## Tools Specification

### `sample_tool`
- Description: Performs sample action.
- Parameters: `count` (number)

## Senses & Stimuli
- Senses: `sample_sense`
- Stimuli: `sample_stimulus`

## Failure Modes & Recovery
- Error: `Sample failure`
  - Cause: Resource exhaustion
"#;

    #[test]
    fn test_parse_manual_sections() {
        let sections = parse_manual_sections(SAMPLE_MD);
        assert_eq!(sections.len(), 6);
        assert_eq!(sections[0].0, "Header");
        assert_eq!(sections[1].0, "Overview");
        assert!(sections[1].1.contains("overview description"));
        assert_eq!(sections[2].0, "Environment & Dependencies");
        assert_eq!(sections[3].0, "Tools Specification");
        assert!(sections[3].1.contains("sample_tool"));
        assert_eq!(sections[4].0, "Senses & Stimuli");
        assert_eq!(sections[5].0, "Failure Modes & Recovery");
    }

    #[test]
    fn test_read_specific_topic() {
        let temp_dir = test_unique_dir("test_manual_read");
        let organ_dir = temp_dir.join("organs/mock");
        std::fs::create_dir_all(&organ_dir).unwrap();
        std::fs::write(organ_dir.join("MANUAL.md"), SAMPLE_MD).unwrap();

        let resp = tool_manual_read(&temp_dir, "mock", Some("tools"));
        assert_eq!(resp.status, "ok");
        assert_eq!(resp.data.get("topic").and_then(Value::as_str), Some("Tools Specification"));
        assert!(resp.data.get("content").and_then(Value::as_str).unwrap().contains("sample_tool"));

        let resp_fail = tool_manual_read(&temp_dir, "mock", Some("nonexistent_topic"));
        assert_eq!(resp_fail.status, "error");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_manual_search() {
        let temp_dir = test_unique_dir("test_manual_search");
        let organ_dir = temp_dir.join("organs/mock");
        std::fs::create_dir_all(&organ_dir).unwrap();
        std::fs::write(organ_dir.join("MANUAL.md"), SAMPLE_MD).unwrap();

        let resp = tool_manual_search(&temp_dir, "exhaustion");
        assert_eq!(resp.status, "ok");
        let matches = resp.data.get("matches").and_then(Value::as_array).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].get("section").and_then(Value::as_str), Some("Failure Modes & Recovery"));
        assert!(matches[0].get("snippet").and_then(Value::as_str).unwrap().contains("Resource exhaustion"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
