# Agents

## Repository

This is a single repo (`oskarhane/homebrew-neo4j-query`) that serves as both the Homebrew tap and the source code. There is no separate `neo4j-query` repo. The formula at `Formula/neo4j-query.rb` lives alongside the Rust source.

## Release

The GitHub Actions release workflow (`.github/workflows/release.yml`) builds binaries, creates a GitHub release, and updates the formula in this same repo — all in one place.
