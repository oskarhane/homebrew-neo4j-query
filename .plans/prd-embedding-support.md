# PRD: Embedding Support for neo4j-query

## Overview

Add first-class embedding generation to `neo4j-query` so users (and AI agents) can run Neo4j vector search directly from the CLI without computing embeddings out-of-band. Introduces a `:embed` parameter modifier (`-P query:embed='text'`), a debug subcommand (`neo4j-query embed 'text'`), and a provider-agnostic backend supporting OpenAI and Ollama in v1. Target release: v0.11.0.

## Goals

- Let users pass free-text strings as query parameters and have the CLI produce `Vec<f32>` embeddings automatically before query execution.
- Support both a paid/hosted provider (OpenAI) and a local/free provider (Ollama) with identical user-facing surface.
- Mirror the existing `NEO4J_*` env + `.env` + CLI-flag configuration model so setup is a one-time `.env` change.
- Honest, actionable error messages when provider is unconfigured, unreachable, or misconfigured.
- Zero overhead for users who never embed (lazy provider init, no env/HTTP client touched unless `:embed` is used).
- Split `src/main.rs` into focused modules (`embed/`, `commands/embed.rs`, `params.rs`) as part of this work.

## Non-Goals

- Batching multiple `:embed` params into a single provider request.
- Providers beyond OpenAI and Ollama (Voyage, Cohere, Bedrock, etc.).
- Local in-process embedding (e.g. `fastembed-rs`) with no external service.
- Document-vs-query embedding hints (`:embed_doc` modifier, Voyage/Cohere `input_type`).
- Caching of repeat `text → vector` within a session.
- Persisting `:embed` results back into Neo4j (this feature is strictly query-time).

## Requirements

### Functional Requirements

- REQ-F-001: Extend `-P` parser grammar to `name[:modifier]=value`. Only `:embed` is recognised in v1; any other modifier errors with `unknown param modifier: :<name>`.
- REQ-F-002: Un-modified params (`-P age=30`, `-P active=true`, `-P x=null`) **retain existing type coercion** (int/float/bool/null/string). Only `:embed` changes a param's resolution path.
- REQ-F-003: Add `embed` subcommand: `neo4j-query embed [TEXT]`. Reads TEXT from stdin when the positional is omitted (trimmed).
- REQ-F-004: The subcommand supports `--format json` (default, single-line JSON array) and `--format raw` (newline-separated floats). **TOON format is intentionally not supported.**
- REQ-F-005: Add embed CLI flags, `#[command(flatten)]`'d onto both root (query mode) and the `embed` subcommand:
    - `--embed-provider` (env `NEO4J_EMBED_PROVIDER`)
    - `--embed-model` (env `NEO4J_EMBED_MODEL`)
    - `--embed-dimensions` (env `NEO4J_EMBED_DIMENSIONS`)
    - `--embed-base-url` (env `NEO4J_EMBED_BASE_URL`)
- REQ-F-006: API keys are env-only (no CLI flag): `OPENAI_API_KEY` (conventional) with `NEO4J_EMBED_API_KEY` as fallback. Ollama needs no key; if `NEO4J_EMBED_API_KEY` is set it is silently ignored.
- REQ-F-007: Config resolution priority: CLI flag > shell env > `.env` (same as existing `NEO4J_*` behavior). `.env` loading uses the existing `load_env()` path — no changes there.
- REQ-F-008: Provider is **lazily initialised**: if no `:embed` param is present in query mode, no embed env vars are read and no HTTP client is built.
- REQ-F-009: OpenAI provider: POST `{base_url}/embeddings` with bearer auth, `{input, model, dimensions?}`; base URL defaults to `https://api.openai.com/v1`.
- REQ-F-010: Ollama provider: POST `{base_url}/api/embed` (plural response shape) with `{model, input}`; base URL defaults to `http://localhost:11434`. No API key sent. Connection refusal → error message names `ollama serve`; HTTP 404 → error message names `ollama pull <model>`.
- REQ-F-011: Error surface:
    - `:embed` used with no `NEO4J_EMBED_PROVIDER` → `embedding provider not configured: set NEO4J_EMBED_PROVIDER`
    - Provider set, model missing → `NEO4J_EMBED_MODEL not set`
    - OpenAI with no API key → `missing API key for openai: set OPENAI_API_KEY`
    - Unknown provider → `unknown provider: <name>`
    - Unknown param modifier → `unknown param modifier: :<name>`

### Non-Functional Requirements

- REQ-NF-001: **Module refactor is part of this feature.** Extract embed code and param parsing into:
    - `src/embed/mod.rs` — `EmbedProvider` trait, `EmbedError`, `EmbedCliArgs`, `EmbedConfig`, factory.
    - `src/embed/openai.rs`
    - `src/embed/ollama.rs`
    - `src/params.rs` — `ParamSpec` enum, `parse_param` with `:modifier` support, and migrate the existing `parse_param_value` / `parse_params` logic here.
    - `src/commands/embed.rs` — `EmbedCmd` (clap `Args`) and its `run()` handler.
    - `src/main.rs` keeps: clap root, `load_env`, query execution, retries, `run_schema`, `run_schema_mode`, `run_query_mode`, `main`.
- REQ-NF-002: Add deps to `Cargo.toml`:
    - `thiserror = "1"` for `EmbedError`.
    - `async-trait = "0.1"` for the `EmbedProvider` trait.
    - `reqwest` (already present) — confirm `json` + `rustls-tls` features cover embed calls (they do).
    - No new dev-deps (no `wiremock`).
- REQ-NF-003: Provider implementations are `Send + Sync` and callable from the tokio current-thread runtime used in `main()`.
- REQ-NF-004: `neo4j-query` with no `:embed` usage must have no observable startup regression — lazy init verified by not instantiating any `reqwest::Client` for embeddings on the non-embed path.

### Testing Requirements

- REQ-T-001: Unit tests (in `tests/unit.rs`, following existing re-implementation pattern) for:
    - `parse_param`: literal (no modifier), `:embed` modifier, unknown modifier error, value-containing-equals, value-containing-colon-in-value (modifier split must only consume the first `:` in the KEY part), empty value.
    - Ensure existing type-coercion tests (`parse_param_value_*`) still pass for the un-modified path.
    - `EmbedConfig::from_sources`: returns `Ok(None)` when no provider configured; returns `Err` when provider set but model missing; precedence (CLI > env); API-key fallback (`OPENAI_API_KEY` present, `NEO4J_EMBED_API_KEY` fallback, Ollama ignores key).
    - Error message strings match REQ-F-011 exactly (snapshot-style `assert_eq!` on `format!("{err}")`).
- REQ-T-002: Integration tests using **real Ollama in a Docker container** (no `wiremock`):
    - Add an `ollama` service to `tests/docker-compose.yml` (pinned image tag).
    - Pin the embed model to **`all-minilm`** (the GGUF build of `sentence-transformers/all-MiniLM-L6-v2`; 22M params, 384 dims, ~45MB). Small enough to pull on every CI run.
    - Provide a setup step (script or make target) that runs `ollama pull all-minilm` inside the container. CI calls it after `docker compose up` and before `cargo test -- --ignored`. No Docker-volume caching in v1 — revisit only if we later move to a larger model.
    - New `#[ignore]` integration tests (gated by an `ollama_available()` helper paralleling `neo4j_available()`):
        - `neo4j-query embed 'hello'` → prints a JSON array of floats.
        - `echo 'hello' | neo4j-query embed` → same.
        - `neo4j-query embed --format raw 'hello'` → newline-separated floats, `wc -l` == dimension count.
        - `neo4j-query -P v:embed='hello' 'RETURN $v AS vec'` → vector round-trips through Neo4j and appears in output.
        - `-P x=42 -P v:embed='hello' 'RETURN $x, $v'` → literal param still integer-typed.
    - Non-`#[ignore]` integration tests (no Ollama needed) verify error paths: missing provider, unknown modifier, OpenAI without key, unknown provider name. Each asserts stderr contains the exact error string from REQ-F-011.
- REQ-T-003: CI workflow (`.github/workflows/release.yml` or equivalent test workflow) must start the Ollama container, warm it with a model pull, then run the ignored integration tests.

### Documentation Requirements

- REQ-D-001: Update `README.md`:
    - Add "Embeddings" section covering the `:embed` modifier, the `embed` subcommand, and both provider setups (OpenAI + Ollama).
    - Add the new env vars / flags to the Configuration table.
    - Include a Neo4j vector-search example mirroring the spec's motivating snippet (`CALL db.index.vector.queryNodes(...)`).
- REQ-D-002: Update BOTH skill files in lock-step (per AGENTS.md sync rule):
    - `skills/neo4j-query/SKILL.md` (compile-time embedded via `include_str!`)
    - `.claude/skills/neo4j-query/SKILL.md` (dev copy)
    - Add a "Vector search / embeddings" section explaining when to use `-P name:embed=...`, note that the feature is opt-in (requires provider configured), and show an Ollama example.
- REQ-D-003: `neo4j-query --help`, `neo4j-query embed --help`, and error messages (via clap derive + `thiserror`) must surface the new flags and modifier clearly. No manual help-text edits expected.
- REQ-D-004: Bump `version` in `Cargo.toml` to `0.11.0` as part of the release PR.

## Technical Considerations

- **Param parser rewrite**: Current `parse_param_value` does type coercion on the value; the new `parse_param` in `src/params.rs` must preserve that coercion for the `Literal` arm. Recommended signature: `pub fn parse_param(raw: &str) -> Result<(String, ParamSpec), String>`, where `ParamSpec::Literal(serde_json::Value)` (already-coerced) or `ParamSpec::Embed(String)` (raw text). This keeps the spec's enum shape but preserves existing behavior exactly.
- **Resolution path**: Introduce `resolve_params(specs, embed_args) -> Result<Map<String, Value>, Box<dyn Error>>` in `src/main.rs` (or `src/params.rs`). Called from `run_query_mode` in place of the current `parse_params(&qa.p)?`. Lazy: builds provider only if any `ParamSpec::Embed` exists.
- **Clap wiring**: `EmbedCliArgs` is a `#[derive(Args)]` struct; `#[command(flatten)]` it onto both `QueryArgs` and the new `EmbedCmd`. Because flags are global-ish in behavior but not `global = true` (each flattening is local), we accept that `--embed-provider` only parses in positions clap expects. This matches the spec.
- **Env file timing**: `load_env()` runs before clap parses args, so `NEO4J_EMBED_*` are visible when clap applies its `env = "..."` defaults — no additional pre-scan needed.
- **Error type bridging**: `EmbedError` → `Box<dyn std::error::Error>` via `thiserror`'s `#[error]` impl; surfaces cleanly through `run()`.
- **Ollama in CI**: docker-compose service pins a specific Ollama image tag for reproducibility. Model is `all-minilm` (~45MB), pulled at container start — fast enough that caching/pre-baking isn't justified in v1.
- **Module refactor risk**: Single-file `main.rs` (617 lines) → multi-module split touches every function. Do the mechanical split in one commit with no behavioral change; layer embed features on top. Existing `tests/unit.rs` re-implements pure helpers and should keep passing verbatim.

## Acceptance Criteria

- [ ] `src/main.rs` split into the module layout in REQ-NF-001 with no behavioral regressions; all existing tests pass unchanged.
- [ ] `-P age=30` still produces an integer param; `-P active=true` still produces a bool; `-P v:embed='hello'` produces a vector.
- [ ] `-P v:embed='text' 'RETURN $v'` against local Ollama returns the embedding vector in TOON and JSON output.
- [ ] `neo4j-query embed 'hello'` outputs a single-line JSON array on stdout.
- [ ] `neo4j-query embed --format raw 'hello'` outputs one float per line; `wc -l` equals model's dimension count.
- [ ] `echo 'hello' | neo4j-query embed` works (stdin).
- [ ] Missing-provider error fires exactly `embedding provider not configured: set NEO4J_EMBED_PROVIDER` when `:embed` is used without config.
- [ ] OpenAI-with-no-key fires `missing API key for openai: set OPENAI_API_KEY`.
- [ ] Unknown modifier `-P v:foo=x` fires `unknown param modifier: :foo`.
- [ ] Ollama unreachable → error mentions `ollama serve`; Ollama 404 → error mentions `ollama pull <model>`.
- [ ] `cargo test` passes (unit tests cover parser, config resolution, error strings).
- [ ] `cargo test -- --ignored` passes against the Ollama Docker container.
- [ ] `README.md` has an Embeddings section + updated Configuration table.
- [ ] Both `SKILL.md` copies updated and identical.
- [ ] `Cargo.toml` version = `0.11.0`.
- [ ] `neo4j-query --help` shows `--embed-provider`, `--embed-model`, `--embed-dimensions`, `--embed-base-url`.
- [ ] `neo4j-query embed --help` shows the same embed flags plus `--format`.
- [ ] No new runtime deps beyond `thiserror` and `async-trait`; no new dev-deps.

## Out of Scope

- Additional providers (Voyage, Cohere, Bedrock).
- Batching `:embed` params into one provider request.
- Local in-process embedding (e.g. `fastembed-rs` behind a cargo feature).
- Document-vs-query embedding input-type hints.
- Result caching.
- Persisting embeddings back to Neo4j from the CLI.
- `wiremock` or other HTTP-mocked tests — all provider integration tests use real Ollama via Docker.

## Open Questions

None.
