# Manual: channel
*Version 0.3.0 — Communication Channel Organ for Presence*

## Overview
The `channel` organ provides agent-to-environment and agent-to-human messaging primitives. It acts as an asynchronous mailbox layer for Presence, writing outgoing transmissions to `memory/outbox.jsonl` and monitoring incoming prompts via `memory/inbox.jsonl`.

## Environment & Dependencies
- **Platforms**: Windows (`x86_64`), Linux (`x86_64`, `aarch64`), macOS (`x86_64`, `aarch64`).
- **Permissions**: Read/Write access to `$PRESENCE_WORKSPACE/memory/`.
- **Runtimes**: Standalone native binary; zero external runtime dependencies.

## Tools Specification

### `send_reply`
Appends a formatted message entry into the outbox for the active conversation or external channel.
- **Parameters**:
  - `message` (`string`, required): The text message to transmit.
  - `channel` (`string`, optional, default: `"active"`): Identifier of the destination stream (e.g., `"active"`, `"telegram"`, `"cli"`).
- **Return Shape**:
  ```json
  {
    "status": "ok",
    "data": {
      "action": "sent",
      "channel": "active",
      "delivered_chars": 24,
      "preview": "Hello from agent"
    }
  }
  ```
- **Gotchas / Edge Cases**:
  - Empty or whitespace-only messages fail with status `"error"`.
  - Automatically creates `$PRESENCE_WORKSPACE/memory/outbox.jsonl` if it does not exist.

### `send_status`
Publishes an ephemeral status message (e.g. current progress or compilation state) to the active outbox.
- **Parameters**:
  - `status` (`string`, required): Short status indicator.
- **Return Shape**:
  ```json
  {
    "status": "ok",
    "data": {
      "action": "status_reported"
    }
  }
  ```

## Senses & Stimuli

### Sense: `incoming_inbox`
Inspects the tail of `memory/inbox.jsonl` and returns the most recent unconsumed entries (up to 5 items).
- **Payload**:
  ```json
  {
    "status": "ok",
    "data": {
      "inbox": [
        {"ts": 1726645000.0, "sender": "owner", "text": "Build release binary"}
      ]
    }
  }
  ```

## Failure Modes & Recovery
- **Missing memory directory**: The organ automatically creates `$PRESENCE_WORKSPACE/memory/` on write.
- **Malformed JSON lines**: Ignored gracefully during inbox inspection without crashing.