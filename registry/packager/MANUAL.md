# Manual: packager

Universal package manager and organ installer for Presence Triad (bridging SFSU, Scoop, and Nix).
Version: 0.3.0

## Overview
`packager` provides unified single-command package management and organ lifecycle control across Windows and Linux.
It prioritizes ultra-fast native tooling:
- **Windows**: `sfsu` (sub-millisecond Rust Scoop driver) -> fallback to `scoop`.
- **Linux**: `nix-env` / `nix profile` -> fallback to `guix`.
- **Presence Organs**: Installs, audits, and checks compatibility of native and TypeScript organs into the active workspace.

## Environment & Dependencies
- **Platforms**: Windows (x86_64), Linux (x86_64, aarch64)
- **Runtime / Binary**: Native compiled binary (`organ-packager.exe`)
- **Underlying Drivers**:
  - Windows: `sfsu` or `scoop` on PATH.
  - Linux: `nix` or `nix-env` on PATH.
- **Permissions**:
  - File write access to `workspace/organs/` or user scoop directory.
  - Non-elevated: executes within user environment without admin prompts.

## Tools Specification

### `packager_search`
- **Description**: Search for software packages in system repositories or discover local Presence organs.
- **Parameters**:
  - `query` (`string`, required): Search keyword or organ name.
- **Output Schema**:
  ```json
  {
    "status": "ok",
    "backend": "sfsu",
    "query": "ripgrep",
    "packages": [
      { "name": "ripgrep", "version": "14.1.0", "description": "Fast line-oriented search tool" }
    ],
    "organs": []
  }
  ```

### `packager_install`
- **Description**: Install a system package or mount a local Presence organ.
- **Parameters**:
  - `package` (`string`, required): Name of package (e.g. `bun`, `ripgrep`) or relative/absolute path to directory containing `organ.yaml`.
- **Output Schema**:
  ```json
  {
    "status": "ok",
    "package": "bun",
    "backend": "sfsu",
    "stdout": "Installing bun..."
  }
  ```

### `packager_list`
- **Description**: List installed system packages or active Presence organs in workspace.
- **Parameters**:
  - `filter` (`string`, optional): Substring filter for package names.

### `packager_update`
- **Description**: Update specific package or all system packages.
- **Parameters**:
  - `package` (`string`, optional): If omitted, checks and updates system packages.

### `packager_uninstall`
- **Description**: Uninstall a package cleanly.
- **Parameters**:
  - `package` (`string`, required): Name of package to uninstall.

### `packager_info`
- **Description**: Inspect metadata, homepage, and dependencies of a package.
- **Parameters**:
  - `package` (`string`, required): Package name.

## Senses & Stimuli
- **Senses**:
  - `outdated_organs`: Audits installed Presence organs against local registry or Git upstream.
- **Stimuli**:
  - `outdated_organs` (cadence: 3600s, action: `notify_update`, target: `mechanic`): Autonomous hourly check for organ updates.

## Failure Modes & Recovery
- **Error: `No suitable package manager backend detected`**:
  - *Cause*: Neither `sfsu`, `scoop`, nor `nix` found on PATH.
  - *Recovery*: Install Scoop via PowerShell: `irm get.scoop.io | iex`, or install `sfsu` via `scoop install sfsu`.
- **Error: `Presence version requirement not satisfied`**:
  - *Cause*: Target organ requires newer or incompatible Presence daemon version.
  - *Recovery*: Audit organ SemVer in `organ.yaml` against host daemon version.
