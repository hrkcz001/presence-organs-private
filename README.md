# Presence Organs Registry & SDK

The central organ catalog and developer SDK for the [Presence](https://github.com/hrkcz001/presence) cognitive daemon.

## Architecture

Presence strictly separates the core cognitive engine (Cortex, Stem, Cord) from its sensory and action organs:
- **`presence`**: The daemon engine runtime and hermeneutic circle.
- **`presence-organs`**: The catalog of sensory/action capabilities and developer SDK.
- **Third-Party Repositories**: Authors develop organs in their own repositories and submit PRs adding their `organ.yaml` to the `registry/`.

## Directory Layout

```text
presence-organs/
├── Cargo.toml                    # Root workspace for native Rust organs
├── crates/
│   ├── presence-organ-sdk/       # Core Rust SDK for binary organs
│   └── organs/                   # Native compiled Rust organs (winsense, vox)
│       ├── winsense/
│       └── vox/
├── registry/                     # Manifest catalog (index of all known organs)
│   ├── channel/organ.yaml
│   ├── git/organ.yaml
│   ├── io/organ.yaml
│   ├── monologue/organ.yaml
│   ├── notify/organ.yaml
│   ├── plan/organ.yaml
│   ├── state/organ.yaml
│   ├── winsense/organ.yaml
│   ├── vox/organ.yaml
│   └── browser/organ.yaml        # Example external community organ
├── organs/                       # Standard built-in organ scripts
└── templates/                    # Starter templates for Rust & TypeScript
```

## Developing an Organ

### 1. TypeScript Organ (Fast dynamic scripting)
- Implemented in clean typed TypeScript without build steps.
- Directly executed via Node 26+ native TS runtime, Bun, or Deno.
- See `templates/typescript/`.

### 2. Rust Organ (High performance / native OS API)
- Implemented as a standalone Cargo crate using `presence-organ-sdk`.
- Builds standalone via `cargo build --release`.
- See `templates/rust/`.

## Submitting to the Registry

To register your organ, create a PR adding `registry/<your-organ>/organ.yaml`:
```yaml
name: my-organ
version: "1.0.0"
author: "your-github"
description: "High-precision sensory organ"
source:
  type: "git"
  url: "https://github.com/your-username/presence-organ-my-organ"
  tag: "v1.0.0"
entrypoint: "my-organ.exe"
type: "cli"
tools:
  - name: sample_tool
    description: "Sample tool action"
```

## License
MIT