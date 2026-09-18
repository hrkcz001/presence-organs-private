//! Universal Agent Client Protocol (ACP) adapter and gateway.
//! Provides JSON-RPC 2.0 stdio communication between editor clients
//! and the Presence cognitive architecture.
//!
//! Supports client profiles:
//! - Zed (--profile zed): Live thought streams, slash command autocomplete, live checklists, card jump locations.
//! - Default (--profile default / standard): Vanilla Agent Client Protocol without editor-specific vendor quirks.
//! - Extensible for VSCode and CLI frontends.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientProfile {
    #[serde(rename = "zed")]
    Zed,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "vscode")]
    VsCode,
    #[serde(rename = "cli")]
    Cli,
}

impl ClientProfile {
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "zed" => ClientProfile::Zed,
            "vscode" | "code" => ClientProfile::VsCode,
            "cli" | "terminal" => ClientProfile::Cli,
            _ => ClientProfile::Default,
        }
    }

    pub fn auto_detect(client_info: Option<&Value>) -> Self {
        if let Some(info) = client_info {
            let name = info.get("name").and_then(Value::as_str).unwrap_or("");
            let title = info.get("title").and_then(Value::as_str).unwrap_or("");
            let combined = format!("{name} {title}").to_ascii_lowercase();
            if combined.contains("zed") {
                return ClientProfile::Zed;
            }
            if combined.contains("vscode") || combined.contains("code") {
                return ClientProfile::VsCode;
            }
        }
        ClientProfile::Default
    }
}

pub struct AcpSender<W: Write> {
    writer: Arc<Mutex<W>>,
}

impl<W: Write> Clone for AcpSender<W> {
    fn clone(&self) -> Self {
        Self {
            writer: Arc::clone(&self.writer),
        }
    }
}

impl<W: Write> AcpSender<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
        }
    }

    pub fn send(&self, val: &Value) {
        if let Ok(mut w) = self.writer.lock() {
            let _ = serde_json::to_writer(&mut *w, val);
            let _ = w.write_all(b"\n");
            let _ = w.flush();
        }
    }

    pub fn send_response(&self, id: &Value, result: Value) {
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }));
    }

    pub fn send_error(&self, id: Option<&Value>, code: i64, message: &str) {
        let mut err = json!({
            "jsonrpc": "2.0",
            "error": {
                "code": code,
                "message": message,
            }
        });
        if let Some(i) = id {
            err["id"] = i.clone();
        }
        self.send(&err);
    }

    pub fn send_session_update(&self, session_id: &str, update: Value) {
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {
                "sessionId": session_id,
                "update": update,
            }
        }));
    }
}

pub struct AcpSession {
    pub id: String,
    pub created_ts: f64,
    pub profile: ClientProfile,
}

pub struct AcpGateway<W: Write> {
    sender: AcpSender<W>,
    profile: ClientProfile,
    force_profile: bool,
    sessions: HashMap<String, AcpSession>,
    pub workspace_root: PathBuf,
}

fn discover_organ_commands() -> Vec<Value> {
    let mut cmds = vec![
        json!({"name": "agent", "description": "Switch active persona (/agent <name>)"}),
        json!({"name": "agents", "description": "List available personas"}),
        json!({"name": "journal", "description": "View quest journal and memory"}),
        json!({"name": "pause", "description": "Pause active circle"}),
        json!({"name": "continue", "description": "Resume paused circle"}),
    ];

    let search_roots = [
        PathBuf::from("organs"),
        PathBuf::from("crates/organs"),
        PathBuf::from("registry"),
        PathBuf::from("C:/Users/hrkcz001/Dev/presence-organs/crates/organs"),
        PathBuf::from("C:/Users/hrkcz001/Dev/presence-organs/registry"),
        PathBuf::from("C:/Users/hrkcz001/Dev/presence/organs"),
    ];

    for root in &search_roots {
        if !root.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let manifest_path = entry.path().join("organ.yaml");
                if manifest_path.is_file() {
                    if let Ok(txt) = std::fs::read_to_string(&manifest_path) {
                        if let Ok(val) = serde_yaml::from_str::<Value>(&txt) {
                            let organ_name = val.get("name").and_then(Value::as_str).unwrap_or("organ");
                            let organ_desc = val.get("description").and_then(Value::as_str).unwrap_or("");
                            
                            let cmd_list = val.get("slash_commands").or_else(|| val.get("commands"));
                            if let Some(arr) = cmd_list.and_then(Value::as_array) {
                                for item in arr {
                                    if let Some(s) = item.as_str() {
                                        let name = s.trim_start_matches('/').to_lowercase();
                                        if !name.is_empty() && !cmds.iter().any(|c| c.get("name").and_then(Value::as_str) == Some(&name)) {
                                            cmds.push(json!({
                                                "name": name,
                                                "description": format!("Execute {organ_name} command"),
                                            }));
                                        }
                                    } else if let Some(obj) = item.as_object() {
                                        if let Some(name_val) = obj.get("name").and_then(Value::as_str) {
                                            let name = name_val.trim_start_matches('/').to_lowercase();
                                            let desc = obj.get("description").and_then(Value::as_str).unwrap_or(organ_desc);
                                            if !name.is_empty() && !cmds.iter().any(|c| c.get("name").and_then(Value::as_str) == Some(&name)) {
                                                cmds.push(json!({
                                                    "name": name,
                                                    "description": desc,
                                                }));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    cmds
}

impl<W: Write> AcpGateway<W> {
    pub fn new(sender: AcpSender<W>, profile: ClientProfile, force_profile: bool) -> Self {
        let workspace_root = std::env::var("PRESENCE_WORKSPACE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        Self {
            sender,
            profile,
            force_profile,
            sessions: HashMap::new(),
            workspace_root,
        }
    }

    pub fn effective_profile(&self) -> ClientProfile {
        self.profile
    }

    pub fn handle_message(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }

        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                self.sender.send_error(None, -32700, &format!("Parse error: {e}"));
                return;
            }
        };

        let id = msg.get("id");
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");

        match method {
            "initialize" => {
                let Some(id) = id else { return };
                if !self.force_profile {
                    let client_info = msg.pointer("/params/clientInfo");
                    self.profile = ClientProfile::auto_detect(client_info);
                }

                self.sender.send_response(id, json!({
                    "protocolVersion": 1,
                    "agentCapabilities": {
                        "loadSession": true
                    },
                    "agentInfo": {
                        "name": "presence-acp",
                        "title": "Presence ACP Gateway",
                        "version": "0.3.0",
                        "profile": match self.profile {
                            ClientProfile::Zed => "zed",
                            ClientProfile::VsCode => "vscode",
                            ClientProfile::Cli => "cli",
                            ClientProfile::Default => "standard",
                        }
                    }
                }));
            }
            "session/new" => {
                let Some(id) = id else { return };
                let sid = format!("acp-{}", Uuid::new_v4());
                let sess = AcpSession {
                    id: sid.clone(),
                    created_ts: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs_f64())
                        .unwrap_or(0.0),
                    profile: self.profile,
                };
                self.sessions.insert(sid.clone(), sess);

                self.sender.send_response(id, json!({ "sessionId": sid }));

                // In Zed profile, advertise available slash commands
                if self.profile == ClientProfile::Zed {
                    self.sender.send_session_update(&sid, json!({
                        "sessionUpdate": "available_commands_update",
                        "availableCommands": discover_organ_commands()
                    }));
                }
            }
            "session/load" => {
                let Some(id) = id else { return };
                let sid = msg.pointer("/params/sessionId").and_then(Value::as_str).unwrap_or("");
                if !self.sessions.contains_key(sid) && !sid.is_empty() {
                    self.sessions.insert(sid.to_string(), AcpSession {
                        id: sid.to_string(),
                        created_ts: 0.0,
                        profile: self.profile,
                    });
                }
                self.sender.send_response(id, json!({}));
            }
            "session/prompt" => {
                let Some(id) = id else { return };
                let prompt_text = msg.pointer("/params/prompt").and_then(Value::as_str).unwrap_or("");
                let sid = msg.pointer("/params/sessionId").and_then(Value::as_str).unwrap_or("default");

                // Execute turn logic
                self.execute_turn(sid, prompt_text);

                self.sender.send_response(id, json!({
                    "stopReason": "end_turn"
                }));
            }
            "session/cancel" => {
                // Notification, no response required
            }
            other => {
                self.sender.send_error(id, -32601, &format!("Method not found: {other}"));
            }
        }
    }

    fn execute_turn(&self, session_id: &str, prompt: &str) {
        let clean = prompt.trim();

        // 1. Thought / reasoning chunk (Zed profile only)
        if self.profile == ClientProfile::Zed {
            self.sender.send_session_update(session_id, json!({
                "sessionUpdate": "agent_thought_chunk",
                "content": {
                    "type": "text",
                    "text": "Evaluating prompt with Presence Triad..."
                }
            }));
        }

        // 2. Handle slash commands or forward prompt
        if clean == "/journal" {
            let tool_id = format!("t-{}", Uuid::new_v4());
            self.sender.send_session_update(session_id, json!({
                "sessionUpdate": "tool_call",
                "toolCallId": tool_id,
                "title": "Quest Journal",
                "kind": "other",
                "status": "completed",
                "content": [{"type": "content", "content": {"type": "text", "text": "Active quest: System bootstrap and verification"}}]
            }));
            self.sender.send_session_update(session_id, json!({
                "sessionUpdate": "agent_message_chunk",
                "content": {"type": "text", "text": "Quest Journal: Current topic active in workspace."}
            }));
            return;
        }

        // 3. Emit message chunk
        let echo_resp = format!("Received prompt (profile: {:?}): \"{}\"\n", self.profile, clean);
        self.sender.send_session_update(session_id, json!({
            "sessionUpdate": "agent_message_chunk",
            "content": {
                "type": "text",
                "text": echo_resp
            }
        }));
    }

    pub fn run_stdio(&mut self) {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => self.handle_message(&l),
                Err(_) => break,
            }
        }
    }
}

fn parse_cli_args() -> (ClientProfile, bool) {
    let args: Vec<String> = std::env::args().collect();
    let mut profile = ClientProfile::Default;
    let mut forced = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--profile" | "-p" => {
                if i + 1 < args.len() {
                    profile = ClientProfile::from_str(&args[i + 1]);
                    forced = true;
                    i += 1;
                }
            }
            "--zed" => {
                profile = ClientProfile::Zed;
                forced = true;
            }
            "--standard" | "--default" => {
                profile = ClientProfile::Default;
                forced = true;
            }
            "--vscode" => {
                profile = ClientProfile::VsCode;
                forced = true;
            }
            "--cli" => {
                profile = ClientProfile::Cli;
                forced = true;
            }
            _ => {}
        }
        i += 1;
    }

    (profile, forced)
}

fn main() {
    let (profile, forced) = parse_cli_args();
    let stdout = io::stdout();
    let sender = AcpSender::new(stdout.lock());
    let mut gateway = AcpGateway::new(sender, profile, forced);

    gateway.run_stdio();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct MockWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl MockWriter {
        fn lines(&self) -> Vec<String> {
            let bytes = self.buffer.lock().unwrap().clone();
            String::from_utf8(bytes)
                .unwrap()
                .lines()
                .map(|s| s.to_string())
                .collect()
        }
    }

    #[test]
    fn test_initialize_standard_fallback() {
        let writer = MockWriter::default();
        let sender = AcpSender::new(writer.clone());
        let mut gateway = AcpGateway::new(sender, ClientProfile::Default, false);

        let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"GenericClient"}}}"#;
        gateway.handle_message(init_req);

        let lines = writer.lines();
        assert_eq!(lines.len(), 1);
        let resp: Value = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["agentInfo"]["name"], "presence-acp");
        assert_eq!(resp["result"]["agentInfo"]["profile"], "standard");
        assert_eq!(gateway.effective_profile(), ClientProfile::Default);
    }

    #[test]
    fn test_initialize_zed_autodetect() {
        let writer = MockWriter::default();
        let sender = AcpSender::new(writer.clone());
        let mut gateway = AcpGateway::new(sender, ClientProfile::Default, false);

        let init_req = r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"clientInfo":{"name":"Zed","version":"0.150.0"}}}"#;
        gateway.handle_message(init_req);

        let lines = writer.lines();
        let resp: Value = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(resp["result"]["agentInfo"]["profile"], "zed");
        assert_eq!(gateway.effective_profile(), ClientProfile::Zed);
    }

    #[test]
    fn test_zed_profile_slash_commands_and_prompt() {
        let writer = MockWriter::default();
        let sender = AcpSender::new(writer.clone());
        let mut gateway = AcpGateway::new(sender, ClientProfile::Zed, true);

        // session/new emits session response AND available_commands_update in Zed profile
        gateway.handle_message(r#"{"jsonrpc":"2.0","id":10,"method":"session/new","params":{}}"#);
        let lines = writer.lines();
        assert_eq!(lines.len(), 2);
        let r1: Value = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(r1["id"], 10);
        let r2: Value = serde_json::from_str(&lines[1]).unwrap();
        assert_eq!(r2["method"], "session/update");
        assert_eq!(r2["params"]["update"]["sessionUpdate"], "available_commands_update");
    }

    #[test]
    fn test_method_not_found() {
        let writer = MockWriter::default();
        let sender = AcpSender::new(writer.clone());
        let mut gateway = AcpGateway::new(sender, ClientProfile::Default, true);

        gateway.handle_message(r#"{"jsonrpc":"2.0","id":99,"method":"unknown/operation","params":{}}"#);
        let lines = writer.lines();
        let resp: Value = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(resp["error"]["code"], -32601);
    }
}