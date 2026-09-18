//! Native Presence Organ: mood
//! Implements mechanical affect, posture derivation, and cognitive strain tracking.
//! Law: "Felt, not narrated" - zero model calls; mood is computed from operational friction.

use presence_organ_sdk::{organ_err, organ_ok, OrganArgs};
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
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

fn get_friction_paths(root: &Path) -> Vec<PathBuf> {
    let candidates = [
        root.join("logs").join("friction.jsonl"),
        root.join("memory").join("friction.jsonl"),
    ];
    let mut found = Vec::new();
    for c in &candidates {
        if c.is_file() {
            found.push(c.clone());
        }
    }
    if found.is_empty() {
        // Default target for new writes is logs/friction.jsonl
        let _ = std::fs::create_dir_all(root.join("logs"));
        found.push(root.join("logs").join("friction.jsonl"));
    }
    found
}

fn count_friction_since(root: &Path, since: u64) -> usize {
    let paths = get_friction_paths(root);
    let mut count = 0;
    for path in paths {
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                if let Ok(val) = serde_json::from_str::<Value>(line) {
                    if let Some(ts) = val.get("ts").and_then(Value::as_u64) {
                        if ts >= since {
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    count
}

fn total_friction_count(root: &Path) -> usize {
    count_friction_since(root, 0)
}

/// Derive mood from friction count and cycle count.
/// Weight: light | steady | heavy
/// Posture: fresh | steady | restless | agitated
fn derive_mood(friction_count: usize, cycles: u32) -> (String, String) {
    let n = friction_count as u32;
    let load = n.saturating_mul(2) + cycles.min(4);
    let weight = match load {
        0..=2 => "light",
        3..=6 => "steady",
        _ => "heavy",
    };
    let posture = match n {
        0 => {
            if cycles >= 3 {
                "steady"
            } else {
                "fresh"
            }
        }
        1..=2 => "steady",
        3..=5 => "restless",
        _ => "agitated",
    };
    (weight.to_string(), posture.to_string())
}

fn sync_mood_md(root: &Path, weight: &str, posture: &str, friction_recent: usize, cycles: u32) {
    let mem_dir = root.join("memory");
    let _ = std::fs::create_dir_all(&mem_dir);
    let path = mem_dir.join("mood.md");
    let body = format!(
        "# mood\n\nweight: {weight}\nposture: {posture}\nfriction_since_last_boundary: {friction_recent}\ncycles: {cycles}\nupdated_ts: {}\n",
        now_ts()
    );
    let _ = std::fs::write(path, body);
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

fn tool_mood_status(args: &OrganArgs) -> Result<Value, String> {
    let root = find_workspace_root();
    let since = extract_arg(args, "since")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| now_ts().saturating_sub(900)); // Default: last 15 min

    let cycles: u32 = extract_arg(args, "cycles")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);

    let recent = count_friction_since(&root, since);
    let total = total_friction_count(&root);
    let (weight, posture) = derive_mood(recent, cycles);

    sync_mood_md(&root, &weight, &posture, recent, cycles);

    Ok(json!({
        "status": "ok",
        "weight": weight,
        "posture": posture,
        "friction_recent": recent,
        "friction_total": total,
        "cycles": cycles,
        "window_seconds": now_ts().saturating_sub(since),
    }))
}

fn tool_mood_record(args: &OrganArgs) -> Result<Value, String> {
    let kind = extract_arg(args, "kind").unwrap_or_else(|| "tool_failed".to_string());
    let detail = extract_arg(args, "detail").unwrap_or_else(|| "unspecified friction".to_string());

    let root = find_workspace_root();
    let paths = get_friction_paths(&root);
    let target = paths.first().cloned().unwrap_or_else(|| root.join("logs").join("friction.jsonl"));

    let bounded: String = detail.chars().take(250).collect();
    let entry = json!({
        "ts": now_ts(),
        "kind": kind,
        "detail": bounded,
    });

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&target)
        .map_err(|e| format!("Failed to append to {}: {e}", target.display()))?;

    writeln!(file, "{}", serde_json::to_string(&entry).unwrap_or_default())
        .map_err(|e| format!("Write failed: {e}"))?;

    Ok(json!({
        "status": "ok",
        "action": "recorded",
        "kind": kind,
        "file": target.display().to_string(),
    }))
}

fn sense_current_mood() -> Result<Value, String> {
    let root = find_workspace_root();
    let since = now_ts().saturating_sub(900); // 15 min window
    let recent = count_friction_since(&root, since);
    let total = total_friction_count(&root);
    let (weight, posture) = derive_mood(recent, 1);

    sync_mood_md(&root, &weight, &posture, recent, 1);

    let markdown_grounding = format!(
        "--- mood (felt, not narrated) ---\nweight: {weight}\nposture: {posture}\nfriction_recent: {recent}\n"
    );

    Ok(json!({
        "weight": weight,
        "posture": posture,
        "friction_recent": recent,
        "friction_total": total,
        "grounding": markdown_grounding,
    }))
}

fn stimulus_cognitive_strain(args: &OrganArgs) -> Result<Value, String> {
    let root = find_workspace_root();
    let threshold: usize = extract_arg(args, "threshold")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(3);

    let window_secs: u64 = extract_arg(args, "window_seconds")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(600); // 10 minutes

    let since = now_ts().saturating_sub(window_secs);
    let count = count_friction_since(&root, since);
    let (weight, posture) = derive_mood(count, 1);
    let triggered = count >= threshold;

    Ok(json!({
        "status": "ok",
        "stimulus": "cognitive_strain",
        "triggered": triggered,
        "friction_in_window": count,
        "threshold": threshold,
        "window_seconds": window_secs,
        "weight": weight,
        "posture": posture,
    }))
}

fn print_meta() {
    let meta = json!({
        "name": "mood",
        "version": "0.3.0",
        "type": "cli",
        "description": "Mechanical affect, cognitive strain, and posture organ for Presence (Felt, not narrated)",
        "tools": [
            "mood_status",
            "mood_record_friction"
        ],
        "senses": ["current_mood"],
        "stimuli": ["cognitive_strain"],
        "slash_commands": ["mood"]
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
        let tools = json!(["mood_status", "mood_record_friction"]);
        println!("{}", tools);
        return;
    }

    if let Some(pos) = args.iter().position(|a| a == "--stimulus") {
        let stim = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        let organ_args = OrganArgs::from_env();
        match stim {
            "cognitive_strain" | "" => match stimulus_cognitive_strain(&organ_args) {
                Ok(val) => organ_ok!("result" => val),
                Err(e) => organ_err!(e),
            },
            other => organ_err!(format!("Unknown stimulus: {other}")),
        }
    }

    if let Some(pos) = args.iter().position(|a| a == "--sense") {
        let sense = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        match sense {
            "current_mood" | "" => match sense_current_mood() {
                Ok(val) => organ_ok!("mood" => val),
                Err(e) => organ_err!(e),
            },
            other => organ_err!(format!("Unknown sense: {other}")),
        }
    }

    if let Some(pos) = args.iter().position(|a| a == "--tool") {
        let tool_name = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        let organ_args = OrganArgs::from_env();

        let res = match tool_name {
            "mood_status" => tool_mood_status(&organ_args),
            "mood_record_friction" => tool_mood_record(&organ_args),
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
    fn test_derive_mood_matrix() {
        let (w1, p1) = derive_mood(0, 0);
        assert_eq!(w1, "light");
        assert_eq!(p1, "fresh");

        let (w2, p2) = derive_mood(0, 4);
        assert_eq!(w2, "steady");
        assert_eq!(p2, "steady");

        let (w3, p3) = derive_mood(2, 1);
        assert_eq!(w3, "steady");
        assert_eq!(p3, "steady");

        let (w4, p4) = derive_mood(4, 1);
        assert_eq!(w4, "heavy");
        assert_eq!(p4, "restless");

        let (w5, p5) = derive_mood(8, 2);
        assert_eq!(w5, "heavy");
        assert_eq!(p5, "agitated");
    }

    #[test]
    fn test_record_and_count_friction() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("logs")).unwrap();

        std::env::set_var("PRESENCE_WORKSPACE", root.to_str().unwrap());

        let res = tool_mood_record(&OrganArgs::from_args(["--kind", "phase_strike", "--detail", "invalid schema"])).unwrap();
        assert_eq!(res["status"], "ok");

        let count = count_friction_since(root, 0);
        assert_eq!(count, 1);

        let status = tool_mood_status(&OrganArgs::from_args(["--cycles", "1"])).unwrap();
        assert_eq!(status["status"], "ok");
        assert_eq!(status["friction_recent"], 1);
    }
}
