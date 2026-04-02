use langfuse_core::types::{
    ChatMessage, CostDetails, MediaReference, ObservationType, PromptType, PropagateAttributes,
    ScoreBody, ScoreDataType, ScoreValue, SpanLevel, UsageDetails,
};

// ── ObservationType ──────────────────────────────────────────────────

#[test]
fn observation_type_serializes_to_lowercase() {
    let cases = [
        (ObservationType::Span, "\"span\""),
        (ObservationType::Generation, "\"generation\""),
        (ObservationType::Event, "\"event\""),
        (ObservationType::Embedding, "\"embedding\""),
        (ObservationType::Agent, "\"agent\""),
        (ObservationType::Tool, "\"tool\""),
        (ObservationType::Chain, "\"chain\""),
        (ObservationType::Retriever, "\"retriever\""),
        (ObservationType::Evaluator, "\"evaluator\""),
        (ObservationType::Guardrail, "\"guardrail\""),
    ];
    for (variant, expected) in &cases {
        let json = serde_json::to_string(variant).unwrap();
        assert_eq!(&json, expected, "serializing {variant:?}");
    }
}

#[test]
fn observation_type_deserializes_from_lowercase() {
    let json = "\"generation\"";
    let parsed: ObservationType = serde_json::from_str(json).unwrap();
    assert_eq!(parsed, ObservationType::Generation);
}

// ── SpanLevel ────────────────────────────────────────────────────────

#[test]
fn span_level_serializes_to_screaming_snake_case() {
    let cases = [
        (SpanLevel::Debug, "\"DEBUG\""),
        (SpanLevel::Default, "\"DEFAULT\""),
        (SpanLevel::Warning, "\"WARNING\""),
        (SpanLevel::Error, "\"ERROR\""),
    ];
    for (variant, expected) in &cases {
        let json = serde_json::to_string(variant).unwrap();
        assert_eq!(&json, expected, "serializing {variant:?}");
    }
}

#[test]
fn span_level_round_trips() {
    let original = SpanLevel::Warning;
    let json = serde_json::to_string(&original).unwrap();
    let parsed: SpanLevel = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, original);
}

// ── ScoreDataType ────────────────────────────────────────────────────

#[test]
fn score_data_type_serializes_to_screaming_snake_case() {
    let cases = [
        (ScoreDataType::Numeric, "\"NUMERIC\""),
        (ScoreDataType::Categorical, "\"CATEGORICAL\""),
        (ScoreDataType::Boolean, "\"BOOLEAN\""),
    ];
    for (variant, expected) in &cases {
        let json = serde_json::to_string(variant).unwrap();
        assert_eq!(&json, expected, "serializing {variant:?}");
    }
}

#[test]
fn score_data_type_round_trips() {
    let original = ScoreDataType::Categorical;
    let json = serde_json::to_string(&original).unwrap();
    let parsed: ScoreDataType = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, original);
}

// ── ScoreValue (untagged) ────────────────────────────────────────────

#[test]
fn score_value_numeric_serializes_to_number() {
    let val = ScoreValue::Numeric(3.5);
    let json = serde_json::to_string(&val).unwrap();
    assert_eq!(json, "3.5");
}

#[test]
fn score_value_categorical_serializes_to_string() {
    let val = ScoreValue::Categorical("good".to_string());
    let json = serde_json::to_string(&val).unwrap();
    assert_eq!(json, "\"good\"");
}

#[test]
fn score_value_boolean_serializes_to_bool() {
    let val = ScoreValue::Boolean(true);
    let json = serde_json::to_string(&val).unwrap();
    assert_eq!(json, "true");
}

#[test]
fn score_value_deserializes_numeric() {
    let parsed: ScoreValue = serde_json::from_str("42.0").unwrap();
    assert_eq!(parsed, ScoreValue::Numeric(42.0));
}

#[test]
fn score_value_deserializes_categorical() {
    let parsed: ScoreValue = serde_json::from_str("\"bad\"").unwrap();
    assert_eq!(parsed, ScoreValue::Categorical("bad".to_string()));
}

#[test]
fn score_value_deserializes_boolean() {
    let parsed: ScoreValue = serde_json::from_str("false").unwrap();
    assert_eq!(parsed, ScoreValue::Boolean(false));
}

// ── ScoreBody ────────────────────────────────────────────────────────

#[test]
fn score_body_serializes_camel_case_and_skips_none() {
    let body = ScoreBody {
        name: "accuracy".to_string(),
        value: ScoreValue::Numeric(0.95),
        trace_id: Some("trace-123".to_string()),
        observation_id: None,
        comment: None,
        metadata: None,
        config_id: None,
        data_type: Some(ScoreDataType::Numeric),
    };
    let json = serde_json::to_string(&body).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();

    // camelCase keys present
    assert_eq!(v["name"], "accuracy");
    assert_eq!(v["traceId"], "trace-123");
    assert_eq!(v["dataType"], "NUMERIC");

    // None fields absent
    assert!(v.get("observationId").is_none());
    assert!(v.get("comment").is_none());
    assert!(v.get("metadata").is_none());
    assert!(v.get("configId").is_none());
}

#[test]
fn score_body_round_trips() {
    let body = ScoreBody {
        name: "quality".to_string(),
        value: ScoreValue::Categorical("excellent".to_string()),
        trace_id: Some("t-1".to_string()),
        observation_id: Some("o-1".to_string()),
        comment: Some("looks great".to_string()),
        metadata: Some(serde_json::json!({"key": "val"})),
        config_id: Some("cfg-1".to_string()),
        data_type: Some(ScoreDataType::Categorical),
    };
    let json = serde_json::to_string(&body).unwrap();
    let parsed: ScoreBody = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, body.name);
    assert_eq!(parsed.trace_id, body.trace_id);
    assert_eq!(parsed.data_type, body.data_type);
}

// ── UsageDetails ─────────────────────────────────────────────────────

#[test]
fn usage_details_serializes_camel_case_and_skips_none() {
    let usage = UsageDetails {
        input: Some(100),
        output: None,
        total: Some(150),
    };
    let json = serde_json::to_string(&usage).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(v["input"], 100);
    assert_eq!(v["total"], 150);
    assert!(v.get("output").is_none());
}

#[test]
fn usage_details_default_is_all_none() {
    let usage = UsageDetails::default();
    assert_eq!(usage.input, None);
    assert_eq!(usage.output, None);
    assert_eq!(usage.total, None);
}

// ── CostDetails ──────────────────────────────────────────────────────

#[test]
fn cost_details_serializes_camel_case_and_skips_none() {
    let cost = CostDetails {
        input: Some(0.001),
        output: Some(0.002),
        total: None,
    };
    let json = serde_json::to_string(&cost).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(v["input"], 0.001);
    assert_eq!(v["output"], 0.002);
    assert!(v.get("total").is_none());
}

#[test]
fn cost_details_default_is_all_none() {
    let cost = CostDetails::default();
    assert_eq!(cost.input, None);
    assert_eq!(cost.output, None);
    assert_eq!(cost.total, None);
}

// ── ChatMessage ──────────────────────────────────────────────────────

#[test]
fn chat_message_round_trips() {
    let msg = ChatMessage {
        role: "user".to_string(),
        content: "Hello, world!".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.role, "user");
    assert_eq!(parsed.content, "Hello, world!");
}

// ── PropagateAttributes ──────────────────────────────────────────────

#[test]
fn propagate_attributes_default_is_all_none() {
    let attrs = PropagateAttributes::default();
    assert_eq!(attrs.user_id, None);
    assert_eq!(attrs.session_id, None);
    assert_eq!(attrs.metadata, None);
    assert_eq!(attrs.version, None);
    assert_eq!(attrs.tags, None);
    assert_eq!(attrs.trace_name, None);
    assert!(!attrs.as_baggage);
}

#[test]
fn propagate_attributes_serializes_camel_case_and_skips_none() {
    let attrs = PropagateAttributes {
        user_id: Some("u-1".to_string()),
        session_id: None,
        metadata: None,
        version: Some("1.0".to_string()),
        tags: Some(vec!["prod".to_string()]),
        trace_name: None,
        as_baggage: false,
    };
    let json = serde_json::to_string(&attrs).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(v["userId"], "u-1");
    assert_eq!(v["version"], "1.0");
    assert_eq!(v["tags"], serde_json::json!(["prod"]));
    assert!(v.get("sessionId").is_none());
    assert!(v.get("metadata").is_none());
    assert!(v.get("traceName").is_none());
    assert!(v.get("asBaggage").is_none());
}

#[test]
fn propagate_attributes_serializes_as_baggage_when_true() {
    let attrs = PropagateAttributes {
        as_baggage: true,
        ..Default::default()
    };

    let json = serde_json::to_string(&attrs).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["asBaggage"], true);
}

// ── PromptType ───────────────────────────────────────────────────────

#[test]
fn prompt_type_serializes_to_lowercase() {
    let cases = [
        (PromptType::Text, "\"text\""),
        (PromptType::Chat, "\"chat\""),
    ];
    for (variant, expected) in &cases {
        let json = serde_json::to_string(variant).unwrap();
        assert_eq!(&json, expected, "serializing {variant:?}");
    }
}

#[test]
fn prompt_type_round_trips() {
    let original = PromptType::Chat;
    let json = serde_json::to_string(&original).unwrap();
    let parsed: PromptType = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, original);
}

// ── MediaReference ───────────────────────────────────────────────────

#[test]
fn media_reference_serializes_camel_case() {
    let mref = MediaReference {
        media_id: "m-123".to_string(),
        content_type: "image/png".to_string(),
        source: "base64_data_uri".to_string(),
    };
    let json = serde_json::to_string(&mref).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(v["mediaId"], "m-123");
    assert_eq!(v["contentType"], "image/png");
    assert_eq!(v["source"], "base64_data_uri");
}

#[test]
fn media_reference_round_trips() {
    let mref = MediaReference {
        media_id: "m-456".to_string(),
        content_type: "audio/wav".to_string(),
        source: "bytes".to_string(),
    };
    let json = serde_json::to_string(&mref).unwrap();
    let parsed: MediaReference = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, mref);
}
