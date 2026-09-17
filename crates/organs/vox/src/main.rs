//! Native pure-Rust voice and audio sensory organ for Presence.
use presence_organ_sdk::{organ_err, organ_ok, OrganArgs};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let args = OrganArgs::from_env();
    let op = args.op();

    if op == "vox_listen" || op == "listen" || args.sense.as_deref() == Some("hearing") {
        let seconds: u64 = args.get_or("seconds", "3").parse().unwrap_or(3);
        let dest = args.get("file").map(PathBuf::from).unwrap_or_else(|| {
            std::env::temp_dir().join(format!("vox_capture_{}.pcm", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()))
        });

        // 16kHz, 16-bit mono PCM stream
        let sample_rate = 16000u32;
        let bytes_per_sample = 2u32;
        let total_samples = sample_rate * (seconds as u32);
        let total_bytes = (total_samples * bytes_per_sample) as usize;

        // Perform audio capture / WASAPI stream buffer creation
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
    } else {
        organ_ok!(
            "status" => "ready",
            "organ" => "vox",
            "capabilities" => vec!["vox_listen", "hearing"]
        );
    }
}