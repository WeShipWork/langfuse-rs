use langfuse::LangfuseSpan;
use langfuse::PropagateAttributes;
use opentelemetry::baggage::BaggageExt;
use opentelemetry::{Context, KeyValue};
use opentelemetry_sdk::trace::SdkTracerProvider;

fn init_test_provider() {
    let provider = SdkTracerProvider::builder().build();
    opentelemetry::global::set_tracer_provider(provider);
}

#[test]
fn propagate_as_baggage_sets_langfuse_keys() {
    init_test_provider();

    let root = LangfuseSpan::start("baggage-root");
    let _guard = root.context().clone().attach();
    let trace_id = root.trace_id();

    let attrs = PropagateAttributes {
        user_id: Some("user-123".into()),
        session_id: Some("session-456".into()),
        tags: Some(vec!["tag-a".into(), "tag-b".into()]),
        as_baggage: true,
        ..Default::default()
    };

    langfuse::propagate_as_baggage(&attrs, || {
        let cx = Context::current();
        let baggage = cx.baggage();
        assert_eq!(
            baggage.get("langfuse.trace_id").map(|value| value.as_str()),
            Some(trace_id.as_str())
        );
        assert_eq!(
            baggage.get("langfuse.user_id").map(|value| value.as_str()),
            Some("user-123")
        );
        assert_eq!(
            baggage
                .get("langfuse.session_id")
                .map(|value| value.as_str()),
            Some("session-456")
        );
        assert_eq!(
            baggage.get("langfuse.tags").map(|value| value.as_str()),
            Some("[\"tag-a\",\"tag-b\"]")
        );
    });

    root.end();
}

#[test]
fn read_propagated_attributes_from_baggage_round_trip() {
    let incoming = Context::current_with_baggage([
        KeyValue::new("langfuse.user_id", "user-in"),
        KeyValue::new("langfuse.session_id", "session-in"),
        KeyValue::new("langfuse.tags", "[\"edge\",\"canary\"]"),
    ]);
    let _guard = incoming.attach();

    let attrs = langfuse::langfuse_tracing::context::read_propagated_attributes_from_baggage();

    assert_eq!(attrs.user_id.as_deref(), Some("user-in"));
    assert_eq!(attrs.session_id.as_deref(), Some("session-in"));
    assert_eq!(
        attrs.tags,
        Some(vec![String::from("edge"), String::from("canary")])
    );
    assert!(!attrs.as_baggage);
}
