# neo4j-query CLI Tool

## Context
Rust CLI tool to query Neo4j via its **Query API v2** (`POST /db/{db}/query/v2`), outputting results in TOON format. Distributed via Homebrew. Includes a Claude Code skill for easy integration.

## Credential Handling
Env vars + CLI flag overrides (clap `env` attribute):
- `NEO4J_URI` / `--uri` — default `http://localhost:7474`
- `NEO4J_USER` / `--user` — default `neo4j`
- `NEO4J_PASSWORD` / `--password` — required
- `NEO4J_DATABASE` / `--database` — default `neo4j`

## Files to Create

### Core Application

**`Cargo.toml`**
```toml
[package]
name = "neo4j-query"
version = "0.1.0"
edition = "2021"
description = "Query Neo4j databases, output TOON"
license = "MIT"

[dependencies]
reqwest = { version = "0.12", features = ["json"] }
toon-format = "0.4"
tokio = { version = "1", features = ["rt", "macros"] }
clap = { version = "4", features = ["derive", "env"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

**`src/main.rs`** (~180 lines)
- CLI struct (clap derive) with `env` fallbacks
- `-p key=value` repeatable param flag → `HashMap<String, Value>` (auto-detect number/bool/string)
- Query resolution: positional arg → stdin (if not TTY) → error
- HTTP: `POST {uri}/db/{database}/query/v2`, Basic auth, body `{"statement": "...", "parameters": {...}}`
- Response: zip `data.fields` with each `data.values[]` row → `Vec<Map<String, Value>>` → `toon_format::encode_default` → stdout
- Errors from response → stderr, exit 1

### Tests

**`tests/unit.rs`** — Unit-style tests
- `resolve_query()` with arg present
- `parse_params()` — string, number, bool values
- Response JSON → TOON conversion (mock the HTTP response JSON, test the parsing/conversion logic)

**`tests/integration.rs`** — Integration tests (needs Docker)
- Uses `assert_cmd` to run the binary
- Spins up Neo4j via `docker run` in a setup step (or assumes running container)
- Tests:
  - Basic query: `RETURN 1 as n`
  - Node query: create + match
  - Query with `-p` params
  - Stdin input
  - Missing password → error
  - Bad query → error from Neo4j

**`tests/docker-compose.yml`** — Neo4j for integration tests
```yaml
services:
  neo4j:
    image: neo4j:5
    ports:
      - "7474:7474"
      - "7687:7687"
    environment:
      NEO4J_AUTH: neo4j/testpassword
```

### README

**`README.md`**
- What it does (one-liner)
- Install (brew tap + brew install)
- Usage examples: positional arg, stdin pipe, with params
- Credential config (env vars + flags)
- TOON format link
- Build from source instructions

### Claude Skill

**`.claude/skills/neo4j-query/SKILL.md`**
```yaml
---
name: neo4j-query
description: Query Neo4j databases using neo4j-query CLI tool. Use when user asks to query Neo4j, explore graph data, or run Cypher queries.
user-invocable: true
allowed-tools: Bash(neo4j-query *)
argument-hint: "[cypher query]"
---
```
Body: instructions for Claude on how to use `neo4j-query`, examples, env var setup, output format explanation (TOON).

### Homebrew Distribution

**`.github/workflows/release.yml`** — GitHub Actions release workflow
- **Trigger:** push tag `v*`
- **Matrix:** 4 targets
  - `x86_64-apple-darwin` (macos-latest)
  - `aarch64-apple-darwin` (macos-latest, cross)
  - `x86_64-unknown-linux-gnu` (ubuntu-latest)
  - `aarch64-unknown-linux-gnu` (ubuntu-latest, cross)
- **Steps per target:**
  1. Checkout
  2. Install Rust toolchain + target
  3. Install `cross` for cross-compilation targets
  4. `cargo build --release --target $TARGET` (or `cross build` for cross targets)
  5. `tar -czf neo4j-query-$TARGET.tar.gz -C target/$TARGET/release neo4j-query`
  6. Upload tarball as artifact
- **Final job** (needs all builds):
  1. Download all artifacts
  2. Create GitHub Release from tag with `gh release create`
  3. Attach all 4 tarballs to the release

**Separate repo: `homebrew-neo4j-query`** (not created here, documented in README)
- `Formula/neo4j-query.rb` — downloads from GitHub Releases
- User installs: `brew tap oskarhane/neo4j-query && brew install neo4j-query`

The formula file template will live in this repo at `dist/homebrew/neo4j-query.rb` as reference.

**`dist/homebrew/neo4j-query.rb`** — Formula template
```ruby
class Neo4jQuery < Formula
  desc "Query Neo4j databases, output TOON"
  homepage "https://github.com/oskarhane/neo4j-query"
  version "<VERSION>"
  
  on_macos do
    on_arm do
      url "https://github.com/oskarhane/neo4j-query/releases/download/v#{version}/neo4j-query-aarch64-apple-darwin.tar.gz"
      sha256 "<SHA>"
    end
    on_intel do
      url "https://github.com/oskarhane/neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-apple-darwin.tar.gz"
      sha256 "<SHA>"
    end
  end
  on_linux do
    url "https://github.com/oskarhane/neo4j-query/releases/download/v#{version}/neo4j-query-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "<SHA>"
  end

  def install
    bin.install "neo4j-query"
  end
end
```

## Implementation Order
1. `Cargo.toml` + `src/main.rs` — core CLI
2. `cargo build` + manual test against local Neo4j
3. `tests/unit.rs` — unit tests
4. `tests/docker-compose.yml` + `tests/integration.rs` — integration tests
5. `README.md`
6. `.claude/skills/neo4j-query/SKILL.md`
7. `.github/workflows/release.yml`
8. `dist/homebrew/neo4j-query.rb`

## Verification
1. `cargo build` — compiles
2. `cargo test` — unit tests pass
3. `docker compose -f tests/docker-compose.yml up -d && cargo test -- --ignored` — integration tests
4. Manual: `NEO4J_PASSWORD=test ./target/debug/neo4j-query "RETURN 1 as n"`
5. Manual: `echo "RETURN 1 as n" | NEO4J_PASSWORD=test ./target/debug/neo4j-query`

## Resolved
- Repo: `https://github.com/oskarhane/homebrew-neo4j-query.git` (single repo for source + homebrew tap)
- License: MIT
