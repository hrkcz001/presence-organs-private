//! Native pure-Rust physical input/output and filesystem manipulation organ for Presence.
use presence_organ_sdk::{organ_err, organ_ok, OrganArgs};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

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

fn resolve_path(workspace: &Path, p: &str) -> PathBuf {
    let path = PathBuf::from(p);
    if path.is_absolute() {
        path
    } else {
        workspace.join(path)
    }
}

pub fn tool_read_file(args: &OrganArgs) -> Result<Value, String> {
    let raw_path = args.get("path")
        .or_else(|| args.get("file"))
        .unwrap_or("")
        .trim();

    if raw_path.is_empty() {
        return Err("Argument 'path' cannot be empty".to_string());
    }

    let workspace = find_workspace(args);
    let full_path = resolve_path(&workspace, raw_path);

    if !full_path.exists() {
        return Err(format!("File does not exist: {}", full_path.display()));
    }

    if full_path.is_dir() {
        return Err(format!("Path is a directory, not a file: {}", full_path.display()));
    }

    match fs::read_to_string(&full_path) {
        Ok(content) => {
            let lines = content.lines().count();
            let bytes = content.len();
            Ok(json!({
                "status": "ok",
                "path": full_path.to_string_lossy().to_string(),
                "content": content,
                "lines": lines,
                "bytes": bytes
            }))
        }
        Err(e) => Err(format!("Failed to read file '{}': {e}", full_path.display())),
    }
}

pub fn tool_write_file(args: &OrganArgs) -> Result<Value, String> {
    let raw_path = args.get("path")
        .or_else(|| args.get("file"))
        .unwrap_or("")
        .trim();

    if raw_path.is_empty() {
        return Err("Argument 'path' cannot be empty".to_string());
    }

    let content = args.get("content").unwrap_or("");
    let workspace = find_workspace(args);
    let full_path = resolve_path(&workspace, raw_path);

    if let Some(parent) = full_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return Err(format!("Failed to create parent directory for '{}': {e}", full_path.display()));
        }
    }

    match fs::write(&full_path, content) {
        Ok(_) => Ok(json!({
            "status": "ok",
            "path": full_path.to_string_lossy().to_string(),
            "bytes_written": content.len()
        })),
        Err(e) => Err(format!("Failed to write file '{}': {e}", full_path.display())),
    }
}

pub fn tool_list_files(args: &OrganArgs) -> Result<Value, String> {
    let raw_path = args.get("path").unwrap_or(".").trim();
    let workspace = find_workspace(args);
    let full_path = if raw_path.is_empty() || raw_path == "." {
        workspace.clone()
    } else {
        resolve_path(&workspace, raw_path)
    };

    if !full_path.exists() {
        return Err(format!("Directory does not exist: {}", full_path.display()));
    }

    if !full_path.is_dir() {
        return Err(format!("Path is not a directory: {}", full_path.display()));
    }

    match fs::read_dir(&full_path) {
        Ok(read_dir) => {
            let mut entries = Vec::new();
            for item in read_dir.flatten() {
                let name = item.file_name().to_string_lossy().to_string();
                let metadata = item.metadata().ok();
                let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                entries.push(json!({
                    "name": name,
                    "is_dir": is_dir,
                    "size": size
                }));
            }
            Ok(json!({
                "status": "ok",
                "path": full_path.to_string_lossy().to_string(),
                "entries": entries
            }))
        }
        Err(e) => Err(format!("Failed to read directory '{}': {e}", full_path.display())),
    }
}

pub fn sense_fs_changes(args: &OrganArgs) -> Result<String, String> {
    let workspace = find_workspace(args);
    let mut modified = Vec::new();
    let now = std::time::SystemTime::now();

    if let Ok(entries) = fs::read_dir(&workspace) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(mtime) = meta.modified() {
                    if let Ok(dur) = now.duration_since(mtime) {
                        if dur.as_secs() < 300 {
                            modified.push(entry.file_name().to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    let report = if modified.is_empty() {
        "--- organ io: workspace stable, no recent file modifications ---".to_string()
    } else {
        format!("--- organ io: recent changes: {} ---", modified.join(", "))
    };
    Ok(report)
}

fn print_meta() {
    let meta = json!({
        "name": "io",
        "version": "0.3.0",
        "description": "Native pure-Rust physical input/output and filesystem manipulation organ",
        "tools": ["read_file", "write_file", "list_files"],
        "senses": ["fs_changes"],
        "commands": ["cat", "ls"]
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
        let tools = json!(["read_file", "write_file", "list_files"]);
        println!("{}", tools);
        return;
    }

    if let Some(pos) = args.iter().position(|a| a == "--sense") {
        let sense = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        let organ_args = OrganArgs::from_env();
        match sense {
            "fs_changes" | "" => match sense_fs_changes(&organ_args) {
                Ok(val) => organ_ok!("fs_changes" => val),
                Err(e) => organ_err!(e),
            },
            other => organ_err!(format!("Unknown sense: {other}")),
        }
    }

    if let Some(pos) = args.iter().position(|a| a == "--tool") {
        let tool_name = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        let organ_args = OrganArgs::from_env();

        let res = match tool_name {
            "read_file" => tool_read_file(&organ_args),
            "write_file" => tool_write_file(&organ_args),
            "list_files" => tool_list_files(&organ_args),
            other => Err(format!("Unknown tool: {other}")),
        };

        match res {
            Ok(val) => organ_ok!("result" => val),
            Err(e) => organ_err!(e),
        }
    }

    // Direct operation CLI fallback
    let organ_args = OrganArgs::from_env();
    let op = organ_args.op();
    let res = match op.as_ref() {
        "read_file" | "cat" => tool_read_file(&organ_args),
        "write_file" => tool_write_file(&organ_args),
        "list_files" | "ls" => tool_list_files(&organ_args),
        _ => Err(format!("Unknown operation: {op}")),
    };

    match res {
        Ok(val) => organ_ok!("result" => val),
        Err(e) => organ_err!(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_io_write_and_read_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("sub/test.txt");

        let write_args = OrganArgs::from_args([
            "--path", file_path.to_str().unwrap(),
            "--content", "Hello Native Presence IO!",
        ]);
        let w_res = tool_write_file(&write_args).unwrap();
        assert_eq!(w_res["status"], "ok");

        let read_args = OrganArgs::from_args([
            "--path", file_path.to_str().unwrap(),
        ]);
        let r_res = tool_read_file(&read_args).unwrap();
        assert_eq!(r_res["status"], "ok");
        assert_eq!(r_res["content"], "Hello Native Presence IO!");
        assert_eq!(r_res["lines"], 1);
    }

    #[test]
    fn test_io_list_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "a").unwrap();
        fs::write(dir.path().join("b.txt"), "b").unwrap();

        let list_args = OrganArgs::from_args([
            "--path", dir.path().to_str().unwrap(),
        ]);
        let l_res = tool_list_files(&list_args).unwrap();
        assert_eq!(l_res["status"], "ok");
        let entries = l_res["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
    }
}
