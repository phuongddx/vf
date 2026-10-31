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
}
