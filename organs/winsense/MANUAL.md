# Manual: winsense

Native pure-Rust Windows sensory and vegetative perception organ for Presence Triad.
Version: 0.3.0

## Overview
`winsense` bridges the Presence Triad with the host Windows operating system. It provides real-time window tracking, input idle monitoring, power/battery sensing, CPU load calculation, audio device enumeration, and display lock detection.
- **Cortex**: Conscious system state queries via `inspect_window`.
- **Stem**: Autonomous background stimuli (`user_idle`, `battery_low`, `network_state`, `display_locked`, `high_cpu`).
- **Cord**: Sub-millisecond protective reflexes (`lower_cpu_priority`).

## Environment & Dependencies
- **Platforms**: Windows (x86_64, Windows 10/11)
- **Runtime / Binary**: Native compiled binary (`organ-winsense.exe`), zero external DLL or C++ runtime dependencies.
- **Permissions**:
  - `user32.dll`, `kernel32.dll` standard desktop APIs.
  - Non-elevated: runs cleanly in standard user context.
  - Job Object: default memory limit 512MB, timeout 10s.

## Tools Specification

### `inspect_window`
- **Description**: Query active window, enumerate open desktop windows, or inspect host vitals (power, audio, idle).
- **Parameters**:
  - `action` (`string`, optional, default: `"overview"`):
    - `"foreground"`: Title and PID of the active foreground window.
    - `"windows"`: List of all visible, non-system top-level windows.
    - `"overview"`: Full system snapshot (foreground, idle, power, windows, audio).
    - `"idle"`: Idle duration in seconds.
    - `"power"`: AC/battery status and battery charge percentage.
    - `"audio"`: Friendly names of active audio output endpoints.
- **Output Schema**:
  ```json
  {
    "status": "ok",
    "overview": {
      "foreground": { "title": "Visual Studio Code", "pid": 1234 },
      "user_idle_seconds": 12.5,
      "power": { "power_source": "ac", "battery_percent": 100 },
      "open_windows": [
        { "title": "Terminal", "pid": 5678 }
      ],
      "audio_devices": ["Speakers (Realtek Audio)"]
    }
  }
  ```
- **Gotchas & Edge Cases**:
  - Filtered system windows: Windows Input Experience, Program Manager, and background Settings windows are automatically excluded from `windows` output.
  - Headless/RDP: If running inside a service without desktop access, `OpenInputDesktop` may fail gracefully returning empty list.

## Senses & Stimuli
- **Senses**:
  - `active_window`: Returns current foreground window title and PID.
  - `idle_time`: Returns user idle duration in seconds via Win32 `LASTINPUTINFO`.
  - `power_state`: Returns power source (`"ac"` or `"battery"`) and `battery_percent`.
- **Stimuli**:
  - `user_idle` (cadence: 30s, action: `modulate_pulse`): Triggers when idle duration exceeds `threshold_secs` (default 900s). Slows heartbeat pulse to conserve CPU.
  - `battery_low` (cadence: 60s, action: `alert`): Triggers when battery level drops below `threshold_percent` (default 15%).
  - `network_state` (cadence: 15s, action: `switch_offline_mode`): Triggers if DNS ping to 1.1.1.1/8.8.8.8 fails.
  - `display_locked` (cadence: 30s, action: `modulate_pulse`): Triggers when workstation lock is active.
  - `high_cpu` (cadence: 30s, action: `modulate_pulse`): Triggers if total CPU load exceeds 85%.
- **Reflexes**:
  - `lower_cpu_priority`: Drops agent process priority when owner actively types or interacts with foreground window.

## Failure Modes & Recovery
- **Error: `Failed to open input desktop`**:
  - *Cause*: Screen saver active, UAC secure desktop active, or session locked.
  - *Recovery*: `winsense` handles this cleanly and returns `display_locked = true` without crashing.
- **Error: `Platform 'linux' is not supported`**:
  - *Cause*: Invoking Windows-only organ on non-Windows environment.
  - *Recovery*: Cord/Stem suppresses execution and notifies packager of missing Linux platform organ.
