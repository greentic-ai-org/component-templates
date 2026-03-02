use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[test]
fn locales_and_en_assets_are_well_formed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let i18n_dir = root.join("assets/i18n");
    let locales_path = i18n_dir.join("locales.json");
    let en_path = i18n_dir.join("en.json");

    let locales_raw = fs::read_to_string(&locales_path).expect("read locales.json");
    let locales: Vec<String> = serde_json::from_str(&locales_raw).expect("parse locales.json");
    assert_eq!(locales.len(), 67, "unexpected locale count");
    assert!(locales.iter().any(|l| l == "fr-FR"), "fr-FR locale missing");
    assert!(locales.iter().any(|l| l == "nl-NL"), "nl-NL locale missing");

    let en_raw = fs::read_to_string(&en_path).expect("read en.json");
    let en_map: BTreeMap<String, String> = serde_json::from_str(&en_raw).expect("parse en.json");
    assert!(!en_map.is_empty(), "en.json must contain at least one key");
    for (key, value) in &en_map {
        assert!(!key.trim().is_empty(), "en.json contains empty key");
        assert!(
            !value.trim().is_empty(),
            "en.json key `{}` has empty translation",
            key
        );
    }

    for locale in locales {
        let file = i18n_dir.join(format!("{locale}.json"));
        assert!(file.exists(), "missing locale file: {}", file.display());
        let raw = fs::read_to_string(&file).expect("read locale file");
        let parsed: Value = serde_json::from_str(&raw).expect("parse locale json");
        assert!(
            parsed.is_object(),
            "locale file must contain a top-level object: {}",
            file.display()
        );
    }
}
