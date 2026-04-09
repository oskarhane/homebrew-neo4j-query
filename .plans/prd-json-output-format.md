# PRD: JSON Output Format

## Overview

Add `--output-format` flag to `neo4j-query` supporting `toon` (default) and `json` output formats. JSON output serializes the same record array as TOON but as standard JSON, enabling easier integration with tools like `jq`.

## Goals

- Support JSON as an alternative output format for regular Cypher queries
- Maintain TOON as the default output format
- Zero transformation difference between formats — same record array, different serialization

## Non-Goals

- Changing `.schema` output (always TOON regardless of flag)
- Adding other output formats (CSV, table, etc.)
- Pretty-printing or formatting options for JSON output

## Requirements

### Functional Requirements

- REQ-F-001: Add `--output-format` CLI flag accepting `toon` or `json` (case-insensitive enum via clap)
- REQ-F-002: Default to `toon` when `--output-format` is omitted
- REQ-F-003: When `json`, output the record array as a JSON array to stdout (same shape as TOON, just `serde_json::to_string` instead of `toon_format::encode_default`)
- REQ-F-004: `.schema` command ignores `--output-format` and always outputs TOON
- REQ-F-005: Invalid `--output-format` values produce a clear clap error

### Non-Functional Requirements

- REQ-NF-001: No new dependencies — `serde_json` is already in Cargo.toml
- REQ-NF-002: Integration tests covering JSON output with real Neo4j
- REQ-NF-003: Update README.md — add `--output-format` to the Configuration table and a brief note in the Output section. Keep it minimal; TOON is the recommended format.
- REQ-NF-004: Update `.claude/skills/neo4j-query/SKILL.md` — mention `--output-format json` exists but keep TOON as the default and primary format in all examples.

## Technical Considerations

- Add a `OutputFormat` enum (`Toon`, `Json`) with clap `ValueEnum` derive
- Add `--output-format` field to `Cli` struct with `default_value = "toon"`
- In `run()`, branch on the format enum at the output step (~line 413 in `src/main.rs`): TOON path unchanged, JSON path uses `serde_json::to_string(&records)`
- `.schema` path (~line 377) remains unchanged — always TOON
- Short flag not needed to avoid conflicts with `-p`

## Acceptance Criteria

- [ ] `neo4j-query "RETURN 1 as n"` outputs TOON (unchanged default)
- [ ] `neo4j-query --output-format json "RETURN 1 as n"` outputs valid JSON array
- [ ] `neo4j-query --output-format toon "RETURN 1 as n"` outputs TOON
- [ ] `neo4j-query --output-format xml "RETURN 1"` fails with clap error
- [ ] `.schema` always outputs TOON even with `--output-format json`
- [ ] Integration tests exist for JSON output: literal return, multi-column, params, null values, empty result set
- [ ] `cargo test` (unit) passes
- [ ] `cargo test -- --ignored` (integration) passes
- [ ] README.md updated with `--output-format` flag (minimal mention, TOON stays primary)
- [ ] SKILL.md updated to mention `--output-format json` without changing existing TOON-first examples

## Out of Scope

- Pretty-print / `--pretty` flag for JSON
- Environment variable for default format
- JSON output for `.schema`

## Open Questions

None.
