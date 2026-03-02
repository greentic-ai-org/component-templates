#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use std::collections::{BTreeMap, BTreeSet};

use handlebars::{Handlebars, RenderError};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use ciborium::value::Value as CborValue;
use greentic_types::ChannelMessageEnvelope;
use greentic_types::cbor::canonical;
use greentic_types::i18n_text::I18nText;
use greentic_types::schemas::common::schema_ir::{AdditionalProperties, SchemaIr};
use greentic_types::schemas::component::v0_6_0::{
    ComponentDescribe, ComponentInfo, ComponentOperation, ComponentQaSpec, ComponentRunInput,
    ComponentRunOutput, QaMode as QaModeSpec, Question,
    QuestionKind, schema_hash,
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

const DEFAULT_OUTPUT_PATH: &str = "text";
const SUPPORTED_OPERATION: &str = "text";
const COMPONENT_NAME: &str = "component-templates";
const COMPONENT_ORG: &str = "ai.greentic";
const COMPONENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const COMPONENT_ID: &str = "ai.greentic.component-templates";
const COMPONENT_ROLE: &str = "tool";

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

#[derive(Debug, Deserialize, Serialize)]
struct ComponentInvocation {
    config: Value,
    msg: ChannelMessageEnvelope,
    payload: Value,
    #[serde(default)]
    _connections: Vec<String>,
}

/// Try to parse input as `ComponentInvocation`. If that fails (e.g. flow YAML sends
/// a flat format with `template`/`output_path`/`wrap`/`msg` at the top level instead
/// of nested under `config.templates`), reconstruct the expected shape and retry.
fn parse_invocation_json(input: &str) -> Result<ComponentInvocation, InvokeFailure> {
    // Try the canonical format first.
    if let Ok(inv) = serde_json::from_str::<ComponentInvocation>(input) {
        return Ok(inv);
    }

    // Flat-format fallback: extract template/output_path/wrap from top level,
    // build config.templates, and fill in defaults for missing fields.
    let mut raw: Value =
        serde_json::from_str(input).map_err(|e| InvokeFailure::InvalidInput(e.to_string()))?;

    if let Some(obj) = raw.as_object_mut() {
        // If there's a `template` field at top level, it's the flat format.
        if obj.contains_key("template") {
            let text = obj
                .remove("template")
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            let output_path = obj
                .remove("output_path")
                .and_then(|v| v.as_str().map(String::from));
            let wrap = obj
                .remove("wrap")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let mut templates = serde_json::Map::new();
            templates.insert("text".into(), Value::String(text));
            templates.insert("wrap".into(), Value::Bool(wrap));
            if let Some(op) = output_path {
                templates.insert("output_path".into(), Value::String(op));
            }

            let config = json!({ "templates": Value::Object(templates) });
            obj.insert("config".into(), config);

            if !obj.contains_key("payload") {
                obj.insert("payload".into(), json!({}));
            }
        }
        // Runner InvocationEnvelope format: {ctx, flow_id, op, payload: [bytes], metadata: [bytes]}
        // Decode the binary payload and reconstruct ComponentInvocation.
        else if obj.contains_key("ctx")
            && obj.get("payload").map_or(false, Value::is_array)
        {
            let payload_bytes: Vec<u8> = obj
                .get("payload")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect())
                .unwrap_or_default();
            let decoded_payload: Value = serde_json::from_slice(&payload_bytes)
                .unwrap_or(Value::Object(Default::default()));

            // The decoded payload typically has {"text": "template..."} from flow mappings.
            let template_text = decoded_payload
                .get("text")
                .or_else(|| decoded_payload.get("template"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let output_path = decoded_payload
                .get("output_path")
                .and_then(Value::as_str)
                .map(String::from);
            let wrap = decoded_payload
                .get("wrap")
                .and_then(Value::as_bool)
                .unwrap_or(true);

            let mut templates = serde_json::Map::new();
            templates.insert("text".into(), Value::String(template_text));
            templates.insert("wrap".into(), Value::Bool(wrap));
            if let Some(op) = output_path {
                templates.insert("output_path".into(), Value::String(op));
            }

            let config = json!({ "templates": Value::Object(templates) });

            // Build msg from ctx for scope validation.
            let ctx_val = obj.get("ctx").cloned().unwrap_or(json!({}));
            let flow_id = obj
                .get("flow_id")
                .and_then(Value::as_str)
                .unwrap_or("flow")
                .to_string();
            let session_id = ctx_val
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or(&flow_id)
                .to_string();

            let msg = json!({
                "id": &flow_id,
                "tenant": ctx_val,
                "channel": "flow",
                "session_id": session_id,
                "text": "",
                "metadata": {}
            });

            obj.clear();
            obj.insert("config".into(), config);
            obj.insert("msg".into(), msg);
            obj.insert("payload".into(), decoded_payload);
        }
    }

    serde_json::from_value::<ComponentInvocation>(raw)
        .map_err(|e| InvokeFailure::InvalidInput(e.to_string()))
}

fn parse_invocation_cbor(bytes: &[u8]) -> Result<ComponentInvocation, InvokeFailure> {
    // Try canonical format first.
    if let Ok(inv) = decode_cbor::<ComponentInvocation>(bytes) {
        return Ok(inv);
    }

    // Fallback: decode as generic Value, transform, then re-parse.
    let raw: Value = decode_cbor(bytes)?;
    let json_str =
        serde_json::to_string(&raw).map_err(|e| InvokeFailure::InvalidInput(e.to_string()))?;
    parse_invocation_json(&json_str)
}

#[derive(Debug, Deserialize)]
struct TemplatesConfig {
    text: String,
    #[serde(default)]
    output_path: Option<String>,
    #[serde(default = "default_wrap")]
    wrap: bool,
    #[serde(default)]
    routing: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TemplateConfig {
    templates: TemplatesConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct ComponentError {
    kind: String,
    message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct ComponentResult {
    payload: Value,
    #[serde(default = "empty_object")]
    state_updates: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    control: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<ComponentError>,
}

#[derive(Debug)]
pub enum InvokeFailure {
    InvalidInput(String),
    InvalidScope(String),
    UnsupportedOperation(String),
}

impl core::fmt::Display for InvokeFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InvokeFailure::InvalidInput(msg) => write!(f, "{msg}"),
            InvokeFailure::InvalidScope(msg) => write!(f, "{msg}"),
            InvokeFailure::UnsupportedOperation(msg) => write!(f, "{msg}"),
        }
    }
}

fn default_wrap() -> bool {
    true
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}

fn encode_cbor<T: Serialize>(value: &T) -> Vec<u8> {
    canonical::to_canonical_cbor_allow_floats(value).expect("encode cbor")
}

fn decode_cbor<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, InvokeFailure> {
    canonical::from_cbor(bytes).map_err(|err| InvokeFailure::InvalidInput(err.to_string()))
}

fn string_schema(min_len: u64) -> SchemaIr {
    SchemaIr::String {
        min_len: Some(min_len),
        max_len: None,
        regex: None,
        format: None,
    }
}

fn bool_schema() -> SchemaIr {
    SchemaIr::Bool
}

fn config_schema_ir() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::from([(
            "templates".to_string(),
            SchemaIr::Object {
                properties: BTreeMap::from([
                    ("text".to_string(), string_schema(1)),
                    ("output_path".to_string(), string_schema(1)),
                    ("wrap".to_string(), bool_schema()),
                    ("routing".to_string(), string_schema(1)),
                ]),
                required: vec!["text".to_string()],
                additional: AdditionalProperties::Forbid,
            },
        )]),
        required: vec!["templates".to_string()],
        additional: AdditionalProperties::Forbid,
    }
}

fn message_schema_ir() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::from([
            ("id".to_string(), string_schema(1)),
            ("channel".to_string(), string_schema(1)),
            ("text".to_string(), string_schema(0)),
        ]),
        required: vec!["id".to_string(), "channel".to_string()],
        additional: AdditionalProperties::Allow,
    }
}

fn payload_schema_ir() -> SchemaIr {
    SchemaIr::OneOf {
        variants: vec![
            SchemaIr::Object {
                properties: BTreeMap::from([("text".to_string(), string_schema(0))]),
                required: Vec::new(),
                additional: AdditionalProperties::Allow,
            },
            SchemaIr::String {
                min_len: Some(0),
                max_len: None,
                regex: None,
                format: None,
            },
            SchemaIr::Int {
                min: Some(i64::MIN),
                max: Some(i64::MAX),
            },
            SchemaIr::Float {
                min: Some(f64::MIN),
                max: Some(f64::MAX),
            },
            SchemaIr::Bool,
            SchemaIr::Null,
            SchemaIr::Array {
                items: Box::new(string_schema(0)),
                min_items: None,
                max_items: None,
            },
        ],
    }
}

fn connections_schema_ir() -> SchemaIr {
    SchemaIr::Array {
        items: Box::new(string_schema(1)),
        min_items: Some(0),
        max_items: None,
    }
}

fn input_schema_ir() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::from([
            ("config".to_string(), config_schema_ir()),
            ("msg".to_string(), message_schema_ir()),
            ("payload".to_string(), payload_schema_ir()),
            ("connections".to_string(), connections_schema_ir()),
        ]),
        required: vec![
            "config".to_string(),
            "msg".to_string(),
            "payload".to_string(),
        ],
        additional: AdditionalProperties::Forbid,
    }
}

fn output_payload_schema_ir() -> SchemaIr {
    SchemaIr::OneOf {
        variants: vec![
            SchemaIr::Object {
                properties: BTreeMap::from([("text".to_string(), string_schema(0))]),
                required: Vec::new(),
                additional: AdditionalProperties::Allow,
            },
            SchemaIr::String {
                min_len: Some(0),
                max_len: None,
                regex: None,
                format: None,
            },
            SchemaIr::Null,
        ],
    }
}

fn output_schema_ir() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::from([
            ("payload".to_string(), output_payload_schema_ir()),
            (
                "state_updates".to_string(),
                SchemaIr::Object {
                    properties: BTreeMap::from([("state".to_string(), string_schema(0))]),
                    required: Vec::new(),
                    additional: AdditionalProperties::Allow,
                },
            ),
            (
                "control".to_string(),
                SchemaIr::Object {
                    properties: BTreeMap::from([("routing".to_string(), string_schema(1))]),
                    required: Vec::new(),
                    additional: AdditionalProperties::Allow,
                },
            ),
            (
                "error".to_string(),
                SchemaIr::Object {
                    properties: BTreeMap::from([
                        ("kind".to_string(), string_schema(1)),
                        ("message".to_string(), string_schema(1)),
                        (
                            "details".to_string(),
                            SchemaIr::Object {
                                properties: BTreeMap::from([(
                                    "info".to_string(),
                                    string_schema(0),
                                )]),
                                required: Vec::new(),
                                additional: AdditionalProperties::Allow,
                            },
                        ),
                    ]),
                    required: vec!["kind".to_string(), "message".to_string()],
                    additional: AdditionalProperties::Forbid,
                },
            ),
        ]),
        required: Vec::new(),
        additional: AdditionalProperties::Forbid,
    }
}

fn component_info() -> ComponentInfo {
    ComponentInfo {
        id: COMPONENT_ID.to_string(),
        version: COMPONENT_VERSION.to_string(),
        role: COMPONENT_ROLE.to_string(),
        display_name: Some(I18nText::new(
            "templates.display_name",
            Some("Templates".to_string()),
        )),
    }
}

fn component_describe() -> ComponentDescribe {
    let input_schema = input_schema_ir();
    let output_schema = output_schema_ir();
    let config_schema = config_schema_ir();
    let schema_hash =
        schema_hash(&input_schema, &output_schema, &config_schema).expect("schema hash");

    ComponentDescribe {
        info: component_info(),
        provided_capabilities: Vec::new(),
        required_capabilities: Vec::new(),
        metadata: BTreeMap::new(),
        operations: vec![ComponentOperation {
            id: SUPPORTED_OPERATION.to_string(),
            display_name: Some(I18nText::new(
                "templates.operation.text",
                Some("Render template text".to_string()),
            )),
            input: ComponentRunInput {
                schema: input_schema,
            },
            output: ComponentRunOutput {
                schema: output_schema,
            },
            defaults: BTreeMap::new(),
            redactions: Vec::new(),
            constraints: BTreeMap::new(),
            schema_hash,
        }],
        config_schema,
    }
}

fn component_info_cbor() -> Vec<u8> {
    encode_cbor(&component_info())
}

fn component_describe_cbor() -> Vec<u8> {
    encode_cbor(&component_describe())
}

fn input_schema_cbor() -> Vec<u8> {
    encode_cbor(&input_schema_ir())
}

fn output_schema_cbor() -> Vec<u8> {
    encode_cbor(&output_schema_ir())
}

fn config_schema_cbor() -> Vec<u8> {
    encode_cbor(&config_schema_ir())
}

fn qa_spec_for_mode(mode: QaModeSpec) -> ComponentQaSpec {
    let title = I18nText::new(
        "templates.qa.title",
        Some("Templates configuration".to_string()),
    );
    let question = Question {
        id: "templates.text".to_string(),
        label: I18nText::new("templates.qa.text.label", Some("Template text".to_string())),
        help: None,
        error: None,
        kind: QuestionKind::Text,
        required: true,
        default: Some(CborValue::Text("Hello {{name}}".to_string())),
    };

    ComponentQaSpec {
        mode,
        title,
        description: None,
        questions: vec![question],
        defaults: BTreeMap::new(),
    }
}

#[cfg(target_arch = "wasm32")]
fn qa_spec_cbor(mode: exports::greentic::component::component_qa::QaMode) -> Vec<u8> {
    let mapped = match mode {
        exports::greentic::component::component_qa::QaMode::Default => QaModeSpec::Default,
        exports::greentic::component::component_qa::QaMode::Setup => QaModeSpec::Setup,
        exports::greentic::component::component_qa::QaMode::Upgrade => QaModeSpec::Update,
        exports::greentic::component::component_qa::QaMode::Remove => QaModeSpec::Remove,
    };
    let spec = qa_spec_for_mode(mapped);
    encode_cbor(&spec)
}

#[cfg(target_arch = "wasm32")]
fn apply_answers_cbor(
    mode: exports::greentic::component::component_qa::QaMode,
    current_config: Vec<u8>,
    answers: Vec<u8>,
) -> Vec<u8> {
    let _ = mode;
    let current: Result<Value, _> = canonical::from_cbor(&current_config);
    let incoming: Result<Value, _> = canonical::from_cbor(&answers);
    let merged = match (current.ok(), incoming.ok()) {
        (_, Some(value @ Value::Object(_))) => value,
        (Some(value @ Value::Object(_)), _) => value,
        _ => Value::Object(Map::new()),
    };
    encode_cbor(&merged)
}

fn i18n_keys() -> Vec<String> {
    let mut keys = BTreeSet::new();
    for mode in [
        QaModeSpec::Default,
        QaModeSpec::Setup,
        QaModeSpec::Update,
        QaModeSpec::Remove,
    ] {
        let spec = qa_spec_for_mode(mode);
        for key in spec.i18n_keys() {
            keys.insert(key);
        }
    }
    keys.into_iter().collect()
}

fn run_component_cbor(input: Vec<u8>, _state: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    let invocation = parse_invocation_cbor(&input);
    let result = match invocation {
        Ok(invocation) => {
            invoke_template_from_invocation(invocation).unwrap_or_else(|err| ComponentResult {
                payload: Value::Null,
                state_updates: empty_object(),
                control: None,
                error: Some(ComponentError {
                    kind: "InvalidInput".to_string(),
                    message: err.to_string(),
                    details: None,
                }),
            })
        }
        Err(err) => ComponentResult {
            payload: Value::Null,
            state_updates: empty_object(),
            control: None,
            error: Some(ComponentError {
                kind: "InvalidInput".to_string(),
                message: err.to_string(),
                details: None,
            }),
        },
    };
    (encode_cbor(&result), encode_cbor(&empty_object()))
}

/// Returns the component manifest JSON payload.
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

/// Entry point used by both sync and streaming invocations.
pub fn invoke_template(_operation: &str, input: &str) -> Result<String, InvokeFailure> {
    if _operation != SUPPORTED_OPERATION && _operation != "handlebars" {
        return Err(InvokeFailure::UnsupportedOperation(format!(
            "operation `{}` is not supported; use `{}` or `handlebars`",
            _operation, SUPPORTED_OPERATION
        )));
    }

    let invocation = parse_invocation_json(input)?;

    let result = invoke_template_from_invocation(invocation)?;

    serde_json::to_string(&result).map_err(|err| InvokeFailure::InvalidInput(err.to_string()))
}

fn invoke_template_from_invocation(
    invocation: ComponentInvocation,
) -> Result<ComponentResult, InvokeFailure> {
    ensure_scope(&invocation.msg)?;

    let config: TemplateConfig = serde_json::from_value(invocation.config.clone())
        .map_err(|err| InvokeFailure::InvalidInput(err.to_string()))?;

    let context = build_context(&invocation);
    let outcome = render_template(&config, &context);

    let result = match outcome {
        Ok(rendered) => ComponentResult {
            payload: build_payload(&rendered, &config),
            state_updates: empty_object(),
            control: build_control(&config),
            error: None,
        },
        Err(err) => ComponentResult {
            payload: Value::Null,
            state_updates: empty_object(),
            control: None,
            error: Some(err.into_component_error()),
        },
    };

    Ok(result)
}

fn build_context(invocation: &ComponentInvocation) -> Value {
    let msg_value = serde_json::to_value(&invocation.msg).unwrap_or(Value::Null);
    let mut root = Map::new();
    root.insert("msg".to_owned(), msg_value);
    root.insert("payload".to_owned(), invocation.payload.clone());
    root.insert(
        "payload_json".to_owned(),
        Value::String(serde_json::to_string(&invocation.payload).unwrap_or_default()),
    );

    Value::Object(root)
}

fn render_template(config: &TemplateConfig, context: &Value) -> Result<String, TemplateError> {
    let mut engine = Handlebars::new();
    engine.set_strict_mode(false);
    let template = normalize_template(&config.templates.text);
    engine
        .render_template(&template, context)
        .map_err(TemplateError::from_render_error)
}

fn build_payload(rendered: &str, config: &TemplateConfig) -> Value {
    if !config.templates.wrap {
        return Value::String(rendered.to_owned());
    }

    let path = config
        .templates
        .output_path
        .as_deref()
        .filter(|path| !path.is_empty())
        .unwrap_or(DEFAULT_OUTPUT_PATH);
    nest_payload(path, rendered)
}

fn nest_payload(path: &str, rendered: &str) -> Value {
    let mut value = Value::String(rendered.to_owned());
    for segment in path.split('.').rev().filter(|segment| !segment.is_empty()) {
        let mut map = Map::new();
        map.insert(segment.to_owned(), value);
        value = Value::Object(map);
    }
    value
}

fn normalize_template(raw: &str) -> String {
    let mut normalized = raw.to_owned();
    let replacements = [
        ("{{{ payload }}}", "{{{payload_json}}}"),
        ("{{{payload}}}", "{{{payload_json}}}"),
        ("{{ payload }}", "{{payload_json}}"),
        ("{{payload}}", "{{payload_json}}"),
    ];

    for (from, to) in replacements {
        normalized = normalized.replace(from, to);
    }

    normalized
}

fn build_control(config: &TemplateConfig) -> Option<Value> {
    let routing = config
        .templates
        .routing
        .clone()
        .filter(|route| !route.trim().is_empty())
        .unwrap_or_else(|| "out".to_string());
    Some(json!({ "routing": routing }))
}

fn ensure_scope(msg: &ChannelMessageEnvelope) -> Result<(), InvokeFailure> {
    let tenant = msg.tenant.clone();
    let tenant_id = tenant.tenant.to_string();
    let env_id = tenant.env.to_string();
    let session_id = msg.session_id.clone();

    if tenant_id.is_empty() || env_id.is_empty() || session_id.is_empty() {
        return Err(InvokeFailure::InvalidScope(
            "missing scope identifiers (tenant/env/session)".to_string(),
        ));
    }

    Ok(())
}

#[derive(Debug)]
struct TemplateError {
    message: String,
    details: Option<Value>,
}

impl TemplateError {
    fn from_render_error(err: RenderError) -> Self {
        let mut details = Map::new();
        details.insert("error".to_owned(), Value::String(err.to_string()));
        if let Some(line) = err.line_no {
            details.insert("line".to_owned(), Value::Number(line.into()));
        }
        if let Some(column) = err.column_no {
            details.insert("column".to_owned(), Value::Number(column.into()));
        }
        Self {
            message: err.to_string(),
            details: Some(Value::Object(details)),
        }
    }

    fn into_component_error(self) -> ComponentError {
        ComponentError {
            kind: "TemplateError".to_owned(),
            message: self.message,
            details: self.details,
        }
    }
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
            id: "templates.text".to_string(),
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
    if let Some(value) = map
        .get("config")
        .and_then(|v| v.as_object())
        .and_then(|v| v.get("templates"))
        .and_then(|v| v.as_object())
        .and_then(|v| v.get("text"))
        .and_then(|v| v.as_str())
    {
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
    // Compatibility: older flows may send a wrapped object like
    // { "component": "...", "config": { ... } }. Unwrap to preserve the
    // component config contract expected by schema validation.
    let normalized_current_config = current_config
        .as_object()
        .and_then(|map| map.get("config"))
        .and_then(|value| value.as_object())
        .map(|map| serde_json::Value::Object(map.clone()))
        .unwrap_or(current_config);

    let mut config = match normalized_current_config {
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
    use core::convert::TryFrom;

    use super::*;
    use greentic_types::cbor::canonical;
    use greentic_types::schemas::component::v0_6_0::ComponentQaSpec;

    fn sample_invocation(template: &str, payload: Value) -> ComponentInvocation {
        let mut tenant_ctx = greentic_types::TenantCtx::new(
            greentic_types::EnvId::try_from("dev").unwrap(),
            greentic_types::TenantId::try_from("tenant").unwrap(),
        );
        tenant_ctx.session_id = Some("session-1".to_string());

        ComponentInvocation {
            config: json!({ "templates": { "text": template } }),
            msg: ChannelMessageEnvelope {
                id: "msg-1".to_string(),
                tenant: tenant_ctx,
                channel: "chat".to_string(),
                session_id: "session-1".to_string(),
                reply_scope: None,
                from: None,
                to: Vec::new(),
                correlation_id: None,
                text: Some("hello".to_string()),
                attachments: Vec::new(),
                metadata: Default::default(),
            },
            payload,
            _connections: Vec::new(),
        }
    }

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
        assert_eq!(first.id, "templates.text");
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
            assert_eq!(question.id, "templates.text");
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

    #[test]
    fn template_error_is_reported() {
        let invocation = sample_invocation("{{#if}}", json!({}));

        let result = invoke_template(
            SUPPORTED_OPERATION,
            &serde_json::to_string(&invocation).expect("serialize invocation"),
        )
        .expect("invoke should succeed");

        let json: Value = serde_json::from_str(&result).expect("result json");
        assert!(json["payload"].is_null());
        assert_eq!(json["error"]["kind"], "TemplateError");
        assert!(json["state_updates"].as_object().unwrap().is_empty());
    }

    #[test]
    fn supports_output_path_and_wrap_toggle() {
        let invocation = ComponentInvocation {
            config: json!({ "templates": { "text": "Hi", "output_path": "reply.body", "wrap": true } }),
            ..sample_invocation("unused", json!({}))
        };

        let result = invoke_template(
            SUPPORTED_OPERATION,
            &serde_json::to_string(&invocation).expect("serialize invocation"),
        )
        .expect("invoke should succeed");

        let json: Value = serde_json::from_str(&result).expect("result json");
        assert_eq!(json["payload"], json!({ "reply": { "body": "Hi" } }));

        let raw_invocation = ComponentInvocation {
            config: json!({ "templates": { "text": "Hi", "wrap": false } }),
            ..sample_invocation("unused", json!({}))
        };

        let raw_result = invoke_template(
            SUPPORTED_OPERATION,
            &serde_json::to_string(&raw_invocation).expect("serialize invocation"),
        )
        .expect("invoke should succeed");

        let raw_json: Value = serde_json::from_str(&raw_result).expect("result json");
        assert_eq!(raw_json["payload"], json!("Hi"));
    }

    #[test]
    fn explicit_payload_stays_explicit() {
        let invocation = sample_invocation("{{payload.name}}", json!({ "name": "PayloadName" }));

        let result = invoke_template(
            SUPPORTED_OPERATION,
            &serde_json::to_string(&invocation).expect("serialize invocation"),
        )
        .expect("invoke");

        let json: Value = serde_json::from_str(&result).expect("result json");
        assert_eq!(json["payload"]["text"], "PayloadName");
    }

    #[test]
    fn debug_stringification_is_compact() {
        let invocation =
            sample_invocation("payload={{payload}}", json!({ "foo": "bar", "count": 2 }));

        let result = invoke_template(
            SUPPORTED_OPERATION,
            &serde_json::to_string(&invocation).expect("serialize invocation"),
        )
        .expect("invoke");

        let json: Value = serde_json::from_str(&result).expect("result json");
        let rendered = json["payload"]["text"]
            .as_str()
            .expect("rendered string")
            .to_string();

        assert!(rendered.contains("payload={"));
        assert!(rendered.contains("&quot;foo&quot;:&quot;bar&quot;"));
        assert!(rendered.contains("&quot;count&quot;:2"));
        assert!(!rendered.contains('\n'));
    }

    #[test]
    fn missing_scope_fails_closed() {
        let mut invocation = sample_invocation("{{payload.name}}", json!({ "name": "x" }));
        invocation.msg.session_id = "".to_string();

        let result = invoke_template(
            SUPPORTED_OPERATION,
            &serde_json::to_string(&invocation).expect("serialize invocation"),
        );

        assert!(matches!(result, Err(InvokeFailure::InvalidScope(_))));
    }

    #[test]
    fn different_scopes_do_not_leak() {
        let inv_a = sample_invocation("{{payload.name}}", json!({ "name": "TenantA" }));
        let mut inv_b = sample_invocation("{{payload.name}}", json!({ "name": "TenantB" }));
        inv_b.msg.tenant = {
            let mut t = greentic_types::TenantCtx::new(
                greentic_types::EnvId::try_from("prod").unwrap(),
                greentic_types::TenantId::try_from("tenant-b").unwrap(),
            );
            t.session_id = Some("session-2".to_string());
            t
        };
        inv_b.msg.session_id = "session-2".to_string();

        let res_a = invoke_template(SUPPORTED_OPERATION, &serde_json::to_string(&inv_a).unwrap())
            .expect("invoke");
        let res_b = invoke_template(SUPPORTED_OPERATION, &serde_json::to_string(&inv_b).unwrap())
            .expect("invoke");

        let json_a: Value = serde_json::from_str(&res_a).unwrap();
        let json_b: Value = serde_json::from_str(&res_b).unwrap();
        assert_eq!(json_a["payload"]["text"], "TenantA");
        assert_eq!(json_b["payload"]["text"], "TenantB");
    }

    #[test]
    fn rejects_unsupported_operation() {
        let invocation = sample_invocation("Hi", json!({}));
        let result = invoke_template("unknown_op", &serde_json::to_string(&invocation).unwrap());
        assert!(matches!(
            result,
            Err(InvokeFailure::UnsupportedOperation(_))
        ));
    }

    #[test]
    fn apply_answers_unwraps_legacy_wrapped_component_config_shape() {
        let current = serde_json::json!({
            "component": "ai.greentic.component-templates",
            "config": {
                "templates": {
                    "text": "Old value",
                    "output_path": "text"
                }
            }
        });

        let updated = apply_template_answers(current, serde_json::json!({ "text": "New value" }));
        assert_eq!(updated["templates"]["text"], "New value");
        assert_eq!(updated["templates"]["output_path"], "text");
        assert!(updated.get("component").is_none());
        assert!(updated.get("config").is_none());
    }

    #[test]
    fn invocation_envelope_format_works() {
        // Runner wraps flow input in InvocationEnvelope with binary payload bytes.
        let template = "Setup complete for {{msg.channel}}.";
        let payload_json = json!({"text": template});
        let payload_bytes: Vec<u8> = serde_json::to_vec(&payload_json).unwrap();
        let payload_arr: Vec<Value> = payload_bytes.iter().map(|b| json!(*b as u64)).collect();

        let envelope_input = json!({
            "ctx": {
                "env": "dev",
                "tenant": "default",
                "tenant_id": "default",
                "session_id": "session-1",
                "attempt": 0
            },
            "flow_id": "setup_default",
            "node_id": "collect_inputs",
            "op": "handlebars",
            "payload": payload_arr,
            "metadata": [110, 117, 108, 108]
        });

        let result = invoke_template(
            SUPPORTED_OPERATION,
            &serde_json::to_string(&envelope_input).unwrap(),
        )
        .expect("InvocationEnvelope format should work");

        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            parsed["payload"]["text"],
            "Setup complete for flow."
        );
        assert!(parsed["error"].is_null());
    }

    #[test]
    fn flat_format_fallback_works() {
        // Flow YAML sends template/output_path/wrap at top level (not nested in config)
        let flat_input = json!({
            "msg": {
                "id": "msg-1",
                "tenant": { "env": "dev", "tenant": "tenant", "tenant_id": "tenant", "session_id": "session-1", "attempt": 0 },
                "channel": "setup",
                "session_id": "session-1",
                "text": "hello",
                "metadata": {}
            },
            "template": "Setup complete for {{msg.channel}}.",
            "output_path": "text",
            "wrap": true
        });

        let result = invoke_template(
            SUPPORTED_OPERATION,
            &serde_json::to_string(&flat_input).unwrap(),
        )
        .expect("flat format should work");

        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            parsed["payload"]["text"],
            "Setup complete for setup."
        );
        assert!(parsed["error"].is_null());
    }
}
