# Manual: git

Structured non-interactive Git version control organ for Presence Triad.
Version: 1.0.0

## Overview
`git` manages version control checkpoints, diff inspections, status sampling, and branch drift detection.
It follows the Haven invariant: non-destructive Git checkpoints without human interruption, strictly rejecting force-pushes or history rewrites without owner confirmation.

## Environment & Dependencies
- **Platforms**: Windows, Linux, macOS
- **Runtime / Binary**: `bun` (Node 20+ fallback), `git` CLI on PATH.
- **Permissions**:
  - Non-interactive operations only.
  - Timeout: 30s.

## Tools Specification

### `git_status`
- **Description**: Query structured repository state and modified files list.

### `git_checkpoint`
- **Description**: Stage coherent edits and create a commit.
- **Parameters**:
  - `message` (`string`, required): Descriptive commit message following conventional commits.
  - `path` (`string`, optional): File or directory path to stage. Defaults to repository root.

### `git_diff`
- **Description**: Return unified diff of unstaged changes.

### `git_log`
- **Description**: Inspect recent commit history.
- **Parameters**:
  - `max_count` (`integer`, optional, default: 5): Number of commits to retrieve.

## Senses & Stimuli
- **Senses**:
  - `git_status`: Returns current branch name, staged/unstaged files, and untracked list.
- **Stimuli**:
  - `git_dirty_drift` (cadence: 60s, action: `alert`, target: `coder`): Emitted when working directory has uncommitted drift.
  - `git_upstream_behind` (cadence: 300s, action: `alert`, target: `coder`): Emitted when local branch is behind remote.

## Failure Modes & Recovery
- **Error: `Not a git repository`**:
  - *Cause*: Workspace directory lacks `.git`.
  - *Recovery*: Initializes git or locates nearest parent repo.
