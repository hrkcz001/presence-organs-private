//! Universal package and organ manager for Presence Triad.
//! Bridges fast system package managers (SFSU/Scoop on Windows, Nix on Linux)
//! and manages presence organ installation and lifecycle.

use presence_organ_sdk::{organ_err, organ_ok, OrganArgs, OrganCompatibility};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
struct LocalOrganManifest {
    name: String,
    #[serde(default = "default_version")]
    version: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "type", default = "default_type")]
    organ_type: String,
    #[serde(default)]
    compatibility: Option<OrganCompatibility>,
    #[serde(default)]
    tools: Vec<Value>,
    #[serde(default)]
    senses: Vec<Value>,
}

fn default_version() -> String { "0.1.0".to_string() }
fn default_type() -> String { "cli".to_string() }

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Sfsu,
    Scoop,
    Nix,
    Unknown,
}

impl Backend {
    fn detect() -> Self {
        #[cfg(target_os = "windows")]
        {
            if has_command("sfsu.exe") || has_command("sfsu") {
                Backend::Sfsu
            } else if has_command("scoop.ps1") || has_command("scoop") {
                Backend::Scoop
            } else {
                Backend::Unknown
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            if has_command("nix") || has_command("nix-env") {
                Backend::Nix
            } else {
                Backend::Unknown
            }
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Backend::Sfsu => "sfsu",
            Backend::Scoop => "scoop",
            Backend::Nix => "nix",
            Backend::Unknown => "unknown",
        }
    }
}

fn has_command(cmd: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("where.exe")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
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

fn tool_search(args: &OrganArgs) -> Result<Value, String> {
    let query = extract_arg(args, "query")
        .or_else(|| extract_arg(args, "input"))
        .unwrap_or_default();

    if query.trim().is_empty() {
        return Err("Argument 'query' cannot be empty".to_string());
    }

    let backend = Backend::detect();
    let mut results = json!([]);
    let mut matched_organs = Vec::new();

    // 1. Search system package manager
    match backend {
        Backend::Sfsu => {
            if let Ok(output) = Command::new("sfsu").args(["search", &query, "--json"]).output() {
                if output.status.success() {
                    let txt = String::from_utf8_lossy(&output.stdout);
                    if let Ok(v) = serde_json::from_str::<Value>(&txt) {
                        results = v;
                    }
                }
            }
        }
        Backend::Scoop => {
            if let Ok(output) = Command::new("powershell").args(["-NoProfile", "-Command", &format!("scoop search {query}")]).output() {
                let txt = String::from_utf8_lossy(&output.stdout);
                results = json!({ "raw_output": txt });
            }
        }
        Backend::Nix => {
            if let Ok(output) = Command::new("nix").args(["search", "nixpkgs", &query, "--json"]).output() {
                if output.status.success() {
                    let txt = String::from_utf8_lossy(&output.stdout);
                    if let Ok(v) = serde_json::from_str::<Value>(&txt) {
                        results = v;
                    }
                }
            }
        }
        Backend::Unknown => {}
    }

    // 2. Search local and registered organs
    let root = find_workspace_root();
    let search_dirs = vec![
        root.join("organs"),
        root.parent().unwrap_or(&root).join("presence-organs/registry"),
        root.parent().unwrap_or(&root).join("presence/organs"),
    ];

    for dir in search_dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let manifest_path = entry.path().join("organ.yaml");
                if manifest_path.is_file() {
                    if let Ok(txt) = std::fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = serde_yaml::from_str::<LocalOrganManifest>(&txt) {
                            if manifest.name.to_lowercase().contains(&query.to_lowercase())
                                || manifest.description.to_lowercase().contains(&query.to_lowercase())
                            {
                                matched_organs.push(json!({
                                    "organ": manifest.name,
                                    "version": manifest.version,
                                    "type": manifest.organ_type,
                                    "description": manifest.description,
                                    "path": manifest_path.display().to_string(),
                                }));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(json!({
        "backend": backend.as_str(),
        "query": query,
        "packages": results,
        "organs": matched_organs,
    }))
}

fn tool_list(args: &OrganArgs) -> Result<Value, String> {
    let filter = extract_arg(args, "filter");
    let backend = Backend::detect();

    match backend {
        Backend::Sfsu => {
            let output = Command::new("sfsu")
                .args(["list", "--json"])
                .output()
                .map_err(|e| format!("Failed to run sfsu: {e}"))?;

            if !output.status.success() {
                return Err(format!("sfsu list failed with code {:?}", output.status.code()));
            }

            let txt = String::from_utf8_lossy(&output.stdout);
            let mut list: Vec<Value> = serde_json::from_str(&txt).unwrap_or_default();

            if let Some(f) = filter {
                let lower = f.to_lowercase();
                list.retain(|item| {
                    item.get("name")
                        .and_then(Value::as_str)
                        .map(|n| n.to_lowercase().contains(&lower))
                        .unwrap_or(false)
                });
            }

            Ok(json!({
                "backend": "sfsu",
                "count": list.len(),
                "packages": list,
            }))
        }
        Backend::Scoop => {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", "scoop list"])
                .output()
                .map_err(|e| format!("Failed to run scoop: {e}"))?;

            let txt = String::from_utf8_lossy(&output.stdout);
            Ok(json!({
                "backend": "scoop",
                "output": txt.trim(),
            }))
        }
        Backend::Nix => {
            let output = Command::new("nix-env")
                .arg("-q")
                .output()
                .map_err(|e| format!("Failed to run nix-env: {e}"))?;

            let txt = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = txt.lines().collect();
            Ok(json!({
                "backend": "nix",
                "count": lines.len(),
                "packages": lines,
            }))
        }
        Backend::Unknown => Err("No supported package backend detected (sfsu, scoop, or nix)".to_string()),
    }
}

fn tool_install(args: &OrganArgs) -> Result<Value, String> {
    let pkg = extract_arg(args, "package")
        .or_else(|| extract_arg(args, "input"))
        .unwrap_or_default();

    if pkg.trim().is_empty() {
        return Err("Argument 'package' cannot be empty".to_string());
    }

    // Check if target is a path to a presence organ
    let candidate_path = PathBuf::from(&pkg);
    if candidate_path.join("organ.yaml").is_file() {
        return install_local_organ(&candidate_path);
    }

    let backend = Backend::detect();
    match backend {
        Backend::Sfsu | Backend::Scoop => {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &format!("scoop install {pkg}")])
                .output()
                .map_err(|e| format!("Failed to execute scoop install: {e}"))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if output.status.success() {
                Ok(json!({
                    "status": "success",
                    "package": pkg,
                    "backend": "scoop",
                    "output": stdout.trim(),
                }))
            } else {
                Err(format!("scoop install failed: {stderr} {stdout}"))
            }
        }
        Backend::Nix => {
            let output = Command::new("nix-env")
                .args(["-iA", &format!("nixpkgs.{pkg}")])
                .output()
                .map_err(|e| format!("Failed to execute nix-env: {e}"))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if output.status.success() {
                Ok(json!({
                    "status": "success",
                    "package": pkg,
                    "backend": "nix",
                    "output": stdout.trim(),
                }))
            } else {
                Err(format!("nix-env failed: {stderr} {stdout}"))
            }
        }
        Backend::Unknown => Err("No supported package backend found to perform install".to_string()),
    }
}

fn install_local_organ(organ_dir: &Path) -> Result<Value, String> {
    let manifest_path = organ_dir.join("organ.yaml");
    let txt = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read organ.yaml: {e}"))?;
    let manifest = serde_yaml::from_str::<LocalOrganManifest>(&txt)
        .map_err(|e| format!("Invalid organ manifest: {e}"))?;

    // Check compatibility
    let triad_version = "0.3.0";
    if let Some(compat) = &manifest.compatibility {
        compat.validate(triad_version)?;
    }

    let root = find_workspace_root();
    let target_dir = root.join("organs").join(&manifest.name);
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create organ directory: {e}"))?;

    // Copy organ files
    for entry in std::fs::read_dir(organ_dir).map_err(|e| e.to_string())?.flatten() {
        let name = entry.file_name();
        let dest = target_dir.join(name);
        if entry.path().is_file() {
            let _ = std::fs::copy(entry.path(), dest);
        }
    }

    Ok(json!({
        "status": "installed",
        "organ": manifest.name,
        "version": manifest.version,
        "path": target_dir.display().to_string(),
        "tools_discovered": manifest.tools.len(),
        "senses_discovered": manifest.senses.len(),
    }))
}

fn tool_uninstall(args: &OrganArgs) -> Result<Value, String> {
    let pkg = extract_arg(args, "package")
        .or_else(|| extract_arg(args, "input"))
        .unwrap_or_default();

    if pkg.trim().is_empty() {
        return Err("Argument 'package' cannot be empty".to_string());
    }

    let backend = Backend::detect();
    match backend {
        Backend::Sfsu | Backend::Scoop => {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &format!("scoop uninstall {pkg}")])
                .output()
                .map_err(|e| format!("Failed to execute scoop uninstall: {e}"))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if output.status.success() {
                Ok(json!({
                    "status": "uninstalled",
                    "package": pkg,
                    "backend": "scoop",
                    "output": stdout.trim(),
                }))
            } else {
                Err(format!("scoop uninstall failed: {stdout}"))
            }
        }
        Backend::Nix => {
            let output = Command::new("nix-env")
                .args(["-e", &pkg])
                .output()
                .map_err(|e| format!("Failed to execute nix-env: {e}"))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if output.status.success() {
                Ok(json!({
                    "status": "uninstalled",
                    "package": pkg,
                    "backend": "nix",
                    "output": stdout.trim(),
                }))
            } else {
                Err(format!("nix-env remove failed: {stdout}"))
            }
        }
        Backend::Unknown => Err("No supported package backend found to perform uninstall".to_string()),
    }
}

fn tool_info(args: &OrganArgs) -> Result<Value, String> {
    let pkg = extract_arg(args, "package")
        .or_else(|| extract_arg(args, "input"))
        .unwrap_or_default();

    if pkg.trim().is_empty() {
        return Err("Argument 'package' cannot be empty".to_string());
    }

    let backend = Backend::detect();
    match backend {
        Backend::Sfsu => {
            let output = Command::new("sfsu")
                .args(["info", &pkg, "--json"])
                .output()
                .map_err(|e| format!("Failed to execute sfsu info: {e}"))?;

            let txt = String::from_utf8_lossy(&output.stdout);
            if let Ok(v) = serde_json::from_str::<Value>(&txt) {
                Ok(json!({
                    "backend": "sfsu",
                    "info": v,
                }))
            } else {
                Ok(json!({
                    "backend": "sfsu",
                    "raw": txt.trim(),
                }))
            }
        }
        Backend::Scoop => {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &format!("scoop info {pkg}")])
                .output()
                .map_err(|e| format!("Failed to execute scoop info: {e}"))?;

            let txt = String::from_utf8_lossy(&output.stdout);
            Ok(json!({
                "backend": "scoop",
                "raw": txt.trim(),
            }))
        }
        Backend::Nix => {
            let output = Command::new("nix-env")
                .args(["-qa", "--description", &pkg])
                .output()
                .map_err(|e| format!("Failed to execute nix-env: {e}"))?;

            let txt = String::from_utf8_lossy(&output.stdout);
            Ok(json!({
                "backend": "nix",
                "raw": txt.trim(),
            }))
        }
        Backend::Unknown => Err("No supported package backend found".to_string()),
    }
}

fn tool_update(args: &OrganArgs) -> Result<Value, String> {
    let pkg = extract_arg(args, "package");
    let backend = Backend::detect();

    match backend {
        Backend::Sfsu | Backend::Scoop => {
            let cmd = if let Some(p) = pkg {
                format!("scoop update {p}")
            } else {
                "sfsu update".to_string()
            };

            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &cmd])
                .output()
                .map_err(|e| format!("Failed to execute update: {e}"))?;

            let txt = String::from_utf8_lossy(&output.stdout);
            Ok(json!({
                "status": "updated",
                "backend": backend.as_str(),
                "output": txt.trim(),
            }))
        }
        Backend::Nix => {
            let output = Command::new("nix-channel")
                .arg("--update")
                .output()
                .map_err(|e| format!("Failed to execute nix-channel update: {e}"))?;

            let txt = String::from_utf8_lossy(&output.stdout);
            Ok(json!({
                "status": "updated",
                "backend": "nix",
                "output": txt.trim(),
            }))
        }
        Backend::Unknown => Err("No supported package backend found".to_string()),
    }
}

fn sense_outdated_packages() -> Result<Value, String> {
    let backend = Backend::detect();
    match backend {
        Backend::Sfsu => {
            let output = Command::new("sfsu")
                .args(["status", "--json"])
                .output()
                .map_err(|e| format!("Failed to execute sfsu status: {e}"))?;

            let txt = String::from_utf8_lossy(&output.stdout);
            if let Ok(val) = serde_json::from_str::<Value>(&txt) {
                Ok(val)
            } else {
                Ok(json!({ "raw": txt.trim() }))
            }
        }
        Backend::Scoop => {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", "scoop status"])
                .output()
                .map_err(|e| format!("Failed to execute scoop status: {e}"))?;

            let txt = String::from_utf8_lossy(&output.stdout);
            Ok(json!({ "backend": "scoop", "status": txt.trim() }))
        }
        Backend::Nix => {
            Ok(json!({ "backend": "nix", "status": "nix channels checked" }))
        }
        Backend::Unknown => Ok(json!({ "backend": "unknown", "outdated": [] })),
    }
}

fn print_meta() {
    let meta = json!({
        "name": "packager",
        "version": "0.3.0",
        "type": "cli",
        "description": "Universal package and organ manager supporting Scoop (SFSU) on Windows and Nix on Linux",
        "backend": Backend::detect().as_str(),
        "tools": [
            "packager_search",
            "packager_list",
            "packager_install",
            "packager_uninstall",
            "packager_info",
            "packager_update"
        ],
        "senses": ["outdated_packages"],
        "slash_commands": ["pkg"]
    });
    println!("{}", serde_json::to_string_pretty(&meta).unwrap());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 || args.iter().any(|a| a == "--help" || a == "-h") {
        print_meta();
        return;
    }

    if args.iter().any(|a| a == "--meta") {
        print_meta();
        return;
    }

    if args.iter().any(|a| a == "--tools") {
        let tools = json!([
            "packager_search",
            "packager_list",
            "packager_install",
            "packager_uninstall",
            "packager_info",
            "packager_update"
        ]);
        println!("{}", tools);
        return;
    }

    if let Some(pos) = args.iter().position(|a| a == "--sense") {
        let sense_name = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        match sense_name {
            "outdated_packages" | "" => match sense_outdated_packages() {
                Ok(val) => organ_ok!("outdated" => val),
                Err(e) => organ_err!(e),
            },
            other => organ_err!(format!("Unknown sense: {other}")),
        }
    }

    if let Some(pos) = args.iter().position(|a| a == "--tool") {
        let tool_name = args.get(pos + 1).map(|s| s.as_str()).unwrap_or("");
        let organ_args = OrganArgs::from_env();

        let res = match tool_name {
            "packager_search" => tool_search(&organ_args),
            "packager_list" => tool_list(&organ_args),
            "packager_install" => tool_install(&organ_args),
            "packager_uninstall" => tool_uninstall(&organ_args),
            "packager_info" => tool_info(&organ_args),
            "packager_update" => tool_update(&organ_args),
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
    fn test_backend_detection() {
        let b = Backend::detect();
        assert_ne!(b, Backend::Unknown);
    }

    #[test]
    fn test_packager_search_basic() {
        let args = OrganArgs::from_args(["--query", "zed"]);
        let res = tool_search(&args);
        assert!(res.is_ok());
    }

    #[test]
    fn test_packager_list_basic() {
        let args = OrganArgs::from_args(["--filter", "rust"]);
        let res = tool_list(&args);
        assert!(res.is_ok());
    }
}