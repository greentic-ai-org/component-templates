#[path = "src/i18n_bundle.rs"]
mod i18n_bundle;

use std::env;
use std::fs;
use std::path::Path;

// Build-time embedding pipeline:
// 1) Read assets/i18n/*.json
// 2) Pack canonical CBOR bundle
// 3) Emit OUT_DIR constants included by src/i18n.rs
fn main() {
    let i18n_dir = Path::new("assets/i18n");
    let cargo_toml = Path::new("Cargo.toml");
    let component_manifest = Path::new("component.manifest.json");
    println!("cargo:rerun-if-changed={}", i18n_dir.display());
    println!("cargo:rerun-if-changed={}", cargo_toml.display());
    println!("cargo:rerun-if-changed={}", component_manifest.display());

    sync_manifest_version(component_manifest);

    let locales = i18n_bundle::load_locale_files(i18n_dir)
        .unwrap_or_else(|err| panic!("failed to load locale files: {err}"));
    let bundle = i18n_bundle::pack_locales_to_cbor(&locales)
        .unwrap_or_else(|err| panic!("failed to pack locale bundle: {err}"));

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let bundle_path = Path::new(&out_dir).join("i18n.bundle.cbor");
    fs::write(&bundle_path, bundle).expect("write i18n.bundle.cbor");

    let rs_path = Path::new(&out_dir).join("i18n_bundle.rs");
    fs::write(
        &rs_path,
        "pub const I18N_BUNDLE_CBOR: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/i18n.bundle.cbor\"));\n",
    )
    .expect("write i18n_bundle.rs");
}

fn sync_manifest_version(manifest_path: &Path) {
    let expected_version = env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION must be set");
    let manifest_text = fs::read_to_string(manifest_path).expect("read component.manifest.json");
    let mut manifest_json: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("component.manifest.json must be valid json");

    let current_version = manifest_json
        .get("version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if current_version == expected_version {
        return;
    }

    manifest_json["version"] = serde_json::Value::String(expected_version);
    let updated = serde_json::to_string_pretty(&manifest_json)
        .expect("serialize component.manifest.json")
        + "\n";
    fs::write(manifest_path, updated).expect("update component.manifest.json version");
}
