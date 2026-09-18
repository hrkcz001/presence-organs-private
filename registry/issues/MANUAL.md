# Manual: issues

Autonomous bug, UX friction, and GitHub telemetry reporting organ for Presence.
Version: 0.3.0

## Overview
`organ-issues` captures operational anomalies, tool failures, and ergonomic friction without interrupting the agent's work.
It strictly enforces user privacy and read-only diagnostics scopes.

## Privacy & Safety Directives
- **Zero PII Logging**: All personal data, usernames (`$USERNAME`), user profile paths, and credential tokens (`sk-...`, `ghp_...`, `Bearer ...`) are automatically sanitized and redacted to `<username>`, `~`, and `<redacted-token>`.
- **Restricted Write Scope**: The organ can only write to `logs/issues/` and `logs/friction.jsonl`. It possesses zero write permissions to project source code or workspace files.

## Tools Specification

### `report_issue`
- **Description**: Compiles structured markdown issue reports, saves locally to `logs/issues/`, and optionally exports to GitHub Issues if `gh` is authenticated.
- **Parameters**: `title`, `body`, `category` (`bug`, `friction`, `feature_gap`, `model_limitation`, `perf`), `submit_github`.

### `log_friction`
- **Description**: Fast append-only lightweight event logging to `logs/friction.jsonl` with automatic redaction.
- **Parameters**: `summary`, `category`, `details`, `severity`.

### `list_issues`
- **Description**: Queries recorded local issues.
