# Manual: notify

Cross-platform desktop notification toast organ for Presence Triad.
Version: 1.0.0

## Overview
`notify` emits non-intrusive desktop toasts on Windows (via PowerShell WinRT / BurntToast) and Linux (via `notify-send`) to alert the user of important asynchronous task completions or urgent stimuli.

## Environment & Dependencies
- **Platforms**: Windows 10/11, Linux (Freedesktop notification spec)
- **Runtime / Binary**: `bun` (Node 20+ fallback), system notification service
- **Permissions**:
  - Desktop notification display permission.
  - Timeout: 15s.

## Tools Specification

### `toast`
- **Description**: Display a desktop notification toast popup.
- **Parameters**:
  - `title` (`string`, required): Notification header.
  - `message` (`string`, required): Notification body message.
  - `urgency` (`string`, optional, enum: `["low", "normal", "critical"]`, default: `"normal"`): Urgency priority level.

## Failure Modes & Recovery
- **Error: `Notification daemon unavailable`**:
  - *Cause*: Headless Linux server without dbus/notification-daemon.
  - *Recovery*: Prints notification to stderr as graceful fallback.
