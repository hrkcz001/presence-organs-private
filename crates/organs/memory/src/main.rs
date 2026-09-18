//! Two-tier episodic and semantic memory management organ for Presence.
//!
//! Tools:
//! - memory_store: Stores or updates a memory item (Tier 1 episodic or Tier 2 consolidated).
//! - memory_recall: Searches and ranks memory items using keyword matching, importance, and temporal decay.
//! - memory_consolidate: Promotes high-value episodic items into consolidated semantic memory and prunes stale scratch items.
//!
//! Senses:
//! - memory_stats: Reports memory pool counts and health metrics.

use presence_organ_sdk::{organ_err, organ_ok, OrganArgs, OrganCompatibility, OrganResponse};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_DECAY_LAMBDA_TIER1: f64 = 0.005; // half-life ~ 6 days
const DEFAULT_DECAY_LAMBDA_TIER2: f64 = 0.0005; // half-life ~ 60 days

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemoryItem {
    pub id: String,
    pub key: String,
    pub content: String,
    pub tier: String, // "tier1" (episodic) or "tier2" (semantic)
    pub tags: Vec<String>,
    pub importance: f64,
    pub created_ts: f64,
    pub updated_ts: f64,
    pub access_count: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MemoryIndex {
    pub items: Vec<MemoryItem>,
    pub last_consolidated_ts: f64,
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

fn store_path(root: &Path) -> PathBuf {
    root.join("memory").join("store")
}

fn index_file(root: &Path) -> PathBuf {
    store_path(root).join("index.json")
}

fn load_index(root: &Path) -> MemoryIndex {
    let file = index_file(root);
    if !file.is_file() {
        return MemoryIndex::default();
    }
    if let Ok(f) = File::open(&file) {
        let reader = BufReader::new(f);
        if let Ok(idx) = serde_json::from_reader(reader) {
            return idx;
        }
    }
    MemoryIndex::default()
}

fn save_index(root: &Path, idx: &MemoryIndex) -> Result<(), String> {
    let dir = store_path(root);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = index_file(root);
    let tmp = dir.join("index.json.tmp");
    {
        let f = File::create(&tmp).map_err(|e| e.to_string())?;
        let writer = BufWriter::new(f);
        serde_json::to_writer_pretty(writer, idx).map_err(|e| e.to_string())?;
    }
    fs::rename(tmp, file).map_err(|e| e.to_string())?;
    Ok(())
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}

fn parse_tags(raw: &str) -> Vec<String> {
    if raw.starts_with('[') {
        if let Ok(list) = serde_json::from_str::<Vec<String>>(raw) {
            return list;
        }
    }
    raw.split([',', ';', ' '])
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn tool_memory_store(
    root: &Path,
    key: &str,
    content: &str,
    tier: &str,
    tags_str: &str,
    importance: f64,
) -> OrganResponse {
    let k = key.trim();
    let c = content.trim();
    if k.is_empty() {
        organ_err!("key cannot be empty");
    }
    if c.is_empty() {
        organ_err!("content cannot be empty");
    }

    let mut idx = load_index(root);
    let ts = now_ts();
    let tier_val = if tier.eq_ignore_ascii_case("tier2") { "tier2" } else { "tier1" };
    let tags = parse_tags(tags_str);
    let imp = importance.clamp(1.0, 10.0);

    let mut found = false;
    let mut updated_id = String::new();

    for item in &mut idx.items {
        if item.key.eq_ignore_ascii_case(k) {
            item.content = c.to_string();
            item.tier = tier_val.to_string();
            for tag in &tags {
                if !item.tags.contains(tag) {
                    item.tags.push(tag.clone());
                }
            }
            item.importance = imp;
            item.updated_ts = ts;
            item.access_count += 1;
            updated_id = item.id.clone();
            found = true;
            break;
        }
    }

    if !found {
        let new_id = format!("mem-{}", uuid::Uuid::new_v4());
        updated_id = new_id.clone();
        idx.items.push(MemoryItem {
            id: new_id,
            key: k.to_string(),
            content: c.to_string(),
            tier: tier_val.to_string(),
            tags,
            importance: imp,
            created_ts: ts,
            updated_ts: ts,
            access_count: 0,
        });
    }

    if let Err(e) = save_index(root, &idx) {
        organ_err!(format!("failed to save memory index: {e}"));
    }

    organ_ok! {
        "action" => if found { "updated" } else { "created" },
        "id" => updated_id,
        "key" => k,
        "tier" => tier_val,
        "importance" => imp
    }
}

pub fn tool_memory_recall(
    root: &Path,
    query: &str,
    tier_filter: &str,
    tags_filter: &str,
    limit: usize,
) -> OrganResponse {
    let mut idx = load_index(root);
    let now = now_ts();
    let q_tokens = tokenize(query);
    let filter_tags = parse_tags(tags_filter);

    let mut scored: Vec<(f64, usize)> = Vec::new();

    for (i, item) in idx.items.iter().enumerate() {
        if !tier_filter.is_empty() && !tier_filter.eq_ignore_ascii_case("all") {
            if !item.tier.eq_ignore_ascii_case(tier_filter) {
                continue;
            }
        }

        if !filter_tags.is_empty() {
            let has_tag = filter_tags.iter().any(|ft| item.tags.iter().any(|t| t == ft));
            if !has_tag {
                continue;
            }
        }

        let mut match_score = 0.0;
        if q_tokens.is_empty() {
            match_score = 1.0;
        } else {
            let key_tokens = tokenize(&item.key);
            let content_tokens = tokenize(&item.content);

            for q in &q_tokens {
                let in_key = key_tokens.iter().filter(|k| k.contains(q)).count();
                let in_content = content_tokens.iter().filter(|c| c.contains(q)).count();
                let in_tags = item.tags.iter().filter(|t| t.contains(q)).count();

                match_score += (in_key as f64) * 3.0 + (in_content as f64) * 1.0 + (in_tags as f64) * 2.0;
            }
        }

        if match_score > 0.0 {
            let age_hours = ((now - item.updated_ts) / 3600.0).max(0.0);
            let lambda = if item.tier == "tier2" {
                DEFAULT_DECAY_LAMBDA_TIER2
            } else {
                DEFAULT_DECAY_LAMBDA_TIER1
            };
            let recency_factor = (-lambda * age_hours).exp();
            let importance_multiplier = 1.0 + 0.15 * (item.importance - 1.0);
            let final_score = match_score * importance_multiplier * recency_factor;
            scored.push((final_score, i));
        }
    }

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let cap = limit.max(1);
    let top = scored.into_iter().take(cap);

    let mut results: Vec<Value> = Vec::new();
    let mut modified = false;

    for (score, idx_num) in top {
        let item = &mut idx.items[idx_num];
        item.access_count += 1;
        modified = true;
        results.push(json!({
            "id": item.id,
            "key": item.key,
            "content": item.content,
            "tier": item.tier,
            "tags": item.tags,
            "importance": item.importance,
            "score": (score * 100.0).round() / 100.0,
            "age_hours": ((now - item.updated_ts) / 3600.0).round()
        }));
    }

    if modified {
        let _ = save_index(root, &idx);
    }

    let count = results.len();
    organ_ok! {
        "query" => query,
        "count" => count,
        "results" => results
    }
}

pub fn tool_memory_consolidate(root: &Path) -> OrganResponse {
    let mut idx = load_index(root);
    let now = now_ts();
    let mut promoted = 0;
    

    for item in &mut idx.items {
        if item.tier == "tier1" {
            // Promote to tier2 if accessed multiple times or high importance
            if item.access_count >= 3 || item.importance >= 8.0 {
                item.tier = "tier2".to_string();
                item.updated_ts = now;
                promoted += 1;
            }
        }
    }

    // Prune stale low-importance tier1 items older than 14 days
    let fourteen_days_sec = 14.0 * 86400.0;
    let initial_len = idx.items.len();
    idx.items.retain(|item| {
        if item.tier == "tier1" && item.importance <= 3.0 && (now - item.updated_ts) > fourteen_days_sec {
            false
        } else {
            true
        }
    });
    let pruned = initial_len - idx.items.len();

    idx.last_consolidated_ts = now;
    if let Err(e) = save_index(root, &idx) {
        organ_err!(format!("failed to save consolidated index: {e}"));
    }

    organ_ok! {
        "action" => "consolidated",
        "promoted_to_tier2" => promoted,
        "pruned_stale" => pruned,
        "total_items" => idx.items.len()
    }
}

pub fn sense_memory_stats(root: &Path) -> OrganResponse {
    let idx = load_index(root);
    let mut tier1 = 0;
    let mut tier2 = 0;
    for item in &idx.items {
        if item.tier == "tier2" {
            tier2 += 1;
        } else {
            tier1 += 1;
        }
    }

    organ_ok! {
        "total_memories" => idx.items.len(),
        "tier1_episodic" => tier1,
        "tier2_semantic" => tier2,
        "last_consolidated_ts" => idx.last_consolidated_ts
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

    let response = if op == "memory_store" || args.has("store") || args.has("memory_store") {
        let key = args.get_or("key", "");
        let content = args.get_or("content", &args.get_or("value", ""));
        let tier = args.get_or("tier", "tier1");
        let tags = args.get_or("tags", "");
        let imp: f64 = args.get_or("importance", "5.0").parse().unwrap_or(5.0);
        tool_memory_store(&root, &key, &content, &tier, &tags, imp)
    } else if op == "memory_recall" || args.has("recall") || args.has("memory_recall") || args.has("query") {
        let q = args.get_or("query", &args.get_or("q", ""));
        let tier = args.get_or("tier", "all");
        let tags = args.get_or("tags", "");
        let limit: usize = args.get_or("limit", "5").parse().unwrap_or(5);
        tool_memory_recall(&root, &q, &tier, &tags, limit)
    } else if op == "memory_consolidate" || args.has("consolidate") || args.has("memory_consolidate") {
        tool_memory_consolidate(&root)
    } else if op == "memory_stats" || args.has("stats") || args.has("sense") {
        sense_memory_stats(&root)
    } else {
        organ_err!("unknown tool or sense operation for organ-memory. Available: memory_store, memory_recall, memory_consolidate, memory_stats")
    };

    response.print_and_exit();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_recall_and_stats() {
        let temp_dir = std::env::temp_dir().join(format!("presence_mem_test_{}", uuid::Uuid::new_v4()));
        let _ = fs::create_dir_all(&temp_dir);

        let store_res = tool_memory_store(
            &temp_dir,
            "architecture.triad",
            "Presence Triad consists of Cortex, Stem, and Cord.",
            "tier1",
            "arch,triad,core",
            9.0,
        );
        assert_eq!(store_res.status, "ok");

        let recall_res = tool_memory_recall(&temp_dir, "Cortex Cord", "all", "", 5);
        assert_eq!(recall_res.status, "ok");
        assert_eq!(recall_res.data["count"], 1);

        let stats_res = sense_memory_stats(&temp_dir);
        assert_eq!(stats_res.status, "ok");
        assert_eq!(stats_res.data["total_memories"], 1);
        assert_eq!(stats_res.data["tier1_episodic"], 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_consolidation_promotes_tier() {
        let temp_dir = std::env::temp_dir().join(format!("presence_mem_test2_{}", uuid::Uuid::new_v4()));
        let _ = fs::create_dir_all(&temp_dir);

        let _ = tool_memory_store(
            &temp_dir,
            "important_fact",
            "This is a high importance lesson.",
            "tier1",
            "lesson",
            8.5,
        );

        let cons_res = tool_memory_consolidate(&temp_dir);
        assert_eq!(cons_res.status, "ok");
        assert_eq!(cons_res.data["promoted_to_tier2"], 1);

        let stats_res = sense_memory_stats(&temp_dir);
        assert_eq!(stats_res.data["tier2_semantic"], 1);
        assert_eq!(stats_res.data["tier1_episodic"], 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
