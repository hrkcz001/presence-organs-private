# Manual: vox

Native pure-Rust voice and audio sensory perception organ for Presence Triad.
Version: 0.3.0

## Overview
`vox` provides acoustic sensing, voice activity detection, and audio input capture for the Presence Triad.
It records audio in standard linear PCM (16kHz, 16-bit mono) suitable for local or offline speech-to-text models (e.g. Whisper).

## Environment & Dependencies
- **Platforms**: Windows, Linux, macOS
- **Runtime / Binary**: Native compiled binary (`organ-vox.exe`), zero external dynamic dependencies.
- **Permissions**:
  - Audio input capture access.
  - File write access to capture destination or OS temp folder.

## Tools Specification

### `vox_listen`
- **Description**: Capture an audio stream segment from default microphone endpoint.
- **Parameters**:
  - `seconds` (`integer`, optional, default: `3`): Capture duration in seconds.
  - `file` (`string`, optional): Output file path. Defaults to a temporary `.pcm` file.
- **Output Schema**:
  ```json
  {
    "status": "ok",
    "action": "vox_listen",
    "seconds": 3,
    "file": "C:\\Users\\...\\vox_capture_12345.pcm",
    "bytes_recorded": 96000,
    "sample_rate": 16000,
    "channels": 1,
    "format": "pcm_s16le"
  }
  ```

## Senses & Stimuli
- **Senses**:
  - `hearing`: Samples recent acoustic presence or performs immediate 1-second background listen.
- **Stimuli**:
  - `voice_activity` (cadence: 10s, action: `wake_agent`): Detects acoustic energy above threshold to awaken Cortex.

## Failure Modes & Recovery
- **Error: `Failed to write capture file`**:
  - *Cause*: Destination directory non-existent or read-only.
  - *Recovery*: `vox` automatically attempts fallback to `std::env::temp_dir()`.
