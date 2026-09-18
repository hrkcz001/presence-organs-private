# Manual: manual

Manual discovery, targeted section reading, and full-text documentation search organ for Presence Triad.
Version: 0.3.0

## Overview
`manual` serves as the Triad's interactive documentation librarian. It allows agents (Arche, Mechanic, Organcrafter, Arbiter) to dynamically look up organ specifications, gotchas, error recovery instructions, and parameter schemas on demand without loading entire documentation sets into the system prompt context.

## Environment & Dependencies
- **Platforms**: Windows, Linux, macOS (cross-platform)
- **Runtime / Binary**: Native compiled binary (`organ-manual.exe`), zero external dynamic dependencies.
- **Permissions**:
  - Read-only access across workspace, `organs/`, and `registry/`.
  - Timeout: 10s.

## Tools Specification

### `manual_list`
- **Description**: Discover and enumerate all installed and registered organs along with available manual topics.
- **Parameters**: None.
- **Output Schema**:
  ```json
  {
    "status": "ok",
    "total_organs": 10,
    "organs": [
      {
        "name": "winsense",
        "version": "0.3.0",
        "description": "Native pure-Rust Windows environment sensory organ",
        "manual_found": true,
        "topics": ["Overview", "Environment & Dependencies", "Tools Specification", "Senses & Stimuli", "Failure Modes & Recovery"]
      }
    ]
  }
  ```

### `manual_read`
- **Description**: Read the full manual or an isolated section (`topic`) of a specific organ.
- **Parameters**:
  - `organ` (`string`, required): Name of target organ (e.g. `"winsense"`, `"packager"`, `"channel"`).
  - `topic` (`string`, optional): Target section heading (e.g. `"tools"`, `"environment"`, `"recovery"`, `"senses"`). If omitted or `"all"`, returns full manual markdown.
- **Output Schema**:
  ```json
  {
    "status": "ok",
    "organ": "winsense",
    "topic": "Tools Specification",
    "content": "### `inspect_window`\n..."
  }
  ```

### `manual_search`
- **Description**: Fast case-insensitive text search across all discovered organ manuals in workspace.
- **Parameters**:
  - `query` (`string`, required): Search string or error keyword.
- **Output Schema**:
  ```json
  {
    "status": "ok",
    "query": "idle",
    "total_matches": 3,
    "matches": [
      {
        "organ": "winsense",
        "section": "Senses & Stimuli",
        "line": 48,
        "snippet": "- `idle_time`: Returns user idle duration in seconds via Win32 `LASTINPUTINFO`."
      }
    ]
  }
  ```

## Failure Modes & Recovery
- **Error: `Organ '<name>' not found`**:
  - *Cause*: Organ is not installed in local workspace or registered directories.
  - *Recovery*: Call `manual_list` to view available organs or run `packager_install` to install it.
- **Error: `Topic '<topic>' not found in organ '<name>'`**:
  - *Cause*: Section header spelling mismatch.
  - *Recovery*: Check available topics list returned in the error message.
