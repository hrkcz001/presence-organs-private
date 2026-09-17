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
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn print_and_exit(self) -> ! {
        let is_ok = self.status == "ok";
        println!("{}", self.to_json_string());
        std::process::exit(if is_ok { 0 } else { 1 });
    }
}

/// Parsed CLI input arguments for an organ invocation.
#[derive(Debug, Clone, Default)]
pub struct OrganArgs {
    pub tool: Option<String>,
    pub sense: Option<String>,
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
            "action" => self.action = Some(val.to_string()),
            _ => {
                self.params.insert(key.to_string(), val.to_string());
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.params.get(key).map(|s| s.as_str()).unwrap_or(default)
    }

    pub fn op(&self) -> String {
        self.tool
            .clone()
            .or_else(|| self.sense.clone())
            .or_else(|| self.action.clone())
            .unwrap_or_default()
            .to_lowercase()
    }
}

/// Helper macro for returning structured output from native organs.
#[macro_export]
macro_rules! organ_ok {
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut res = $crate::OrganResponse::ok();
        $(
            res = res.with_field($key, serde_json::json!($val));
        )*
        res
    }};
}

#[macro_export]
macro_rules! organ_err {
    ($msg:expr) => {
        $crate::OrganResponse::error($msg)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        let args = vec![
            "--tool", "inspect",
            "--action=overview",
            "--window", "chrome.exe",
            "--verbose"
        ];
        let parsed = OrganArgs::from_args(args);
        assert_eq!(parsed.tool.as_deref(), Some("inspect"));
        assert_eq!(parsed.action.as_deref(), Some("overview"));
        assert_eq!(parsed.get("window"), Some("chrome.exe"));
        assert_eq!(parsed.get("verbose"), Some("true"));
        assert_eq!(parsed.op(), "inspect");
    }

    #[test]
    fn test_response_serialization() {
        let res = organ_ok!(
            "cpu" => 12.5,
            "running" => true,
            "tasks" => vec!["scan", "listen"]
        );
        let json = res.to_json_string();
        assert!(json.contains("\"status\": \"ok\""));
        assert!(json.contains("\"cpu\": 12.5"));
    }
}