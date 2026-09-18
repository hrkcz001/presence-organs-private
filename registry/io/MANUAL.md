# Manual: io

Physical filesystem input/output and workspace manipulation organ for Presence Triad.
Version: 1.0.0

## Overview
`io` provides fundamental file reading, writing, directory listing, and disk space sensory capabilities to the Presence Triad.
All operations honor the active agent's capability bounds (`allow_write` / `exclude_write`).

## Environment & Dependencies
- **Platforms**: Windows, Linux, macOS
- **Runtime / Binary**: `bun` (Node 20+ fallback)
- **Permissions**:
  - `filesystem_read`: true
  - `filesystem_write`: true (bounded by persona card)
  - Default timeout: 15s

## Tools Specification

### `read_file`
- **Description**: Read full UTF-8 text content of a file.
- **Parameters**:
  - `path` (`string`, required): Relative or absolute path.
- **Output Schema**:
  ```json
  {
    "status": "ok",
    "content": "file contents..."
  }
  ```

### `write_file`
- **Description**: Write text content to target file, automatically creating parent directories if missing.
- **Parameters**:
  - `path` (`string`, required): File path.
  - `content` (`string`, required): String content to write.
- **Gotchas & Edge Cases**:
  - Writes are checked against persona capability constraints (`exclude_write`, `src/**`, `.git/**`).
  - Path traversals (`../`) are checked and rejected if violating sandbox bounds.

### `list_files`
- **Description**: Enumerate files and directories under specified path.
- **Parameters**:
  - `path` (`string`, optional, default: `"."`): Directory to scan.

## Senses & Stimuli
- **Senses**:
  - `fs_changes`: Recent modifications in active workspace files.
- **Stimuli**:
  - `disk_space_low` (cadence: 60s, action: `alert`, target: `mechanic`): Triggers when free disk space falls below 5 GB.
  - `workspace_bloat` (cadence: 300s, action: `alert`, target: `mechanic`): Triggers when temporary files exceed 500 MB.

## Failure Modes & Recovery
- **Error: `Path violates persona exclusion rules`**:
  - *Cause*: Attempting to write into protected directories (e.g. `src/**` by non-mechanic).
  - *Recovery*: Switch to authorized persona (e.g. `mechanic`) or write into permitted workspace.
