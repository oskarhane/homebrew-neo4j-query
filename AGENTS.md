# Agents

## Repository

This is a single repo (`oskarhane/homebrew-neo4j-query`) that serves as both the Homebrew tap and the source code. There is no separate `neo4j-query` repo. The formula at `Formula/neo4j-query.rb` lives alongside the Rust source.

## Feedback Instructions

- **RUN**: `cargo run --bin neo4j-query -- <args>` (multiple binaries exist; bare `cargo run` fails)
- **BUILD**: `cargo build`
- **TEST (unit)**: `cargo test`
- **TEST (integration, needs Neo4j)**: `cargo test -- --ignored`
  - **Always start a fresh container for integration tests — never reuse an existing one.** `docker rm -f neo4j-test 2>/dev/null; docker run -d --name neo4j-test -p 7474:7474 -p 7687:7687 -e NEO4J_AUTH=neo4j/testpassword neo4j:5` then wait for `http://localhost:7474` before running tests. Stop/remove after.
- **LINT**: `cargo clippy`
- **FORMAT**: `cargo fmt --check`

## Testing conventions

- `tests/unit.rs` re-implements private functions locally rather than importing from the crate (no `src/lib.rs`). When testing `src/*` logic, copy the function verbatim into `tests/unit.rs` and call it — matches the pattern used for `parse_param_value`, `parse_params`, `truncate_arrays`, `parse_param`, `EmbedError`, `resolve_api_key`.
- Avoid `3.14` in tests — rust 1.94 `clippy::approx_constant` denies it. Use `2.5` or another non-PI float, or add `#![allow(clippy::approx_constant)]` at the top of the test file.
- `serde_json` `preserve_order` is enabled transitively via `toon-format` — JSON object key order IS preserved in TOON output. Build output `Map`s via explicit `insert` in the order you want emitted; don't rely on alphabetical sorting.
- **Neo4j version-dependent queries**: verify behavior on BOTH `neo4j:5` and `neo4j:latest` containers (CI matrix covers both). Silent-fallback bugs (wrong setting name, wrong value shape) often look correct on one version and wrong on the other. Default-Cypher setting is `db.query.default_language` (not `..._version`); value is `CYPHER_5` / `CYPHER_25` (strip `CYPHER_` prefix). Override at container start via `-e NEO4J_db_query_default__language=CYPHER_25` (double underscore escapes a literal underscore in env-var form).

## CLI Architecture

- Connects via Neo4j's **HTTP API** (not Bolt). Default ports: `http://<host>:7474`, `https://<host>:7473`.
- `ConnectionArgs.password` is `Option<String>` — subcommands that don't need DB access (like `skill`) work without it. Use `require_password()` in modes that need it.
- `args_conflicts_with_subcommands = true` on Cli struct separates query-mode flags from subcommand flags.
- **Flattened arg structs that subcommands need must mark every field `global = true`.** Flatten the struct ONCE onto `QueryArgs` (see `ConnectionArgs`, `EmbedCliArgs`). Never flatten the same struct onto both the root and a subcommand — flags typed before the subcommand land on the root copy and are invisible to the subcommand handler, producing silent config misses. Subcommand handlers receive the global args by destructuring `cli.query_args.<field>` at dispatch. When adding a new cross-cutting flag group, always add an integration test that passes the flags BEFORE the subcommand name.

## AI Agent Skill

The repo ships an AI agent skill (`skills/neo4j-query/SKILL.md`) that gets **embedded into the binary** at compile time via `include_str!("../skills/neo4j-query/SKILL.md")` in `src/skill.rs`. The `neo4j-query skill install` command writes this embedded content to `~/.local/share/neo4j-query/skills/neo4j-query/SKILL.md` and symlinks it into each detected agent's skills directory.

A second copy lives at `.claude/skills/neo4j-query/SKILL.md` for local development use. **Both copies must be kept in sync** — one is the compile-time source, the other is what Claude Code loads during development in this repo.

## Release

The GitHub Actions release workflow (`.github/workflows/release.yml`) builds binaries, creates a GitHub release, and updates the formula in this same repo — all in one place.
