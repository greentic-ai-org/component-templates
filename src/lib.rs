#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use handlebars::{Handlebars, RenderError};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

use ciborium::value::Value as CborValue;
use greentic_types::ChannelMessageEnvelope;
use greentic_types::cbor::canonical;
use greentic_types::schemas::common::schema_ir::{AdditionalProperties, SchemaIr};
use greentic_types::schemas::component::v0_6_0::{
    ComponentDescribe, ComponentInfo, ComponentOperation, ComponentQaSpec, ComponentRunInput,
    ComponentRunOutput, I18nText, QaMode as ComponentQaMode, Question, QuestionKind, schema_hash,
};

const DEFAULT_OUTPUT_PATH: &str = "text";
const SUPPORTED_OPERATION: &str = "text";
const COMPONENT_NAME: &str = "templates";
const COMPONENT_ORG: &str = "ai.greentic";
const COMPONENT_VERSION: &str = "0.1.2";
const COMPONENT_ID: &str = "ai.greentic.component-templates";
const COMPONENT_ROLE: &str = "tool";

#[cfg(target_arch = "wasm32")]
#[used]
#[unsafe(link_section = ".greentic.wasi")]
static WASI_TARGET_MARKER: [u8; 13] = *b"wasm32-wasip2";

#[cfg(target_arch = "wasm32")]
mod component {
    use greentic_interfaces_guest::component_v0_6::{
        component_descriptor, component_i18n, component_qa, component_runtime, component_schema,
    };

    use super::{
        apply_answers_cbor, component_describe_cbor, component_info_cbor, config_schema_cbor,
        i18n_keys, input_schema_cbor, output_schema_cbor, qa_spec_cbor, run_component_cbor,
    };

    pub(super) struct Component;

    impl component_descriptor::Guest for Component {
        fn get_component_info() -> Vec<u8> {
            component_info_cbor()
        }

        fn describe() -> Vec<u8> {
            component_describe_cbor()
        }
    }

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

    impl component_runtime::Guest for Component {
        fn run(input: Vec<u8>, state: Vec<u8>) -> component_runtime::RunResult {
            let (output, new_state) = run_component_cbor(input, state);
            component_runtime::RunResult { output, new_state }
        }
    }

    impl component_qa::Guest for Component {
        fn qa_spec(mode: component_qa::QaMode) -> Vec<u8> {
            qa_spec_cbor(mode)
        }

        fn apply_answers(
            mode: component_qa::QaMode,
            current_config: Vec<u8>,
            answers: Vec<u8>,
        ) -> Vec<u8> {
            apply_answers_cbor(mode, current_config, answers)
        }
    }

    impl component_i18n::Guest for Component {
        fn i18n_keys() -> Vec<String> {
            i18n_keys()
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod exports {
    use greentic_interfaces_guest::component_v0_6::{
        component_descriptor, component_i18n, component_qa, component_runtime, component_schema,
    };

    use super::component::Component;

    #[unsafe(export_name = "greentic:component/component-descriptor@0.6.0#get-component-info")]
    unsafe extern "C" fn export_get_component_info() -> *mut u8 {
        unsafe { component_descriptor::_export_get_component_info_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-descriptor@0.6.0#get-component-info")]
    unsafe extern "C" fn post_return_get_component_info(arg0: *mut u8) {
        unsafe { component_descriptor::__post_return_get_component_info::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-descriptor@0.6.0#describe")]
    unsafe extern "C" fn export_describe() -> *mut u8 {
        unsafe { component_descriptor::_export_describe_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-descriptor@0.6.0#describe")]
    unsafe extern "C" fn post_return_describe(arg0: *mut u8) {
        unsafe { component_descriptor::__post_return_describe::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-schema@0.6.0#input-schema")]
    unsafe extern "C" fn export_input_schema() -> *mut u8 {
        unsafe { component_schema::_export_input_schema_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-schema@0.6.0#input-schema")]
    unsafe extern "C" fn post_return_input_schema(arg0: *mut u8) {
        unsafe { component_schema::__post_return_input_schema::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-schema@0.6.0#output-schema")]
    unsafe extern "C" fn export_output_schema() -> *mut u8 {
        unsafe { component_schema::_export_output_schema_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-schema@0.6.0#output-schema")]
    unsafe extern "C" fn post_return_output_schema(arg0: *mut u8) {
        unsafe { component_schema::__post_return_output_schema::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-schema@0.6.0#config-schema")]
    unsafe extern "C" fn export_config_schema() -> *mut u8 {
        unsafe { component_schema::_export_config_schema_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-schema@0.6.0#config-schema")]
    unsafe extern "C" fn post_return_config_schema(arg0: *mut u8) {
        unsafe { component_schema::__post_return_config_schema::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-runtime@0.6.0#run")]
    unsafe extern "C" fn export_run(
        arg0: *mut u8,
        arg1: usize,
        arg2: *mut u8,
        arg3: usize,
    ) -> *mut u8 {
        unsafe { component_runtime::_export_run_cabi::<Component>(arg0, arg1, arg2, arg3) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-runtime@0.6.0#run")]
    unsafe extern "C" fn post_return_run(arg0: *mut u8) {
        unsafe { component_runtime::__post_return_run::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-qa@0.6.0#qa-spec")]
    unsafe extern "C" fn export_qa_spec(arg0: i32) -> *mut u8 {
        unsafe { component_qa::_export_qa_spec_cabi::<Component>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-qa@0.6.0#qa-spec")]
    unsafe extern "C" fn post_return_qa_spec(arg0: *mut u8) {
        unsafe { component_qa::__post_return_qa_spec::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-qa@0.6.0#apply-answers")]
    unsafe extern "C" fn export_apply_answers(
        arg0: i32,
        arg1: *mut u8,
        arg2: usize,
        arg3: *mut u8,
        arg4: usize,
    ) -> *mut u8 {
        unsafe {
            component_qa::_export_apply_answers_cabi::<Component>(arg0, arg1, arg2, arg3, arg4)
        }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-qa@0.6.0#apply-answers")]
    unsafe extern "C" fn post_return_apply_answers(arg0: *mut u8) {
        unsafe { component_qa::__post_return_apply_answers::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/component-i18n@0.6.0#i18n-keys")]
    unsafe extern "C" fn export_i18n_keys() -> *mut u8 {
        unsafe { component_i18n::_export_i18n_keys_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/component-i18n@0.6.0#i18n-keys")]
    unsafe extern "C" fn post_return_i18n_keys(arg0: *mut u8) {
        unsafe { component_i18n::__post_return_i18n_keys::<Component>(arg0) };
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ComponentInvocation {
    config: Value,
    msg: ChannelMessageEnvelope,
    payload: Value,
    #[serde(default)]
    _connections: Vec<String>,
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
                                properties: BTreeMap::from([("info".to_string(), string_schema(0))]),
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

fn qa_spec_for_mode(mode: ComponentQaMode) -> ComponentQaSpec {
    let title = I18nText::new(
        "templates.qa.title",
        Some("Templates configuration".to_string()),
    );
    let question = Question {
        id: "templates.text".to_string(),
        label: I18nText::new(
            "templates.qa.text.label",
            Some("Template text".to_string()),
        ),
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
fn qa_spec_cbor(mode: greentic_interfaces_guest::component_v0_6::component_qa::QaMode) -> Vec<u8> {
    let mapped = match mode {
        greentic_interfaces_guest::component_v0_6::component_qa::QaMode::Default => {
            ComponentQaMode::Default
        }
        greentic_interfaces_guest::component_v0_6::component_qa::QaMode::Setup => {
            ComponentQaMode::Setup
        }
        greentic_interfaces_guest::component_v0_6::component_qa::QaMode::Upgrade => {
            ComponentQaMode::Upgrade
        }
        greentic_interfaces_guest::component_v0_6::component_qa::QaMode::Remove => {
            ComponentQaMode::Remove
        }
    };
    let spec = qa_spec_for_mode(mapped);
    encode_cbor(&spec)
}

#[cfg(target_arch = "wasm32")]
fn apply_answers_cbor(
    mode: greentic_interfaces_guest::component_v0_6::component_qa::QaMode,
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
        ComponentQaMode::Default,
        ComponentQaMode::Setup,
        ComponentQaMode::Upgrade,
        ComponentQaMode::Remove,
    ] {
        let spec = qa_spec_for_mode(mode);
        for key in spec.i18n_keys() {
            keys.insert(key);
        }
    }
    keys.into_iter().collect()
}

fn run_component_cbor(input: Vec<u8>, _state: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    let invocation: Result<ComponentInvocation, InvokeFailure> = decode_cbor(&input);
    let result = match invocation {
        Ok(invocation) => invoke_template_from_invocation(invocation)
            .unwrap_or_else(|err| ComponentResult {
                payload: Value::Null,
                state_updates: empty_object(),
                control: None,
                error: Some(ComponentError {
                    kind: "InvalidInput".to_string(),
                    message: err.to_string(),
                    details: None,
                }),
            }),
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
    json!({
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

/// Entry point used by both sync and streaming invocations.
pub fn invoke_template(_operation: &str, input: &str) -> Result<String, InvokeFailure> {
    if _operation != SUPPORTED_OPERATION {
        return Err(InvokeFailure::UnsupportedOperation(format!(
            "operation `{}` is not supported; use `{}`",
            _operation, SUPPORTED_OPERATION
        )));
    }

    let invocation: ComponentInvocation =
        serde_json::from_str(input).map_err(|err| InvokeFailure::InvalidInput(err.to_string()))?;

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

#[cfg(test)]
mod tests {
    use core::convert::TryFrom;
    use serde_json::json;

    use super::*;

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
    fn renders_basic_template() {
        let invocation = sample_invocation(
            "Hello! You asked: {{payload.text}}",
            json!({ "text": "weather?" }),
        );

        let result = invoke_template(
            SUPPORTED_OPERATION,
            &serde_json::to_string(&invocation).expect("serialize invocation"),
        )
        .expect("invoke should succeed");

        let json: Value = serde_json::from_str(&result).expect("result json");
        assert_eq!(
            json["payload"],
            json!({ "text": "Hello! You asked: weather?" })
        );
        assert!(json["error"].is_null());
        assert_eq!(json["state_updates"], json!({}));
    }

    #[test]
    fn missing_fields_render_empty() {
        let invocation = sample_invocation("Hello! {{payload.missing}}", json!({ "text": "ping" }));

        let result = invoke_template(
            SUPPORTED_OPERATION,
            &serde_json::to_string(&invocation).expect("serialize invocation"),
        )
        .expect("invoke should succeed");

        let json: Value = serde_json::from_str(&result).expect("result json");
        assert_eq!(json["payload"], json!({ "text": "Hello! " }));
        assert!(json["error"].is_null());
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
        let result = invoke_template("handlebars", &serde_json::to_string(&invocation).unwrap());
        assert!(matches!(
            result,
            Err(InvokeFailure::UnsupportedOperation(_))
        ));
    }
}
