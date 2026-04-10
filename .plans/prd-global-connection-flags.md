# PRD: Global Connection Flags

## Overview

Connection flags (`-p`, `-u`, `--uri`, `--db`, `--env`) don't work when placed before the `schema` subcommand. `neo4j-query -p password schema` fails with "password required" because clap assigns pre-subcommand flags to the parent struct while `Schema` has its own separate `ConnectionArgs`.

## Goals

- Connection flags work in any position: before, after, or split around subcommands
- No regressions in query mode or skill subcommands
- Thorough test coverage for flag/subcommand/query interactions

## Non-Goals

- Making `-P` (query params) or `--format` global — these are query-mode only
- Changing the `load_env()` pre-scan mechanism
- Adding new flags or subcommands

## Requirements

### Functional Requirements

- REQ-F-001: `neo4j-query -p password schema` must pass password to schema handler
- REQ-F-002: `neo4j-query schema -p password` must work (flags after subcommand)
- REQ-F-003: `neo4j-query -u neo4j schema -p password` must work (flags split around subcommand)
- REQ-F-004: Long-form flags (`--password`, `--username`) must work in same positions as short flags
- REQ-F-005: `--db` and `--env` must work before and after schema subcommand
- REQ-F-006: `skill list` must continue working without requiring connection args
- REQ-F-007: `schema` without password must still error with "password required"
- REQ-F-008: Query mode (`neo4j-query -p pass "RETURN 1"`) must be unaffected

### Non-Functional Requirements

- REQ-NF-001: No new dependencies
- REQ-NF-002: All new tests runnable without Neo4j (use dead port + error type assertions)

## Technical Considerations

- Add `global = true` to all `ConnectionArgs` fields — all qualify (have defaults or are `Option<T>`)
- Remove `ConnectionArgs` from `Commands::Schema`, make it a unit variant
- Pass parent's `cli.query_args.conn` to `run_schema_mode()`
- Keep `subcommand_negates_reqs = true` — still needed so positional `query` arg isn't required when subcommands are used
- `load_env()` pre-scans raw argv for `--env` before clap parses — unaffected by this change

### Files to modify

- `src/main.rs` — `ConnectionArgs` (add `global = true`), `Commands` enum (simplify Schema), `run()` match arm
- `tests/integration.rs` — add `cmd_no_neo4j()` helper + 13 new tests

## Acceptance Criteria

- [ ] `neo4j-query -p password schema` no longer errors with "password required"
- [ ] `neo4j-query schema -p password` works
- [ ] `neo4j-query -u user schema -p password` works (split flags)
- [ ] `neo4j-query schema --db testdb -p password` works
- [ ] `neo4j-query schema --env file.env` works
- [ ] `neo4j-query skill list` still works without connection args
- [ ] `neo4j-query schema` (no password) still errors appropriately
- [ ] All existing tests pass (`cargo test`)
- [ ] `cargo clippy` clean
- [ ] 13 new tests added and passing

## Out of Scope

- Globalizing query-specific flags (`-P`, `--format`)
- Schema output format options
- New subcommands

## Open Questions

None.
