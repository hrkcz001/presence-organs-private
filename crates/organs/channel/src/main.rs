//! Universal client bridge and communication channel organ for Presence.
//! Acts as:
//! 1. An organ tool/sense (send_reply, send_status, incoming_inbox) for agent interactions.
//! 2. An ACP client adapter (organ-channel --acp) bridging Zed JSON-RPC to Presence daemon.

use presence_organ_sdk::{organ_err, organ_ok, OrganArgs, OrganCompatibility, OrganResponse};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
struct ReplyEntry {
    ts: f64,
    channel: String,
    #[serde(rename = "type")]
    entry_type: String,
    message: String,
}

#[derive(Serialize, Deserialize)]
struct StatusEntry {
    ts: f64,
    channel: String,
    #[serde(rename = "type")]
    entry_type: String,
    status: String,
}

fn now_ts() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
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

fn get_outbox_file(root: &Path) -> PathBuf {
    let mem = root.join("memory");
    let _ = std::fs::create_dir_all(&mem);
    mem.join("outbox.jsonl")
}

fn get_inbox_file(root: &Path) -> PathBuf {
    let mem = root.join("memory");
    let _ = std::fs::create_dir_all(&mem);
    mem.join("inbox.jsonl")
}

pub fn tool_send_reply(root: &Path, message: &str, channel: &str) -> OrganResponse {
    let msg = message.trim();
    if msg.is_empty() {
        organ_err!("message cannot be empty");
    }
    let outbox = get_outbox_file(root);
    let entry = ReplyEntry {
        ts: now_ts(),
        channel: channel.to_string(),
        entry_type: "reply".to_string(),
        message: msg.to_string(),
    };
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&outbox) {
        if let Ok(line) = serde_json::to_string(&entry) {
            let _ = writeln!(f, "{line}");
        }
    }
    let preview: String = msg.chars().take(80).collect();
    organ_ok! {
        "action" => "sent",
        "channel" => channel,
        "delivered_chars" => msg.chars().count(),
        "preview" => preview
    }
}

pub fn tool_send_status(root: &Path, status: &str) -> OrganResponse {
    let st = status.trim();
    if st.is_empty() {
        organ_err!("status cannot be empty");
    }
    let outbox = get_outbox_file(root);
    let entry = StatusEntry {
        ts: now_ts(),
        channel: "active".to_string(),
        entry_type: "status".to_string(),
        status: st.to_string(),
    };
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&outbox) {
        if let Ok(line) = serde_json::to_string(&entry) {
            let _ = writeln!(f, "{line}");
        }
    }
    organ_ok!("action" => "status_reported")
}

pub fn sense_inbox(root: &Path) -> OrganResponse {
    let inbox = get_inbox_file(root);
    if !inbox.is_file() {
        organ_ok!("inbox" => Vec::<Value>::new());
    }
    match std::fs::read_to_string(&inbox) {
        Ok(raw) => {
            let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
            let mut entries: Vec<Value> = Vec::new();
            let start = if lines.len() > 5 { lines.len() - 5 } else { 0 };
            for line in &lines[start..] {
                if let Ok(v) = serde_json::from_str::<Value>(line) {
                    entries.push(v);
                }
            }
            organ_ok!("inbox" => entries)
        }
        Err(e) => organ_err!(format!("failed to read inbox: {e}")),
    }
}

/// Standalone ACP JSON-RPC bridge for Zed and external editors.
pub fn run_acp_bridge() {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err_res = json!({
                    "jsonrpc": "2.0",
                    "error": {"code": -32700, "message": format!("parse error: {e}")}
                });
                let _ = writeln!(stdout, "{err_res}");
                let _ = stdout.flush();
                continue;
            }
        };

        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");

        match method {
            "initialize" => {
                if let Some(id) = id {
                    let resp = json!({
                        "jsonrpc": "2.0", "id": id,
                        "result": {
                            "protocolVersion": 1,
                            "agentCapabilities": {"loadSession": true},
                            "agentInfo": {"name": "presence-channel", "title": "Presence Channel (ACP)", "version": "0.3.0"},
                        }
                    });
                    let _ = writeln!(stdout, "{resp}");
                    let _ = stdout.flush();
                }
            }
            "session/new" => {
                if let Some(id) = id {
                    let sid = format!("c-{}", uuid::Uuid::new_v4());
                    let resp = json!({
                        "jsonrpc": "2.0", "id": id,
                        "result": {"sessionId": sid}
                    });
                    let _ = writeln!(stdout, "{resp}");
                    let _ = stdout.flush();
                }
            }
            "session/prompt" => {
                if let Some(id) = id {
                    let prompt_text = msg.pointer("/params/prompt").and_then(Value::as_str).unwrap_or("");
                    let sid = msg.pointer("/params/sessionId").and_then(Value::as_str).unwrap_or("active");
                    // In ACP mode, channel can emit turn_finished and forward to Presence core
                    let resp = json!({
                        "jsonrpc": "2.0", "id": id,
                        "result": {"stopReason": "end_turn"}
                    });
                    let _ = writeln!(stdout, "{resp}");
                    let _ = stdout.flush();

                    // Echo out to outbox as user prompt event
                    let root = find_workspace_root();
                    let _ = tool_send_reply(&root, prompt_text, sid);
                }
            }
            "session/cancel" => {
                // Notification, no id needed
            }
            other => {
                if let Some(id) = id {
                    let resp = json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": {"code": -32601, "message": format!("method not found: {other}")}
                    });
                    let _ = writeln!(stdout, "{resp}");
                    let _ = stdout.flush();
                }
            }
        }
    }
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
        println!("{}", serde_json::to_string(&compat).unwrap()); std::process::exit(0);
    }

    if args.has("acp") || args.has("bridge") {
        run_acp_bridge();
        return;
    }

    let root = find_workspace_root();
    let op = args.op().to_lowercase();

    let response = if op.contains("status") || args.has("status") {
        let st = args.get_or("status", &args.get_or("message", ""));
        tool_send_status(&root, &st)
    } else if op.contains("inbox") || args.has("sense") || args.has("incoming_inbox") {
        sense_inbox(&root)
    } else {
        let msg = args.get_or("message", "");
        let ch = args.get_or("channel", "active");
        tool_send_reply(&root, &msg, &ch)
    };

    response.print_and_exit();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_reply_and_inbox() {
        let temp_dir = std::env::temp_dir().join(format!("presence_test_chan_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let res = tool_send_reply(&temp_dir, "Hello from test channel", "test-ch");
        assert_eq!(res.status, "ok");
        assert_eq!(res.data.get("channel").and_then(Value::as_str), Some("test-ch"));

        let outbox = get_outbox_file(&temp_dir);
        assert!(outbox.is_file());
        let content = std::fs::read_to_string(&outbox).unwrap();
        assert!(content.contains("Hello from test channel"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_send_status() {
        let temp_dir = std::env::temp_dir().join(format!("presence_test_stat_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let res = tool_send_status(&temp_dir, "Compiling crates...");
        assert_eq!(res.status, "ok");
        assert_eq!(res.data.get("action").and_then(Value::as_str), Some("status_reported"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}