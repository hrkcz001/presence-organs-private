# Presence Organs

> Modular peripheral organs catalog, SDK, and GUI dashboard for the Presence autonomous runtime.

## Overview
Organs are standalone binaries communicating via JSON over standard streams or FFI. Each organ adheres to the 5-faceted contract:
- `tool`: Conscious execution requested by the LLM Cortex
- `sense`: Synchronous state snapshot injected into Cortex prompts
- `stimulus`: Asynchronous threshold-based poll checked by the Stem heartbeat (0 tokens)
- `reflex`: Involuntary microsecond action intercepted by Cord
- `action`: User slash-command via client protocol

---

## Quickstart: How to Build All Organs

### 1. Requirements
- Rust toolchain (`cargo`, `rustc` 1.80+)

### 2. Build All Organs at Once
```powershell
cargo build --release
```
Compiled organ binaries will be in `target/release/`:
- `organ-shell.exe` — Isolated command runner with timeout and UTF-8 bounding
- `organ-vitals.exe` — Native CPU/RAM/Battery telemetry
- `organ-ask.exe` — Owner interactive query modal
- `organ-packager.exe` — Dependency resolver (Scoop/Nix)
- `presence-dashboard.exe` — Native egui/eframe GUI dashboard

### 3. Deploy to Core Runtime
Copy the compiled binaries into the `organs/` directory of your `presence-core-private` clone:
```powershell
# Example:
Copy-Item target/release/organ-*.exe ../presence-core-private/organs/
```

---

## Tinkering & Wire-Up Guide for Agent

### 1. Running Unit Tests
```powershell
cargo test --workspace
```
All organs and the `presence-organ-sdk` should pass.

### 2. Adding / Linking an Organ
Each organ has its crate in `crates/organs/<name>` and manifest in `registry/<name>/organ.yaml` or `crates/organs/<name>/organ.yaml`. The SDK (`presence-organ-sdk`) provides `resolve_any_binary()` and `OrganContext`.
