# vf Network Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use text2prod:subagent-driven-development (recommended) or text2prod:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Real LLM-backed spec generation for `vf make` over HTTPS (Z.ai default) with graceful offline fallback.

**Architecture:** Cargo feature `tls` (default-on) restores HTTPS; built-in Z.ai provider profiles merge over file config; a retry/timeout wrapper sits around the existing OpenAI/Anthropic adapters; `make`'s spec stage gains an LLM → validate → retry-with-feedback → template-fallback chain; doctor gains TLS and network checks.

**Tech Stack:** Rust 1.85 / edition 2024, reqwest 0.12 (json + optional rustls-tls), tokio, wiremock 0.6, assert_cmd 2, GitHub Actions.

**Spec:** `docs/specs/2026-10-31-network-layer-design.md` (same repo)

## Global Constraints

- Local test command is `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked` run with workdir `vf/` — the dev container has NO C compiler, so **no test may require the `tls` feature**. All feature-conditional tests use `#[cfg(...)]`.
- `reqwest` stays `default-features = false, features = ["json"]`; the new `tls` feature adds exactly `reqwest/rustls-tls`.
- Z.ai values (verbatim): anthropic base `https://api.z.ai/api/anthropic`, openai base `https://api.z.ai/api/paas/v4`, key envs `ZAI_CODING_API_KEY` / `ZAI_API_KEY`, default model `glm-5.2`.
- API keys exist only via `api_key_env` env indirection — never stored, never logged.
- Timeouts/retries: 30 s per request; 2 retries on HTTP 429/5xx/network errors; delays 1 s then 3 s, each ±200 ms jitter.
- All user-facing strings go through `src/i18n.rs` (`Lang::from_env()`, `VF_LANG=vi` → Vietnamese).
- `vf make` NEVER fails because of LLM problems — degrade to template spec + warning, exit 0.
- Edition 2024: `std::env::set_var`/`remove_var` are unsafe — do NOT use them in unit tests. Use never-set unique var names (`VF_TEST_*`) or subprocess env via `assert_cmd`'s `.env()`.
- One commit per task, conventional-commit style, message given in each task.
- Endpoints in tests are wiremock HTTP only (`MockServer::start()`), never real network.

---

### Task 1: TLS cargo feature + CI feature matrix

**Files:**
- Modify: `Cargo.toml`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: none
- Produces: cargo feature `tls` (default-on) — later tasks gate code with `#[cfg(feature = "tls")]`

- [ ] **Step 1: Add the feature to Cargo.toml**

Replace the reqwest dependency block's trailing comment area and add a `[features]` table (keep all other deps unchanged):

```toml
reqwest = { version = "0.12", default-features = false, features = ["json"] }
anyhow = "1"
dirs = "5"

[features]
default = ["tls"]
tls = ["reqwest/rustls-tls"]
```

(Delete the old `# SDD ruling: rustls-tls dropped for Plan 1...` comment block above reqwest — it is superseded.)

- [ ] **Step 2: Verify the container path still compiles and passes**

Run (workdir `vf/`): `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked`
Expected: PASS — 47 tests, zero errors. (The `tls` feature is NOT built here; rustls never compiles locally.)

- [ ] **Step 3: Add the CI matrix**

In `.github/workflows/ci.yml`, replace the `jobs:` block so tests run under both feature combos:

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        flags: ["--locked", "--no-default-features --locked"]
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2

      - name: Format check (advisory)
        run: cargo fmt --all -- --check
        continue-on-error: true

      - name: Clippy
        run: cargo clippy --all-targets ${{ matrix.flags }} -- -A dead_code

      - name: Tests
        run: cargo test --all ${{ matrix.flags }}

      - name: Static musl build
        run: |
          sudo apt-get update && sudo apt-get install -y musl-tools
          rustup target add x86_64-unknown-linux-musl
          cargo build --release ${{ matrix.flags }} --target x86_64-unknown-linux-musl
```

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock .github/workflows/ci.yml
git commit -m "feat: tls cargo feature (default-on) + CI feature matrix"
```

---

### Task 2: TLS capability guard

**Files:**
- Modify: `src/llm/mod.rs`
- Modify: `src/llm/registry.rs`

**Interfaces:**
- Consumes: none
- Produces: `pub const TLS_ENABLED: bool` and `pub fn check_tls(base_url: &str) -> anyhow::Result<()>` in `crate::llm`; registry returns a clear error for https base URLs on no-tls builds.

- [ ] **Step 1: Write the failing test** (append inside `mod tests` in `src/llm/mod.rs`)

```rust
    #[test]
    fn tls_guard_classifies_urls() {
        assert!(super::check_tls("http://localhost:9/x").is_ok());
        #[cfg(not(feature = "tls"))]
        {
            let err = super::check_tls("https://api.z.ai/api/anthropic")
                .err()
                .unwrap();
            assert!(err.to_string().contains("without TLS"));
        }
        #[cfg(feature = "tls")]
        assert!(super::check_tls("https://api.z.ai/api/anthropic").is_ok());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked tls_guard`
Expected: FAIL — `check_tls` not defined (compile error).

- [ ] **Step 3: Implement** (add above `mod tests` in `src/llm/mod.rs`)

```rust
/// Whether this binary supports HTTPS (cargo feature `tls`).
pub const TLS_ENABLED: bool = cfg!(feature = "tls");

/// Reject https base URLs when the binary lacks the `tls` feature.
pub fn check_tls(base_url: &str) -> anyhow::Result<()> {
    if base_url.starts_with("https://") && !TLS_ENABLED {
        anyhow::bail!(
            "binary built without TLS cannot use https endpoint '{base_url}' — \
             use a release binary (github.com/phuongddx/vf/releases) \
             or rebuild with: cargo build --release --features tls"
        );
    }
    Ok(())
}
```

In `src/llm/registry.rs::build`, insert directly after the `let key = ...` line:

```rust
    crate::llm::check_tls(&p.base_url)?;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked tls_guard`
Expected: PASS. Then run the full suite (`... test --no-default-features --locked`): 48 passed.

- [ ] **Step 5: Commit**

```bash
git add src/llm/mod.rs src/llm/registry.rs
git commit -m "feat: tls capability guard for https base urls"
```

---

### Task 3: Built-in profiles + effective config + VF_CONFIG override

**Files:**
- Modify: `src/config.rs`

**Interfaces:**
- Consumes: existing `Config`, `Provider`, `Protocol`, `Routing`
- Produces: `Config::builtin() -> Config`; `Config::effective(self) -> Config` (file providers win per key; `routing.spec`/`codegen` default to `"glm-coding"`); `Config::path()` honors `VF_CONFIG` env override.

- [ ] **Step 1: Write the failing tests** (append inside `mod tests` in `src/config.rs`)

```rust
    #[test]
    fn builtin_profiles_match_spec() {
        let b = Config::builtin();
        let coding = &b.providers["glm-coding"];
        assert_eq!(coding.protocol, Protocol::Anthropic);
        assert_eq!(coding.base_url, "https://api.z.ai/api/anthropic");
        assert_eq!(coding.api_key_env, "ZAI_CODING_API_KEY");
        assert_eq!(coding.model, "glm-5.2");
        let std = &b.providers["glm"];
        assert_eq!(std.protocol, Protocol::OpenAi);
        assert_eq!(std.base_url, "https://api.z.ai/api/paas/v4");
        assert_eq!(std.api_key_env, "ZAI_API_KEY");
    }

    #[test]
    fn effective_overlays_file_over_builtin_and_defaults_routing() {
        let mut file = Config::default();
        file.providers.insert(
            "glm".into(),
            Provider {
                protocol: Protocol::OpenAi,
                base_url: "http://localhost:1/v4".into(),
                api_key_env: "LOCAL_KEY".into(),
                model: "m".into(),
            },
        );
        let eff = file.effective();
        assert_eq!(eff.providers["glm"].base_url, "http://localhost:1/v4");
        assert!(eff.providers.contains_key("glm-coding"));
        assert_eq!(eff.routing.spec.as_deref(), Some("glm-coding"));
        assert_eq!(eff.routing.codegen.as_deref(), Some("glm-coding"));
    }

    #[test]
    fn effective_keeps_file_routing() {
        let mut file = Config::default();
        file.routing.spec = Some("glm".into());
        let eff = file.effective();
        assert_eq!(eff.routing.spec.as_deref(), Some("glm"));
        assert_eq!(eff.routing.codegen.as_deref(), Some("glm-coding"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked config::`
Expected: FAIL — no `builtin`/`effective` methods.

- [ ] **Step 3: Implement** (in `src/config.rs`, inside `impl Config`)

Change `path()`:

```rust
    pub fn path() -> PathBuf {
        if let Ok(p) = std::env::var("VF_CONFIG") {
            if !p.is_empty() {
                return PathBuf::from(p);
            }
        }
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("vf")
            .join("config.toml")
    }
```

Add:

```rust
    /// Built-in Z.ai provider profiles. File config overrides per provider key.
    pub fn builtin() -> Self {
        let mut providers = BTreeMap::new();
        providers.insert(
            "glm-coding".into(),
            Provider {
                protocol: Protocol::Anthropic,
                base_url: "https://api.z.ai/api/anthropic".into(),
                api_key_env: "ZAI_CODING_API_KEY".into(),
                model: "glm-5.2".into(),
            },
        );
        providers.insert(
            "glm".into(),
            Provider {
                protocol: Protocol::OpenAi,
                base_url: "https://api.z.ai/api/paas/v4".into(),
                api_key_env: "ZAI_API_KEY".into(),
                model: "glm-5.2".into(),
            },
        );
        Self { providers, routing: Routing::default() }
    }

    /// Merge self (file config) over built-ins; default routing to glm-coding.
    pub fn effective(self) -> Self {
        let mut providers = Self::builtin().providers;
        for (k, v) in self.providers {
            providers.insert(k, v);
        }
        let routing = Routing {
            spec: self.routing.spec.or_else(|| Some("glm-coding".into())),
            codegen: self.routing.codegen.or_else(|| Some("glm-coding".into())),
        };
        Self { providers, routing }
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked`
Expected: PASS — 51 tests (48 + 3 new).

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: built-in glm-coding/glm profiles + effective config merge + VF_CONFIG override"
```

---

### Task 4: `vf config` shows effective providers + key hints

**Files:**
- Modify: `src/cmd/config_cmd.rs`
- Modify: `src/i18n.rs` (new `config_hint`)
- Test: `src/i18n.rs` unit test + `tests/cli.rs` integration test

**Interfaces:**
- Consumes: `Config::load()?.effective()`, `Lang::from_env()`
- Produces: `i18n::config_hint(lang: Lang, key_env: &str) -> String`; `vf config` output lines listing every effective provider, key env, set/missing state, routing target.

- [ ] **Step 1: Write the failing tests**

In `src/i18n.rs` tests:

```rust
    #[test]
    fn config_hint_bilingual() {
        assert!(config_hint(Lang::En, "ZAI_CODING_API_KEY").contains("export ZAI_CODING_API_KEY"));
        assert!(config_hint(Lang::Vi, "ZAI_CODING_API_KEY").contains("export ZAI_CODING_API_KEY"));
        assert!(config_hint(Lang::Vi, "X").contains("gợi ý"));
    }
```

In `tests/cli.rs`:

```rust
#[test]
fn config_shows_builtin_profiles_and_hints() {
    let dir = std::env::temp_dir().join(format!("vf-cfg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let cfg_path = dir.join("config.toml");
    std::fs::write(&cfg_path, "# empty file = built-ins only\n").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("vf").unwrap();
    cmd.arg("config")
        .env("VF_CONFIG", &cfg_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("glm-coding"))
        .stdout(predicate::str::contains("https://api.z.ai/api/anthropic"))
        .stdout(predicate::str::contains("ZAI_CODING_API_KEY"))
        .stdout(predicate::str::contains("glm"))
        .stdout(predicate::str::contains("https://api.z.ai/api/paas/v4"))
        .stdout(predicate::str::contains("export ZAI_CODING_API_KEY"));
    std::fs::remove_dir_all(&dir).ok();
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked config`
Expected: FAIL — `config_hint` undefined; integration output lacks glm-coding.

- [ ] **Step 3: Implement**

In `src/i18n.rs` add:

```rust
pub fn config_hint(lang: Lang, key_env: &str) -> String {
    match lang {
        Lang::En => format!("hint: export {key_env}=... to enable LLM spec generation"),
        Lang::Vi => format!("gợi ý: export {key_env}=... để bật sinh spec bằng LLM"),
    }
}
```

Replace `src/cmd/config_cmd.rs` `run()` body:

```rust
pub async fn run() -> anyhow::Result<()> {
    let lang = crate::i18n::Lang::from_env();
    let cfg = Config::load()?.effective();
    println!("config: {}", Config::path().display());
    let spec_target = cfg.routing.spec.clone().unwrap_or_default();
    for (name, p) in &cfg.providers {
        let proto = match p.protocol {
            crate::config::Protocol::OpenAi => "openai",
            crate::config::Protocol::Anthropic => "anthropic",
        };
        let state = if std::env::var(&p.api_key_env).map(|v| !v.is_empty()).unwrap_or(false) {
            "set"
        } else {
            "missing"
        };
        let routing_note = if *name == spec_target { "  [routing.spec]" } else { "" };
        println!("{name}  {proto}  {}  key: {state}: {}{routing_note}", p.base_url, p.api_key_env);
    }
    let key_env = cfg
        .providers
        .get(&spec_target)
        .map(|p| p.api_key_env.clone())
        .unwrap_or_default();
    println!("{}", crate::i18n::config_hint(lang, &key_env));
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked config`
Expected: PASS (unit + `config_command_prints_path` + new integration).

- [ ] **Step 5: Commit**

```bash
git add src/cmd/config_cmd.rs src/i18n.rs tests/cli.rs
git commit -m "feat: vf config shows effective providers and key hints"
```

---

### Task 5: Usage reporting + 30 s client timeout

**Files:**
- Modify: `src/llm/mod.rs`
- Modify: `src/llm/openai.rs`
- Modify: `src/llm/anthropic.rs`

**Interfaces:**
- Consumes: existing `ChatResponse`, adapters
- Produces: `pub struct Usage { pub input_tokens: u64, pub output_tokens: u64 }`; `ChatResponse { pub text: String, pub usage: Option<Usage> }`; both adapters build clients with 30 s timeout and parse provider usage fields.

- [ ] **Step 1: Write the failing tests**

In `src/llm/mod.rs` tests, update the `UpperEcho` impl to the new struct:

```rust
        async fn complete(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
            Ok(ChatResponse { text: req.user.to_uppercase(), usage: None })
        }
```

In `src/llm/openai.rs` test: add usage to the mock body and assert:

```rust
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{ "message": { "role": "assistant", "content": "hello back" } }],
                "usage": { "prompt_tokens": 7, "completion_tokens": 3 }
            })))
```

and after the existing `assert_eq!(resp.text, "hello back");`:

```rust
        let u = resp.usage.expect("usage parsed");
        assert_eq!((u.input_tokens, u.output_tokens), (7, 3));
```

In `src/llm/anthropic.rs` test: mock body gains `"usage": { "input_tokens": 5, "output_tokens": 2 }` and after the text assert:

```rust
        let u = resp.usage.expect("usage parsed");
        assert_eq!((u.input_tokens, u.output_tokens), (5, 2));
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked llm::`
Expected: FAIL — no `usage` field (compile errors across fakes and asserts).

- [ ] **Step 3: Implement**

`src/llm/mod.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub text: String,
    pub usage: Option<Usage>,
}
```

In BOTH adapters, replace `http: reqwest::Client::new(),` with:

```rust
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
```

`openai.rs` — replace the final `Ok(ChatResponse { text })` block:

```rust
        let usage = body["usage"].as_object().map(|u| Usage {
            input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            output_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        });
        Ok(ChatResponse { text, usage })
```

`anthropic.rs` — same pattern:

```rust
        let usage = val["usage"].as_object().map(|u| Usage {
            input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        });
        Ok(ChatResponse { text, usage })
```

(Import `Usage` in both adapters: `use super::{ChatRequest, ChatResponse, LlmClient, Usage};`.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked`
Expected: PASS — 54 tests.

- [ ] **Step 5: Commit**

```bash
git add src/llm/mod.rs src/llm/openai.rs src/llm/anthropic.rs
git commit -m "feat: usage reporting in chat responses + 30s client timeout"
```

---

### Task 6: Typed LLM errors + retry with backoff

**Files:**
- Create: `src/llm/retry.rs`
- Modify: `src/llm/mod.rs` (declare module + `LlmError`)
- Modify: `src/llm/openai.rs`, `src/llm/anthropic.rs` (error mapping)

**Interfaces:**
- Consumes: `LlmClient`, `ChatResponse`
- Produces: `pub enum LlmError { Http(u16), Network }` (impl `Display`+`Error`); `pub struct RetryPolicy { pub max_retries: u32, pub delays: Vec<Duration> }` (Default: 2 retries, [1 s, 3 s]); `pub struct RetryingClient` implementing `LlmClient` (`RetryingClient::new(inner: Box<dyn LlmClient>)` uses Default policy; `RetryingClient::with_policy(inner, policy)`); adapters return `anyhow::Error` carrying `LlmError` (downcastable).

- [ ] **Step 1: Write the failing tests** (`src/llm/retry.rs`, include tests in same file)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{ChatRequest, ChatResponse, LlmClient, LlmError};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct Flaky {
        fails: u32,
        code: u16,
        calls: AtomicU32,
    }

    #[async_trait]
    impl LlmClient for Flaky {
        async fn complete(&self, _req: &ChatRequest) -> anyhow::Result<ChatResponse> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n < self.fails {
                Err(anyhow::Error::new(if self.code == 0 {
                    LlmError::Network
                } else {
                    LlmError::Http(self.code)
                }))
            } else {
                Ok(ChatResponse { text: "ok".into(), usage: None })
            }
        }
    }

    fn fast() -> RetryPolicy {
        RetryPolicy { max_retries: 2, delays: vec![ms(1), ms(1)] }
    }

    fn ms(n: u64) -> std::time::Duration {
        std::time::Duration::from_millis(n)
    }

    #[tokio::test]
    async fn retries_5xx_then_succeeds() -> anyhow::Result<()> {
        let inner = Box::new(Flaky { fails: 2, code: 500, calls: AtomicU32::new(0) });
        let c = RetryingClient::with_policy(inner, fast());
        let r = c.complete(&ChatRequest { system: None, user: "x".into() }).await?;
        assert_eq!(r.text, "ok");
        Ok(())
    }

    #[tokio::test]
    async fn exhausts_retries_on_persistent_500() {
        let inner = Box::new(Flaky { fails: 99, code: 500, calls: AtomicU32::new(0) });
        let c = RetryingClient::with_policy(inner, fast());
        assert!(c.complete(&ChatRequest { system: None, user: "x".into() }).await.is_err());
    }

    #[tokio::test]
    async fn does_not_retry_400() {
        let inner = Box::new(Flaky { fails: 99, code: 400, calls: AtomicU32::new(0) });
        let c = RetryingClient::with_policy(inner, fast());
        assert!(c.complete(&ChatRequest { system: None, user: "x".into() }).await.is_err());
        assert_eq!(inner.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_network_errors() -> anyhow::Result<()> {
        let inner = Box::new(Flaky { fails: 1, code: 0, calls: AtomicU32::new(0) });
        let c = RetryingClient::with_policy(inner, fast());
        assert!(c.complete(&ChatRequest { system: None, user: "x".into() }).await.is_ok());
        Ok(())
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked retry`
Expected: FAIL — module `retry` does not exist.

- [ ] **Step 3: Implement**

`src/llm/mod.rs` — add:

```rust
#[derive(Debug)]
pub enum LlmError {
    Http(u16),
    Network,
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::Http(code) => write!(f, "provider returned HTTP {code}"),
            LlmError::Network => write!(f, "network error"),
        }
    }
}

impl std::error::Error for LlmError {}
```

and `pub mod retry;` beside the other module declarations.

Create `src/llm/retry.rs`:

```rust
use super::{ChatRequest, ChatResponse, LlmClient, LlmError};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub delays: Vec<std::time::Duration>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            delays: vec![
                std::time::Duration::from_secs(1),
                std::time::Duration::from_secs(3),
            ],
        }
    }
}

fn is_retryable(err: &anyhow::Error) -> bool {
    match err.downcast_ref::<LlmError>() {
        Some(LlmError::Http(429)) => true,
        Some(LlmError::Http(c)) if (500..600).contains(c) => true,
        Some(LlmError::Network) => true,
        _ => false,
    }
}

/// Delay for `attempt` (0-based): base delay ± 200 ms jitter, never negative.
fn jittered(delays: &[std::time::Duration], attempt: u32) -> std::time::Duration {
    let idx = attempt.min(delays.len().saturating_sub(1) as u32) as usize;
    let base_ms = delays[idx].as_millis() as i64;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let jitter = (nanos % 400) as i64 - 200;
    std::time::Duration::from_millis((base_ms + jitter).max(0) as u64)
}

pub struct RetryingClient {
    inner: Box<dyn LlmClient>,
    policy: RetryPolicy,
}

impl RetryingClient {
    pub fn new(inner: Box<dyn LlmClient>) -> Self {
        Self { inner, policy: RetryPolicy::default() }
    }

    pub fn with_policy(inner: Box<dyn LlmClient>, policy: RetryPolicy) -> Self {
        Self { inner, policy }
    }
}

#[async_trait]
impl LlmClient for RetryingClient {
    async fn complete(&self, req: &ChatRequest) -> anyhow::Result<ChatResponse> {
        let mut attempt = 0u32;
        loop {
            match self.inner.complete(req).await {
                Ok(resp) => return Ok(resp),
                Err(e) if is_retryable(&e) && attempt < self.policy.max_retries => {
                    tokio::time::sleep(jittered(&self.policy.delays, attempt)).await;
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

Adapters — map errors to `LlmError`. In `openai.rs` replace the send/parse section:

```rust
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({ "model": self.model, "messages": messages }))
            .send()
            .await
            .map_err(|e| anyhow::Error::new(crate::llm::LlmError::Network).context(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(anyhow::Error::new(crate::llm::LlmError::Http(status.as_u16())));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| anyhow::Error::new(crate::llm::LlmError::Network).context(e.to_string()))?;
```

Same replacement pattern in `anthropic.rs` (headers/body lines stay as-is). Add `use anyhow::Context;` where needed.

Add one adapter-level test in `openai.rs` tests:

```rust
    #[tokio::test]
    async fn http_500_maps_to_llm_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let client = OpenAiClient::new(
            format!("{}/v1", server.uri()),
            "k".into(),
            "m".into(),
        );
        let err = client
            .complete(&ChatRequest { system: None, user: "x".into() })
            .await
            .err()
            .unwrap();
        assert_eq!(err.downcast_ref::<crate::llm::LlmError>(), Some(&crate::llm::LlmError::Http(500)));
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked`
Expected: PASS — 59 tests.

- [ ] **Step 5: Commit**

```bash
git add src/llm/mod.rs src/llm/retry.rs src/llm/openai.rs src/llm/anthropic.rs
git commit -m "feat: retry policy with backoff and typed llm errors"
```

---

### Task 7: Bilingual LLM strings (errors, offline, fallback, usage)

**Files:**
- Modify: `src/i18n.rs`

**Interfaces:**
- Consumes: `LlmError`, `Lang`
- Produces: `llm_error(lang, err: &anyhow::Error, key_env: &str) -> String`; `offline_mode(lang) -> String`; `llm_fallback(lang, reason: &str) -> String`; `usage_line(lang, input: u64, output: u64, cost: Option<f64>) -> String`

- [ ] **Step 1: Write the failing tests** (append in `src/i18n.rs` tests)

```rust
    #[test]
    fn llm_error_maps_bilingually() {
        use crate::llm::LlmError;
        let key = anyhow::Error::new(LlmError::Http(401));
        assert!(llm_error(Lang::En, &key, "ZAI_CODING_API_KEY").contains("invalid API key"));
        assert!(llm_error(Lang::En, &key, "ZAI_CODING_API_KEY").contains("ZAI_CODING_API_KEY"));
        assert!(llm_error(Lang::Vi, &key, "ZAI_CODING_API_KEY").contains("không hợp lệ"));
        let rl = anyhow::Error::new(LlmError::Http(429));
        assert!(llm_error(Lang::Vi, &rl, "K").contains("giới hạn"));
        let bad = anyhow::Error::new(LlmError::Http(503));
        assert!(llm_error(Lang::En, &bad, "K").contains("upstream"));
        let net = anyhow::Error::new(LlmError::Network);
        assert!(llm_error(Lang::Vi, &net, "K").contains("không kết nối được"));
    }

    #[test]
    fn mode_and_usage_lines_bilingual() {
        assert!(offline_mode(Lang::Vi).contains("chế độ offline"));
        assert!(offline_mode(Lang::En).contains("offline mode"));
        assert!(llm_fallback(Lang::Vi, "bad json").contains("spec mẫu"));
        assert!(usage_line(Lang::En, 10, 20, Some(0.0001)).contains("usage: 10 in / 20 out tokens"));
        assert!(usage_line(Lang::En, 10, 20, Some(0.0001)).contains("~$"));
        assert!(usage_line(Lang::Vi, 10, 20, None).contains("tiêu thụ: 10 vào / 20 ra token"));
        assert!(!usage_line(Lang::Vi, 10, 20, None).contains("~$"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked i18n`
Expected: FAIL — functions undefined.

- [ ] **Step 3: Implement** (add to `src/i18n.rs`)

```rust
pub fn llm_error(lang: Lang, err: &anyhow::Error, key_env: &str) -> String {
    use crate::llm::LlmError;
    match err.downcast_ref::<LlmError>() {
        Some(LlmError::Http(401)) | Some(LlmError::Http(403)) => match lang {
            Lang::En => format!("LLM error: invalid API key — check {key_env}"),
            Lang::Vi => format!("Lỗi LLM: API key không hợp lệ — kiểm tra {key_env}"),
        },
        Some(LlmError::Http(429)) => match lang {
            Lang::En => "LLM error: rate limited (already retried)".into(),
            Lang::Vi => "Lỗi LLM: vượt giới hạn tốc độ (đã thử lại)".into(),
        },
        Some(LlmError::Http(c)) if (500..600).contains(c) => match lang {
            Lang::En => format!("LLM error: upstream server error (HTTP {c})"),
            Lang::Vi => format!("Lỗi LLM: máy chủ nhà cung cấp lỗi (HTTP {c})"),
        },
        Some(LlmError::Network) => match lang {
            Lang::En => "LLM error: cannot reach provider (network)".into(),
            Lang::Vi => "Lỗi LLM: không kết nối được nhà cung cấp (mạng)".into(),
        },
        None => match lang {
            Lang::En => format!("LLM error: {err}"),
            Lang::Vi => format!("Lỗi LLM: {err}"),
        },
    }
}

pub fn offline_mode(lang: Lang) -> String {
    match lang {
        Lang::En => "offline mode: no LLM key — using template spec".into(),
        Lang::Vi => "chế độ offline: chưa có key LLM — dùng spec mẫu".into(),
    }
}

pub fn llm_fallback(lang: Lang, reason: &str) -> String {
    match lang {
        Lang::En => format!("LLM output unusable ({reason}) — fell back to template spec"),
        Lang::Vi => format!("Kết quả LLM không dùng được ({reason}) — đã dùng spec mẫu"),
    }
}

pub fn usage_line(lang: Lang, input: u64, output: u64, cost: Option<f64>) -> String {
    match (lang, cost) {
        (Lang::En, Some(c)) => format!("usage: {input} in / {output} out tokens (~${c:.4})"),
        (Lang::En, None) => format!("usage: {input} in / {output} out tokens"),
        (Lang::Vi, Some(c)) => format!("tiêu thụ: {input} vào / {output} ra token (~${c:.4})"),
        (Lang::Vi, None) => format!("tiêu thụ: {input} vào / {output} ra token"),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked i18n`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/i18n.rs
git commit -m "feat: bilingual llm error mapping and mode notices"
```

---

### Task 8: Token cost estimates

**Files:**
- Create: `src/llm/pricing.rs`
- Modify: `src/llm/mod.rs` (`pub mod pricing;`)

**Interfaces:**
- Consumes: `Usage`
- Produces: `pricing::estimate_cost(model: &str, usage: &Usage) -> Option<f64>` (USD; `None` when the model has no price entry)

- [ ] **Step 1: Write the failing tests** (inside `src/llm/pricing.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Usage;

    #[test]
    fn glm_52_priced_from_docs() {
        let u = Usage { input_tokens: 1_000_000, output_tokens: 1_000_000 };
        let cost = estimate_cost("glm-5.2", &u).unwrap();
        assert!((cost - 5.8).abs() < 1e-9); // $1.4 in + $4.4 out per 1M
    }

    #[test]
    fn unknown_model_returns_none() {
        let u = Usage { input_tokens: 10, output_tokens: 10 };
        assert!(estimate_cost("mystery-9", &u).is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked pricing`
Expected: FAIL — module missing.

- [ ] **Step 3: Implement**

`src/llm/pricing.rs`:

```rust
//! USD per 1M tokens. Source: docs.z.ai pricing page, fetched 2026-10-31.
//! Unknown models report tokens only (estimate_cost -> None).

use super::Usage;

pub fn estimate_cost(model: &str, usage: &Usage) -> Option<f64> {
    let (in_per_m, out_per_m): (f64, f64) = match model {
        "glm-5.2" => (1.4, 4.4),
        _ => return None,
    };
    Some(
        usage.input_tokens as f64 / 1_000_000.0 * in_per_m
            + usage.output_tokens as f64 / 1_000_000.0 * out_per_m,
    )
}
```

`src/llm/mod.rs`: add `pub mod pricing;` beside other modules.

- [ ] **Step 4: Run tests to verify they pass**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked pricing`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add src/llm/pricing.rs src/llm/mod.rs
git commit -m "feat: token cost estimates for glm models"
```

---

### Task 9: LLM spec generation in `vf make` (validate → retry → fallback)

**Files:**
- Modify: `src/cmd/make.rs`
- Test: `src/cmd/make.rs` unit tests + `tests/cli.rs` integration tests

**Interfaces:**
- Consumes: `Config::effective`, `registry::build`, `check_tls`, `RetryingClient`, `VideoSpec::from_json_str/validate`, `i18n::{offline_mode, llm_fallback, usage_line, llm_error}`, `pricing::estimate_cost`
- Produces: `pub enum LlmSpecOutcome { Generated { spec: VideoSpec, usage: Usage, model: String }, Offline(String), Fallback(String) }`; `pub async fn generate_spec_with_llm(cfg: &Config, idea: &str, t: &Template) -> LlmSpecOutcome`; `pub fn extract_json(text: &str) -> Option<String>`; `fn spec_request(idea, t, feedback: Option<&str>) -> ChatRequest`. `run()` writes LLM spec to `vf-projects/<slug>/video.spec.json` before the stage loop and prints one mode/usage line.

- [ ] **Step 1: Write the failing unit tests** (append in `make.rs` tests)

```rust
    #[test]
    fn extract_json_strips_fences_and_prose() {
        let fenced = "```json\n{\"a\":1}\n```";
        assert_eq!(super::extract_json(fenced).as_deref(), Some("{\"a\":1}"));
        let prose = "Here you go:\n{\"b\":2} hope it helps";
        assert_eq!(super::extract_json(prose).as_deref(), Some("{\"b\":2}"));
        assert_eq!(super::extract_json("no json at all"), None);
    }

    #[test]
    fn spec_request_appends_feedback_on_second_attempt() {
        use crate::templates::Template;
        let t = Template::Default;
        let first = super::spec_request("idea", &t, None);
        assert!(first.user.contains("idea"));
        assert!(first.system.as_deref().unwrap_or("").contains("meta.duration"));
        let second = super::spec_request("idea", &t, Some("scenes empty"));
        assert!(second.user.contains("scenes empty"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked make`
Expected: FAIL — functions undefined.

- [ ] **Step 3: Implement**

Add to `src/cmd/make.rs` (above the tests module):

```rust
pub enum LlmSpecOutcome {
    Generated { spec: crate::ir::VideoSpec, usage: crate::llm::Usage, model: String },
    Offline(String),
    Fallback(String),
}

/// Strip markdown fences / surrounding prose; return the outermost {...} span.
pub fn extract_json(text: &str) -> Option<String> {
    let t = text.trim();
    let t = t.strip_prefix("```json").or_else(|| t.strip_prefix("```")).unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    let t = t.trim();
    let start = t.find('{')?;
    let end = t.rfind('}')?;
    (start <= end).then(|| t[start..=end].to_string())
}

fn spec_request(
    idea: &str,
    t: &crate::templates::Template,
    feedback: Option<&str>,
) -> crate::llm::ChatRequest {
    let system = "You are a video producer. Reply with ONLY a JSON object of shape:\n\
{ \"meta\": { \"title\": string, \"duration\": number, \"fps\": number, \
\"ratio\": \"9:16\"|\"16:9\"|\"1:1\", \"lang\": string }, \
\"style\": { \"preset\": string, \"palette\": string[], \"font\": string }, \
\"scenes\": [ { \"id\": string, \"duration\": number, \"narration\": string, \
\"elements\": [], \"transition\": string } ] }\n\
Rules: at least one scene; meta.duration equals the sum of scene durations; fps > 0.";
    let mut user = format!("Idea: {idea}\nTemplate: {t:?}\nOutput the JSON object only.");
    if let Some(fb) = feedback {
        user.push_str(&format!("\nYour previous output failed validation: {fb}. Fix it."));
    }
    crate::llm::ChatRequest { system: Some(system.into()), user }
}

pub async fn generate_spec_with_llm(
    cfg: &crate::config::Config,
    idea: &str,
    t: &crate::templates::Template,
) -> LlmSpecOutcome {
    let Some(provider_name) = cfg.routing.spec.clone() else {
        return LlmSpecOutcome::Offline("no routing".into());
    };
    let Some(p) = cfg.providers.get(&provider_name) else {
        return LlmSpecOutcome::Offline(format!("unknown provider '{provider_name}'"));
    };
    let key_missing = std::env::var(&p.api_key_env).map(|v| v.is_empty()).unwrap_or(true);
    if key_missing {
        return LlmSpecOutcome::Offline(p.api_key_env.clone());
    }
    if crate::llm::check_tls(&p.base_url).is_err() {
        return LlmSpecOutcome::Offline("no tls in this binary".into());
    }
    let Ok(inner) = crate::llm::registry::build(&provider_name, cfg) else {
        return LlmSpecOutcome::Offline("provider build failed".into());
    };
    let client = crate::llm::retry::RetryingClient::new(inner);
    let mut feedback: Option<String> = None;
    for _attempt in 0..2 {
        let req = spec_request(idea, t, feedback.as_deref());
        match client.complete(&req).await {
            Err(e) => {
                let reason = crate::i18n::llm_error(crate::i18n::Lang::En, &e, &p.api_key_env);
                return LlmSpecOutcome::Fallback(reason);
            }
            Ok(resp) => {
                let parsed = extract_json(&resp.text)
                    .and_then(|s| crate::ir::VideoSpec::from_json_str(&s).ok());
                match parsed {
                    Some(spec) => match spec.validate() {
                        Ok(()) => {
                            return LlmSpecOutcome::Generated {
                                spec,
                                usage: resp.usage.unwrap_or(crate::llm::Usage {
                                    input_tokens: 0,
                                    output_tokens: 0,
                                }),
                                model: p.model.clone(),
                            }
                        }
                        Err(e) => feedback = Some(e),
                    },
                    None => feedback = Some("output was not a valid JSON spec".into()),
                }
            }
        }
    }
    LlmSpecOutcome::Fallback(feedback.unwrap_or_else(|| "invalid spec after retry".into()))
}
```

Wire into `run()` — insert directly after `let mut state = ...;` block and before `while let Some(stage) = ...`:

```rust
    let cfg = crate::config::Config::load()?.effective();
    let spec_path = project_dir.join("video.spec.json");
    if !spec_path.exists() {
        match generate_spec_with_llm(&cfg, idea, &t).await {
            LlmSpecOutcome::Generated { spec, usage, model } => {
                std::fs::write(&spec_path, spec.to_json_string()?)?;
                let cost = crate::llm::pricing::estimate_cost(&model, &usage);
                println!(
                    "{}",
                    crate::i18n::usage_line(lang, usage.input_tokens, usage.output_tokens, cost)
                );
            }
            LlmSpecOutcome::Offline(_) => {
                println!("{}", crate::i18n::offline_mode(lang));
            }
            LlmSpecOutcome::Fallback(reason) => {
                println!("{}", crate::i18n::llm_fallback(lang, &reason));
            }
        }
    }
```

And in the `if plan { ... }` block, add before `return Ok(())`:

```rust
        let plan_cfg = crate::config::Config::load()?.effective();
        let target = plan_cfg.routing.spec.clone().unwrap_or_default();
        let has_key = plan_cfg
            .providers
            .get(&target)
            .map(|p| std::env::var(&p.api_key_env).map(|v| !v.is_empty()).unwrap_or(false))
            .unwrap_or(false);
        println!("provider: {target} ({})", if has_key { "llm" } else { "offline" });
```

- [ ] **Step 4: Run unit tests**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked make`
Expected: PASS.

- [ ] **Step 5: Write the failing integration tests** (append in `tests/cli.rs`)

```rust
fn temp_dir_for(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("vf-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[tokio::test]
async fn make_uses_llm_when_key_present() -> anyhow::Result<()> {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let spec_json = serde_json::json!({
        "meta": { "title": "LLM Demo", "duration": 4, "fps": 30, "ratio": "9:16", "lang": "vi" },
        "style": { "preset": "mg-07", "palette": ["#0F172A"], "font": "Inter" },
        "scenes": [ { "id": "s1", "duration": 4, "narration": "xin chào" } ]
    })
    .to_string();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "content": [{ "type": "text", "text": format!("```json\n{spec_json}\n```") }],
            "usage": { "input_tokens": 10, "output_tokens": 20 }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dir = temp_dir_for("make-llm");
    let cfg_path = dir.join("config.toml");
    std::fs::write(
        &cfg_path,
        format!(
            "[providers.glm-coding]\nprotocol = \"anthropic\"\nbase_url = \"{}\"\napi_key_env = \"VF_TEST_LLM_KEY\"\nmodel = \"glm-5.2\"\n\n[routing]\nspec = \"glm-coding\"\n",
            server.uri()
        ),
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("vf").unwrap();
    cmd.arg("make").arg("llm driven idea")
        .current_dir(&dir)
        .env("VF_CONFIG", &cfg_path)
        .env("VF_TEST_LLM_KEY", "dummy")
        .env("VF_ENGINE", "dry-run")
        .env("VF_LANG", "vi")
        .assert()
        .success()
        .stdout(predicate::str::contains("tiêu thụ: 10 vào / 20 ra token"));

    let spec = std::fs::read_to_string(dir.join("vf-projects/llm-driven-idea/video.spec.json"))?;
    assert!(spec.contains("LLM Demo"));
    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

#[tokio::test]
async fn make_falls_back_to_template_on_garbage() -> anyhow::Result<()> {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "content": [{ "type": "text", "text": "sorry, no json today" }]
        })))
        .expect(2) // initial + one validation-feedback retry
        .mount(&server)
        .await;

    let dir = temp_dir_for("make-fallback");
    let cfg_path = dir.join("config.toml");
    std::fs::write(
        &cfg_path,
        format!(
            "[providers.glm-coding]\nprotocol = \"anthropic\"\nbase_url = \"{}\"\napi_key_env = \"VF_TEST_LLM_KEY\"\nmodel = \"glm-5.2\"\n\n[routing]\nspec = \"glm-coding\"\n",
            server.uri()
        ),
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("vf").unwrap();
    cmd.arg("make").arg("fallback idea")
        .current_dir(&dir)
        .env("VF_CONFIG", &cfg_path)
        .env("VF_TEST_LLM_KEY", "dummy")
        .env("VF_ENGINE", "dry-run")
        .env("VF_LANG", "vi")
        .assert()
        .success()
        .stdout(predicate::str::contains("spec mẫu"));

    assert!(dir.join("vf-projects/fallback-idea/video.spec.json").exists());
    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

#[test]
fn make_offline_mode_warns_without_key() {
    let dir = temp_dir_for("make-offline");
    let cfg_path = dir.join("config.toml");
    std::fs::write(&cfg_path, "# defaults only\n").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("vf").unwrap();
    cmd.arg("make").arg("offline idea")
        .current_dir(&dir)
        .env("VF_CONFIG", &cfg_path)
        .env("VF_ENGINE", "dry-run")
        .env("VF_LANG", "vi")
        .env("ZAI_CODING_API_KEY", "")
        .env("ZAI_API_KEY", "")
        .assert()
        .success()
        .stdout(predicate::str::contains("chế độ offline"));

    std::fs::remove_dir_all(&dir).ok();
}
```

- [ ] **Step 6: Run the full suite**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked`
Expected: PASS — 66 tests (59 + 2 unit + 3 integration + plan-mode assertion updates if any).

- [ ] **Step 7: Commit**

```bash
git add src/cmd/make.rs tests/cli.rs
git commit -m "feat: llm spec generation in make with validate-retry-fallback"
```

---

### Task 10: Doctor — TLS status + provider network checks

**Files:**
- Modify: `src/cmd/doctor.rs`
- Test: `src/cmd/doctor.rs` unit tests

**Interfaces:**
- Consumes: `TLS_ENABLED`, `Config::load()?.effective()`
- Produces: `pub async fn check_tls_status() -> String`; `pub async fn check_url(url: &str) -> String`; `run()` prints both (network checks only for providers whose key env is set; https on a no-tls build prints `SKIPPED (no tls)`).

- [ ] **Step 1: Write the failing tests** (append in `doctor.rs` tests)

```rust
    #[test]
    fn tls_status_line_names_state() {
        let line = futures::block_on(super::check_tls_status());
        assert!(line.starts_with("doctor: tls "));
        assert!(line.contains("tls OK (https supported)") || line.contains("tls MISSING"));
    }

    #[tokio::test]
    async fn check_url_reports_reachable_and_unreachable() -> anyhow::Result<()> {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        let ok = super::check_url(&server.uri()).await;
        assert!(ok.contains("OK"), "line was: {ok}");
        let bad = super::check_url("http://127.0.0.1:9/").await;
        assert!(bad.contains("UNREACHABLE"), "line was: {bad}");
        Ok(())
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked doctor`
Expected: FAIL — functions undefined.

- [ ] **Step 3: Implement**

Add to `src/cmd/doctor.rs`:

```rust
pub async fn check_tls_status() -> String {
    if crate::llm::TLS_ENABLED {
        "doctor: tls OK (https supported)".into()
    } else {
        "doctor: tls MISSING (rebuild with --features tls for https)".into()
    }
}

pub async fn check_url(url: &str) -> String {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let start = std::time::Instant::now();
    match client.get(url).send().await {
        Ok(_) => format!("doctor: network {url} OK ({} ms)", start.elapsed().as_millis()),
        Err(_) => format!("doctor: network {url} UNREACHABLE"),
    }
}
```

Extend `run()` after the TOOLS loop:

```rust
    println!("{}", check_tls_status().await);
    let cfg = crate::config::Config::load()?.effective();
    for (_name, p) in &cfg.providers {
        let key_set = std::env::var(&p.api_key_env).map(|v| !v.is_empty()).unwrap_or(false);
        if !key_set {
            continue;
        }
        if p.base_url.starts_with("https://") && !crate::llm::TLS_ENABLED {
            println!("doctor: network {} SKIPPED (no tls in this binary)", p.base_url);
            continue;
        }
        println!("{}", check_url(&p.base_url).await);
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked doctor`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cmd/doctor.rs
git commit -m "feat: doctor tls and provider network checks"
```

---

### Task 11: README network section + final verification

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: everything above (documentation only)
- Produces: updated docs; no code.

- [ ] **Step 1: Update README**

In the "Cấu hình LLM provider" section, replace the `[providers.glm]` sample with:

```toml
# Built-in (không cần cấu hình):
#   glm-coding  anthropic  https://api.z.ai/api/anthropic   key: ZAI_CODING_API_KEY
#   glm         openai     https://api.z.ai/api/paas/v4     key: ZAI_API_KEY
# Override hoặc thêm provider trong file này — file thắng built-in theo tên.
[routing]
spec = "glm-coding"
```

Add a "Chế độ offline / Online" subsection:

```markdown
### Offline / Online

Không có key: `vf make` chạy offline (spec mẫu + dry-run) và in cảnh báo.
Có key (`export ZAI_CODING_API_KEY=...`): spec được sinh bằng GLM, kèm token
usage + ước tính chi phí. LLM lỗi → tự retry → fallback spec mẫu, make không bao giờ chết vì LLM.

Ghi chú GLM Coding Plan: gói subscription của Z.ai giới hạn cho công cụ được
hỗ trợ chính thức; vf dùng endpoint "other tools" (best-effort) — bạn tự chịu
trách nhiệm với điều khoản gói của mình. Xem docs.z.ai/devpack/tool/others.
```

Update the status table row "🚧 LLM adapters ..." to:
`✅ LLM spec generation thật (Z.ai GLM default — Anthropic/OpenAI protocol), retry + fallback offline; codegen thật = phase sau`

Update "Cài đặt / Build từ source": add `cargo build --release --locked` ships TLS by default; `--no-default-features` for environments without a C compiler. Mention `VF_CONFIG` env override and new `doctor` lines (tls/network) in the commands section. Mirror the two changes briefly in the English section.

- [ ] **Step 2: Run the FULL suite one final time (both combos where possible)**

Run: `sh /home/node/.openclaw/workspace/.openclaw/tmp/cargo.sh test --no-default-features --locked`
Expected: PASS — 68 tests.

- [ ] **Step 3: Commit + push**

```bash
git add README.md
git commit -m "docs: network layer section in README"
git push origin main
```

- [ ] **Step 4: Verify CI green on GitHub (both matrix legs), then report**

Check `https://api.github.com/repos/phuongddx/vf/actions/runs?per_page=1` — both `test (—locked)` and `test (--no-default-features --locked)` legs must be `success`.

---

## Self-Review (completed by plan author)

- **Spec coverage:** TLS feature+matrix (T1), capability guard (T2), builtin profiles+merge+VF_CONFIG (T3), config display (T4), usage+timeout (T5), retry/backoff/jitter (T6), bilingual errors (T7), cost table (T8), spec-stage wiring incl. plan-mode provider line (T9), doctor tls/network (T10), README + Coding Plan note (T11). All spec sections map to tasks.
- **Placeholders:** none — every step carries exact code or exact edits.
- **Type consistency:** `Usage{input_tokens,output_tokens}` (T5→T8/T9), `LlmError` (T6→T7/T9), `RetryingClient::new/with_policy` (T6→T9), `check_tls`/`TLS_ENABLED` (T2→T9/T10), `Config::effective` consuming self (T3→T4/T9/T10), i18n fns (T7→T4/T9).
- Known deviation from spec text: doctor checks network only for key-set providers (spec said "each configured provider") — avoids always-failing https checks on no-tls builds; noted in T10.
