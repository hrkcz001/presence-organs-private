# Manual: issues
*Version 0.3.0 — Bug, Friction, and Telemetry Reporting Organ for Presence*

## Overview
The `issues` organ equips Presence with proactive bug reporting, cognitive friction tracking, and GitHub Issues telemetry. In Alpha and Beta releases, this organ runs in an active stance, biasing the agent toward explicitly logging operational anomalies, confusing outputs, tool crashes, and UX bottlenecks rather than silently working around them.

## Environment & Dependencies
- **Platforms**: Windows (`x86_64`), Linux (`x86_64`, `aarch64`), macOS (`x86_64`, `aarch64`).
- **Permissions**: Read/Write access to `$PRESENCE_WORKSPACE/logs/`.
- **Runtimes**: Standalone native binary; optional `gh` CLI on `$PATH` for live GitHub exports.

## Tools Specification

### `report_issue`
Records a detailed markdown issue in `$PRESENCE_WORKSPACE/logs/issues/<timestamp>-<slug>.md`, and optionally exports it directly to GitHub Issues via `gh issue create`.
- **Parameters**:
  - `title` (`string`, required): Concise issue title.
  - `body` (`string`, optional): Detailed issue description, reproduction steps, or context.
  - `category` (`string`, optional, default: `"bug"`): Category (`"bug"`, `"ux"`, `"friction"`, `"feature_request"`).
  - `submit_github` (`boolean`, optional, default: `false`): If true, attempts export to GitHub via `gh` CLI.
- **Return Shape**:
  ```json
  {
    "status": "ok",
    "data": {
      "action": "reported",
      "file": "C:/workspace/logs/issues/1726646000-sandbox-denied.md",
      "filename": "1726646000-sandbox-denied.md",
      "title": "Sandbox Permission Denied",
      "category": "bug",
      "github_status": "local_only"
    }
  }
  ```

### `log_friction`
Ultra-fast, zero-latency append of structured friction events to `$PRESENCE_WORKSPACE/logs/friction.jsonl`.
- **Parameters**:
  - `summary` (`string`, required): Short summary of the friction or failure point.
  - `category` (`string`, optional, default: `"friction"`): Category (e.g. `"tool_failure"`, `"rate_limit"`, `"schema_mismatch"`).
  - `details` (`string`, optional): Full error message or diagnostics.
  - `severity` (`string`, optional, default: `"normal"`): Severity (`"low"`, `"normal"`, `"high"`, `"critical"`).
- **Return Shape**:
  ```json
  {
    "status": "ok",
    "data": {
      "action": "logged",
      "category": "tool_failure",
      "summary": "cargo check failed on missing dep",
      "severity": "high",
      "path": "C:/workspace/logs/friction.jsonl"
    }
  }
  ```

### `list_issues`
Scans `$PRESENCE_WORKSPACE/logs/issues/` and lists previously logged reports.
- **Parameters**:
  - `category` (`string`, optional): Filter by category.
- **Return Shape**:
  ```json
  {
    "status": "ok",
    "data": {
      "count": 2,
      "issues": [
        {"filename": "1726646000-sandbox-denied.md", "title": "Sandbox Permission Denied", "category": "bug", "created_ts": 0.0}
      ]
    }
  }
  ```

## Senses & Stimuli

### Sense: `recent_friction`
Reads the tail of `$PRESENCE_WORKSPACE/logs/friction.jsonl` (last 10 entries) and provides an immediate summary of operational friction for the active session.

### Stimulus: `frequent_friction`
- **Cadence**: 30s.
- **Trigger**: More than 3 friction events recorded in the last 5 minutes.
- **Recommended Reflex**: Initiate self-diagnostic reflection or surface warning to owner.

## Failure Modes & Recovery
- **Missing directories**: Automatically creates `$PRESENCE_WORKSPACE/logs/` and `$PRESENCE_WORKSPACE/logs/issues/`.
- **GitHub CLI (`gh`) missing or unauthenticated**: Gracefully falls back to `local_only` markdown report without crashing.