# Manual: presence-acp
*Version 0.3.0 — Universal Agent Client Protocol (ACP) Adapter & Gateway*

## Overview
`presence-acp` is the dedicated bridge binary between external editor environments (such as Zed, VSCode, and command-line interfaces) and the Presence cognitive agent architecture. It speaks standard JSON-RPC 2.0 over `stdio`, implementing the Agent Client Protocol (ACP) with profile-based adaptability.

## Environment & Dependencies
- **Platforms**: Windows (`x86_64`), Linux (`x86_64`, `aarch64`), macOS (`x86_64`, `aarch64`).
- **Communication Channel**: Standard I/O (`stdin` / `stdout`) using newline-delimited JSON-RPC 2.0 messages.
- **Runtimes**: Standalone native binary; zero external runtime dependencies.

## Profiles & Wire Behavior

### 1. Zed Profile (`--profile zed` / `--zed` / auto-detected)
Tailored for rich Zed editor integration:
- **Thought Streaming**: Emits `sessionUpdate: "agent_thought_chunk"` rendering live gray thinking UI.
- **Command Menu**: Emits `sessionUpdate: "available_commands_update"` populating interactive `/` slash menu.
- **Dynamic Checklists**: Emits `sessionUpdate: "plan"` for real-time task progress tracking.
- **Code Navigation**: Enriches `tool_call_update` with `locations: [{"path": ...}]` for click-to-file jumping.

### 2. Standard / Default Profile (`--profile default` / `--standard`)
Strict adherence to vanilla Agent Client Protocol:
- Suppresses vendor-specific extensions to prevent editor JSON parsing crashes.
- Relays clean `agent_message_chunk` and standard `tool_call` life cycles.

### 3. Future Profiles
- `--vscode`: Adapter for VSCode client integrations (e.g. Cline/Roo-Code ACP protocol).
- `--cli`: Formatted human-readable terminal interaction.

## Protocol Methods Specification

### `initialize`
Negotiates protocol version and announces agent capabilities.
- **Request**:
  ```json
  {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"clientInfo": {"name": "Zed"}}}
  ```
- **Response**:
  ```json
  {
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
      "protocolVersion": 1,
      "agentCapabilities": {"loadSession": true},
      "agentInfo": {"name": "presence-acp", "title": "Presence ACP Gateway", "version": "0.3.0", "profile": "zed"}
    }
  }
  ```

### `session/new`
Initializes a fresh conversation session. In Zed profile, also broadcasts slash commands update.
- **Response**:
  ```json
  {"jsonrpc": "2.0", "id": 2, "result": {"sessionId": "acp-uuid"}}
  ```

### `session/prompt`
Submits a user prompt turn into the agent loop.
- **Params**:
  - `sessionId` (`string`): Target session ID.
  - `prompt` (`string`): Prompt text.
- **Turn Response**:
  ```json
  {"jsonrpc": "2.0", "id": 3, "result": {"stopReason": "end_turn"}}
  ```

### `session/cancel`
Notification requesting cancellation of active processing for the session.

## Failure Modes & Recovery
- **Malformed JSON-RPC**: Emits code `-32700` (`Parse error`) and keeps listening.
- **Unknown Methods**: Emits code `-32601` (`Method not found`).
- **Profile Mismatch**: When in doubt, defaults to standard ACP without proprietary extensions.