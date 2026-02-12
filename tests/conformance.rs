use core::convert::TryFrom;

use component_templates::{describe_payload, invoke_template};
use greentic_types::{ChannelMessageEnvelope, EnvId, TenantCtx, TenantId};
use serde_json::{Value, json};

fn sample_msg() -> ChannelMessageEnvelope {
    let mut tenant = TenantCtx::new(
        EnvId::try_from("dev").unwrap(),
        TenantId::try_from("tenant").unwrap(),
    );
    tenant.session_id = Some("session-1".to_string());
    tenant.attempt = 1;

    ChannelMessageEnvelope {
        id: "msg-1".to_string(),
        tenant,
        channel: "chat".to_string(),
        session_id: "session-1".to_string(),
        reply_scope: None,
        from: None,
        to: Vec::new(),
        correlation_id: None,
        text: Some("hello".to_string()),
        attachments: Vec::new(),
        metadata: Default::default(),
    }
}

#[test]
fn describe_mentions_world() {
    let payload = describe_payload();
    let json: Value = serde_json::from_str(&payload).expect("describe should be json");
    assert_eq!(
        json["component"]["world"],
        "greentic:component/component@0.6.0"
    );
}

#[test]
fn renders_template_into_payload() {
    let invocation = json!({
        "config": { "templates": { "text": "Hi {{payload.text}}", "output_path": "reply.text" } },
        "msg": sample_msg(),
        "payload": { "text": "there" },
        "connections": []
    });

    let result = invoke_template("text", &invocation.to_string()).expect("invoke");
    let json: Value = serde_json::from_str(&result).expect("result json");

    assert_eq!(json["payload"], json!({ "reply": { "text": "Hi there" } }));
    assert!(json["error"].is_null());
}

#[test]
fn template_error_returns_component_error() {
    let invocation = json!({
        "config": { "templates": { "text": "{{#if}}" } },
        "msg": sample_msg(),
        "payload": {},
        "connections": []
    });

    let result = invoke_template("text", &invocation.to_string()).expect("invoke");
    let json: Value = serde_json::from_str(&result).expect("result json");

    assert_eq!(json["error"]["kind"], "TemplateError");
    assert!(json["payload"].is_null());
}
