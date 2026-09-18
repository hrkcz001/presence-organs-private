# Manual: <organ_name>

<Brief 1-line summary of what this organ does and its role in Presence>
Version: 0.3.0

## Overview
<High-level architectural explanation of the organ's responsibilities, design philosophy, and integration with the Presence Triad (Cortex, Stem, Cord).>

## Environment & Dependencies
- **Platforms**: <windows | linux | macos | any>
- **Runtime / Binary**: <native binary (e.g. organ-xxx.exe) | bun | node | pwsh | sfsu>
- **Permissions**: <timeout, memory limits, allow_write/exclude_write scopes, network access>

## Tools Specification

### `<tool_name>`
- **Description**: <Action performed by Cortex via tool-calling>
- **Parameters**:
  - `<param_name>` (`<type>`, <required|optional>): <Purpose and validation rules>
- **Output Schema**:
  ```json
  {
    "status": "ok",
    "<data_key>": "<value>"
  }
  ```
- **Gotchas & Edge Cases**: <Platform differences, buffering limits, error codes, caveats>

## Senses & Stimuli
- **Senses**:
  - `<sense_name>`: <Perceptual input sampled during prompt-building and return payload>
- **Stimuli**:
  - `<stimulus_name>` (cadence: <N>s, action: <action>, target: <agent>): <Trigger conditions for Stem>
- **Reflexes**:
  - `on: <trigger>` -> `action: <involuntary sub-millisecond reflex>`

## Failure Modes & Recovery
- **Error: `<error_message>`**:
  - *Cause*: <Why this occurs>
  - *Recovery*: <Deterministic resolution or fallback procedure>
