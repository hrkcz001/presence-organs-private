# Manual: monologue

Silent cognitive inner speech organ for deliberation, reflection, and hypothesis tracking.
Version: 1.0.0

## Overview
`monologue` provides private deliberation channels for Cortex agents (especially Arche and Arbiter) to think through complex decisions, formulate hypotheses, and track confidence levels without broadcasting intermediate tokens to the user.

## Environment & Dependencies
- **Platforms**: Windows, Linux, macOS
- **Runtime / Binary**: `bun` (Node 20+ fallback)
- **Permissions**:
  - Memory read/write to `memory/monologue.jsonl`.
  - Timeout: 15s.

## Tools Specification

### `ponder`
- **Description**: Silently deliberate or formulate an internal hypothesis without emitting user-facing messages.
- **Parameters**:
  - `thought` (`string`, required): Internal deliberation or hypothesis.
  - `confidence` (`number`, optional, 0.0 - 1.0): Subjective confidence score.

### `reflect`
- **Description**: Summarize and synthesize recent cognitive deliberations into a consolidated insight.
- **Parameters**:
  - `synthesis` (`string`, required): Synthesized conclusion.

## Senses & Stimuli
- **Senses**:
  - `stream_of_consciousness`: Samples recent internal reflections and hypotheses directly into the prompt Observation.

## Failure Modes & Recovery
- **Error: `Monologue write failure`**:
  - *Cause*: Unwritable memory log path.
  - *Recovery*: Retries with in-memory buffer until memory directory is restored.
