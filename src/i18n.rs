#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Vi,
}

impl Lang {
    /// VF_LANG env: "vi" → Vietnamese, anything else → English.
    pub fn from_env() -> Self {
        match std::env::var("VF_LANG").as_deref() {
            Ok("vi") => Self::Vi,
            _ => Self::En,
        }
    }
}

pub fn stage_ok(lang: Lang, stage: &str) -> String {
    match lang {
        Lang::En => format!("stage: {stage} ok"),
        Lang::Vi => format!("giai đoạn: {stage} xong"),
    }
}

pub fn project_line(lang: Lang, dir: &std::path::Path) -> String {
    match lang {
        Lang::En => format!("project: {}", dir.display()),
        Lang::Vi => format!("dự án: {}", dir.display()),
    }
}

pub fn config_hint(lang: Lang, key_env: &str) -> String {
    match lang {
        Lang::En => format!("hint: export {key_env}=... to enable LLM spec generation"),
        Lang::Vi => format!("gợi ý: export {key_env}=... để bật sinh spec bằng LLM"),
    }
}

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
        _ => match lang {
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

pub fn codegen_skip(lang: Lang) -> String {
    match lang {
        Lang::En => "codegen offline: no key — custom scenes render from template".into(),
        Lang::Vi => "codegen offline: chưa có key — scene custom render bằng template".into(),
    }
}

pub fn codegen_written(lang: Lang, n: usize) -> String {
    match lang {
        Lang::En => format!("codegen: wrote {n} custom scene component(s)"),
        Lang::Vi => format!("codegen: đã viết {n} component scene custom"),
    }
}

pub fn codegen_fallback(lang: Lang, scene_id: &str, reason: &str) -> String {
    match lang {
        Lang::En => format!("codegen: scene {scene_id} fell back to template ({reason})"),
        Lang::Vi => format!("codegen: scene {scene_id} dùng lại template ({reason})"),
    }
}

pub fn npm_install_running(lang: Lang) -> String {
    match lang {
        Lang::En => "npm install: installing Remotion dependencies…".into(),
        Lang::Vi => "npm install: đang cài dependencies Remotion…".into(),
    }
}

pub fn render_running(lang: Lang) -> String {
    match lang {
        Lang::En => "render: running npx remotion render…".into(),
        Lang::Vi => "render: đang chạy npx remotion render…".into(),
    }
}

pub fn render_skipped_manual(lang: Lang, cmd: &str) -> String {
    match lang {
        Lang::En => format!("render skipped (toolchain problem) — run manually in the project dir:\n  {cmd}"),
        Lang::Vi => format!("đã bỏ qua render (lỗi toolchain) — chạy thủ công trong thư mục dự án:\n  {cmd}"),
    }
}

pub fn chrome_first_run_note(lang: Lang) -> String {
    match lang {
        Lang::En => "note: first render downloads Chrome Headless Shell (~150 MB)".into(),
        Lang::Vi => "ghi chú: lần render đầu tải Chrome Headless Shell (~150 MB)".into(),
    }
}

pub fn ffmpeg_copy_fallback(lang: Lang) -> String {
    match lang {
        Lang::En => "ffmpeg missing — final.mp4 is a copy of draft.mp4".into(),
        Lang::Vi => "thiếu ffmpeg — final.mp4 là bản copy của draft.mp4".into(),
    }
}

pub fn codegen_usage_combined(
    lang: Lang,
    spec_in: u64,
    spec_out: u64,
    code_in: u64,
    code_out: u64,
    cost: Option<f64>,
) -> String {
    let cost_s = cost.map(|c| format!(" (~${c:.4})")).unwrap_or_default();
    match lang {
        Lang::En => format!("usage: spec {spec_in}/{spec_out} · codegen {code_in}/{code_out} tokens{cost_s}"),
        Lang::Vi => format!("tiêu thụ: spec {spec_in}/{spec_out} · codegen {code_in}/{code_out} token{cost_s}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translations_map() {
        assert_eq!(stage_ok(Lang::En, "render"), "stage: render ok");
        assert_eq!(stage_ok(Lang::Vi, "render"), "giai đoạn: render xong");
        assert!(project_line(Lang::Vi, std::path::Path::new("/p")).contains("dự án"));
        assert!(project_line(Lang::En, std::path::Path::new("/p")).contains("project"));
    }

    #[test]
    fn from_env_defaults_to_english() {
        // VF_LANG is not set in this test process
        assert_eq!(Lang::from_env(), Lang::En);
    }

    #[test]
    fn config_hint_bilingual() {
        assert!(config_hint(Lang::En, "ZAI_CODING_API_KEY").contains("export ZAI_CODING_API_KEY"));
        assert!(config_hint(Lang::Vi, "ZAI_CODING_API_KEY").contains("export ZAI_CODING_API_KEY"));
        assert!(config_hint(Lang::Vi, "X").contains("gợi ý"));
    }

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

    #[test]
    fn b2_strings_bilingual() {
        assert!(codegen_skip(Lang::Vi).contains("codegen offline"));
        assert!(codegen_written(Lang::En, 2).contains("2 custom scene"));
        assert!(codegen_fallback(Lang::Vi, "s1", "tsc").contains("s1"));
        assert!(npm_install_running(Lang::En).contains("npm install"));
        assert!(render_running(Lang::Vi).contains("render"));
        assert!(render_skipped_manual(Lang::En, "npx remotion render").contains("npx remotion render"));
        assert!(chrome_first_run_note(Lang::Vi).contains("Chrome"));
        assert!(ffmpeg_copy_fallback(Lang::En).contains("ffmpeg"));
        assert!(codegen_usage_combined(Lang::Vi, 10, 20, 30, 40, None).contains("spec 10/20 · codegen 30/40"));
        assert!(codegen_usage_combined(Lang::En, 1, 2, 3, 4, Some(0.5)).contains("~$0.5000"));
    }
}
