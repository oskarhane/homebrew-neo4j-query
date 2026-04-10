# Agents

## Repository

This is a single repo (`oskarhane/homebrew-neo4j-query`) that serves as both the Homebrew tap and the source code. There is no separate `neo4j-query` repo. The formula at `Formula/neo4j-query.rb` lives alongside the Rust source.

## Feedback Instructions

- **RUN**: `cargo run --bin neo4j-query -- <args>` (multiple binaries exist; bare `cargo run` fails)
- **BUILD**: `cargo build`
- **TEST (unit)**: `cargo test`
- **TEST (integration, needs Neo4j)**: `cargo test -- --ignored`
- **LINT**: `cargo clippy`
- **FORMAT**: `cargo fmt --check`

## CLI Architecture

- `ConnectionArgs.password` is `Option<String>` — subcommands that don't need DB access (like `skill`) work without it. Use `require_password()` in modes that need it.
- `args_conflicts_with_subcommands = true` on Cli struct separates query-mode flags from subcommand flags.

## Release

The GitHub Actions release workflow (`.github/workflows/release.yml`) builds binaries, creates a GitHub release, and updates the formula in this same repo — all in one place.
