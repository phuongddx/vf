# vf HyperFrames Engine — Design Spec (Sub-project B3, Delegate Mode)

- **Date:** 2026-10-31
- **Status:** draft (awaiting owner review)
- **Repo state at write time:** main = fc53ac8, v0.3.0 released (82/82 tests), full-LLM Remotion demo live in README

## Problem

vf ships exactly one real engine (Remotion). Spec v0.4 locks two engines shipped
all-in-one: Remotion (default) + HyperFrames. HyperFrames is not integrated at all.

## Goals

1. `VF_ENGINE=hyperframes vf make <idea>` produces a real MP4 through the
   HyperFrames CLI — end-to-end, same UX as the Remotion path.
2. Delegate mode: the LLM writes the HyperFrames composition HTML directly
   (vf does NOT translate its IR spec to HTML); vf orchestrates compose →
   lint → render.
3. Invariants preserved: make-never-dies, offline fallback, bilingual warnings,
   usage/cost reporting, manual-command hints.
4. Demo: CI renders a HyperFrames demo video → `docs/demo-hyperframes.mp4`
   embedded in the README next to the Remotion demo.

## Decisions locked (owner, 2026-10-31)

| # | Decision | Choice |
|---|----------|--------|
| Q1 | Integration depth | **(b) Delegate** — vf orchestrates; LLM writes HTML per HyperFrames' contract; vf adds no translator |
| D2 | IR role | `video.spec.json` skipped entirely on this path — the HTML composition IS the artifact |
| D3 | Contract source | Vendored distilled contract inside vf (pinned to docs researched 2026-10-31); no runtime skill downloads |
| D4 | Default engine | Remotion stays default; HyperFrames is opt-in via `VF_ENGINE=hyperframes` |

## Research facts (docs 2026-10-31, hyperframes.heygen.com)

- Project = folder of HTML: `index.html` (main composition), `hyperframes.json`
  (settings), `compositions/` (scenes), `assets/`, `renders/`; optional
  BRIEF/STORYBOARD/SCRIPT docs. All tools edit the same files.
- CLI (`npx hyperframes <cmd>`, project dir = first arg default cwd):
  - `init <name>` — scaffold a project
  - `lint` — HTML structure validation (exit code signals findings; same rule
    engine as the render-time render-gate, from `@hyperframes/lint`)
  - `check` — deeper browser/layout/motion/contrast checks
  - `render --output <file>` — render MP4 (Puppeteer headless Chrome + FFmpeg)
  - `doctor`, `timeline --json`, `snapshot`, etc. (not needed by vf v1)
- Most commands accept `--json`; agent-harness integration exists
  (`HF_TOOL_CONTEXT`) — out of scope here.
- Rendering needs headless Chrome → gateway still cannot render; real renders
  happen on CI runners (same as Remotion path).

## Design

### 1. Compose stage (make.rs, runs before the stage loop when engine is hyperframes)

- Idempotent: if `index.html` exists in the project dir → skip.
- Offline (no key / no TLS): write the **vendored template composition**
  (`index.html` + `hyperframes.json`, minimal but valid — always renders) +
  `compose_offline` warning.
- Online: one LLM call (existing provider → RetryingClient machinery):
  - system prompt = vendored HyperFrames contract: project structure
    (index.html + hyperframes.json + compositions/ optional), timing via data
    attributes, seekable animations (CSS/GSAP), 9:16 default sizing, no
    external network assets beyond relative `assets/` paths.
  - user prompt = the idea + duration hint + feedback on retry.
  - Response → `extract_html` (strip code fences, require `<html`) → write
    `index.html` (+ minimal `hyperframes.json` if absent).
- `ComposeOutcome { Offline, Composed { usage }, Fallback { reason } }` —
  usage feeds the same combined usage line (codegen column reused; the line
  stays "spec/codegen" for remotion and becomes "compose" text for
  hyperframes via i18n).

### 2. Validate (lint gate)

- `npx hyperframes lint` via Executor in the project dir (exit 0 = pass).
- On failure → 1 retry with lint output attached → still failing → replace
  with vendored template + `compose_fallback` warning.
- CLI unavailable (no npx) → light check (`<html` present) + warning that
  lint was skipped.

### 3. Render (HyperframesEngine implements Engine)

- Toolchain pre-check: node + npx (reuse `tool_available`) — missing →
  render_skipped_manual + break (same as Remotion).
- Command: `npx hyperframes render --output <ABSOLUTE>/draft.mp4` with
  cwd = project dir (**absolute output** — the B2 nested-path lesson applies).
- `Engine::render(spec, …)` ignores the spec content (delegate mode); the
  trait signature stays unchanged for the remotion/dry-run engines.
- First run downloads the CLI + engine deps via npx (runner disk is fine;
  gateway cannot run this — CI only, as with Remotion).
- Post stage unchanged (ffmpeg remux or copy fallback).

### 4. Wiring (make.rs)

- Read `VF_ENGINE` once in `run()`; branch:
  - `hyperframes` → compose stage (idempotent) → stage loop → render via
    HyperframesEngine.
  - default → existing Remotion path untouched.
- Stage loop unchanged (style stage writes style.json — harmless for this
  path; spec/codegen blocks skip because video.spec.json is not written).

### 5. i18n additions

`compose_offline`, `compose_written`, `compose_fallback(reason)`,
`render_hf_manual(cmd)` (manual hint names the hyperframes command) — VI/EN.

### 6. Demo workflow (demo.yml)

- New job step after the Remotion demo: `VF_ENGINE=hyperframes vf make
  "<demo idea>"` → copies draft.mp4 → `docs/demo-hyperframes.mp4` → same bot
  commit. README gains a second `<video>` under "## Demo" with a caption
  naming the engine.

## Error handling summary

make never dies: compose failure → vendored template; lint failure → retry →
template; render failure → manual command + break; every degradation is a
bilingual warning; artifacts preserved.

## Testing

- Unit: extract_html fences/prose; vendored template files exist + contain
  required markers; compose prompt contract; ComposeOutcome wiring; render
  command shape via MockExecutor (program `npx`, args contain `render`,
  `--output` ABSOLUTE).
- Integration (wiremock LLM + Executor mocks): happy path writes index.html +
  lint pass (mocked exit 0) + render command executed; lint fail ×2 →
  template fallback + warning; offline → template + warning.
- Real Chrome renders only in the demo workflow (CI), never in cargo tests.

## Non-goals

`hyperframes check`, `snapshot`, `timeline` editing, Studio, cloud/lambda
render, TTS/media commands, SDK/Player embedding, `HF_TOOL_CONTEXT` harness
contract. All candidates for later phases.

## Files touched (estimate)

`src/engine/mod.rs` (+hyperframes module), `src/engine/hyperframes.rs` (new),
`src/engine/hf_template.rs` (vendored index.html + hyperframes.json consts),
`src/cmd/make.rs` (compose stage + wiring), `src/i18n.rs`, `tests/cli.rs`,
`.github/workflows/demo.yml`, `README.md`, `docs/demo-hyperframes.mp4` (CI artifact).

## References

- hyperframes.heygen.com/developers/cli.md (command table)
- hyperframes.heygen.com/packages/cli.md (full CLI reference)
- hyperframes.heygen.com/packages/lint.md (lint library + render-gate)
- hyperframes.heygen.com/concepts/index.md (project structure)
- github.com/heygen-com/hyperframes (Apache-2.0)
