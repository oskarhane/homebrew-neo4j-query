# PRD: Schema subcommand — indexes & constraints

## Overview

Extend the `schema` subcommand in `neo4j-query` to also introspect and emit indexes and constraints alongside the existing nodes and relationships. Agents using this CLI to write correct/performant Cypher need to see indexed properties, uniqueness rules, and vector-index configuration (dimensions, similarity function). Today the schema output omits this entirely.

Source plan: `/Users/oskarhane/.claude/plans/yes-please-do-make-eager-nest.md`.

## Goals

- Emit `indexes` and `constraints` as sibling keys to `nodes` and `relationships` in `schema` output.
- Preserve enough metadata per index/constraint that an agent can pick the right index (name, type, entity type, labels/types, properties, state) and read vector/fulltext config (`options`).
- Cover the new output in the existing `schema_command` integration test.
- No new dependencies.

## Non-Goals

- Filtering, grouping, or summarizing indexes/constraints (everything Neo4j returns for the user's DB is included).
- A separate `indexes` or `constraints` subcommand.
- Schema output in formats other than the existing TOON/JSON paths (those already render the new keys automatically).
- Backfilling for Neo4j versions that don't support `SHOW INDEXES` / `SHOW CONSTRAINTS` with the listed columns — Neo4j 5.x is the target.

## Requirements

### Functional Requirements

- REQ-F-001: Extend `run_schema` in `src/main.rs` (lines 292-443) to add two additional `run_cypher` calls, reusing the existing helper at `src/main.rs:276`:
  - `SHOW INDEXES YIELD name, type, entityType, labelsOrTypes, properties, state, owningConstraint, options`
  - `SHOW CONSTRAINTS YIELD name, type, entityType, labelsOrTypes, properties, ownedIndex, propertyType`
- REQ-F-002: Map each returned index row to a JSON object containing: `name`, `type`, `entityType`, `labelsOrTypes`, `properties`, `state`, `owningConstraint`, and `options`. Pass `options` through as an opaque JSON object so vector index config (`indexConfig.vector.dimensions`, `vector.similarity_function`) and fulltext analyzer config surface to agents.
- REQ-F-003: Map each returned constraint row to a JSON object containing: `name`, `type`, `entityType`, `labelsOrTypes`, `properties`, `ownedIndex`, `propertyType`.
- REQ-F-004: Include all indexes and constraints returned by Neo4j — do not filter out system-created or LOOKUP indexes.
- REQ-F-005: Sort `indexes` and `constraints` arrays by `name` (ascending) for stable output.
- REQ-F-006: Extend the returned schema JSON to add `indexes` and `constraints` as sibling keys to `nodes` and `relationships`. Shape:

  ```json
  {
    "nodes": [...],
    "relationships": [...],
    "indexes": [...],
    "constraints": [...]
  }
  ```

- REQ-F-007: Skip empty or null fields in each emitted index/constraint object to keep TOON output compact (consistent with existing node/relationship emission).
- REQ-F-008: Integration test `schema_command` in `tests/integration.rs` (lines 308-338) must seed one uniqueness constraint and one range index before running `schema`, assert both surface in the TOON output, and drop them during cleanup:
  - Seed: `CREATE CONSTRAINT schema_test_unique IF NOT EXISTS FOR (n:SchemaTest) REQUIRE n.x IS UNIQUE` and `CREATE INDEX schema_test_z_idx IF NOT EXISTS FOR (n:SchemaTarget) ON (n.z)`.
  - Assertions (substring): `indexes`, `constraints`, `schema_test_unique`, `schema_test_z_idx`, `RANGE`, and a uniqueness marker (e.g. `UNIQUE`).
  - Cleanup: `DROP CONSTRAINT schema_test_unique IF EXISTS` and `DROP INDEX schema_test_z_idx IF EXISTS`, in addition to the existing node deletes.
- REQ-F-009: Update `README.md` schema section (~line 211) with a one-line note that schema output includes `indexes` and `constraints`.
- REQ-F-010: Update both `skills/neo4j-query/SKILL.md` (the compile-time embed source) and `.claude/skills/neo4j-query/SKILL.md` (the installed copy) so the agent-facing documentation matches.

### Non-Functional Requirements

- REQ-NF-001: No new Cargo dependencies. Reuse `run_cypher`, `serde_json::json!`, and the existing `HashMap`/`Vec<Value>` patterns already in `run_schema`.
- REQ-NF-002: Existing `toon_format::encode_default` (called at `src/main.rs:603`) must render the new sections without any format code changes.
- REQ-NF-003: Non-ignored `cargo test` continues to pass. `cargo build` stays clean (no new warnings beyond what's already accepted).
- REQ-NF-004: Integration test remains `#[ignore]` and live-Neo4j gated, using the existing `cmd()` helper and `neo4j_available()` check — no new test infrastructure.

## Technical Considerations

- **Cypher column availability**: `SHOW INDEXES`/`SHOW CONSTRAINTS` on Neo4j 5.x reliably return the listed columns. Older Neo4j 4.x (`db.indexes()` / `db.constraints()`) is out of scope — the repo already assumes 5.x for the existing schema procedures.
- **Constraint type naming**: Neo4j 5 returns constraint `type` values like `UNIQUENESS`, `NODE_PROPERTY_EXISTENCE`, `RELATIONSHIP_PROPERTY_EXISTENCE`, `NODE_KEY`, `NODE_PROPERTY_TYPE`. Pass these through as-is; do not normalize.
- **`options` shape**: This column comes back as a nested map (indexConfig + indexProvider). Accept it as an opaque `Value` and emit directly — TOON will render the nested tree.
- **Sort key**: `name` is always present in Neo4j 5's `SHOW` output (auto-generated when user omits one). Safe to sort by.
- **Empty field elision**: Use the same "omit when None/empty array" pattern that existing node/relationship emission follows, so `schema` output stays scannable in TOON.
- **Test ordering**: Run `DROP ... IF EXISTS` before node deletes so constraints don't block deletion of constrained nodes (belt-and-braces; `DETACH DELETE` already handles rels).

## Acceptance Criteria

- [ ] `neo4j-query schema` output includes `indexes` and `constraints` keys alongside `nodes` and `relationships`.
- [ ] Each emitted index entry carries `name`, `type`, `entityType`, `labelsOrTypes`, `properties`, `state`, `owningConstraint` (when set), and `options`.
- [ ] Each emitted constraint entry carries `name`, `type`, `entityType`, `labelsOrTypes`, `properties`, `ownedIndex`, and `propertyType` (when set).
- [ ] `indexes` and `constraints` arrays are sorted by `name`.
- [ ] System/LOOKUP indexes are present in output (no filtering).
- [ ] Running `schema` against a DB with a vector index shows `dimensions` and `similarity_function` under that index's `options` in the TOON tree.
- [ ] `schema_command` integration test seeds a uniqueness constraint and a range index, asserts their presence in output, and cleans both up.
- [ ] `cargo build` is clean.
- [ ] `cargo test` (non-ignored) passes.
- [ ] `cargo test -- --ignored schema_command` passes against a local Neo4j 5.x instance.
- [ ] `README.md` schema section mentions indexes/constraints.
- [ ] `skills/neo4j-query/SKILL.md` and `.claude/skills/neo4j-query/SKILL.md` both mention indexes/constraints in schema output.

## Out of Scope

- Backward compatibility shims for Neo4j 4.x schema procedures.
- Separate `indexes` or `constraints` subcommands.
- Filtering/curation of which indexes or constraints are emitted.
- Snapshot-based integration tests (continues the substring-assertion style already used).
- Bumping `Cargo.toml` version (handled in a separate release step).

## Open Questions

None. Plan confirmed by user: include full `options` JSON; include all indexes/constraints with no filtering.
