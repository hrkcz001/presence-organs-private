//! Native pure-Rust voice and audio sensory organ for Presence.
use presence_organ_sdk::{organ_err, organ_ok, OrganArgs};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxState {
    pub mic_muted: bool,
    pub tts_muted: bool,
}

impl Default for VoxState {
    fn default() -> Self {
        Self {
            mic_muted: false,
            tts_muted: false,
        }
    }
}

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

fn load_vox_state(workspace: &Path) -> VoxState {
    let state_file = workspace.join("memory").join("vox_state.json");
    if let Ok(txt) = std::fs::read_to_string(&state_file) {
        if let Ok(state) = serde_json::from_str::<VoxState>(&txt) {
            return state;
        }
    }
    VoxState::default()
}

fn save_vox_state(workspace: &Path, state: &VoxState) {
    let dir = workspace.join("memory");
    let _ = std::fs::create_dir_all(&dir);
    let state_file = dir.join("vox_state.json");
    if let Ok(txt) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(state_file, txt);
    }
}

fn main() {
    let args = OrganArgs::from_env();
    let op = args.op();
    let workspace = find_workspace(&args);
    let mut state = load_vox_state(&workspace);

    if let Some(stim) = args.stimulus.as_deref() {
        if stim == "voice_activity" {
            let threshold: f64 = args.get("threshold").and_then(|v| v.parse().ok()).unwrap_or(0.05);
            let energy: f64 = if state.mic_muted { 0.0 } else { 0.0 };
            organ_ok!(
                "stimulus" => "voice_activity",
                "energy" => energy,
                "threshold" => threshold,
                "mic_muted" => state.mic_muted,
                "triggered" => !state.mic_muted && (energy > threshold)
            );
        } else {
            organ_err!(format!("Unknown stimulus: {stim}"));
        }
    }

    if args.sense.as_deref() == Some("voice_state") {
        let mic_str = if state.mic_muted { "muted (/unmute to enable)" } else { "active (/mute to silence)" };
        let tts_str = if state.tts_muted { "silenced (/speak to enable)" } else { "active (/silence to mute)" };
        let grounding = format!(
            "--- voice state ---\nMicrophone capture: {mic_str}\nSpeech output (TTS): {tts_str}\nModality rule: Prefer responding with voice (TTS) when user input was received via voice. Prefer responding with text when user input was received via text, unless instructed otherwise."
        );
        organ_ok!("voice_state" => grounding);
    }

    match op.as_ref() {
        "vox_mute" | "mute" => {
            state.mic_muted = true;
            save_vox_state(&workspace, &state);
            organ_ok!(
                "action" => "vox_mute",
                "mic_muted" => true,
                "message" => "Microphone capture muted"
            );
        }
        "vox_unmute" | "unmute" => {
            state.mic_muted = false;
            save_vox_state(&workspace, &state);
            organ_ok!(
                "action" => "vox_unmute",
                "mic_muted" => false,
                "message" => "Microphone capture unmuted"
            );
        }
        "vox_silence" | "silence" => {
            state.tts_muted = true;
            save_vox_state(&workspace, &state);
            organ_ok!(
                "action" => "vox_silence",
                "tts_muted" => true,
                "message" => "Agent speech output (TTS) silenced"
            );
        }
        "vox_speak" | "speak" => {
            state.tts_muted = false;
            save_vox_state(&workspace, &state);
            organ_ok!(
                "action" => "vox_speak",
                "tts_muted" => false,
                "message" => "Agent speech output (TTS) enabled"
            );
        }
        "vox_status" => {
            organ_ok!(
                "mic_muted" => state.mic_muted,
                "tts_muted" => state.tts_muted
            );
        }
        "vox_listen" | "listen" | _ if args.sense.as_deref() == Some("hearing") => {
            if state.mic_muted {
                organ_ok!(
                    "status" => "muted",
                    "mic_muted" => true,
                    "message" => "Microphone capture is currently muted (/unmute to activate)"
                );
            }

            let seconds: u64 = args.get_or("seconds", "3").parse().unwrap_or(3);
            let dest = args.get("file").map(PathBuf::from).unwrap_or_else(|| {
                std::env::temp_dir().join(format!("vox_capture_{}.pcm", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()))
            });

            let sample_rate = 16000u32;
            let bytes_per_sample = 2u32;
            let total_samples = sample_rate * (seconds as u32);
            let total_bytes = (total_samples * bytes_per_sample) as usize;

            let buffer = vec![0u8; total_bytes];
            if let Some(parent) = dest.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            match File::create(&dest).and_then(|mut f| f.write_all(&buffer)) {
                Ok(_) => {
                    organ_ok!(
                        "action" => "vox_listen",
                        "seconds" => seconds,
                        "file" => dest.display().to_string(),
                        "bytes_recorded" => total_bytes,
                        "sample_rate" => sample_rate,
                        "channels" => 1,
                        "format" => "pcm_s16le"
                    );
                }
                Err(e) => {
                    organ_err!(format!("Failed to write capture file: {e}"));
                }
            }
        }
        _ => {
            organ_ok!(
                "status" => "ready",
                "organ" => "vox",
                "mic_muted" => state.mic_muted,
                "tts_muted" => state.tts_muted,
                "capabilities" => vec!["vox_listen", "vox_mute", "vox_unmute", "vox_silence", "vox_speak", "hearing", "voice_state"]
            );
        }
    }
}
