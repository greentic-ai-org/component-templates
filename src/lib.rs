#[cfg(target_arch = "wasm32")]
use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::BTreeMap;

#[cfg(target_arch = "wasm32")]
use greentic_types::cbor::canonical;
#[cfg(any(target_arch = "wasm32", test))]
use greentic_types::i18n_text::I18nText;
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::common::schema_ir::{AdditionalProperties, SchemaIr};
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::component::v0_6_0::{
    ComponentDescribe, ComponentInfo, ComponentOperation, ComponentRunInput, ComponentRunOutput,
    schema_hash,
};
#[cfg(any(target_arch = "wasm32", test))]
use greentic_types::schemas::component::v0_6_0::{
    ComponentQaSpec, QaMode as QaModeSpec, Question, QuestionKind,
};
#[cfg(target_arch = "wasm32")]
mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "component-v0-v6-v0",
    });
}
#[cfg(target_arch = "wasm32")]
use bindings::exports::greentic::component::{
    component_descriptor, component_i18n,
    component_qa::{self, QaMode},
    component_runtime, component_schema,
};

pub mod i18n;
pub mod i18n_bundle;
pub mod qa;

const COMPONENT_NAME: &str = "component-templates";
const COMPONENT_ORG: &str = "ai.greentic";
const COMPONENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(target_arch = "wasm32")]
#[used]
#[unsafe(link_section = ".greentic.wasi")]
static WASI_TARGET_MARKER: [u8; 13] = *b"wasm32-wasip2";

#[cfg(target_arch = "wasm32")]
struct Component;

#[cfg(target_arch = "wasm32")]
impl component_descriptor::Guest for Component {
    fn get_component_info() -> Vec<u8> {
        component_info_cbor()
    }

    fn describe() -> Vec<u8> {
        component_describe_cbor()
    }
}

#[cfg(target_arch = "wasm32")]
impl component_schema::Guest for Component {
    fn input_schema() -> Vec<u8> {
        input_schema_cbor()
    }

    fn output_schema() -> Vec<u8> {
        output_schema_cbor()
    }

    fn config_schema() -> Vec<u8> {
        config_schema_cbor()
    }
}

#[cfg(target_arch = "wasm32")]
impl component_runtime::Guest for Component {
    fn run(input: Vec<u8>, state: Vec<u8>) -> component_runtime::RunResult {
        let value = parse_payload(&input);
        let input_text = value
            .get("input")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| value.to_string());

        let output = serde_json::json!({
            "message": handle_message("handle_message", &input_text)
        });

        component_runtime::RunResult {
            output: encode_cbor(&output),
            new_state: state,
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl component_qa::Guest for Component {
    fn qa_spec(mode: QaMode) -> Vec<u8> {
        let mode_key = mode_key(mode);
        encode_cbor(&qa_spec_payload(mode_key))
    }

    fn apply_answers(mode: QaMode, current_config: Vec<u8>, answers: Vec<u8>) -> Vec<u8> {
        let _ = mode;
        let updated =
            apply_template_answers(parse_payload(&current_config), parse_payload(&answers));
        encode_cbor(&updated)
    }
}

#[cfg(target_arch = "wasm32")]
impl component_i18n::Guest for Component {
    fn i18n_keys() -> Vec<String> {
        i18n::all_keys()
    }
}

#[cfg(target_arch = "wasm32")]
bindings::export!(Component with_types_in bindings);

pub fn describe_payload() -> String {
    serde_json::json!({
        "component": {
            "name": COMPONENT_NAME,
            "org": COMPONENT_ORG,
            "version": COMPONENT_VERSION,
            "world": "greentic:component/component@0.6.0",
            "schemas": {
                "component": "schemas/component.schema.json",
                "input": "schemas/io/input.schema.json",
                "output": "schemas/io/output.schema.json"
            }
        }
    })
    .to_string()
}

pub fn handle_message(operation: &str, input: &str) -> String {
    format!("{COMPONENT_NAME}::{operation} => {}", input.trim())
}

#[cfg(any(target_arch = "wasm32", test))]
fn qa_spec_payload(mode_key: &str) -> ComponentQaSpec {
    let mode = match mode_key {
        "default" => QaModeSpec::Default,
        "setup" => QaModeSpec::Setup,
        "update" => QaModeSpec::Update,
        "remove" => QaModeSpec::Remove,
        _ => QaModeSpec::Default,
    };
    let asks_template_text = matches!(mode_key, "default" | "setup" | "update");
    let required = matches!(mode_key, "default" | "setup");
    let questions = if asks_template_text {
        vec![Question {
            id: "text".to_string(),
            label: I18nText::new("qa.text.label", None),
            help: None,
            error: None,
            kind: QuestionKind::Text,
            required,
            default: None,
        }]
    } else {
        Vec::new()
    };

    ComponentQaSpec {
        mode,
        title: I18nText::new(format!("qa.{mode_key}.title"), None),
        description: Some(I18nText::new(format!("qa.{mode_key}.description"), None)),
        questions,
        defaults: BTreeMap::new(),
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn extract_template_text_answer(answers: &serde_json::Value) -> Option<String> {
    if let Some(value) = answers.as_str() {
        return Some(value.to_string());
    }
    let map = answers.as_object()?;

    if let Some(value) = map.get("text").and_then(|v| v.as_str()) {
        return Some(value.to_string());
    }
    if let Some(value) = map.get("template").and_then(|v| v.as_str()) {
        return Some(value.to_string());
    }
    if let Some(value) = map.get("templates.text").and_then(|v| v.as_str()) {
        return Some(value.to_string());
    }
    map.get("templates")
        .and_then(|v| v.as_object())
        .and_then(|v| v.get("text"))
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
}

#[cfg(any(target_arch = "wasm32", test))]
fn apply_template_answers(
    current_config: serde_json::Value,
    answers: serde_json::Value,
) -> serde_json::Value {
    let mut config = match current_config {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };

    if let Some(text) = extract_template_text_answer(&answers) {
        let mut templates = match config.remove("templates") {
            Some(serde_json::Value::Object(map)) => map,
            _ => serde_json::Map::new(),
        };
        templates.insert("text".to_string(), serde_json::Value::String(text));
        config.insert(
            "templates".to_string(),
            serde_json::Value::Object(templates),
        );
    }

    serde_json::Value::Object(config)
}

#[cfg(target_arch = "wasm32")]
fn encode_cbor<T: serde::Serialize>(value: &T) -> Vec<u8> {
    canonical::to_canonical_cbor_allow_floats(value).expect("encode cbor")
}

#[cfg(target_arch = "wasm32")]
fn parse_payload(input: &[u8]) -> serde_json::Value {
    if let Ok(value) = canonical::from_cbor(input) {
        return value;
    }
    serde_json::from_slice(input).unwrap_or_else(|_| serde_json::json!({}))
}

#[cfg(target_arch = "wasm32")]
fn mode_key(mode: QaMode) -> &'static str {
    match mode {
        QaMode::Default => "default",
        QaMode::Setup => "setup",
        QaMode::Update => "update",
        QaMode::Remove => "remove",
    }
}

#[cfg(target_arch = "wasm32")]
fn input_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::from([(
            "input".to_string(),
            SchemaIr::String {
                min_len: Some(0),
                max_len: None,
                regex: None,
                format: None,
            },
        )]),
        required: vec!["input".to_string()],
        additional: AdditionalProperties::Allow,
    }
}

#[cfg(target_arch = "wasm32")]
fn output_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::from([(
            "message".to_string(),
            SchemaIr::String {
                min_len: Some(0),
                max_len: None,
                regex: None,
                format: None,
            },
        )]),
        required: vec!["message".to_string()],
        additional: AdditionalProperties::Allow,
    }
}

#[cfg(target_arch = "wasm32")]
fn config_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::from([(
            "templates".to_string(),
            SchemaIr::Object {
                properties: BTreeMap::from([(
                    "text".to_string(),
                    SchemaIr::String {
                        min_len: Some(0),
                        max_len: None,
                        regex: None,
                        format: None,
                    },
                )]),
                required: vec!["text".to_string()],
                additional: AdditionalProperties::Allow,
            },
        )]),
        required: Vec::new(),
        additional: AdditionalProperties::Allow,
    }
}

#[cfg(target_arch = "wasm32")]
fn component_info() -> ComponentInfo {
    ComponentInfo {
        id: format!("{COMPONENT_ORG}.{COMPONENT_NAME}"),
        version: COMPONENT_VERSION.to_string(),
        role: "tool".to_string(),
        display_name: Some(I18nText::new(
            "component.display_name",
            Some(COMPONENT_NAME.to_string()),
        )),
    }
}

#[cfg(target_arch = "wasm32")]
fn component_describe() -> ComponentDescribe {
    let input = input_schema();
    let output = output_schema();
    let config = config_schema();
    let op_schema_hash = schema_hash(&input, &output, &config).unwrap_or_default();

    ComponentDescribe {
        info: component_info(),
        provided_capabilities: Vec::new(),
        required_capabilities: Vec::new(),
        metadata: BTreeMap::new(),
        operations: vec![ComponentOperation {
            id: "handle_message".to_string(),
            display_name: Some(I18nText::new("component.operation.handle_message", None)),
            input: ComponentRunInput { schema: input },
            output: ComponentRunOutput { schema: output },
            defaults: BTreeMap::new(),
            redactions: Vec::new(),
            constraints: BTreeMap::new(),
            schema_hash: op_schema_hash,
        }],
        config_schema: config,
    }
}

#[cfg(target_arch = "wasm32")]
fn component_info_cbor() -> Vec<u8> {
    encode_cbor(&component_info())
}

#[cfg(target_arch = "wasm32")]
fn component_describe_cbor() -> Vec<u8> {
    encode_cbor(&component_describe())
}

#[cfg(target_arch = "wasm32")]
fn input_schema_cbor() -> Vec<u8> {
    encode_cbor(&input_schema())
}

#[cfg(target_arch = "wasm32")]
fn output_schema_cbor() -> Vec<u8> {
    encode_cbor(&output_schema())
}

#[cfg(target_arch = "wasm32")]
fn config_schema_cbor() -> Vec<u8> {
    encode_cbor(&config_schema())
}

#[cfg(test)]
mod tests {
    use super::*;
    use greentic_types::cbor::canonical;
    use greentic_types::schemas::component::v0_6_0::ComponentQaSpec;

    #[test]
    fn describe_payload_is_json() {
        let payload = describe_payload();
        let json: serde_json::Value = serde_json::from_str(&payload).expect("valid json");
        assert_eq!(json["component"]["name"], "component-templates");
    }

    #[test]
    fn handle_message_round_trips() {
        let body = handle_message("handle", "demo");
        assert!(body.contains("demo"));
    }

    #[test]
    fn qa_spec_default_includes_text_question() {
        let spec = qa_spec_payload("default");
        let first = spec.questions.first().expect("text question");
        assert_eq!(first.id, "text");
        assert_eq!(first.label.key, "qa.text.label");
    }

    #[test]
    fn qa_spec_modes_round_trip_as_canonical_cbor() {
        for (mode, expected_required) in [("default", true), ("setup", true), ("update", false)] {
            let spec = qa_spec_payload(mode);
            let cbor = canonical::to_canonical_cbor_allow_floats(&spec).expect("encode cbor");
            let decoded: ComponentQaSpec = canonical::from_cbor(&cbor).expect("decode cbor");
            let question = decoded.questions.first().expect("text question");

            assert_eq!(decoded.mode.to_string(), mode);
            assert_eq!(question.id, "text");
            assert_eq!(question.required, expected_required);
            assert_eq!(question.label.key, "qa.text.label");
        }
    }

    #[test]
    fn apply_answers_sets_templates_text() {
        let current = serde_json::json!({
            "templates": {
                "output_path": "text"
            }
        });
        let answers = serde_json::json!({ "text": "Hi {{name}}" });

        let updated = apply_template_answers(current, answers);
        assert_eq!(updated["templates"]["text"], "Hi {{name}}");
        assert_eq!(updated["templates"]["output_path"], "text");
    }

    #[test]
    fn apply_answers_supports_nested_templates_text() {
        let updated = apply_template_answers(
            serde_json::json!({}),
            serde_json::json!({ "templates": { "text": "Hello {{name}}" } }),
        );
        assert_eq!(updated["templates"]["text"], "Hello {{name}}");
    }

    #[test]
    fn apply_answers_leaves_existing_text_when_no_answer_is_present() {
        let current = serde_json::json!({
            "templates": {
                "text": "Existing value",
                "output_path": "text"
            }
        });
        let updated = apply_template_answers(current.clone(), serde_json::json!({}));
        assert_eq!(updated["templates"]["text"], "Existing value");
        assert_eq!(updated["templates"]["output_path"], "text");
        assert_eq!(updated, current);
    }
}
