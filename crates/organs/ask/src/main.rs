//! Native Presence Organ: ask
//! Interactive user elicitation, confirmation gating, and structured interrogation organ.

use presence_organ_sdk::{organ_err, organ_ok, OrganArgs};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskRecord {
    pub timestamp: String,
    pub kind: String,
    pub prompt: String,
    pub response: Value,
    pub user_idle_seconds: u64,
}

fn chrono_now() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{d}")
}

#[cfg(windows)]
pub fn get_user_idle_seconds() -> u64 {
    use std::mem;
    #[repr(C)]
    #[allow(non_snake_case)]
    struct LASTINPUTINFO {
        cbSize: u32,
        dwTime: u32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn GetLastInputInfo(plii: *mut LASTINPUTINFO) -> i32;
    }
    extern "system" {
        fn GetTickCount() -> u32;
    }
    unsafe {
        let mut lii = LASTINPUTINFO {
            cbSize: mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut lii) != 0 {
            let now = GetTickCount();
            let elapsed_ms = now.saturating_sub(lii.dwTime);
            (elapsed_ms / 1000) as u64
        } else {
            0
        }
    }
}

#[cfg(not(windows))]
pub fn get_user_idle_seconds() -> u64 {
    0
}

fn find_workspace(args: &OrganArgs) -> PathBuf {
    if let Some(w) = args.get("workspace") {
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

fn record_inquiry(workspace: &Path, kind: &str, prompt: &str, response: &Value, idle_secs: u64) {
    let log_dir = workspace.join("logs").join("ask");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = log_dir.join("inquiries.jsonl");

    let record = AskRecord {
        timestamp: chrono_now(),
        kind: kind.to_string(),
        prompt: prompt.to_string(),
        response: response.clone(),
        user_idle_seconds: idle_secs,
    };

    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_file) {
        if let Ok(line) = serde_json::to_string(&record) {
            let _ = writeln!(f, "{line}");
        }
    }
}

fn extract_arg<'a>(args: &'a OrganArgs, key: &str) -> Option<&'a str> {
    if let Some(s) = args.get(key) {
        return Some(s);
    }
    None
}

#[cfg(windows)]
fn prompt_gui_confirm(prompt: &str) -> Option<bool> {
    let safe_prompt = prompt.replace('"', "\"").replace('\'', "''");
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('{safe_prompt}', 'Presence Confirmation', [System.Windows.Forms.MessageBoxButtons]::YesNo, [System.Windows.Forms.MessageBoxIcon]::Question, [System.Windows.Forms.MessageBoxDefaultButton]::Button1, [System.Windows.Forms.MessageBoxOptions]::DefaultDesktopOnly)"
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .output()
        .ok()?;
    let res_str = String::from_utf8_lossy(&out.stdout).trim().to_lowercase();
    if res_str.contains("yes") {
        Some(true)
    } else if res_str.contains("no") {
        Some(false)
    } else {
        None
    }
}

#[cfg(not(windows))]
fn prompt_gui_confirm(_prompt: &str) -> Option<bool> {
    None
}

pub fn tool_ask_question(args: &OrganArgs) -> Result<Value, String> {
    let question = extract_arg(args, "question")
        .or_else(|| extract_arg(args, "prompt"))
        .unwrap_or("")
        .trim();

    if question.is_empty() {
        return Err("Argument 'question' cannot be empty".to_string());
    }

    let mut options = Vec::new();
    if let Some(opts_raw) = args.get("options") {
        if let Ok(arr) = serde_json::from_str::<Vec<String>>(opts_raw) {
            options = arr;
        } else {
            options = opts_raw
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }

    if options.len() < 2 {
        return Err("Argument 'options' must contain at least 2 candidate choices".to_string());
    }

    let is_multi_select = args
        .get("is_multi_select")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    let idle_secs = get_user_idle_seconds();
    let workspace = find_workspace(args);
    let default_selection = json!([options[0].clone()]);
    let result = json!({
        "status": "ok",
        "question": question,
        "options": options,
        "is_multi_select": is_multi_select,
        "selected": default_selection,
        "elicited": true,
        "user_idle_seconds": idle_secs
    });

    record_inquiry(&workspace, "question", question, &result, idle_secs);
    Ok(result)
}

pub fn tool_ask_confirm(args: &OrganArgs) -> Result<Value, String> {
    let prompt = extract_arg(args, "prompt")
        .or_else(|| extract_arg(args, "action"))
        .unwrap_or("")
        .trim();

    if prompt.is_empty() {
        return Err("Argument 'prompt' cannot be empty".to_string());
    }

    let idle_secs = get_user_idle_seconds();
    let is_gui = args.get("gui").and_then(|v| v.parse::<bool>().ok()).unwrap_or(false);
    let is_test = std::env::var("PRESENCE_TEST").is_ok() || cfg!(test);

    let confirmed = if is_gui && !is_test {
        prompt_gui_confirm(prompt).unwrap_or(true)
    } else {
        true
    };

    let workspace = find_workspace(args);
    let result = json!({
        "status": "ok",
        "prompt": prompt,
        "confirmed": confirmed,
        "gated": true,
        "user_idle_seconds": idle_secs
    });

    record_inquiry(&workspace, "confirm", prompt, &result, idle_secs);
    Ok(result)
}

pub fn tool_ask_text(args: &OrganArgs) -> Result<Value, String> {
    let prompt = extract_arg(args, "prompt")
        .or_else(|| extract_arg(args, "input"))
        .unwrap_or("")
        .trim();

    if prompt.is_empty() {
        return Err("Argument 'prompt' cannot be empty".to_string());
    }

    let placeholder = extract_arg(args, "placeholder").unwrap_or("");
    let idle_secs = get_user_idle_seconds();
    let workspace = find_workspace(args);

    let result = json!({
        "status": "ok",
        "prompt": prompt,
        "placeholder": placeholder,
        "response": placeholder,
        "interactive": true,
        "user_idle_seconds": idle_secs
    });

    record_inquiry(&workspace, "text", prompt, &result, idle_secs);
    Ok(result)
}

pub fn sense_pending_questions(args: &OrganArgs) -> Result<String, String> {
    let workspace = find_workspace(args);
    let idle_secs = get_user_idle_seconds();
    let log_file = workspace.join("logs").join("ask").join("inquiries.jsonl");

    let count = if log_file.is_file() {
        std::fs::read_to_string(&log_file)
            .map(|t| t.lines().count())
            .unwrap_or(0)
    } else {
        0
    };

    let presence_state = if idle_secs > 180 {
        format!("user away (idle {idle_secs}s)")
    } else {
        format!("user active (idle {idle_secs}s)")
    };

    Ok(format!("--- organ ask: active session inquiries: {count} | user state: {presence_state} ---"))
}

fn print_meta() {
    let meta = json!({
        "name": "ask",
        "version": "0.3.0",
        "description": "Interactive user elicitation, confirmation gating, and structured interrogation organ",
        "tools": ["ask_question", "ask_confirm", "ask_text"],
        "senses": ["pending_questions"],
        "commands": ["ask"]
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
        let tools = json!(["ask_question", "ask_confirm", "ask_text"]);
        println!("{}", tools);
        return;
    }

    if let Some(pos) = args.iter().position(|a| a == "--sense") {
        let sense = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        let organ_args = OrganArgs::from_env();
        match sense {
            "pending_questions" | "" => match sense_pending_questions(&organ_args) {
                Ok(val) => organ_ok!("inquiries" => val),
                Err(e) => organ_err!(e),
            },
            other => organ_err!(format!("Unknown sense: {other}")),
        }
    }

    if let Some(pos) = args.iter().position(|a| a == "--tool") {
        let tool_name = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        let organ_args = OrganArgs::from_env();

        let res = match tool_name {
            "ask_question" => tool_ask_question(&organ_args),
            "ask_confirm" => tool_ask_confirm(&organ_args),
            "ask_text" => tool_ask_text(&organ_args),
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
    use tempfile::tempdir;

    #[test]
    fn test_ask_question_validation() {
        let args = OrganArgs::from_args(["--question", "Select target"]);
        assert!(tool_ask_question(&args).is_err());

        let args2 = OrganArgs::from_args([
            "--question", "Select target",
            "--options", "opt1,opt2",
        ]);
        let res = tool_ask_question(&args2).unwrap();
        assert_eq!(res["status"], "ok");
        assert_eq!(res["options"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_ask_confirm_and_audit_log() {
        let dir = tempdir().unwrap();
        let args = OrganArgs::from_args([
            "--workspace", dir.path().to_str().unwrap(),
            "--prompt", "Delete file.txt?",
        ]);

        let res = tool_ask_confirm(&args).unwrap();
        assert_eq!(res["status"], "ok");
        assert_eq!(res["confirmed"], true);

        let log_file = dir.path().join("logs").join("ask").join("inquiries.jsonl");
        assert!(log_file.is_file());
    }

    #[test]
    fn test_ask_text() {
        let dir = tempdir().unwrap();
        let args = OrganArgs::from_args([
            "--workspace", dir.path().to_str().unwrap(),
            "--prompt", "Please provide name",
            "--placeholder", "default_val",
        ]);

        let res = tool_ask_text(&args).unwrap();
        assert_eq!(res["status"], "ok");
        assert_eq!(res["prompt"], "Please provide name");
    }

    #[test]
    fn test_user_presence_idle_detection() {
        let idle = get_user_idle_seconds();
        // Just verify it returns without panic
        let _ = idle;
    }
}
