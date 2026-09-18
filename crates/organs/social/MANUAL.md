# Manual: social

Relational memory, interlocutor profiles, habits, and communication register for Presence.
Version: 0.3.0

## Overview
`social` bridges the gap between semantic memory (facts, files, codebase concepts) and relational memory (who is speaking, how they think, what habits they have, what boundaries must be respected).
Rather than baking prompt instructions like *"reply in Russian"* or *"be concise"* into global system prompts, `social` maintains living, transparent profiles under `memory/social/profiles/`.

## Environment & Dependencies
- **Platforms**: Windows, Linux, macOS
- **Runtime / Binary**: Native binary (`organ-social.exe` on Windows, `organ-social` on Linux)
- **Dependencies**: None (native Rust executable)
- **Permissions**:
  - Read/write access to `memory/social/profiles/`.
  - Timeout: 15s.

## Tools Specification

### `social_profile`
- **Description**: Fetch the relational dossier, preferences, communication register, and boundaries for a given person.
- **Parameters**:
  - `person_id` (`string`, optional): Defaults to `"owner"`.
- **Example**: `social_profile(person_id: "owner")`
- **Response**:
  ```json
  {
    "status": "ok",
    "person_id": "owner",
    "language": "Russian dialogue, English code/artifacts",
    "register": {
      "tone": "pragmatic",
      "brevity": "concise",
      "directness": "high"
    },
    "habits": [
      { "category": "preference", "observation": "Prefers deterministic execution over narration" }
    ],
    "boundaries": [
      "Never delete project code files without explicit approval"
    ]
  }
  ```

### `social_note_habit`
- **Description**: Record an observed habit, preference, or conversational boundary for an interlocutor.
- **Parameters**:
  - `person_id` (`string`, required): Person ID.
  - `category` (`string`, required): One of `preference`, `boundary`, `lexicon`, `workflow`.
  - `observation` (`string`, required): Factual statement of the habit.
- **Example**: `social_note_habit(person_id: "owner", category: "preference", observation: "Values pure hermeneutic core with zero hardcoded bloat")`

### `social_tune_register`
- **Description**: Adjust tone, brevity, or language register for an interlocutor.
- **Parameters**:
  - `person_id` (`string`, required): Person ID.
  - `tone` (`string`, optional): Communication tone (`pragmatic`, `formal`, `direct`, `pedagogical`).
  - `brevity` (`string`, optional): `concise`, `normal`, `detailed`.
  - `language` (`string`, optional): e.g. `"Russian dialogue, English code"`.

## Senses & Stimuli
- **Senses**:
  - `active_interlocutor` (interval: 30s): Samples active person profile, formatting a concise markdown grounding block for inclusion in the observation/prompt phase.
- **Stimuli**:
  - `prolonged_absence` (cadence: 300s, action: `prepare_catchup`): Emitted when interlocutor has been absent for $> 24$ hours, preparing a catch-up context on return.

## Failure Modes & Recovery
- **Missing profiles directory**: Automatically initialized under `memory/social/profiles/` on startup.
- **Missing owner profile**: Seeded automatically with default values (Russian conversation, English technical terms, concise, pragmatic register).
