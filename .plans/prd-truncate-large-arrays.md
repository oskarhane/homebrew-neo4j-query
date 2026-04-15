# PRD: Truncate Large Arrays in Query Responses

## Overview

Recursively detect and fully truncate arrays exceeding a configurable length threshold in Neo4j query responses. Primary motivation: prevent forwarding embedding vectors (typically 1536+ items) to downstream consumers.

## Goals

- Strip large arrays (embeddings, etc.) from query output before formatting
- Make the threshold configurable via CLI flag
- Clearly indicate truncation happened in TOON output; use empty array in JSON output

## Non-Goals

- Truncating arrays in schema mode output
- Partial truncation (keeping first N items) — arrays are fully removed
- Filtering based on array content type (applies to all arrays regardless of element type)

## Requirements

### Functional Requirements

- REQ-F-001: Add `--truncate-arrays-over <N>` CLI flag to query mode. Default: `100`. `0` disables truncation.
- REQ-F-002: After `rows_to_records` produces `Vec<Value>`, walk each record recursively. Any `Value::Array` with length > N gets replaced.
- REQ-F-003: In TOON mode, replace truncated arrays with the string `"[array truncated: <original_length> items]"`.
- REQ-F-004: In JSON mode, replace truncated arrays with `[]` (empty array).
- REQ-F-005: Recursion must handle arbitrary nesting: arrays inside objects inside arrays, etc.
- REQ-F-006: When N is `0`, no truncation occurs — arrays pass through unchanged.

### Non-Functional Requirements

- REQ-NF-001: Truncation function must be a pure function (`fn(&mut Value, usize)` or similar) testable without Neo4j.
- REQ-NF-002: Unit tests for the truncation logic covering: no truncation needed, top-level array, nested array, deeply nested, threshold=0, boundary (exactly N vs N+1).
- REQ-NF-003: Integration tests with real Neo4j (via docker-compose) verifying end-to-end truncation in both TOON and JSON modes.

### Documentation Requirements

- REQ-D-001: Update `README.md` to document the `--truncate-arrays-over` flag, its default, and behavior.
- REQ-D-002: Update the AI agent skill files (`skills/neo4j-query/SKILL.md` and `.claude/skills/neo4j-query/SKILL.md`) to mention the flag — both copies must stay in sync (see AGENTS.md).
- REQ-D-003: Ensure `--help` output includes the new flag with a clear description (handled by clap derive).

## Technical Considerations

- The truncation function operates on `serde_json::Value` in-place before the format branch in `run_query_mode`.
- Two-phase approach: truncate with a mode-aware replacement value. For TOON: `Value::String("[array truncated: ...]")`. For JSON: `Value::Array(vec![])`.
- The flag belongs on `QueryArgs` since it only applies to query mode (not schema or skill subcommands).
- Pure function should live in `src/main.rs` alongside other helpers (`parse_params`, `rows_to_records`, etc.) and be re-implemented in `tests/unit.rs` following the existing test pattern.

## Acceptance Criteria

- [ ] `neo4j-query "RETURN range(0, 200) as big" --truncate-arrays-over 100` outputs truncation annotation in TOON
- [ ] Same query with `--format json` outputs `[]` for the truncated array
- [ ] `--truncate-arrays-over 0` passes all arrays through unchanged
- [ ] Default behavior (no flag) truncates arrays over 100 items
- [ ] Nested arrays (e.g., node properties containing embeddings) are also truncated
- [ ] Unit tests pass: `cargo test`
- [ ] Integration tests pass with Docker Neo4j: `cargo test -- --ignored`
- [ ] README documents the new flag
- [ ] Both skill files updated and in sync
- [ ] `neo4j-query --help` shows the flag

## Out of Scope

- Schema mode truncation
- Per-field or per-type truncation rules
- Configuring the replacement text format
- Environment variable for the flag (just CLI arg for now)

## Open Questions

None.
