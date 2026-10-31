use crate::ir::Style;

pub struct StylePreset {
    pub name: &'static str,
    pub palette: &'static [&'static str],
    pub font: &'static str,
}

/// Built-in motion-graphics presets mg-01..mg-15 (dark base + accent pairs,
/// distinct typefaces). Default pipeline style: mg-07.
pub const PRESETS: &[StylePreset] = &[
    StylePreset { name: "mg-01", palette: &["#0F172A", "#38BDF8"], font: "Inter" },
    StylePreset { name: "mg-02", palette: &["#1E1B4B", "#A78BFA"], font: "Poppins" },
    StylePreset { name: "mg-03", palette: &["#052E16", "#4ADE80"], font: "Montserrat" },
    StylePreset { name: "mg-04", palette: &["#450A0A", "#F87171"], font: "Be Vietnam Pro" },
    StylePreset { name: "mg-05", palette: &["#0C4A6E", "#7DD3FC"], font: "Roboto" },
    StylePreset { name: "mg-06", palette: &["#422006", "#FBBF24"], font: "Oswald" },
    StylePreset { name: "mg-07", palette: &["#111827", "#E5E7EB"], font: "Raleway" },
    StylePreset { name: "mg-08", palette: &["#3B0764", "#C084FC"], font: "Nunito" },
    StylePreset { name: "mg-09", palette: &["#1C1917", "#D6D3D1"], font: "Playfair Display" },
    StylePreset { name: "mg-10", palette: &["#082F49", "#67E8F9"], font: "Source Sans 3" },
    StylePreset { name: "mg-11", palette: &["#4C0519", "#FB7185"], font: "Lato" },
    StylePreset { name: "mg-12", palette: &["#134E4A", "#2DD4BF"], font: "Rubik" },
    StylePreset { name: "mg-13", palette: &["#164E63", "#22D3EE"], font: "Work Sans" },
    StylePreset { name: "mg-14", palette: &["#3F0F0F", "#FDA4AF"], font: "Manrope" },
    StylePreset { name: "mg-15", palette: &["#1E293B", "#94A3B8"], font: "DM Sans" },
];

pub fn resolve(name: &str) -> Option<Style> {
    PRESETS.iter().find(|p| p.name == name).map(|p| Style {
        preset: p.name.to_string(),
        palette: p.palette.iter().map(|s| s.to_string()).collect(),
        font: p.font.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_and_rejects_unknown() {
        assert_eq!(resolve("mg-07").unwrap().preset, "mg-07");
        assert_eq!(resolve("mg-03").unwrap().font, "Montserrat");
        assert!(resolve("mg-99").is_none());
        assert!(resolve("").is_none());
    }

    #[test]
    fn has_fifteen_presets() {
        assert_eq!(PRESETS.len(), 15);
        for i in 1..=15 {
            assert!(resolve(&format!("mg-{i:02}")).is_some(), "missing mg-{i:02}");
        }
    }
}
