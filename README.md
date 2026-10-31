<div align="center">

# vf — Video Forge

**Text → product video, một câu lệnh.**

[![CI](https://github.com/phuongddx/vf/actions/workflows/ci.yml/badge.svg)](https://github.com/phuongddx/vf/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/phuongddx/vf)](https://github.com/phuongddx/vf/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Việt Nam 🇻🇳 / [English](#english)

</div>

---

`vf` biến một ý tưởng thành video sản phẩm qua pipeline: **idea → spec (IR JSON) → engine render → post-process**. CLI viết bằng Rust, chạy qua agent skills, hỗ trợ tiếng Việt và tiếng Anh.

```sh
vf make "app fintech cho Gen Z" --template launch
```

## Cài đặt

**Binary (Linux x86_64, static musl — không cần dependency):**

Tải `vf-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz` từ [Releases](https://github.com/phuongddx/vf/releases), giải nén, chạy.

**Build từ source (Rust ≥ 1.85):**

```sh
git clone https://github.com/phuongddx/vf && cd vf
cargo build --release --locked
```

## Lệnh chính

| Lệnh | Chức năng |
|---|---|
| `vf make <idea>` | Chạy toàn pipeline cho một ý tưởng |
| `vf make <idea> --plan` | In kế hoạch + ước tính chi phí LLM, không chạy |
| `vf make <idea> --max-cost 0.5` | Dừng nếu ước tính vượt 0.5 USD |
| `vf make <idea> --template launch` | Dùng template `default` / `launch` / `explainer` |
| `vf skills <git-url>` | Cài skill từ git repo (`vf skills` để liệt kê) |
| `vf doctor` | Kiểm tra toolchain: git, node, npx, ffmpeg |
| `vf status <project>` | Xem stage hiện tại của project |
| `vf config` | In đường dẫn config hiện tại |

**Ngôn ngữ output:** `VF_LANG=vi` (mặc định) hoặc `VF_LANG=en`.

## Cấu hình LLM provider

File: `~/.config/vf/config.toml` (Linux). API key không nằm trong file — chỉ tên biến môi trường.

```toml
[providers.glm]
protocol = "openai"                              # hoặc "anthropic"
base_url = "https://open.bigmodel.cn/api/paas/v4" # GLM (OpenAI-compatible); OpenAI: gồm cả /v1
api_key_env = "GLM_API_KEY"
model = "glm-4.7"

[routing]
spec = "glm"      # provider dùng cho giai đoạn sinh spec
codegen = "glm"   # provider dùng cho giai đoạn sinh code
```

## Trạng thái (v0.1.0 — foundation)

- ✅ Pipeline end-to-end với **dry-run engine** (mặc định): `video.spec.json` + scaffold + `draft.mp4` + `final.mp4`
- ✅ Remotion engine: sinh project scaffold + lệnh `npx` render
- ✅ Skill manager: install từ git + lockfile pin; style presets; spec templates
- ✅ `vf doctor` / `vf status` / i18n VI-EN / cost guard (`--plan`, `--max-cost`)
- 🚧 LLM adapters (OpenAI / Anthropic protocol) đã có ở tầng library (unit-tested qua mock HTTP) — đang chờ wiring vào pipeline + TLS ở bản sau

Kiến trúc engine hướng tới: **Remotion** (mặc định) + **HyperFrames**, cùng ship trong một bản release all-in-one.

## Giấy phép

- Code của `vf`: **[MIT](LICENSE)**
- ⚠️ **Remotion**: engine render mặc định được gọi qua `npx` lúc runtime. Remotion có [license riêng](https://www.remotion.dev/docs/license) — **miễn phí cho công ty ≤ 3 người**, ngoài ra cần license trả phí. `vf` không bundle Remotion.
- **HyperFrames**: [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0)
- Skills do người dùng cài vào chịu license của repo tương ứng.

---

## English

`vf` (Video Forge) is a Rust CLI that turns an idea into a product video through a pipeline: **idea → spec (IR JSON) → engine render → post-process**, driven by installable agent skills, with Vietnamese/English output.

```sh
vf make "fintech app for Gen Z" --template launch --plan
```

Install a static musl binary from [Releases](https://github.com/phuongddx/vf/releases) or build with Rust ≥ 1.85 (`cargo build --release --locked`). Configure providers in `~/.config/vf/config.toml` (GLM/OpenAI-compatible and Anthropic protocols; API keys read from environment variables). v0.1.0 ships the full pipeline with the dry-run engine default, Remotion scaffolding, skill manager, styles, templates, doctor, i18n and cost guardrails; real LLM calls + TLS land in the next release.

vf code is MIT-licensed. Remotion (invoked via `npx` at runtime, not bundled) is free only for companies of up to 3 people — see their [license](https://www.remotion.dev/docs/license). HyperFrames is Apache-2.0.
