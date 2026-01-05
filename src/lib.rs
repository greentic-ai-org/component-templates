use handlebars::{Handlebars, RenderError};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use greentic_types::ChannelMessageEnvelope;

const DEFAULT_OUTPUT_PATH: &str = "text";

#[cfg(target_arch = "wasm32")]
#[used]
#[unsafe(link_section = ".greentic.wasi")]
static WASI_TARGET_MARKER: [u8; 13] = *b"wasm32-wasip2";

#[cfg(target_arch = "wasm32")]
mod component {
    use greentic_interfaces_guest::component::node::{
        self, ExecCtx, InvokeResult, LifecycleStatus, StreamEvent,
    };

    use super::{InvokeFailure, describe_payload, invoke_template};

    pub(super) struct Component;

    impl node::Guest for Component {
        fn get_manifest() -> String {
            describe_payload()
        }

        fn on_start(_ctx: ExecCtx) -> Result<LifecycleStatus, String> {
            Ok(LifecycleStatus::Ok)
        }

        fn on_stop(_ctx: ExecCtx, _reason: String) -> Result<LifecycleStatus, String> {
            Ok(LifecycleStatus::Ok)
        }

        fn invoke(_ctx: ExecCtx, op: String, input: String) -> InvokeResult {
            match invoke_template(&op, &input) {
                Ok(result) => InvokeResult::Ok(result),
                Err(err) => InvokeResult::Err(to_node_error("InvalidInput", err)),
            }
        }

        fn invoke_stream(_ctx: ExecCtx, op: String, input: String) -> Vec<StreamEvent> {
            match invoke_template(&op, &input) {
                Ok(result) => vec![
                    StreamEvent::Progress(0),
                    StreamEvent::Data(result),
                    StreamEvent::Done,
                ],
                Err(err) => vec![StreamEvent::Error(err.to_string())],
            }
        }
    }

    fn to_node_error(code: &str, err: InvokeFailure) -> node::NodeError {
        node::NodeError {
            code: code.to_owned(),
            message: err.to_string(),
            retryable: false,
            backoff_ms: None,
            details: None,
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod exports {
    use greentic_interfaces_guest::component::node;

    use super::component::Component;

    #[unsafe(export_name = "greentic:component/node@0.5.0#get-manifest")]
    unsafe extern "C" fn export_get_manifest() -> *mut u8 {
        unsafe { node::_export_get_manifest_cabi::<Component>() }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#get-manifest")]
    unsafe extern "C" fn post_return_get_manifest(arg0: *mut u8) {
        unsafe { node::__post_return_get_manifest::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/node@0.5.0#on-start")]
    unsafe extern "C" fn export_on_start(arg0: *mut u8) -> *mut u8 {
        unsafe { node::_export_on_start_cabi::<Component>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#on-start")]
    unsafe extern "C" fn post_return_on_start(arg0: *mut u8) {
        unsafe { node::__post_return_on_start::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/node@0.5.0#on-stop")]
    unsafe extern "C" fn export_on_stop(arg0: *mut u8) -> *mut u8 {
        unsafe { node::_export_on_stop_cabi::<Component>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#on-stop")]
    unsafe extern "C" fn post_return_on_stop(arg0: *mut u8) {
        unsafe { node::__post_return_on_stop::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/node@0.5.0#invoke")]
    unsafe extern "C" fn export_invoke(arg0: *mut u8) -> *mut u8 {
        unsafe { node::_export_invoke_cabi::<Component>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#invoke")]
    unsafe extern "C" fn post_return_invoke(arg0: *mut u8) {
        unsafe { node::__post_return_invoke::<Component>(arg0) };
    }

    #[unsafe(export_name = "greentic:component/node@0.5.0#invoke-stream")]
    unsafe extern "C" fn export_invoke_stream(arg0: *mut u8) -> *mut u8 {
        unsafe { node::_export_invoke_stream_cabi::<Component>(arg0) }
    }

    #[unsafe(export_name = "cabi_post_greentic:component/node@0.5.0#invoke-stream")]
    unsafe extern "C" fn post_return_invoke_stream(arg0: *mut u8) {
        unsafe { node::__post_return_invoke_stream::<Component>(arg0) };
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ComponentInvocation {
    config: Value,
    msg: ChannelMessageEnvelope,
    payload: Value,
    state: Value,
    #[serde(default)]
    _connections: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TemplateConfig {
    template: String,
    #[serde(default)]
    output_path: Option<String>,
    #[serde(default = "default_wrap")]
    wrap: bool,
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
}

impl core::fmt::Display for InvokeFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InvokeFailure::InvalidInput(msg) => write!(f, "{msg}"),
        }
    }
}

fn default_wrap() -> bool {
    true
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}

/// Returns the component manifest JSON payload.
pub fn describe_payload() -> String {
    json!({
        "component": {
            "name": "templates",
            "org": "ai.greentic",
            "version": "0.1.2",
            "world": "greentic:component/component@0.5.0",
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
    let invocation: ComponentInvocation =
        serde_json::from_str(input).map_err(|err| InvokeFailure::InvalidInput(err.to_string()))?;

    let config: TemplateConfig = serde_json::from_value(invocation.config.clone())
        .map_err(|err| InvokeFailure::InvalidInput(err.to_string()))?;

    let context = build_context(&invocation);
    let outcome = render_template(&config, &context);

    let result = match outcome {
        Ok(rendered) => ComponentResult {
            payload: build_payload(&rendered, &config),
            state_updates: empty_object(),
            control: None,
            error: None,
        },
        Err(err) => ComponentResult {
            payload: Value::Null,
            state_updates: empty_object(),
            control: None,
            error: Some(err.into_component_error()),
        },
    };

    serde_json::to_string(&result).map_err(|err| InvokeFailure::InvalidInput(err.to_string()))
}

fn build_context(invocation: &ComponentInvocation) -> Value {
    let msg_value = serde_json::to_value(&invocation.msg).unwrap_or(Value::Null);
    json!({
        "msg": msg_value,
        "payload": invocation.payload.clone(),
        "state": invocation.state.clone(),
    })
}

fn render_template(config: &TemplateConfig, context: &Value) -> Result<String, TemplateError> {
    let mut engine = Handlebars::new();
    engine.set_strict_mode(false);
    engine
        .render_template(&config.template, context)
        .map_err(TemplateError::from_render_error)
}

fn build_payload(rendered: &str, config: &TemplateConfig) -> Value {
    if !config.wrap {
        return Value::String(rendered.to_owned());
    }

    let path = config
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

    fn sample_invocation(template: &str, payload: Value, state: Value) -> ComponentInvocation {
        let mut tenant_ctx = greentic_types::TenantCtx::new(
            greentic_types::EnvId::try_from("dev").unwrap(),
            greentic_types::TenantId::try_from("tenant").unwrap(),
        );
        tenant_ctx.session_id = Some("session-1".to_string());

        ComponentInvocation {
            config: json!({ "template": template }),
            msg: ChannelMessageEnvelope {
                id: "msg-1".to_string(),
                tenant: tenant_ctx,
                channel: "chat".to_string(),
                session_id: "session-1".to_string(),
                user_id: None,
                text: Some("hello".to_string()),
                attachments: Vec::new(),
                metadata: Default::default(),
            },
            payload,
            state,
            _connections: Vec::new(),
        }
    }

    #[test]
    fn renders_basic_template() {
        let invocation = sample_invocation(
            "Hello {{state.user.name}}! You asked: {{payload.text}}",
            json!({ "text": "weather?" }),
            json!({ "user": { "name": "Alice" } }),
        );

        let result = invoke_template(
            "invoke",
            &serde_json::to_string(&invocation).expect("serialize invocation"),
        )
        .expect("invoke should succeed");

        let json: Value = serde_json::from_str(&result).expect("result json");
        assert_eq!(
            json["payload"],
            json!({ "text": "Hello Alice! You asked: weather?" })
        );
        assert!(json["error"].is_null());
        assert_eq!(json["state_updates"], json!({}));
    }

    #[test]
    fn missing_fields_render_empty() {
        let invocation = sample_invocation(
            "Hello {{state.user.name}}! {{payload.missing}}",
            json!({ "text": "ping" }),
            json!({}),
        );

        let result = invoke_template(
            "invoke",
            &serde_json::to_string(&invocation).expect("serialize invocation"),
        )
        .expect("invoke should succeed");

        let json: Value = serde_json::from_str(&result).expect("result json");
        assert_eq!(json["payload"], json!({ "text": "Hello ! " }));
        assert!(json["error"].is_null());
    }

    #[test]
    fn template_error_is_reported() {
        let invocation = sample_invocation("{{#if}}", json!({}), json!({}));

        let result = invoke_template(
            "invoke",
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
            config: json!({ "template": "Hi", "output_path": "reply.body", "wrap": true }),
            ..sample_invocation("unused", json!({}), json!({}))
        };

        let result = invoke_template(
            "invoke",
            &serde_json::to_string(&invocation).expect("serialize invocation"),
        )
        .expect("invoke should succeed");

        let json: Value = serde_json::from_str(&result).expect("result json");
        assert_eq!(json["payload"], json!({ "reply": { "body": "Hi" } }));

        let raw_invocation = ComponentInvocation {
            config: json!({ "template": "Hi", "wrap": false }),
            ..sample_invocation("unused", json!({}), json!({}))
        };

        let raw_result = invoke_template(
            "invoke",
            &serde_json::to_string(&raw_invocation).expect("serialize invocation"),
        )
        .expect("invoke should succeed");

        let raw_json: Value = serde_json::from_str(&raw_result).expect("result json");
        assert_eq!(raw_json["payload"], json!("Hi"));
    }
}
