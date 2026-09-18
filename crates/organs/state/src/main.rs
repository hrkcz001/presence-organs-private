//! Native Presence Organ: state
//! Manages cognitive state preservation, persona snapshots, pause protocols, and hibernation.

use presence_organ_sdk::{organ_err, organ_ok, OrganArgs};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn format_timestamp_slug() -> String {
    let secs = now_ts();
    // Deterministic slug using seconds since epoch
    format!("{secs}")
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

fn get_active_agent_card(root: &Path) -> Result<PathBuf, String> {
    let agent = std::env::var("PRESENCE_AGENT").unwrap_or_else(|_| "arche".to_string());
    let candidates = [
        root.join("agents").join(format!("{agent}.agent.md")),
        root.join("agents").join("arche.agent.md"),
        root.join(format!("{agent}.agent.md")),
    ];

    for c in &candidates {
        if c.is_file() {
            return Ok(c.clone());
        }
    }

    Err(format!(
        "No agent card found for agent '{agent}' in {}",
        root.join("agents").display()
    ))
}

fn get_snapshots_dir(root: &Path) -> PathBuf {
    let d = root.join("memory").join("snapshots");
    let _ = std::fs::create_dir_all(&d);
    d
}

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

/// Parses an agent card into its frontmatter dictionary, organs list, and markdown body.
fn parse_agent_card(content: &str) -> (HashMap<String, String>, Vec<String>, String, String) {
    let trimmed = content.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        return (HashMap::new(), Vec::new(), String::new(), trimmed.to_string());
    }

    let rest = &trimmed[3..];
    let end_idx = match rest.find("\n---") {
        Some(idx) => idx,
        None => return (HashMap::new(), Vec::new(), String::new(), trimmed.to_string()),
    };

    let frontmatter = &rest[..end_idx];
    let body = rest[end_idx + 4..].trim_start_matches('\n').trim_start_matches('\r').to_string();

    let mut state_dict = HashMap::new();
    let mut organs = Vec::new();
    let mut mode = "active".to_string();

    let mut in_state = false;
    let mut in_organs = false;

    for line in frontmatter.lines() {
        let line_trimmed = line.trim();
        if line_trimmed == "state:" || line_trimmed.starts_with("state:") {
            in_state = true;
            in_organs = false;
            continue;
        }
        if line_trimmed == "organs:" || line_trimmed.starts_with("organs:") {
            in_organs = true;
            in_state = false;
            continue;
        }

        if in_state {
            if let Some((k, v)) = line_trimmed.split_once(':') {
                if !line.starts_with(' ') && !line.starts_with('\t') {
                    in_state = false;
                } else {
                    state_dict.insert(k.trim().to_string(), v.trim().to_string());
                    continue;
                }
            }
        }

        if in_organs {
            if line_trimmed.starts_with('-') {
                let organ_name = line_trimmed.trim_start_matches('-').trim();
                organs.push(organ_name.to_string());
                continue;
            } else if !line.starts_with(' ') && !line.starts_with('\t') {
                in_organs = false;
            }
        }

        if let Some((k, v)) = line_trimmed.split_once(':') {
            let key = k.trim();
            let val = v.trim();
            if key == "mode" {
                mode = val.to_string();
            }
        }
    }

    (state_dict, organs, mode, body)
}

fn update_agent_frontmatter(
    content: &str,
    new_mode: Option<&str>,
    state_updates: &[(&str, &str)],
) -> String {
    let trimmed = content.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        return content.to_string();
    }

    let rest = &trimmed[3..];
    let end_idx = match rest.find("\n---") {
        Some(idx) => idx,
        None => return content.to_string(),
    };

    let frontmatter = &rest[..end_idx];
    let body = &rest[end_idx..];

    let mut lines: Vec<String> = frontmatter.lines().map(|l| l.to_string()).collect();
    let mut has_mode = false;
    let mut in_state = false;
    let mut updated_keys = std::collections::HashSet::new();

    for line in lines.iter_mut() {
        let t = line.trim().to_string();
        if t.starts_with("mode:") {
            has_mode = true;
            if let Some(m) = new_mode {
                *line = format!("mode: {m}");
            }
        } else if t == "state:" || t.starts_with("state:") {
            in_state = true;
        } else if in_state {
            if let Some((k, _)) = t.split_once(':') {
                let key = k.trim();
                if let Some((_, new_val)) = state_updates.iter().find(|(uk, _)| *uk == key) {
                    *line = format!("  {key}: {new_val}");
                    updated_keys.insert(key.to_string());
                }
            } else if !line.starts_with(' ') && !line.starts_with('\t') {
                in_state = false;
            }
        }
    }

    if !has_mode {
        if let Some(m) = new_mode {
            lines.insert(0, format!("mode: {m}"));
        }
    }

    // Append any unwritten state updates
    for (k, v) in state_updates {
        if !updated_keys.contains(*k) {
            lines.push(format!("  {k}: {v}"));
        }
    }

    format!("---{}\n{}", lines.join("\n"), body.trim_start_matches('\n'))
}

fn tool_snapshot(args: &OrganArgs) -> Result<Value, String> {
    let label = extract_arg(args, "label").unwrap_or_else(|| "manual".to_string());
    let clean_label = label.replace(' ', "_").replace(|c: char| !c.is_alphanumeric() && c != '_', "");

    let root = find_workspace_root();
    let card_path = get_active_agent_card(&root)?;
    let content = std::fs::read_to_string(&card_path)
        .map_err(|e| format!("Failed to read agent card: {e}"))?;

    let agent_slug = card_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.split('.').next().unwrap_or("agent"))
        .unwrap_or("agent");

    let snap_dir = get_snapshots_dir(&root);
    let filename = format!("{}_{}_{}.agent.md", format_timestamp_slug(), clean_label, agent_slug);
    let target = snap_dir.join(&filename);

    std::fs::write(&target, content)
        .map_err(|e| format!("Failed to write snapshot {}: {e}", target.display()))?;

    Ok(json!({
        "status": "ok",
        "snapshot": target.display().to_string(),
        "filename": filename,
        "agent": agent_slug,
        "label": clean_label,
    }))
}

fn tool_pause(args: &OrganArgs) -> Result<Value, String> {
    let reason = extract_arg(args, "reason").unwrap_or_else(|| "user_pause".to_string());
    let root = find_workspace_root();
    let card_path = get_active_agent_card(&root)?;

    // 1. Create a pause snapshot
    let snap_args = OrganArgs::from_args(["--label", &format!("pause_{}", &reason[..reason.len().min(15)])]);
    let snap_res = tool_snapshot(&snap_args)?;

    // 2. Update the agent card frontmatter to mode: rest
    let content = std::fs::read_to_string(&card_path)
        .map_err(|e| format!("Failed to read agent card: {e}"))?;

    let next_val = format!("paused: {reason}");
    let updated = update_agent_frontmatter(
        &content,
        Some("rest"),
        &[("next", &next_val)],
    );

    std::fs::write(&card_path, updated)
        .map_err(|e| format!("Failed to update agent card: {e}"))?;

    // 3. Write pause marker for Presence Core (Mail::Pause coordination)
    let state_dir = root.join("memory").join("state");
    let _ = std::fs::create_dir_all(&state_dir);
    let marker_path = state_dir.join("pause.json");
    let marker = json!({
        "paused": true,
        "reason": reason,
        "timestamp": now_ts(),
        "agent": card_path.file_name().and_then(|f| f.to_str()).unwrap_or(""),
        "snapshot": snap_res.get("snapshot").and_then(Value::as_str).unwrap_or(""),
    });
    let _ = std::fs::write(&marker_path, serde_json::to_string_pretty(&marker).unwrap_or_default());

    Ok(json!({
        "status": "ok",
        "action": "pause",
        "mode": "rest",
        "reason": reason,
        "agent": card_path.display().to_string(),
        "snapshot": snap_res.get("snapshot").and_then(Value::as_str).unwrap_or(""),
    }))
}

fn tool_restore(args: &OrganArgs) -> Result<Value, String> {
    let label = extract_arg(args, "label").unwrap_or_default();
    let root = find_workspace_root();
    let snap_dir = get_snapshots_dir(&root);

    let mut snapshots: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&snap_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().map(|e| e == "md").unwrap_or(false) {
                snapshots.push(p);
            }
        }
    }

    if snapshots.is_empty() {
        return Err(format!("No snapshots found in {}", snap_dir.display()));
    }

    snapshots.sort();

    let target = if label.trim().is_empty() {
        snapshots.last().cloned().unwrap()
    } else {
        snapshots
            .iter()
            .rev()
            .find(|p| p.file_name().and_then(|f| f.to_str()).map(|s| s.contains(&label)).unwrap_or(false))
            .cloned()
            .ok_or_else(|| format!("No snapshot found matching label '{label}' in {}", snap_dir.display()))?
    };

    let snap_content = std::fs::read_to_string(&target)
        .map_err(|e| format!("Failed to read snapshot {}: {e}", target.display()))?;

    let card_path = get_active_agent_card(&root)?;
    std::fs::write(&card_path, snap_content)
        .map_err(|e| format!("Failed to overwrite agent card {}: {e}", card_path.display()))?;

    Ok(json!({
        "status": "ok",
        "action": "restore",
        "restored_from": target.display().to_string(),
        "agent": card_path.display().to_string(),
    }))
}

fn sense_sleep_state() -> Result<Value, String> {
    let root = find_workspace_root();
    let card_path = get_active_agent_card(&root)?;
    let content = std::fs::read_to_string(&card_path)
        .map_err(|e| format!("Failed to read agent card: {e}"))?;

    let (state_dict, organs, mode, _) = parse_agent_card(&content);
    let agent_name = card_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.split('.').next().unwrap_or("agent"))
        .unwrap_or("agent");

    Ok(json!({
        "status": "ok",
        "agent": agent_name,
        "mode": mode,
        "state": state_dict,
        "organs": organs,
        "card_path": card_path.display().to_string(),
    }))
}

fn stimulus_stale_goal(args: &OrganArgs) -> Result<Value, String> {
    let root = find_workspace_root();
    let possible_goals = [
        root.join("GOALS.md"),
        root.join("workspace").join("GOALS.md"),
        root.join("memory").join("goals.md"),
    ];

    let goal_path = possible_goals.iter().find(|p| p.is_file());
    let p = match goal_path {
        Some(p) => p,
        None => {
            return Ok(json!({
                "status": "ok",
                "stimulus": "stale_goal",
                "triggered": false,
                "message": "No active GOALS.md found"
            }));
        }
    };

    let threshold_hours: f64 = extract_arg(args, "threshold_hours")
        .and_then(|s| s.parse().ok())
        .unwrap_or(12.0);

    let metadata = std::fs::metadata(p).map_err(|e| e.to_string())?;
    let modified = metadata.modified().unwrap_or(SystemTime::now());
    let age_secs = SystemTime::now()
        .duration_since(modified)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let age_hours = age_secs / 3600.0;

    let content = std::fs::read_to_string(p).unwrap_or_default();
    let has_unfinished = content.contains("- [ ]");
    let triggered = age_hours >= threshold_hours && has_unfinished;

    Ok(json!({
        "status": "ok",
        "stimulus": "stale_goal",
        "goal_path": p.display().to_string(),
        "age_hours": (age_hours * 10.0).round() / 10.0,
        "threshold_hours": threshold_hours,
        "has_unfinished_goals": has_unfinished,
        "triggered": triggered,
    }))
}

fn print_meta() {
    let meta = json!({
        "name": "state",
        "version": "0.3.0",
        "type": "cli",
        "description": "Presence cognitive state preservation, session pause, and hibernation",
        "tools": [
            "pause_session",
            "snapshot",
            "restore"
        ],
        "senses": ["sleep_state"],
        "stimuli": ["stale_goal"],
        "slash_commands": ["pause", "snapshot"]
    });
    println!("{}", serde_json::to_string_pretty(&meta).unwrap());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 || args.iter().any(|a| a == "--help" || a == "-h" || a == "--meta") {
        print_meta();
        return;
    }

    if args.iter().any(|a| a == "--tools") {
        let tools = json!(["pause_session", "snapshot", "restore"]);
        println!("{}", tools);
        return;
    }

    if let Some(pos) = args.iter().position(|a| a == "--stimulus") {
        let stim = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        let organ_args = OrganArgs::from_env();
        match stim {
            "stale_goal" | "" => match stimulus_stale_goal(&organ_args) {
                Ok(val) => organ_ok!("result" => val),
                Err(e) => organ_err!(e),
            },
            other => organ_err!(format!("Unknown stimulus: {other}")),
        }
    }

    if let Some(pos) = args.iter().position(|a| a == "--sense") {
        let sense = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        match sense {
            "sleep_state" | "" => match sense_sleep_state() {
                Ok(val) => organ_ok!("state" => val),
                Err(e) => organ_err!(e),
            },
            other => organ_err!(format!("Unknown sense: {other}")),
        }
    }

    if let Some(pos) = args.iter().position(|a| a == "--tool") {
        let tool_name = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        let organ_args = OrganArgs::from_env();

        let res = match tool_name {
            "pause_session" => tool_pause(&organ_args),
            "snapshot" => tool_snapshot(&organ_args),
            "restore" => tool_restore(&organ_args),
            other => Err(format!("Unknown tool: {other}")),
        };

        match res {
            Ok(val) => organ_ok!("result" => val),
            Err(e) => organ_err!(e),
        }
    }

    print_meta();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_agent_card_roundtrip() {
        let raw = r#"---
name: arche
mode: active
organs:
  - io
  - plan
state:
  current_goal: test_execution
  iteration: 3
---
# Arche Personality
Living context body.
"#;
        let (state, organs, mode, body) = parse_agent_card(raw);
        assert_eq!(mode, "active");
        assert_eq!(organs, vec!["io", "plan"]);
        assert_eq!(state.get("current_goal").map(|s| s.as_str()), Some("test_execution"));
        assert!(body.contains("Living context body."));

        let updated = update_agent_frontmatter(raw, Some("rest"), &[("iteration", "4")]);
        let (state2, _, mode2, _) = parse_agent_card(&updated);
        assert_eq!(mode2, "rest");
        assert_eq!(state2.get("iteration").map(|s| s.as_str()), Some("4"));
    }

    #[test]
    fn test_snapshot_and_restore_cycle() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("agents")).unwrap();
        std::fs::create_dir_all(root.join("memory")).unwrap();

        let card_path = root.join("agents/arche.agent.md");
        let initial = "---
name: arche
mode: active
state:
  step: 1
---
Body 1
";
        std::fs::write(&card_path, initial).unwrap();

        std::env::set_var("PRESENCE_WORKSPACE", root.to_str().unwrap());
        std::env::set_var("PRESENCE_AGENT", "arche");

        // Take snapshot
        let snap_res = tool_snapshot(&OrganArgs::from_args(["--label", "init"])).unwrap();
        assert_eq!(snap_res["status"], "ok");

        // Modify card
        let modified = "---
name: arche
mode: active
state:
  step: 2
---
Body 2
";
        std::fs::write(&card_path, modified).unwrap();

        // Pause
        let pause_res = tool_pause(&OrganArgs::from_args(["--reason", "lunch"])).unwrap();
        assert_eq!(pause_res["mode"], "rest");
        let read_paused = std::fs::read_to_string(&card_path).unwrap();
        assert!(read_paused.contains("mode: rest"));
        assert!(read_paused.contains("paused: lunch"));

        // Restore init snapshot
        let restore_res = tool_restore(&OrganArgs::from_args(["--label", "init"])).unwrap();
        assert_eq!(restore_res["status"], "ok");
        let read_restored = std::fs::read_to_string(&card_path).unwrap();
        assert!(read_restored.contains("Body 1"));
        assert!(read_restored.contains("step: 1"));
    }
}
