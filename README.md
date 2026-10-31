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

## Demo

<div align="center">

<video src="docs/demo.mp4" controls muted width="270"></video>

*Remotion — spec → scaffold → render → mp4.*

<video src="docs/demo-hyperframes.mp4" controls muted width="270"></video>

*HyperFrames — LLM viết composition HTML → lint → render → mp4.*

</div>

## Cài đặt

**Binary (Linux x86_64, static musl — không cần dependency):**

Tải `vf-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz` từ [Releases](https://github.com/phuongddx/vf/releases), giải nén, chạy.

**Build từ source (Rust ≥ 1.85):**

```sh
git clone https://github.com/phuongddx/vf && cd vf
cargo build --release --locked   # TLS (HTTPS) bật mặc định
# Môi trường không có C compiler: thêm --no-default-features
```

## Lệnh chính

| Lệnh | Chức năng |
|---|---|
| `vf make <idea>` | Chạy toàn pipeline cho một ý tưởng |
| `vf make <idea> --plan` | In kế hoạch + ước tính chi phí + provider sẽ dùng, không chạy |
| `vf make <idea> --max-cost 0.5` | Dừng nếu ước tính vượt 0.5 USD |
| `vf make <idea> --template launch` | Dùng template `default` / `launch` / `explainer` |
| `vf skills <git-url>` | Cài skill từ git repo (`vf skills` để liệt kê) |
| `vf doctor` | Kiểm tra toolchain (git, node, npx, ffmpeg) + TLS + network provider |
| `vf status <project>` | Xem stage hiện tại của project |
| `vf config` | In config hiệu lực (built-in + override) + hướng dẫn set key |

**Ngôn ngữ output:** `VF_LANG=vi` hoặc `VF_LANG=en`.

## Cấu hình LLM provider

File: `~/.config/vf/config.toml` (Linux; env `VF_CONFIG` override đường dẫn). API key không nằm trong file — chỉ tên biến môi trường.

**Built-in (cài là dùng được, không cần config):**
- `glm-coding` — Anthropic protocol, `https://api.z.ai/api/anthropic`, key `ZAI_CODING_API_KEY`, model `glm-5.2` *(GLM Coding Plan)*
- `glm` — OpenAI protocol, `https://api.z.ai/api/paas/v4`, key `ZAI_API_KEY` *(standard Z.ai API)*

```toml
# File này chỉ cần khi override hoặc thêm provider — file thắng built-in theo tên.
[routing]
spec = "glm-coding"     # provider cho giai đoạn sinh spec
codegen = "glm-coding"  # provider cho giai đoạn sinh code
```

### Chế độ offline / Online

Không có key: `vf make` chạy offline (spec mẫu + dry-run engine) và in cảnh báo.
Có key (`export ZAI_CODING_API_KEY=***`): spec được sinh bằng GLM, kèm token usage + ước tính chi phí sau mỗi run. LLM lỗi → tự retry (429/5xx/network) → fallback spec mẫu — **make không bao giờ chết vì LLM**.

> Ghi chú GLM Coding Plan: gói subscription của Z.ai giới hạn cho công cụ được hỗ trợ chính thức; vf dùng endpoint dạng "other tools" (best-effort) — bạn tự chịu trách nhiệm với điều khoản gói của mình. Xem [docs.z.ai/devpack/tool/others](https://docs.z.ai/devpack/tool/others).

### Render thật

Có `node` + `npx` là `vf make` render video thật (mặc định `VF_ENGINE=remotion`):
`npm install` (tự skip khi hash khớp) → `npx remotion render vf-main draft.mp4` → ffmpeg → `final.mp4`.

- Lần render đầu tải npm dependencies + Chrome Headless Shell (~150 MB)
- Thiếu `ffmpeg`? `final.mp4` là bản copy của `draft.mp4` (warning)
- Thiếu toolchain? vf in đúng lệnh manual, giữ nguyên artifacts, exit 0
- `VF_TSC=0` tắt validate TypeScript của scene LLM (mặc định chạy `npx tsc --noEmit`)

### Đổi engine

| Engine | `VF_ENGINE` | Cách hoạt động |
|---|---|---|
| Remotion (mặc định) | `remotion` | IR spec → React scaffold + custom scenes (TSX) → `npx remotion render` |
| HyperFrames | `hyperframes` | LLM viết composition HTML trực tiếp → `npx hyperframes lint` → `npx hyperframes render` |

Cả hai đều có: offline template fallback, make-never-dies, usage/cost reporting.

## Trạng thái (v0.3.0)

- ✅ Pipeline end-to-end với **dry-run engine** (mặc định): `video.spec.json` + scaffold + `draft.mp4` + `final.mp4`
- ✅ **LLM spec generation + hybrid codegen**: Z.ai GLM default; scene `custom` → LLM sinh component Remotion thật (validate `tsc`, fail → fallback template)
- ✅ **Render thật end-to-end**: scaffold Remotion project hoàn chỉnh (Player data-driven + custom registry) + `npm install` + `npx remotion render` → mp4 thật
- ✅ **HyperFrames engine** (delegate): LLM viết composition HTML → lint gate → render qua `npx hyperframes render`
- ✅ Skill manager: install từ git + lockfile pin; style presets; spec templates
- ✅ `vf doctor` (toolchain + TLS + network) / `vf status` / i18n VI-EN / cost guard (`--plan`, `--max-cost`)

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

Install a static musl binary from [Releases](https://github.com/phuongddx/vf/releases) or build with Rust ≥ 1.85 (`cargo build --release --locked`; TLS is on by default — use `--no-default-features` on hosts without a C compiler). Providers work out of the box: built-in `glm-coding` (Anthropic protocol, Z.ai Coding Plan) and `glm` (OpenAI protocol) profiles — export `ZAI_CODING_API_KEY` or `ZAI_API_KEY` to enable real LLM-backed spec generation AND hybrid scene codegen (custom scenes become LLM-written Remotion components, tsc-validated with template fallback); without a key, vf runs in offline template mode. With node + npx present, `vf make` renders a REAL mp4 end-to-end (npm install + npx remotion render; first run downloads Chrome Headless Shell ~150 MB). `vf make` never fails because of LLM or render problems — it degrades gracefully and prints manual commands. Override or add providers via `~/.config/vf/config.toml` (`VF_CONFIG` relocates the path). The demo videos above were both rendered by vf itself — the first through the Remotion engine, the second through the HyperFrames engine (`VF_ENGINE=hyperframes`: the LLM writes the HTML composition directly, linted and rendered via `npx hyperframes`).

vf code is MIT-licensed. Remotion (invoked via `npx` at runtime, not bundled) is free only for companies of up to 3 people — see their [license](https://www.remotion.dev/docs/license). HyperFrames is Apache-2.0.
