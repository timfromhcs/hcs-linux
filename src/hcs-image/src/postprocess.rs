//! Image optimization & metadata tagging (post inference).

use std::collections::BTreeMap;

/// Deterministic metadata block embedded alongside the PNG (sidecar-safe).
pub fn tag_png_metadata(
    prompt: &str,
    seed: i64,
    steps: u32,
    model: &str,
) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert(
        "HCS-Generator".into(),
        "hcs-image/1.0.0 (sd.cpp LCM Q4)".into(),
    );
    m.insert("HCS-Prompt".into(), prompt.to_string());
    m.insert("HCS-Seed".into(), seed.to_string());
    m.insert("HCS-Steps".into(), steps.to_string());
    m.insert("HCS-Model".into(), model.to_string());
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_metadata_tags() {
        let m = tag_png_metadata("neon glass", 42, 6, "sd15-lcm-q4_0");
        assert_eq!(m["HCS-Seed"], "42");
        assert_eq!(m["HCS-Steps"], "6");
    }
}
