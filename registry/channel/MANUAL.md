# Manual: channel

Universal client bridge and bi-directional communication channel organ for Presence Triad.
Version: 0.3.0

## Overview
`channel` fulfills two dual architectural roles:
1. **Organ Tool & Sense**: Allows Cortex to send interactive replies (`send_reply`), report real-time execution status (`send_status`), and inspect incoming messages (`incoming_inbox`).
2. **ACP Protocol Bridge**: Functions as an ACP (Agent Client Protocol) server adapter via `organ-channel.exe --acp`, bridging editor UIs (e.g. Zed, IDE extensions) to Presence.

## Environment & Dependencies
- **Platforms**: Windows, Linux, macOS (cross-platform)
- **Runtime / Binary**: Native compiled binary (`organ-channel.exe`), zero runtime dependencies.
- **Permissions**:
  - File append to `memory/outbox.jsonl` and read from `memory/inbox.jsonl`.
  - Non-elevated: operates purely in user context.

## Tools Specification

### `send_reply`
- **Description**: Send a formatted response or conversational message to the user/client channel.
- **Parameters**:
  - `message` (`string`, required): Message body to dispatch.
  - `channel` (`string`, optional, default: `"active"`): Target communication channel ID.
- **Output Schema**:
  ```json
  {
    "status": "ok",
    "action": "reply_dispatched",
    "channel": "active",
    "bytes_written": 142
  }
  ```

### `send_status`
- **Description**: Emit transient progress status or heart-beat state visible in client editor UI.
- **Parameters**:
  - `status` (`string`, required): Concise description of active progress (e.g. `"Compiling crate..."`).

## Senses & Stimuli
- **Senses**:
  - `incoming_inbox`: Samples the last 5 messages from `memory/inbox.jsonl` to provide Cortex with external context prompts.
- **Commands**:
  - `/reply <message>`: Quick slash command routing to `send_reply`.

## ACP Bridge Mode
Invoking `organ-channel.exe --acp` initiates an interactive JSON-RPC stdio loop supporting:
- `initialize`: Returns protocolVersion 1, agent capabilities, and agentInfo.
- `session/new`: Instantiates a new session ID.
- `session/prompt`: Dispatches incoming prompts to outbox and waits for completion.
- `session/cancel`: Signals cancellation.

## Failure Modes & Recovery
- **Error: `failed to read inbox`**:
  - *Cause*: `memory/inbox.jsonl` does not exist or has file lock.
  - *Recovery*: `channel` gracefully treats missing file as empty inbox and returns `{ "inbox": [] }`.
