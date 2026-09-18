//! Native Presence Organ: social
//! Implements relational memory, interlocutor profiles, habit tracking, and communication registers.

use presence_organ_sdk::{organ_err, organ_ok, OrganArgs};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
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

fn get_workspace(args: &OrganArgs) -> PathBuf {
    if let Some(w) = extract_arg(args, "workspace") {
        let p = PathBuf::from(w);
        if p.is_dir() {
            return p;
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitItem {
    pub category: String,
    pub observation: String,
    pub recorded_ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterConfig {
    pub tone: String,
    pub brevity: String,
    pub directness: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonProfile {
    pub person_id: String,
    pub name: String,
    pub role: String,
    pub language: String,
    pub register: RegisterConfig,
    pub habits: Vec<HabitItem>,
    pub boundaries: Vec<String>,
    pub last_seen_ts: u64,
    pub interaction_count: u64,
}

impl Default for PersonProfile {
    fn default() -> Self {
        PersonProfile {
            person_id: "owner".to_string(),
            name: "Owner".to_string(),
            role: "creator".to_string(),
            language: "Russian dialogue, English code/artifacts".to_string(),
            register: RegisterConfig {
                tone: "pragmatic".to_string(),
                brevity: "concise".to_string(),
                directness: "high".to_string(),
            },
            habits: vec![
                HabitItem {
                    category: "preference".to_string(),
                    observation: "Prefers deterministic execution over narration".to_string(),
                    recorded_ts: now_ts(),
                },
                HabitItem {
                    category: "preference".to_string(),
                    observation: "Values pure hermeneutic core with zero hardcoded bloat".to_string(),
                    recorded_ts: now_ts(),
                },
                HabitItem {
                    category: "lexicon".to_string(),
                    observation: "Uses Russian for conversation, English for commits and manuals".to_string(),
                    recorded_ts: now_ts(),
                },
            ],
            boundaries: vec![
                "Never delete project code files without explicit approval".to_string(),
            ],
            last_seen_ts: now_ts(),
            interaction_count: 1,
        }
    }
}

fn profiles_dir(root: &Path) -> PathBuf {
    root.join("memory").join("social").join("profiles")
}

fn load_profile(root: &Path, person_id: &str) -> PersonProfile {
    let dir = profiles_dir(root);
    let path = dir.join(format!("{person_id}.json"));

    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(mut profile) = serde_json::from_str::<PersonProfile>(&content) {
            profile.last_seen_ts = now_ts();
            profile.interaction_count += 1;
            let _ = save_profile(root, &profile);
            return profile;
        }
    }

    // If owner profile is missing, initialize default
    if person_id == "owner" {
        let profile = PersonProfile::default();
        let _ = save_profile(root, &profile);
        return profile;
    }

    // Generic fallback for other people
    PersonProfile {
        person_id: person_id.to_string(),
        name: person_id.to_string(),
        role: "interlocutor".to_string(),
        language: "auto".to_string(),
        register: RegisterConfig {
            tone: "formal".to_string(),
            brevity: "normal".to_string(),
            directness: "medium".to_string(),
        },
        habits: Vec::new(),
        boundaries: Vec::new(),
        last_seen_ts: now_ts(),
        interaction_count: 1,
    }
}

fn save_profile(root: &Path, profile: &PersonProfile) -> Result<(), String> {
    let dir = profiles_dir(root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create profiles dir {}: {e}", dir.display()))?;
    let path = dir.join(format!("{}.json", profile.person_id));
    let json_text = serde_json::to_string_pretty(profile).map_err(|e| format!("Serialization failed: {e}"))?;
    std::fs::write(&path, json_text).map_err(|e| format!("Write failed to {}: {e}", path.display()))
}

fn tool_social_profile(args: &OrganArgs) -> Result<Value, String> {
    let root = get_workspace(args);
    let person_id = extract_arg(args, "person_id").unwrap_or_else(|| "owner".to_string());
    let profile = load_profile(&root, &person_id);
    Ok(json!(profile))
}

fn tool_social_note_habit(args: &OrganArgs) -> Result<Value, String> {
    let root = get_workspace(args);
    let person_id = extract_arg(args, "person_id").unwrap_or_else(|| "owner".to_string());
    let category = extract_arg(args, "category").unwrap_or_else(|| "preference".to_string());
    let observation = extract_arg(args, "observation")
        .ok_or_else(|| "Parameter 'observation' is required".to_string())?;

    let mut profile = load_profile(&root, &person_id);

    if category == "boundary" {
        if !profile.boundaries.contains(&observation) {
            profile.boundaries.push(observation.clone());
        }
    } else {
        profile.habits.push(HabitItem {
            category: category.clone(),
            observation: observation.clone(),
            recorded_ts: now_ts(),
        });
    }

    save_profile(&root, &profile)?;

    Ok(json!({
        "status": "ok",
        "action": "noted",
        "person_id": person_id,
        "category": category,
        "observation": observation,
        "total_habits": profile.habits.len(),
    }))
}

fn tool_social_tune_register(args: &OrganArgs) -> Result<Value, String> {
    let root = get_workspace(args);
    let person_id = extract_arg(args, "person_id").unwrap_or_else(|| "owner".to_string());
    let tone = extract_arg(args, "tone");
    let brevity = extract_arg(args, "brevity");
    let language = extract_arg(args, "language");

    let mut profile = load_profile(&root, &person_id);

    if let Some(t) = tone {
        profile.register.tone = t;
    }
    if let Some(b) = brevity {
        profile.register.brevity = b;
    }
    if let Some(l) = language {
        profile.language = l;
    }

    save_profile(&root, &profile)?;

    Ok(json!({
        "status": "ok",
        "action": "tuned",
        "person_id": person_id,
        "register": profile.register,
        "language": profile.language,
    }))
}

fn sense_active_interlocutor(args: &OrganArgs) -> Result<Value, String> {
    let root = get_workspace(args);
    let person_id = extract_arg(args, "person_id").unwrap_or_else(|| "owner".to_string());
    let profile = load_profile(&root, &person_id);

    let mut habit_lines = Vec::new();
    for h in profile.habits.iter().rev().take(5) {
        habit_lines.push(format!("- [{}] {}", h.category, h.observation));
    }

    let mut boundary_lines = Vec::new();
    for b in &profile.boundaries {
        boundary_lines.push(format!("- [boundary] {}", b));
    }

    let grounding = format!(
        "--- active interlocutor: {} ---\nlanguage: {}\nregister: tone={}, brevity={}, directness={}\nhabits:\n{}\nboundaries:\n{}\n",
        profile.person_id,
        profile.language,
        profile.register.tone,
        profile.register.brevity,
        profile.register.directness,
        if habit_lines.is_empty() { "(none yet)".to_string() } else { habit_lines.join("\n") },
        if boundary_lines.is_empty() { "(none)".to_string() } else { boundary_lines.join("\n") }
    );

    Ok(json!({
        "person_id": profile.person_id,
        "language": profile.language,
        "register": profile.register,
        "grounding": grounding,
    }))
}

fn stimulus_prolonged_absence(args: &OrganArgs) -> Result<Value, String> {
    let root = get_workspace(args);
    let person_id = extract_arg(args, "person_id").unwrap_or_else(|| "owner".to_string());
    let threshold_secs: u64 = extract_arg(args, "threshold_seconds")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(86400); // 24 hours

    let profile = load_profile(&root, &person_id);
    let elapsed = now_ts().saturating_sub(profile.last_seen_ts);
    let triggered = elapsed >= threshold_secs;

    Ok(json!({
        "status": "ok",
        "stimulus": "prolonged_absence",
        "person_id": person_id,
        "elapsed_seconds": elapsed,
        "threshold_seconds": threshold_secs,
        "triggered": triggered,
    }))
}

fn print_meta() {
    let meta = json!({
        "name": "social",
        "version": "0.3.0",
        "type": "cli",
        "description": "Relational memory, interlocutor profiles, habits, and communication register for Presence",
        "tools": [
            "social_profile",
            "social_note_habit",
            "social_tune_register"
        ],
        "senses": ["active_interlocutor"],
        "stimuli": ["prolonged_absence"],
        "slash_commands": ["whoami", "social"]
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
        let tools = json!(["social_profile", "social_note_habit", "social_tune_register"]);
        println!("{}", tools);
        return;
    }

    if let Some(pos) = args.iter().position(|a| a == "--sense") {
        let sense = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        let organ_args = OrganArgs::from_env();
        match sense {
            "active_interlocutor" | "" => match sense_active_interlocutor(&organ_args) {
                Ok(val) => organ_ok!("interlocutor" => val),
                Err(e) => organ_err!(e),
            },
            other => organ_err!(format!("Unknown sense: {other}")),
        }
    }

    if let Some(pos) = args.iter().position(|a| a == "--stimulus") {
        let stim = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        let organ_args = OrganArgs::from_env();
        match stim {
            "prolonged_absence" | "" => match stimulus_prolonged_absence(&organ_args) {
                Ok(val) => organ_ok!("result" => val),
                Err(e) => organ_err!(e),
            },
            other => organ_err!(format!("Unknown stimulus: {other}")),
        }
    }

    if let Some(pos) = args.iter().position(|a| a == "--tool") {
        let tool_name = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        let organ_args = OrganArgs::from_env();

        let res = match tool_name {
            "social_profile" => tool_social_profile(&organ_args),
            "social_note_habit" => tool_social_note_habit(&organ_args),
            "social_tune_register" => tool_social_tune_register(&organ_args),
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
    fn test_default_owner_profile_creation() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let profile = load_profile(root, "owner");
        assert_eq!(profile.person_id, "owner");
        assert!(profile.language.contains("Russian"));
        assert_eq!(profile.register.brevity, "concise");
        assert!(!profile.habits.is_empty());
    }

    #[test]
    fn test_note_habit_and_tune_register() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let res = tool_social_note_habit(&OrganArgs::from_args([
            "--workspace", root.to_str().unwrap(),
            "--person_id", "owner",
            "--category", "preference",
            "--observation", "Prefers Rust over TypeScript",
        ])).unwrap();
        assert_eq!(res["status"], "ok");

        let tuned = tool_social_tune_register(&OrganArgs::from_args([
            "--workspace", root.to_str().unwrap(),
            "--person_id", "owner",
            "--tone", "pedagogical",
        ])).unwrap();
        assert_eq!(tuned["status"], "ok");

        let updated = load_profile(root, "owner");
        assert_eq!(updated.register.tone, "pedagogical");
        assert!(updated.habits.iter().any(|h| h.observation.contains("Prefers Rust")));
    }

    #[test]
    fn test_active_interlocutor_grounding() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let res = sense_active_interlocutor(&OrganArgs::from_args([
            "--workspace", root.to_str().unwrap(),
        ])).unwrap();
        let grounding = res["grounding"].as_str().unwrap();
        assert!(grounding.contains("--- active interlocutor: owner ---"));
        assert!(grounding.contains("Russian dialogue"));
    }
}
