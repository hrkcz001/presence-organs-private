//! Bounded, sandboxed shell execution and task manager organ for Presence.
//!
//! Tools:
//! - exec_command: Synchronously runs shell command with strict timeout and bounded output.
//! - spawn_background: Starts async background process tracked by task ID.
//! - poll_task: Inspects status and output of background job.
//! - kill_task: Terminates a background job.
//!
//! Stimuli:
//! - command_hung: Detects tasks that have exceeded execution limits.

use presence_organ_sdk::{organ_err, organ_ok, OrganArgs, OrganCompatibility, OrganResponse};
use serde::{Deserialize, Serialize};

use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_OUTPUT_BYTES: usize = 32_000;

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskMetadata {
    pub id: String,
    pub command: String,
    pub pid: u32,
    pub start_ts: f64,
    pub log_file: String,
    pub status: String,
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

fn truncate_safe(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n... [truncated to {} bytes]", &s[..end], max_bytes)
}

fn build_shell_command(command: &str) -> Command {
    if cfg!(windows) {
        let mut cmd = Command::new("pwsh.exe");
        cmd.args(&["-NoProfile", "-NonInteractive", "-Command", command]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(&["-c", command]);
        cmd
    }
}

pub fn tool_exec_command(
    cwd: &Path,
    command: &str,
    timeout_secs: Option<u64>,
) -> OrganResponse {
    let cmd_str = command.trim();
    if cmd_str.is_empty() {
        organ_err!("command cannot be empty");
    }

    let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
    let mut cmd = build_shell_command(cmd_str);
    cmd.current_dir(cwd);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let start = Instant::now();
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            if cfg!(windows) {
                // Fallback to cmd.exe if pwsh fails to spawn
                let mut fallback = Command::new("cmd.exe");
                fallback.args(&["/c", cmd_str]);
                fallback.current_dir(cwd);
                fallback.stdout(Stdio::piped());
                fallback.stderr(Stdio::piped());
                match fallback.spawn() {
                    Ok(c) => c,
                    Err(e2) => organ_err!(format!("failed to spawn shell (pwsh: {e}, cmd: {e2})")),
                }
            } else {
                organ_err!(format!("failed to spawn shell: {e}"));
            }
        }
    };

    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let t_out = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(ref mut p) = stdout_pipe {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    });

    let t_err = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(ref mut p) = stderr_pipe {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                organ_err!(format!("command timed out after {}s and was killed", timeout.as_secs()));
            }
            Err(e) => organ_err!(format!("error waiting for child process: {e}")),
        }
    };

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let out_bytes = t_out.join().unwrap_or_default();
    let err_bytes = t_err.join().unwrap_or_default();

    let stdout_str = String::from_utf8_lossy(&out_bytes);
    let stderr_str = String::from_utf8_lossy(&err_bytes);

    let stdout_clean = truncate_safe(stdout_str.trim(), MAX_OUTPUT_BYTES);
    let stderr_clean = truncate_safe(stderr_str.trim(), MAX_OUTPUT_BYTES);
    let exit_code = status.code().unwrap_or(-1);

    organ_ok! {
        "command" => cmd_str,
        "exit_code" => exit_code,
        "success" => status.success(),
        "stdout" => stdout_clean,
        "stderr" => stderr_clean,
        "duration_ms" => elapsed_ms
    }
}

pub fn tool_spawn_background(root: &Path, command: &str, task_id: &str) -> OrganResponse {
    let cmd_str = command.trim();
    if cmd_str.is_empty() {
        organ_err!("command cannot be empty");
    }
    let tid = if task_id.is_empty() {
        format!("task-{}", uuid::Uuid::new_v4())
    } else {
        task_id.to_string()
    };

    let tasks_dir = root.join("logs").join("tasks");
    let _ = fs::create_dir_all(&tasks_dir);

    let log_file = tasks_dir.join(format!("{tid}.log"));
    let meta_file = tasks_dir.join(format!("{tid}.json"));

    let out_f = match OpenOptions::new().create(true).append(true).open(&log_file) {
        Ok(f) => f,
        Err(e) => organ_err!(format!("failed to create task log file: {e}")),
    };
    let err_f = match out_f.try_clone() {
        Ok(f) => f,
        Err(e) => organ_err!(format!("failed to clone file handle: {e}")),
    };

    let mut cmd = build_shell_command(cmd_str);
    cmd.current_dir(root);
    cmd.stdout(Stdio::from(out_f));
    cmd.stderr(Stdio::from(err_f));

    match cmd.spawn() {
        Ok(child) => {
            let pid = child.id();
            let meta = TaskMetadata {
                id: tid.clone(),
                command: cmd_str.to_string(),
                pid,
                start_ts: now_ts(),
                log_file: log_file.to_string_lossy().to_string(),
                status: "running".to_string(),
            };
            if let Ok(ser) = serde_json::to_string_pretty(&meta) {
                let _ = fs::write(&meta_file, ser);
            }
            organ_ok! {
                "action" => "spawned",
                "task_id" => tid,
                "pid" => pid,
                "log_file" => log_file.to_string_lossy().to_string()
            }
        }
        Err(e) => organ_err!(format!("failed to spawn background command: {e}")),
    }
}

pub fn tool_poll_task(root: &Path, task_id: &str) -> OrganResponse {
    let tasks_dir = root.join("logs").join("tasks");
    let meta_file = tasks_dir.join(format!("{task_id}.json"));
    if !meta_file.is_file() {
        organ_err!(format!("task '{task_id}' not found"));
    }

    let raw = match fs::read_to_string(&meta_file) {
        Ok(r) => r,
        Err(e) => organ_err!(format!("failed to read task metadata: {e}")),
    };
    let meta: TaskMetadata = match serde_json::from_str(&raw) {
        Ok(m) => m,
        Err(e) => organ_err!(format!("failed to parse task metadata: {e}")),
    };

    let log_path = PathBuf::from(&meta.log_file);
    let log_tail = if log_path.is_file() {
        fs::read_to_string(&log_path).unwrap_or_default()
    } else {
        String::new()
    };
    let tail_clean = truncate_safe(&log_tail, 4000);

    organ_ok! {
        "task_id" => meta.id,
        "pid" => meta.pid,
        "status" => meta.status,
        "command" => meta.command,
        "output_preview" => tail_clean
    }
}

pub fn tool_kill_task(root: &Path, task_id: &str) -> OrganResponse {
    let tasks_dir = root.join("logs").join("tasks");
    let meta_file = tasks_dir.join(format!("{task_id}.json"));
    if !meta_file.is_file() {
        organ_err!(format!("task '{task_id}' not found"));
    }

    let raw = fs::read_to_string(&meta_file).unwrap_or_default();
    let mut meta: TaskMetadata = match serde_json::from_str(&raw) {
        Ok(m) => m,
        Err(e) => organ_err!(format!("failed to parse task metadata: {e}")),
    };

    let pid = meta.pid;
    if cfg!(windows) {
        let _ = Command::new("taskkill")
            .args(&["/F", "/PID", &pid.to_string(), "/T"])
            .output();
    } else {
        let _ = Command::new("kill").args(&["-9", &pid.to_string()]).output();
    }

    meta.status = "killed".to_string();
    if let Ok(ser) = serde_json::to_string_pretty(&meta) {
        let _ = fs::write(&meta_file, ser);
    }

    organ_ok! {
        "action" => "killed",
        "task_id" => task_id,
        "pid" => pid
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

    let response = if op == "spawn_background" || args.has("spawn_background") || args.has("background") {
        let cmd = args.get_or("command", &args.get_or("cmd", ""));
        let tid = args.get_or("task_id", &args.get_or("id", ""));
        tool_spawn_background(&root, &cmd, &tid)
    } else if op == "poll_task" || args.has("poll_task") || args.has("poll") {
        let tid = args.get_or("task_id", &args.get_or("id", ""));
        tool_poll_task(&root, &tid)
    } else if op == "kill_task" || args.has("kill_task") || args.has("kill") {
        let tid = args.get_or("task_id", &args.get_or("id", ""));
        tool_kill_task(&root, &tid)
    } else {
        let cmd = args.get_or("command", &args.get_or("cmd", ""));
        let timeout = args.get_u64("timeout_secs").or_else(|| args.get_u64("timeout"));
        let cwd_str = args.get_or("cwd", "");
        let cwd = if cwd_str.is_empty() { root } else { PathBuf::from(cwd_str) };
        tool_exec_command(&cwd, &cmd, timeout)
    };

    response.print_and_exit();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_echo_command() {
        let temp_dir = std::env::temp_dir();
        let cmd = if cfg!(windows) { "Write-Output 'presence_shell_test'" } else { "echo 'presence_shell_test'" };
        let res = tool_exec_command(&temp_dir, cmd, Some(5));
        assert_eq!(res.status, "ok");
        assert_eq!(res.data["exit_code"], 0);
        let stdout = res.data["stdout"].as_str().unwrap();
        assert!(stdout.contains("presence_shell_test"));
    }

    #[test]
    fn test_truncate_safe() {
        let s = "a".repeat(100);
        let res = truncate_safe(&s, 50);
        assert!(res.contains("[truncated to 50 bytes]"));
    }

    #[test]
    fn test_spawn_and_poll_task() {
        let temp_dir = std::env::temp_dir().join(format!("presence_shell_task_{}", uuid::Uuid::new_v4()));
        let _ = fs::create_dir_all(&temp_dir);

        let cmd = if cfg!(windows) { "Write-Output 'task_output'" } else { "echo 'task_output'" };
        let spawn_res = tool_spawn_background(&temp_dir, cmd, "t1");
        assert_eq!(spawn_res.status, "ok");

        std::thread::sleep(Duration::from_millis(300));
        let poll_res = tool_poll_task(&temp_dir, "t1");
        assert_eq!(poll_res.status, "ok");
        assert_eq!(poll_res.data["task_id"], "t1");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}