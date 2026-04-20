# PRD: HuggingFace Embed Provider

## Overview

Add a third `EmbedProvider` implementation (`huggingface`) alongside the existing `openai` and `ollama` providers. Lets users embed text via HuggingFace's serverless Inference API (default) or a user-deployed dedicated Inference Endpoint (via `--embed-base-url` override). Originally requested for `clip-ViT-B-32` — that exact model is NOT serverless on HF, so we document two workarounds: switch to `clip-ViT-B-32-multilingual-v1` (deployed, 512-dim) or deploy a dedicated endpoint.

## Goals

- Unblock HuggingFace-hosted embedding models from the `:embed` param resolution path and the `embed` subcommand.
- Reuse existing scaffolding (`EmbedProvider` trait, `EmbedConfig`, `EmbedError`, `EmbedCliArgs`, `resolve_api_key`) — no architectural changes.
- Honest, model-specific 404 error messages that point users at the clip-ViT-B-32 workaround.
- Lazy init preserved: zero overhead unless a `:embed` param or `embed` subcommand is invoked.

## Non-Goals

- Image embedding via CLIP (would need `:embed_image` modifier + base64/multipart input handling — separate feature).
- Batching multiple `:embed` params per request (already deferred per the original embedding-support PRD).
- Other HF Inference Providers (Together, Fireworks, SambaNova, Scaleway, etc.) — same router URL but different per-provider quirks; default `hf-inference` covers the originally-requested use case.
- OpenAI-compatible reuse: HF Inference Providers exposes `/v1/chat/completions` as OpenAI-compatible but **not** `/v1/embeddings`. Verified via HF docs ("This OpenAI-compatible endpoint is currently available for chat completion tasks only"). We cannot reuse the OpenAI provider with a different base URL.
- Cargo.toml version bump (versioning is handled by the separate GitHub release workflow).

## Requirements

### Functional Requirements

- REQ-F-001: Add `--embed-provider huggingface` as a recognised value (env `NEO4J_EMBED_PROVIDER=huggingface`). Reuses existing `EmbedCliArgs`; no new flags.
- REQ-F-002: API key required. Resolution order in `resolve_api_key("huggingface")`: `HF_TOKEN` (HF convention) → `NEO4J_EMBED_API_KEY` fallback. Missing key → `EmbedError::MissingApiKey { provider: "huggingface", env_var: "HF_TOKEN" }` returning the exact string `missing API key for huggingface: set HF_TOKEN`.
- REQ-F-003: Default base URL `https://router.huggingface.co/hf-inference/models`. When the user does NOT override `--embed-base-url` (i.e. `EmbedConfig.base_url` is `None`), POST to `{default_base}/{model}/pipeline/feature-extraction`.
- REQ-F-004: When `--embed-base-url` IS overridden (treat as a dedicated HF Inference Endpoint URL), POST directly to that URL — do NOT append `/{model}/pipeline/feature-extraction`. The `model` field is sent in the body for serverless and ignored by dedicated endpoints (which are model-locked).
- REQ-F-005: Request body: `{"inputs": "<text>"}`. Headers: `Authorization: Bearer <token>`, `Content-Type: application/json`.
- REQ-F-006: Response parser must accept BOTH:
    - `[[f32, f32, ...]]` (nested — TEI / sentence-transformers default)
    - `[f32, f32, ...]` (flat — some deployments)
  Return the first vector. Use `serde_json::Value` and branch on shape.
- REQ-F-007: `--embed-dimensions` is accepted at the CLI but silently ignored — HF feature-extraction endpoints don't support variable-dim output in the request.
- REQ-F-008: Error mapping (exact strings asserted by tests):

    | Situation | Message |
    | --- | --- |
    | Missing API key | `missing API key for huggingface: set HF_TOKEN` |
    | HTTP 401 / 403 | `huggingface auth failed: check HF_TOKEN scopes (needs Inference Providers permission)` |
    | HTTP 404 | `huggingface model '<m>' not found or not deployed by any Inference Provider. Try '<m>-multilingual-v1' or deploy a dedicated endpoint and set NEO4J_EMBED_BASE_URL` |
    | Other non-2xx | `huggingface <status>: <body>` |
    | Connection refused / timeout | `huggingface unreachable at <url>: <err>` |
    | Empty response | `huggingface error: empty response` |

  All produced via the existing `EmbedError::ProviderError { provider: "huggingface", message: ... }` and `EmbedError::MissingApiKey` variants. **No new EmbedError variants are needed.**
- REQ-F-009: `EmbedConfig::build` (`src/embed/mod.rs:146-159`) gains a `"huggingface" => Ok(Box::new(huggingface::HuggingFace::new(self.api_key, self.model, self.base_url)?))` arm.
- REQ-F-010: `resolve_api_key` (`src/embed/mod.rs:168-183`) gains a `"huggingface" => HF_TOKEN.or_else(NEO4J_EMBED_API_KEY)` arm (filtering out empty strings, matching the existing `openai` arm pattern).

### Non-Functional Requirements

- REQ-NF-001: New module `src/embed/huggingface.rs` follows the structural template of `src/embed/ollama.rs` (similar URL-construction needs). Bearer-auth pattern lifted from `src/embed/openai.rs:50` (`reqwest::RequestBuilder::bearer_auth`).
- REQ-NF-002: `HuggingFace` struct holds `client: reqwest::Client`, `api_key: String`, `model: String`, `base_url: Option<String>` (so the dedicated-vs-serverless decision is preserved at request time).
- REQ-NF-003: No new runtime dependencies. No new dev-dependencies (no `wiremock`, per the established pattern).
- REQ-NF-004: `Send + Sync` on the provider; async via the existing `async-trait`.
- REQ-NF-005: Lazy init preserved: nothing in `EmbedConfig::from_sources` or `resolve_api_key` should perform HTTP I/O.

### Testing Requirements

- REQ-T-001: Unit tests in `tests/unit.rs` (re-implementation pattern per AGENTS.md) for `resolve_api_key("huggingface")`:
    - With `HF_TOKEN=tok` → returns `Some("tok")`.
    - With only `NEO4J_EMBED_API_KEY=fallback` → returns `Some("fallback")`.
    - With neither set → returns `None`.
    - With `HF_TOKEN=""` (empty) and `NEO4J_EMBED_API_KEY=fallback` → returns `Some("fallback")` (empty filter).
- REQ-T-002: Non-`#[ignore]` integration test `huggingface_missing_api_key_errors`: extends `embed_env_clean()` (`tests/integration.rs:924`) to also `env_remove("HF_TOKEN")`. Asserts stderr contains the exact `missing API key for huggingface: set HF_TOKEN` string.
- REQ-T-003: Non-`#[ignore]` integration test `huggingface_cli_flags_before_subcommand`: regression for the global-args pattern (per AGENTS.md "CLI Architecture" rule). Passes `--embed-provider huggingface --embed-model anything embed "x"` BEFORE the subcommand AND verifies the missing-API-key error fires (proves the flag was parsed, not silently dropped).
- REQ-T-004: Non-`#[ignore]` integration test `huggingface_unknown_provider_typo` already covered by the existing `embed_unknown_provider_errors` test — no change needed; just confirm "huggingface" doesn't accidentally become an alias.
- REQ-T-005: Optional `#[ignore]` integration test `huggingface_serverless_real_call` gated on `HF_TEST_TOKEN` env. When set: `--embed-provider huggingface --embed-model sentence-transformers/clip-ViT-B-32-multilingual-v1 embed "hello"` succeeds and asserts `vector.len() == 512`. Skips with `eprintln!` when `HF_TEST_TOKEN` absent.
- REQ-T-006: All existing tests stay green (`cargo test`, `cargo test -- --ignored`).

### Documentation Requirements

- REQ-D-001: `README.md` Embeddings section: add `huggingface` to the provider list, add an `.env` example, document the **clip-ViT-B-32 caveat** with both workarounds (multilingual variant OR dedicated endpoint).
- REQ-D-002: `README.md` Configuration table: add `HF_TOKEN` row with `—` for flag and `*(required for huggingface)*` for default.
- REQ-D-003: BOTH `skills/neo4j-query/SKILL.md` AND `.claude/skills/neo4j-query/SKILL.md` updated in lock-step (per AGENTS.md sync rule). Add `huggingface` to the provider list in the embeddings section. `diff` between the two files must produce no output.
- REQ-D-004: Help output (`neo4j-query --help`, `neo4j-query embed --help`) — no manual edits needed since `EmbedCliArgs` is unchanged. Verify after implementation that `huggingface` is documented in the `--embed-provider` description; if not, update the doc-comment.

## Technical Considerations

- **Endpoint structure decision**: We default to `https://router.huggingface.co/hf-inference/models` (the new HF router) instead of the legacy `https://api-inference.huggingface.co/models`. Both work; router is the documented forward-looking path.
- **Dedicated vs serverless URL handling**: Detected solely by whether `EmbedConfig.base_url` is `Some(_)`. This is a heuristic, but it matches the user's mental model: "if I overrode the URL, I gave you the full endpoint". The docs explicitly call this out.
- **Reuse vs. new variants**: `EmbedError` already covers everything we need. Adding HF-specific variants would bloat the enum; `ProviderError { provider: "huggingface", message }` per the existing pattern (used by both Ollama and OpenAI) is enough.
- **Why not OpenAI provider with HF router base URL**: Verified that HF's `/v1/embeddings` does not exist. The router exposes OpenAI-compat for chat only. Forcing reuse would break.
- **Empty-token edge case**: `HF_TOKEN=""` is treated as unset, matching the existing `openai` arm (which filters out empty strings before falling through). Important because dotenv files often have stub `HF_TOKEN=` lines.
- **Future image embedding**: Out of scope for this PRD. When added, would likely take a `:embed_image` modifier with a path or base64 value — request shape becomes `{"inputs": <base64>}` for CLIP image endpoints.
- **Error message ergonomics**: The 404 message intentionally names the multilingual variant by suffix (`'<m>-multilingual-v1'`) and the dedicated-endpoint workaround. This trades verbosity for guidance — same philosophy as the Ollama provider's `ollama pull <m>` hint.

## Acceptance Criteria

- [ ] `src/embed/huggingface.rs` created and registered in `src/embed/mod.rs`.
- [ ] `EmbedConfig::build` routes `"huggingface"` to the new provider.
- [ ] `resolve_api_key("huggingface")` resolves `HF_TOKEN` first, `NEO4J_EMBED_API_KEY` second, both empty-filtered.
- [ ] `--embed-provider huggingface` (with `--embed-model X` and `HF_TOKEN=...` set) succeeds against `clip-ViT-B-32-multilingual-v1` serverless and returns a 512-element vector.
- [ ] `--embed-base-url <dedicated-url>` POSTs directly to that URL without appending the model path.
- [ ] Missing-key error: exact string `missing API key for huggingface: set HF_TOKEN`.
- [ ] 401/403 error contains `huggingface auth failed: check HF_TOKEN scopes`.
- [ ] 404 error contains both `not deployed by any Inference Provider` and `'<model>-multilingual-v1'`.
- [ ] `--embed-dimensions 256` accepted but silently ignored (no error, no behavior change).
- [ ] `cargo test` passes (unit + non-ignored integration).
- [ ] `cargo test -- --ignored` passes (existing Neo4j+Ollama tests still green; new HF live test skips when `HF_TEST_TOKEN` unset).
- [ ] `cargo clippy --all-targets` and `cargo fmt --check` clean.
- [ ] `README.md` documents the provider, the `.env` shape, and the clip-ViT-B-32 caveat.
- [ ] Both `SKILL.md` copies updated; `diff skills/neo4j-query/SKILL.md .claude/skills/neo4j-query/SKILL.md` produces no output.
- [ ] `Cargo.toml` version field UNCHANGED (release workflow handles this separately).
- [ ] `neo4j-query --help` shows the `--embed-*` flags as before; flag values list mentions `huggingface` (or doc-comment updated).

## Out of Scope

- Image embedding with CLIP (`:embed_image` modifier).
- Other HF Inference Providers (Together, Fireworks, SambaNova, Scaleway, etc.) selected via the `provider=` query param.
- Batching multiple `:embed` params in a single request.
- OpenAI-compatible reuse (HF doesn't expose `/v1/embeddings`).
- Pinning a specific HF model in CI (live HF test is opt-in via `HF_TEST_TOKEN`, not a default CI dependency — the existing CI already runs Ollama for local-model coverage).
- Cargo.toml version bump (handled by GitHub release workflow).

## Open Questions

None.
