use std::path::Path;

#[test]
fn component_manifest_version_matches_cargo_package_version() {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("component.manifest.json");
    let manifest_text = std::fs::read_to_string(&manifest_path).expect("read component manifest");
    let manifest_json: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("component manifest json");

    let component_manifest_version = manifest_json["version"]
        .as_str()
        .expect("component manifest version string");
    assert_eq!(component_manifest_version, env!("CARGO_PKG_VERSION"));
}
