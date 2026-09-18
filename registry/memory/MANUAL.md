# Manual: memory
*Version 0.3.0 — Two-Tier Episodic and Semantic Memory Subsystem for Presence*

## Overview
The `memory` organ implements unified, multi-tier cognitive storage for Presence. It decouples fast, ephemeral scratchpad observations (Tier 1 Episodic Memory) from long-term, high-importance synthesized knowledge (Tier 2 Semantic Memory). Scoring combines exact keyword relevance, importance multipliers, and exponential temporal decay.

## Environment & Dependencies
- **Platforms**: Windows (`x86_64`), Linux (`x86_64`, `aarch64`), macOS (`x86_64`, `aarch64`).
- **Permissions**: Read/Write access to `$PRESENCE_WORKSPACE/memory/store/`.
- **Runtimes**: Standalone native binary; zero external database or C-library dependencies.

## Architecture & Scoring Formula

### Two-Tier Model
- **Tier 1 (Episodic)**: Scratchpad events, tool execution traces, recent observations. High temporal decay rate ($\lambda = 0.005$, half-life $\approx 6$ days).
- **Tier 2 (Semantic)**: Consolidations, project architecture truths, enduring rules. Low decay rate ($\lambda = 0.0005$, half-life $\approx 60$ days).

### Scoring Model
$$\text{Score} = \text{MatchRelevance} \times (1.0 + 0.15 \times (\text{Importance} - 1.0)) \times e^{-\lambda \times \Delta t_{\text{hours}}}$$
Where $\text{MatchRelevance} = 3 \times \text{KeyTokens} + 2 \times \text{TagTokens} + 1 \times \text{ContentTokens}$.

## Tools Specification

### `memory_store`
Stores or updates a memory item in the unified store.
- **Parameters**:
  - `key` (`string`, required): Unique identifier or cognitive anchor.
  - `content` (`string`, required): Fact or knowledge snippet.
  - `tier` (`string`, optional, default: `"tier1"`): Target tier (`"tier1"` or `"tier2"`).
  - `tags` (`string`, optional): Comma-separated or JSON list of categorization tags.
  - `importance` (`number`, optional, default: `5.0`): Importance rank from 1.0 to 10.0.
- **Return Shape**:
  ```json
  {
    "status": "ok",
    "data": {
      "action": "created",
      "id": "mem-e3f5b721",
      "key": "build.flags",
      "tier": "tier1",
      "importance": 7.5
    }
  }
  ```

### `memory_recall`
Searches memory using query terms, applying importance weighting and decay penalties.
- **Parameters**:
  - `query` (`string`, required): Search terms.
  - `tier` (`string`, optional, default: `"all"`): Scope filter (`"tier1"`, `"tier2"`, or `"all"`).
  - `tags` (`string`, optional): Tag filter.
  - `limit` (`number`, optional, default: `5`): Maximum results to return.
- **Return Shape**:
  ```json
  {
    "status": "ok",
    "data": {
      "count": 1,
      "query": "build flags",
      "results": [
        {
          "id": "mem-e3f5b721",
          "key": "build.flags",
          "content": "Use --release -p organ-memory",
          "tier": "tier1",
          "importance": 7.5,
          "score": 14.82,
          "age_hours": 0.0
        }
      ]
    }
  }
  ```

### `memory_consolidate`
Promotes frequently accessed or high-importance episodic items into semantic memory, and prunes stale scratch items.
- **Parameters**: None.
- **Return Shape**:
  ```json
  {
    "status": "ok",
    "data": {
      "action": "consolidated",
      "promoted_to_tier2": 3,
      "pruned_stale": 1,
      "total_items": 42
    }
  }
  ```

## Senses & Stimuli

### Sense: `memory_stats`
Provides real-time statistics on total memories, episodic vs semantic ratio, and compaction recency.

### Stimulus: `memory_consolidation_due`
- **Cadence**: 1800s (30 minutes).
- **Trigger**: Periodic vegetative compaction interval reached.
- **Action**: Triggers `memory_consolidate`.

## Failure Modes & Recovery
- **Index Corruption**: Employs atomic temp-file rename (`index.json.tmp` -> `index.json`) to prevent write tearing.