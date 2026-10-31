# vf Network Layer — Design Spec (Sub-project B)

- **Date:** 2026-10-31
- **Status:** draft (awaiting owner review)
- **Supersedes:** none (builds on Plan 1–3 foundations)
- **Repo state at write time:** main = 9d54134, v0.1.0 released, 47/47 tests green

## Problem

vf ships an LLM layer (`src/llm/`) that is library-only: adapters are unit-tested
against wiremock over localhost HTTP, never called by `vf make`, and the binary
links `reqwest` without TLS (Plan 1 dropped `rustls-tls` because the dev
container has no C compiler). Users cannot get real model output. The default
provider story targets Z.ai (GLM) for the Vietnamese market, which requires
HTTPS and dedicated Coding Plan endpoints.

## Goals

1. Real HTTPS calls to Z.ai (default) and any OpenAI/Anthropic-compatible
   provider, wired into `vf make`'s spec stage.
2. Graceful degradation: `vf make` works with zero configuration (template +
   dry-run, as today) and upgrades to real LLM output when a key is present.
3. Container dev path stays C-compiler-free.

## Decisions locked (owner, 2026-10-31)

| # | Decision | Choice |
|---|----------|--------|
| Q1 | Default provider profile | `glm-coding` — Anthropic protocol, `https://api.z.ai/api/anthropic`, model `glm-5.2`, key env `ZAI_CODING_API_KEY`. Secondary built-in: `glm` — OpenAI protocol, `https://api.z.ai/api/paas/v4`, key env `ZAI_API_KEY`. |
| Q2 | UX without key | Graceful degradation: offline mode (template spec + dry-run) plus a bilingual warning; never hard-fail. |
| D3 | TLS cargo feature | Default-ON feature `tls` (reqwest `rustls-tls`); container dev and CI's second matrix leg use `--no-default-features`. |
| D4 | LLM transport | One-shot JSON responses (no streaming). Streaming is a later enhancement. |
| D5 | Scope | Spec stage only. Real codegen (Remotion component generation) is B2. |

## Context: Z.ai endpoints (research 2026-10-31, docs.z.ai)

- Standard pay-as-you-go API: `https://api.z.ai/api/paas/v4` (OpenAI-compatible).
- GLM Coding Plan (subscription, separate keys since 2026-10-16):
  - Anthropic Messages: `https://api.z.ai/api/anthropic` (Claude Code / Goose style)
  - OpenAI Chat Completions: `https://api.z.ai/api/coding/paas/v4`
  - OpenAI Responses: `https://api.z.ai/api/v1` (Codex)
- Policy: Coding Plan is "strictly limited to officially supported tools"; an
  "Other Tools" path and a general-purpose agent list (OpenClaw included) are
  served best-effort. vf users bring their own key; README must carry a note.

Adapter fit (already implemented): OpenAI adapter appends `/chat/completions`
to `base_url` (base includes version path) — matches `paas/v4` and
`coding/paas/v4`. Anthropic adapter appends `/v1/messages` to the host root —
matches `api.z.ai/api/anthropic` (ANTHROPIC_BASE_URL pattern).

## Design

### 1. TLS & build matrix

- `Cargo.toml`: `reqwest = { ..., features = ["json"] }`; new optional feature
  `tls = ["reqwest/rustls-tls"]`, included in `default`.
- Runtime capability probe: `#[cfg(feature = "tls")]` sets a `TLS_ENABLED`
  const; errors and doctor read it.
- CI (`ci.yml`) gains a matrix: `features: ["default", "no-default-features"]`
  running `cargo test --locked` per leg. Release workflow unchanged (default
  features → TLS in shipped binaries).
- Local container dev: `cargo.sh test --no-default-features` documented in
  toolchain notes.

### 2. Built-in provider profiles

- `src/config.rs`: `Config::builtin()` returns providers `glm-coding` and
  `glm` (values per Q1). `Config::load()` keeps reading the TOML file; the
  registry merges file providers over built-ins (file wins per key).
- `routing.spec`/`codegen` default to `"glm-coding"` when unset.
- `vf config` prints the effective merged providers (key envs, base URLs,
  protocol), routing targets, and short setup hints (which env var to export).
- Keys are read from `api_key_env` only; never stored.

### 3. Pipeline wiring — spec stage

`src/cmd/make.rs`:

1. Resolve provider: `routing.spec` → merged provider; resolve key from env.
2. No key or no TLS-capable build with an https base URL → offline mode:
   current template-based spec + i18n warning line ("chạy offline, chưa dùng
   LLM" / "running offline, LLM not used").
3. Key present → build prompt (idea + template shape + style preset), call
   `LlmClient::chat`, ask for a `video.spec.json` document.
4. Validate with the existing IR validator (`src/ir.rs`).
5. Invalid → one retry attaching the validation error to the prompt.
6. Still invalid or call fails after retries → template fallback + warning;
   `make` exits 0 with the produced artifacts either way.

Usage report: after a real call, print input/output token counts and an
estimated cost (per-model price table for `glm-5.2`/`glm-4.7` classes; unknown
models report tokens only). `--plan` keeps its pre-flight estimate behavior and
now also prints which provider/mode would be used.

### 4. Timeout, retry, error mapping

- Per-request timeout: 30 s (connect+total).
- Retries: 2 extra attempts on HTTP 429/5xx/network errors; backoff 1 s then
  3 s, plus ±20% jitter.
- Error map (bilingual via `src/i18n.rs`):
  - 401/403 → invalid key (name the env var)
  - 429 → rate limited (mention retries)
  - 5xx → upstream error
  - https URL on a no-TLS build → "binary built without TLS; use a release
    binary or rebuild with `--features tls`"

### 5. Doctor

`vf doctor` adds two checks:
- Network: HEAD/GET to each configured provider's base URL; report
  reachable / unreachable with latency.
- TLS: report whether this binary supports HTTPS (`TLS_ENABLED`).

### 6. Testing

- Unit (wiremock, HTTP, runs without `tls` feature): retry/backoff logic,
  error mapping, profile merge, key resolution, spec-stage fallback chain
  (no key → offline; invalid JSON → retry → fallback).
- Integration: `make` with a temp-dir config pointing `glm-coding` at a local
  wiremock Anthropic-protocol mock; asserts LLM path artifacts and usage line.
  A second test runs offline-mode make (no env) asserting the warning.
- CI matrix proves both feature combos compile and pass.

## Error handling summary

Never crash the pipeline: LLM failures degrade to template spec with warnings;
doctor and config explain configuration problems by name (env var, base URL).

## Non-goals (explicit)

- Real codegen stage (B2), TTS, runtime downloads, premium-tier enforcement,
  streaming, bigmodel.cn endpoints (Z.ai international only for now).

## Files touched (estimate)

`Cargo.toml`, `.github/workflows/ci.yml`, `src/config.rs`, `src/llm/mod.rs`
(retry/timeout + usage types), `src/llm/registry.rs` (merge + builtin),
`src/cmd/make.rs`, `src/cmd/doctor.rs`, `src/cmd/config_cmd.rs`, `src/i18n.rs`
(new strings), `tests/cli.rs`, `README.md` (profiles + Coding Plan note).

## References

- docs.z.ai/devpack/quick-start (Coding Plan endpoints, key separation)
- docs.z.ai/devpack/tool/others (endpoint table, policy on other tools)
- docs.z.ai/devpack/notice/api-key-management (key split since 2026-10-16)
- docs.z.ai llms-full.txt (standard `api/paas/v4` chat/completions usage)
