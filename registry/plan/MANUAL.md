# Manual: plan

Goal formulation, active intent, roadmap execution, and session journal organ for Presence Triad.
Version: 1.0.0

## Overview
`plan` maintains goal progression, tracks multi-step workflows across session restarts, and records cognitive milestones to `memory/journal.md`.
It directly upholds the Disk-First Planning invariant of Haven.

## Environment & Dependencies
- **Platforms**: Windows, Linux, macOS
- **Runtime / Binary**: `bun` (Node 20+ fallback)
- **Permissions**:
  - File read/write access to `memory/journal.md`, `GOALS.md`, and plan scratch files.
  - Timeout: 15s.

## Tools Specification

### `plan_step`
- **Description**: Inspect, set, advance, or complete steps in active plan.
- **Parameters**:
  - `action` (`string`, required, enum: `["show", "next", "set", "complete"]`): Plan progression action.
  - `step` (`string`, optional): Description or identifier for step when setting or completing.
- **Output Schema**:
  ```json
  {
    "status": "ok",
    "active_step": "Phase 1 completion",
    "progress": "3/5"
  }
  ```

### `journal_append`
- **Description**: Append a significant finding, architectural decision, or milestone to `memory/journal.md`.
- **Parameters**:
  - `entry` (`string`, required): Markdown-formatted journal text.

## Senses & Stimuli
- **Senses**:
  - `standing_intent`: Injects current plan step directly into Cortex observation prompt.

## Failure Modes & Recovery
- **Error: `No plan file found`**:
  - *Cause*: Clean workspace initialization.
  - *Recovery*: Initializes default empty plan card.
