//! Autonomous bug, UX friction, and GitHub telemetry reporting organ for Presence.
//!
//! Tools:
//! - report_issue: Formats structured markdown issue report, saves locally to logs/issues/, and optionally exports to GitHub.
//! - log_friction: Fast lightweight friction logging to logs/friction.jsonl.
//! - list_issues: Queries recorded local issues.
//!
//! Senses:
//! - recent_friction: Returns recent friction events and failure counts for the active session.

use presence_organ_sdk::{organ_err, organ_ok, OrganArgs, OrganCompatibility, OrganResponse};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug)]
pub struct FrictionEntry {
    pub ts: f64,
    pub category: String,
    pub summary: String,
    pub details: String,
    pub severity: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IssueSummary {
    pub filename: String,
    pub title: String,
    pub category: String,
    pub created_ts: f64,
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
        if cur.join("memory").is_dir() || cur.join("logs").is_dir() || cur.join("AGENTS.md").is_file() {
            return cur;
        }
        if !cur.pop() {
            break;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}


pub fn sanitize_pii(text: &str) -> String {
    let mut out = text.to_string();

    // 1. Redact username if known from environment (USERNAME on Windows, USER on Linux)
    if let Ok(user) = std::env::var("USERNAME").or_else(|_| std::env::var("USER")) {
        if !user.is_empty() && user.len() > 1 {
            out = out.replace(&user, "<username>");
        }
    }

    // 2. Redact user profile / home directory paths (USERPROFILE, HOME)
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        if !home.is_empty() {
            let backslash_home = home.replace('/', "\\");
            out = out.replace(&backslash_home, "~");
            let fwd_home = home.replace('\\', "/");
            out = out.replace(&fwd_home, "~");
            out = out.replace(&home, "~");
        }
    }

    // 3. Redact common secret patterns (API keys, GitHub tokens, Bearer tokens)
    redact_tokens(&out)
}

fn redact_tokens(s: &str) -> String {
    let mut res = String::with_capacity(s.len());
    for line in s.lines() {
        let mut line_str = line.to_string();
        for prefix in &["sk-", "ghp_", "gho_", "github_pat_", "key-", "bearer "] {
            let mut search_from = 0;
            while let Some(idx) = line_str[search_from..].to_lowercase().find(prefix) {
                let abs_idx = search_from + idx;
                let rest = &line_str[abs_idx..];
                let token_len = rest.find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == ')' || c == ']' || c == '}')
                    .unwrap_or(rest.len());
                if token_len > prefix.len() + 4 {
                    let before = &line_str[..abs_idx];
                    let after = &line_str[abs_idx + token_len..];
                    line_str = format!("{before}<redacted-token>{after}");
                    search_from = abs_idx + "<redacted-token>".len();
                } else {
                    search_from = abs_idx + prefix.len();
                }
            }
        }
        res.push_str(&line_str);
        res.push('\n');
    }
    if !s.ends_with('\n') && res.ends_with('\n') {
        res.pop();
    }
    res
}

fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars().take(40) {
        if c.is_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    if out.is_empty() {
        "issue".to_string()
    } else {
        out.trim_end_matches('-').to_string()
    }
}

pub fn tool_log_friction(
    root: &Path,
    category: &str,
    summary: &str,
    details: &str,
    severity: &str,
) -> OrganResponse {
    let sum_raw = summary.trim();
    if sum_raw.is_empty() {
        organ_err!("summary cannot be empty");
    }
    let sum = sanitize_pii(sum_raw);
    let details = sanitize_pii(details);

    let logs_dir = root.join("logs");
    let _ = fs::create_dir_all(&logs_dir);
    let friction_file = logs_dir.join("friction.jsonl");

    let entry = FrictionEntry {
        ts: now_ts(),
        category: if category.is_empty() { "friction".into() } else { category.into() },
        summary: sum.to_string(),
        details: details.to_string(),
        severity: if severity.is_empty() { "normal".into() } else { severity.into() },
    };

    match OpenOptions::new().create(true).append(true).open(&friction_file) {
        Ok(mut f) => {
            if let Ok(serialized) = serde_json::to_string(&entry) {
                let _ = writeln!(f, "{serialized}");
            }
            organ_ok! {
                "action" => "logged",
                "category" => entry.category,
                "summary" => entry.summary,
                "severity" => entry.severity,
                "path" => friction_file.to_string_lossy().to_string()
            }
        }
        Err(e) => organ_err!(format!("failed to append to friction log: {e}")),
    }
}

pub fn tool_report_issue(
    root: &Path,
    title: &str,
    body: &str,
    category: &str,
    submit_github: bool,
) -> OrganResponse {
    let t_raw = title.trim();
    if t_raw.is_empty() {
        organ_err!("title cannot be empty");
    }
    let t = sanitize_pii(t_raw);
    let body = sanitize_pii(body);

    let issues_dir = root.join("logs").join("issues");
    let _ = fs::create_dir_all(&issues_dir);

    let ts = now_ts();
    let slug = slugify(&t);
    let filename = format!("{:.0}-{slug}.md", ts);
    let filepath = issues_dir.join(&filename);

    let cat = if category.is_empty() { "bug" } else { category };
    let os_info = std::env::consts::OS;

    let content = format!(
        "# Issue: {t}\n\n\
        - **Category**: {cat}\n\
        - **Timestamp**: {ts}\n\
        - **OS**: {os_info}\n\
        - **Presence Version**: 0.3.0-beta\n\n\
        ## Description\n\
        {body}\n"
    );

    if let Err(e) = fs::write(&filepath, &content) {
        organ_err!(format!("failed to write issue file: {e}"));
    }

    let mut github_result = "local_only".to_string();
    if submit_github {
        let gh_check = Command::new("gh").arg("--version").output();
        if let Ok(chk) = gh_check {
            if chk.status.success() {
                let create = Command::new("gh")
                    .args(&["issue", "create", "--title", &t, "--body", &content])
                    .output();
                match create {
                    Ok(out) if out.status.success() => {
                        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        github_result = format!("submitted: {stdout}");
                    }
                    Ok(out) => {
                        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                        github_result = format!("gh error: {stderr}");
                    }
                    Err(e) => {
                        github_result = format!("gh exec failed: {e}");
                    }
                }
            } else {
                github_result = "gh CLI not authenticated or installed".to_string();
            }
        } else {
            github_result = "gh CLI not found on PATH".to_string();
        }
    }

    organ_ok! {
        "action" => "reported",
        "file" => filepath.to_string_lossy().to_string(),
        "filename" => filename,
        "title" => t,
        "category" => cat,
        "github_status" => github_result
    }
}

pub fn tool_list_issues(root: &Path, category_filter: &str) -> OrganResponse {
    let issues_dir = root.join("logs").join("issues");
    if !issues_dir.is_dir() {
        organ_ok!("issues" => Vec::<IssueSummary>::new(), "count" => 0)
    }

    let mut items: Vec<IssueSummary> = Vec::new();
    if let Ok(entries) = fs::read_dir(&issues_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                if let Ok(raw) = fs::read_to_string(&path) {
                    let mut title = name.clone();
                    let mut cat = "unknown".to_string();
                    for line in raw.lines() {
                        if line.starts_with("# Issue: ") {
                            title = line.trim_start_matches("# Issue: ").trim().to_string();
                        } else if line.starts_with("- **Category**: ") {
                            cat = line.trim_start_matches("- **Category**: ").trim().to_string();
                        }
                    }
                    if category_filter.is_empty() || cat.eq_ignore_ascii_case(category_filter) {
                        items.push(IssueSummary {
                            filename: name,
                            title,
                            category: cat,
                            created_ts: 0.0,
                        });
                    }
                }
            }
        }
    }

    let count = items.len();
    organ_ok!("issues" => items, "count" => count)
}

pub fn sense_recent_friction(root: &Path) -> OrganResponse {
    let friction_file = root.join("logs").join("friction.jsonl");
    if !friction_file.is_file() {
        organ_ok!("count" => 0, "entries" => Vec::<Value>::new())
    }

    match fs::read_to_string(&friction_file) {
        Ok(raw) => {
            let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
            let mut entries: Vec<Value> = Vec::new();
            let start = if lines.len() > 10 { lines.len() - 10 } else { 0 };
            for line in &lines[start..] {
                if let Ok(v) = serde_json::from_str::<Value>(line) {
                    entries.push(v);
                }
            }
            let count = entries.len();
            organ_ok!("count" => count, "entries" => entries)
        }
        Err(e) => organ_err!(format!("failed to read friction log: {e}")),
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
        println!("{}", serde_json::to_string(&compat).unwrap());
        std::process::exit(0);
    }

    let root = find_workspace_root();
    let op = args.op().to_lowercase();

    let response = if op == "recent_friction" || op == "sense_recent_friction" || op.starts_with("recent") || (args.has("sense") && !args.has("log_friction")) {
        sense_recent_friction(&root)
    } else if op.contains("report") || args.has("report_issue") || args.has("title") {
        let title = args.get_or("title", "");
        let body = args.get_or("body", &args.get_or("description", ""));
        let category = args.get_or("category", "bug");
        let submit_gh = args.has("submit_github") || args.has("github");
        tool_report_issue(&root, &title, &body, &category, submit_gh)
    } else if op.contains("list") || args.has("list_issues") {
        let cat = args.get_or("category", "");
        tool_list_issues(&root, &cat)
    } else if op.contains("friction") || args.has("log_friction") || args.has("summary") {
        let cat = args.get_or("category", "friction");
        let sum = args.get_or("summary", &args.get_or("message", ""));
        let det = args.get_or("details", "");
        let sev = args.get_or("severity", "normal");
        tool_log_friction(&root, &cat, &sum, &det, &sev)
    } else {
        organ_err!("unknown tool or sense operation for organ-issues. Available: report_issue, log_friction, list_issues, recent_friction")
    };

    response.print_and_exit();
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_log_friction_and_recent_sense() {
        let temp_dir = std::env::temp_dir().join(format!("presence_issues_test_{}", Uuid::new_v4()));
        let _ = fs::create_dir_all(&temp_dir);

        let res = tool_log_friction(&temp_dir, "tool_failure", "Failed to compile crate", "exit status 1", "high");
        assert_eq!(res.status, "ok");

        let sense = sense_recent_friction(&temp_dir);
        assert_eq!(sense.status, "ok");
        assert_eq!(sense.data["count"], 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_report_issue_and_list() {
        let temp_dir = std::env::temp_dir().join(format!("presence_issues_test2_{}", Uuid::new_v4()));
        let _ = fs::create_dir_all(&temp_dir);

        let res = tool_report_issue(&temp_dir, "Sandbox Permission Denied", "Bubblewrap failed to mount tmpfs", "bug", false);
        assert_eq!(res.status, "ok");
        assert_eq!(res.data["title"], "Sandbox Permission Denied");

        let list = tool_list_issues(&temp_dir, "");
        assert_eq!(list.status, "ok");
        assert_eq!(list.data["count"], 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_sanitize_pii_redacts_tokens() {
        let text = "Error with api key sk-1234567890abcdef and token ghp_secret_gh_token_value_xyz in log";
        let sanitized = sanitize_pii(text);
        assert!(!sanitized.contains("sk-1234567890abcdef"));
        assert!(!sanitized.contains("ghp_secret_gh_token_value_xyz"));
        assert!(sanitized.contains("<redacted-token>"));
    }
}
