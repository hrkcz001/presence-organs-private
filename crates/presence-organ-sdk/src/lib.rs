use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Standard structured response returned by all Presence organs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}

impl OrganResponse {
    pub fn ok() -> Self {
        Self {
            status: "ok".to_string(),
            error: None,
            data: HashMap::new(),
        }
    }

    pub fn ok_with(data: HashMap<String, Value>) -> Self {
        Self {
            status: "ok".to_string(),
            error: None,
            data,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            error: Some(msg.into()),
            data: HashMap::new(),
        }
    }

    pub fn with_field(mut self, key: impl Into<String>, val: impl Into<Value>) -> Self {
        self.data.insert(key.into(), val.into());
        self
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{\"status\":\"error\",\"error\":\"json serialization failed\"}".to_string())
    }

    pub fn print_and_exit(&self) -> ! {
        println!("{}", self.to_json_string());
        if self.status == "ok" {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }
}

/// Macro for quick successful organ return and exit.
#[macro_export]
macro_rules! organ_ok {
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut resp = $crate::OrganResponse::ok();
        $(
            resp = resp.with_field($key, serde_json::json!($val));
        )*
        resp.print_and_exit();
    }};
    () => {{
        $crate::OrganResponse::ok().print_and_exit();
    }};
}

/// Macro for quick organ error exit.
#[macro_export]
macro_rules! organ_err {
    ($msg:expr) => {{
        $crate::OrganResponse::error($msg).print_and_exit();
    }};
}

/// Definition of a vegetative Stimulus for Stem (heartbeat and autonomous polling).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrganStimulusDef {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cadence_secs: Option<u64>,
    #[serde(default)]
    pub action: Option<String>,
}

/// Definition of an involuntary Reflex for Cord (sub-millisecond protective reflex arc).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrganReflexDef {
    pub on: String,
    pub action: String,
}

/// Universal CLI Argument parser for Presence Organs.
#[derive(Debug, Clone, Default)]
pub struct OrganArgs {
    pub tool: Option<String>,
    pub sense: Option<String>,
    pub stimulus: Option<String>,
    pub reflex: Option<String>,
    pub action: Option<String>,
    pub params: HashMap<String, String>,
}

impl OrganArgs {
    /// Parse command-line arguments using `--key value` convention.
    pub fn from_env() -> Self {
        Self::from_args(std::env::args().skip(1))
    }

    pub fn from_args<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut parsed = Self::default();
        let items: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
        let mut i = 0;
        while i < items.len() {
            let item = &items[i];
            if let Some(stripped) = item.strip_prefix("--") {
                if let Some((k, v)) = stripped.split_once('=') {
                    parsed.register_param(k, v);
                } else if i + 1 < items.len() && !items[i + 1].starts_with("--") {
                    parsed.register_param(stripped, &items[i + 1]);
                    i += 1;
                } else {
                    parsed.register_param(stripped, "true");
                }
            } else if parsed.action.is_none() {
                parsed.action = Some(item.clone());
            }
            i += 1;
        }
        parsed
    }

    fn register_param(&mut self, key: &str, val: &str) {
        match key {
            "tool" => self.tool = Some(val.to_string()),
            "sense" => self.sense = Some(val.to_string()),
            "stimulus" => self.stimulus = Some(val.to_string()),
            "reflex" => self.reflex = Some(val.to_string()),
            "action" => self.action = Some(val.to_string()),
            _ => {
                self.params.insert(key.to_string(), val.to_string());
            }
        }
    }

    pub fn op(&self) -> &str {
        self.tool
            .as_deref()
            .or(self.stimulus.as_deref())
            .or(self.reflex.as_deref())
            .or(self.sense.as_deref())
            .or(self.action.as_deref())
            .unwrap_or("")
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.params.get(key).map(|s| s.as_str()).unwrap_or(default)
    }

    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.params.get(key).and_then(|v| v.parse::<u64>().ok())
    }

    pub fn get_bool(&self, key: &str) -> bool {
        self.params.get(key).map(|v| v == "true" || v == "1").unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_organ_args_parsing() {
        let args = vec![
            "--tool", "vox_listen",
            "--seconds", "5",
            "--format=wav",
            "--verbose",
        ];
        let parsed = OrganArgs::from_args(args);
        assert_eq!(parsed.tool.as_deref(), Some("vox_listen"));
        assert_eq!(parsed.get("seconds"), Some("5"));
        assert_eq!(parsed.get_u64("seconds"), Some(5));
        assert_eq!(parsed.get("format"), Some("wav"));
        assert!(parsed.get_bool("verbose"));
    }

    #[test]
    fn test_stimulus_and_reflex_args() {
        let args = vec![
            "--stimulus", "user_idle",
            "--cadence", "30",
            "--reflex", "lower_priority",
        ];
        let parsed = OrganArgs::from_args(args);
        assert_eq!(parsed.stimulus.as_deref(), Some("user_idle"));
        assert_eq!(parsed.reflex.as_deref(), Some("lower_priority"));
        assert_eq!(parsed.get_u64("cadence"), Some(30));
        assert_eq!(parsed.op(), "user_idle");
    }

    #[test]
    fn test_organ_response_serialization() {
        let resp = OrganResponse::ok()
            .with_field("transcription", "hello world")
            .with_field("duration", 2.5);
        let json = resp.to_json_string();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"transcription\":\"hello world\""));
    }
}