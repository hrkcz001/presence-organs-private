# Manual: vox

Native pure-Rust voice sensory and audio capture organ for Presence.
Version: 0.3.0

## Overview
`organ-vox` governs auditory capture and verbal modulation. It provides bidirectional muting controls and injects modality matching instructions.

## Tools

- `vox_mute`: Mutes local microphone capture.
- `vox_unmute`: Unmutes local microphone capture.
- `vox_silence`: Silences agent speech synthesis (TTS).
- `vox_speak`: Enables agent speech synthesis (TTS).
- `vox_status`: Queries current microphone and TTS mute states.
- `vox_listen`: Records audio stream from active microphone (if unmuted).

## Slash Commands

- `/mute`: Mute user microphone capture.
- `/unmute`: Unmute user microphone capture.
- `/silence`: Silence agent speech output (TTS).
- `/speak`: Re-enable agent speech output (TTS).
- `/listen`: Capture raw PCM audio stream.

## Modality Matching Directive
`organ-vox` instructs the agent to maintain modality symmetry:
- When user input arrives via audio/voice transcription: prefer responding via speech (TTS).
- When user input arrives via typed text: prefer responding in text, unless voice is explicitly requested.
