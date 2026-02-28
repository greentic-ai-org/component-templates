use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let locales_path = manifest_dir.join("assets/i18n/locales.json");
    println!("cargo:rerun-if-changed={}", locales_path.display());

    let locales_raw = fs::read_to_string(&locales_path).expect("read locales.json");
    let mut locales: Vec<String> = serde_json::from_str(&locales_raw).expect("parse locales.json");
    if !locales.iter().any(|l| l == "en") {
        locales.push("en".to_string());
    }

    let mut generated = String::new();
    generated.push_str("pub(crate) fn supported_locales() -> &'static [&'static str] {\n    &[\n");
    for locale in &locales {
        generated.push_str(&format!("        \"{}\",\n", locale));
        println!("cargo:rerun-if-changed=assets/i18n/{}.json", locale);
    }
    generated.push_str("    ]\n}\n\n");
    generated.push_str("pub(crate) fn locale_json(locale: &str) -> Option<&'static str> {\n");
    generated.push_str("    match locale {\n");
    for locale in &locales {
        generated.push_str(&format!(
            "        \"{0}\" => Some(include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/i18n/{0}.json\"))),\n",
            locale
        ));
    }
    generated.push_str("        _ => None,\n");
    generated.push_str("    }\n");
    generated.push_str("}\n");

    let out_path = PathBuf::from(std::env::var("OUT_DIR").expect("out dir")).join("i18n_bundle.rs");
    fs::write(out_path, generated).expect("write generated i18n bundle");
}
