# Manual: state

Presence cognitive state preservation, session pause, and hibernation organ.
Version: 1.0.0

## Overview
`state` manages agent lifecycle, checkpoints, and safe resting states.
When pausing or resting, it serializes persona working cards, memory pointers, and active objectives into snapshot archives.

## Environment & Dependencies
- **Platforms**: Windows, Linux, macOS
- **Runtime / Binary**: `bun` (Node 20+ fallback)
- **Permissions**:
  - File read/write to `memory/state/` and `memory/STATE.md`.
  - Timeout: 15s.

## Tools Specification

### `pause_session`
- **Description**: Pause current session and transition agent card into `rest` mode.
- **Parameters**:
  - `reason` (`string`, required): Why the session is pausing.

### `snapshot`
- **Description**: Create an explicit labeled state snapshot of the active agent and environment.
- **Parameters**:
  - `label` (`string`, required): Checkpoint identifier.

### `restore`
- **Description**: Restore previously created state snapshot.
- **Parameters**:
  - `label` (`string`, optional): Snapshot name (defaults to latest).

## Senses & Stimuli
- **Senses**:
  - `sleep_state`: Reports whether agent is in active cognition or resting/hibernating mode.
- **Stimuli**:
  - `stale_goal` (cadence: 900s, action: `alert`, target: `arche`): Emitted when active goals in `GOALS.md` remain unupdated for > 12 hours.

## Failure Modes & Recovery
- **Error: `Snapshot not found`**:
  - *Cause*: Requested snapshot label does not exist.
  - *Recovery*: Lists available snapshot labels in `memory/state/`.
