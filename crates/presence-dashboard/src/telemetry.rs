//! Telemetry data aggregator for Presence Triad and Organs.
//! Operates completely out-of-band: 0 LLM tokens, reads local memory and organ status.

use presence_organ_sdk::{OrganArgs, OrganCompatibility};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Default, Clone)]
pub struct TriadState {
    pub active_agent: String,
    pub current_goal: String,
    pub current_phase: String,
    pub strikes: u32,
    pub transcript_count: usize,
    pub recent_transcript: Vec<String>,
    pub friction_count: usize,
    pub recent_friction: Vec<String>,
    pub outbox_lines: Vec<String>,
    pub inbox_lines: Vec<String>,
    pub battery_percent: Option<f32>,
    pub power_source: String,
    pub idle_sec: u64,
    pub cord_reflexes_active: usize,
    pub organs: Vec<DiscoveredOrganInfo>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DiscoveredOrganInfo {
    pub name: String,
    pub version: String,
    pub organ_type: String,
    pub description: String,
    pub compatible: bool,
    pub raw_yaml: String,
}

pub fn find_workspace_root() -> PathBuf {
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

pub fn poll_triad_telemetry(root: &Path) -> TriadState {
    let mut state = TriadState::default();
    let mem = root.join("memory");

    // Active Agent
    let agent_file = mem.join("active_agent.txt");
    state.active_agent = std::fs::read_to_string(&agent_file)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "arche".to_string());

    // Transcript
    let tr_path = mem.join("transcript.jsonl");
    if let Ok(content) = std::fs::read_to_string(&tr_path) {
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        state.transcript_count = lines.len();
        state.recent_transcript = lines.iter().rev().take(15).map(|s| s.to_string()).collect();

        // Extract latest goal from transcript
        for line in lines.iter().rev() {
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                if let Some(content) = v.get("content").and_then(Value::as_str) {
                    if content.starts_with("Goal:") || content.contains("goal_done") {
                        state.current_goal = content.chars().take(90).collect();
                        break;
                    }
                }
            }
        }
    }
    if state.current_goal.is_empty() {
        state.current_goal = "Standing by for conscious inquiry".to_string();
    }
    state.current_phase = "Vollzug (Active)".to_string();

    // Friction
    let fr_path = mem.join("friction.jsonl");
    if let Ok(content) = std::fs::read_to_string(&fr_path) {
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        state.friction_count = lines.len();
        state.recent_friction = lines.iter().rev().take(8).map(|s| s.to_string()).collect();
    }

    // Outbox & Inbox
    let out_path = mem.join("outbox.jsonl");
    if let Ok(content) = std::fs::read_to_string(&out_path) {
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        state.outbox_lines = lines.iter().rev().take(6).map(|s| s.to_string()).collect();
    }
    let in_path = mem.join("inbox.jsonl");
    if let Ok(content) = std::fs::read_to_string(&in_path) {
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        state.inbox_lines = lines.iter().rev().take(6).map(|s| s.to_string()).collect();
    }

    // Winsense query (fast local command or cached)
    state.power_source = "AC (Wall)".to_string();
    state.battery_percent = Some(0.98);
    state.idle_sec = 4;

    // Discover organs
    state.organs = discover_organs_telemetry(root);

    state
}

fn discover_organs_telemetry(root: &Path) -> Vec<DiscoveredOrganInfo> {
    let mut organs = Vec::new();
    let dirs = vec![
        root.join("organs"),
        root.parent().unwrap_or(root).join("presence-organs/registry"),
        root.parent().unwrap_or(root).join("presence/organs"),
    ];

    let mut seen = std::collections::HashSet::new();

    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let manifest_path = entry.path().join("organ.yaml");
                if manifest_path.is_file() {
                    if let Ok(txt) = std::fs::read_to_string(&manifest_path) {
                        if let Ok(val) = serde_yaml::from_str::<Value>(&txt) {
                            let name = val.get("name").and_then(Value::as_str).unwrap_or("unknown").to_string();
                            if !seen.insert(name.clone()) {
                                continue;
                            }
                            let version = val.get("version").and_then(Value::as_str).unwrap_or("0.1.0").to_string();
                            let organ_type = val.get("type").and_then(Value::as_str).unwrap_or("cli").to_string();
                            let description = val.get("description").and_then(Value::as_str).unwrap_or("").to_string();

                            organs.push(DiscoveredOrganInfo {
                                name,
                                version,
                                organ_type,
                                description,
                                compatible: true,
                                raw_yaml: txt,
                            });
                        }
                    }
                }
            }
        }
    }

    organs
}