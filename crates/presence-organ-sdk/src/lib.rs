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

/// Organ compatibility requirements and capabilities metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct OrganCompatibility {
    #[serde(default)]
    pub presence: Option<String>,
    #[serde(default)]
    pub api_version: Option<u32>,
    #[serde(default)]
    pub platforms: Option<Vec<String>>,
    #[serde(default)]
    pub features: Option<Vec<String>>,
}

impl OrganCompatibility {
    /// Check whether this compatibility block matches the current host environment.
    pub fn validate(&self, host_presence_version: &str) -> Result<(), String> {
        if let Some(req_str) = &self.presence {
            let req = semver::VersionReq::parse(req_str)
                .map_err(|e| format!("Invalid SemVer presence constraint '{req_str}': {e}"))?;
            let host_ver = semver::Version::parse(host_presence_version)
                .map_err(|e| format!("Invalid host presence version '{host_presence_version}': {e}"))?;
            if !req.matches(&host_ver) {
                return Err(format!(
                    "Presence version requirement '{req_str}' not satisfied by host '{host_presence_version}'"
                ));
            }
        }
        if let Some(platforms) = &self.platforms {
            let os = std::env::consts::OS;
            if !platforms.iter().any(|p| p.eq_ignore_ascii_case(os)) {
                return Err(format!(
                    "Platform '{os}' is not supported; required: {:?}",
                    platforms
                ));
            }
        }
        Ok(())
    }
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
    #[serde(default)]
    pub target_agent: Option<String>,
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
            other => {
                self.params.insert(other.to_string(), val.to_string());
            }
        }
    }

    pub fn op(&self) -> &str {
        if let Some(t) = &self.tool {
            t.as_str()
        } else if let Some(s) = &self.sense {
            s.as_str()
        } else if let Some(st) = &self.stimulus {
            st.as_str()
        } else if let Some(r) = &self.reflex {
            r.as_str()
        } else if let Some(a) = &self.action {
            a.as_str()
        } else {
            ""
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get(key).unwrap_or(default)
    }

    pub fn get_bool(&self, key: &str) -> bool {
        self.params
            .get(key)
            .map(|s| s == "true" || s == "1")
            .unwrap_or(false)
    }

    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.params.get(key).and_then(|s| s.parse().ok())
    }
}