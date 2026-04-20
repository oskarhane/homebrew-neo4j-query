# PRD: Schema subcommand — database version info

## Overview

Extend the `schema` subcommand in `neo4j-query` to emit a top-level `database` block containing the Neo4j version, edition, and the default Cypher language version. Today agents using this CLI have to guess whether to write Cypher 5 syntax (`CALL db.index.vector.queryNodes(...)`) or Cypher 25 syntax (`SEARCH n IN (VECTOR INDEX ...)`). Surfacing these fields in schema output removes the guess.

## Goals

- Emit `database.neo4jVersion`, `database.edition`, and `database.defaultCypherVersion` as a sibling to `nodes`/`relationships`/`indexes`/`constraints`.
- Work on both Neo4j 5.x (no `db.query.default_language_version` setting → fall back to `"5"`) and Neo4j 2025.x+ (read the setting directly).
- Never break the `schema` command because the settings query failed — swallow errors and fall back.
- Cover the new output in the existing `schema_command` integration test.
- No new dependencies.

## Non-Goals

- Adding a separate `version` or `database` subcommand.
- Emitting every setting, component, or server capability — only the three fields above.
- Parsing or normalizing the version string (passed through as-is from `dbms.components()`).
- Detecting per-session `CYPHER 5`/`CYPHER 25` prefixes.

## Requirements

### Functional Requirements

- REQ-F-001: Extend `run_schema` in `src/main.rs` to issue `CALL dbms.components() YIELD name, versions, edition RETURN name, versions, edition` via the existing `run_cypher` helper. Take the first row where `name == 'Neo4j Kernel'` (or the single returned row) and extract:
  - `neo4jVersion`: first element of `versions`.
  - `edition`: the `edition` field.
- REQ-F-002: Additionally issue `SHOW SETTINGS YIELD name, value WHERE name = 'db.query.default_language_version' RETURN value` via `run_cypher`. If the query returns a row, use that `value` as `defaultCypherVersion`. If the query errors (setting doesn't exist on Neo4j 5.x) or returns zero rows, fall back to `"5"`.
- REQ-F-003: Errors from either query must not abort `schema`. If `dbms.components()` fails, emit a `database` block with only the fields that succeeded (or omit the block entirely if both failed). If the settings query fails, apply the `"5"` fallback.
- REQ-F-004: Attach the result under a new top-level `database` key in the returned schema JSON, alongside `nodes`, `relationships`, `indexes`, and `constraints`:

  ```json
  {
    "database": {
      "neo4jVersion": "2025.01.0",
      "edition": "community",
      "defaultCypherVersion": "25"
    },
    "nodes": [...],
    "relationships": [...],
    "indexes": [...],
    "constraints": [...]
  }
  ```

- REQ-F-005: Elide any field that is null or an empty string, matching the existing `insert_if_present` pattern already used for indexes/constraints.
- REQ-F-006: `defaultCypherVersion` is emitted as a string (not an integer) to stay consistent regardless of how Neo4j returns the setting value.
- REQ-F-007: Update the `schema_command` integration test (`tests/integration.rs`) to assert the TOON output contains the substrings `database`, `neo4jVersion`, `edition`, and `defaultCypherVersion`. Remain content-agnostic on exact values (they depend on the DB under test).
- REQ-F-008: Update `README.md` schema section to mention the new `database` block.
- REQ-F-009: Update both `skills/neo4j-query/SKILL.md` (compile-time embed source) and `.claude/skills/neo4j-query/SKILL.md` (developer copy) to document the `database` block — specifically that `defaultCypherVersion` tells the agent which vector-index syntax to use.

### Non-Functional Requirements

- REQ-NF-001: No new Cargo dependencies. Reuse `run_cypher` (src/main.rs:276), `serde_json::json!`, and the existing `insert_if_present` helper.
- REQ-NF-002: Existing `toon_format::encode_default` renders the new `database` block unchanged.
- REQ-NF-003: Non-ignored `cargo test`, `cargo clippy`, and `cargo fmt --check` remain clean. `cargo build` stays warning-free.
- REQ-NF-004: Integration test remains `#[ignore]`-gated and live-Neo4j only. Per AGENTS.md: always start a fresh container for integration tests — never reuse an existing one.

## Technical Considerations

- **`dbms.components()` shape**: Returns rows with `name` (e.g. "Neo4j Kernel"), `versions` (list of strings, typically one entry), and `edition` (string). On a single-kernel deployment we'll get exactly one row — grab it directly.
- **Settings query on 5.x**: `SHOW SETTINGS WHERE name = '<missing setting>'` returns zero rows (not an error) — normal flow falls through to the `"5"` fallback without needing special error handling. Still wrap in a defensive error-swallow because older 5.x minor versions could reject the syntax.
- **HTTP API error shape**: `run_cypher` already returns `Err` when Neo4j's response has `errors`. The two new queries need their errors captured locally (not propagated) so the schema command still succeeds with partial data.
- **Value type**: Neo4j may return the setting value as a string (`"25"`) or — depending on server version — as an integer. Coerce to string before emitting (`.to_string()` on numbers, pass strings through).
- **Ordering**: Place the `database` block first in the output JSON so it's the first thing an agent scanning the TOON tree reads.

## Acceptance Criteria

- [ ] `neo4j-query schema` output includes a top-level `database` block with `neo4jVersion`, `edition`, and `defaultCypherVersion` fields.
- [ ] On Neo4j 5.x (setting absent), `defaultCypherVersion` is `"5"`.
- [ ] On Neo4j 2025.x+, `defaultCypherVersion` matches the value of `db.query.default_language_version`.
- [ ] If the settings query errors unexpectedly, `schema` still succeeds and emits `defaultCypherVersion: "5"` as the fallback.
- [ ] If `dbms.components()` errors, `schema` still succeeds and emits the other sections.
- [ ] `defaultCypherVersion` is always a string in the emitted JSON.
- [ ] `schema_command` integration test asserts the new fields appear in output.
- [ ] `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt --check` all clean.
- [ ] `cargo test -- --ignored schema_command` passes against a fresh Neo4j 5 container.
- [ ] README.md schema section mentions the `database` block.
- [ ] `skills/neo4j-query/SKILL.md` and `.claude/skills/neo4j-query/SKILL.md` both mention the `database` block and point out `defaultCypherVersion` as the Cypher-syntax selector.

## Out of Scope

- A dedicated `version` or `database` subcommand.
- Parsing/validating the version string.
- Honoring per-session `CYPHER 5`/`CYPHER 25` prefixes.
- Supporting Neo4j 4.x (already out of scope repo-wide).
- Bumping `Cargo.toml` version (happens in the separate release step).

## Open Questions

None. User confirmed: emit all three fields; cover in tests; follow the same PRD → tasks → run workflow.
