//! Convenience helpers for reading Langfuse trace/observation IDs from the
//! current OTel context, and for propagating trace-level attributes.

use langfuse_core::types::PropagateAttributes;
use opentelemetry::Context;
use opentelemetry::baggage::BaggageExt;
use opentelemetry::trace::TraceContextExt;

use super::attributes;
use super::span::LangfuseSpan;

const BAGGAGE_TRACE_ID: &str = "langfuse.trace_id";
const BAGGAGE_USER_ID: &str = "langfuse.user_id";
const BAGGAGE_SESSION_ID: &str = "langfuse.session_id";
const BAGGAGE_TAGS: &str = "langfuse.tags";

/// Return the current trace ID from the thread-local OTel context, or `None`
/// if no valid span is active.
pub fn get_current_trace_id() -> Option<String> {
    let cx = Context::current();
    let span = cx.span();
    let sc = span.span_context();
    if sc.is_valid() {
        Some(sc.trace_id().to_string())
    } else {
        cx.baggage()
            .get(BAGGAGE_TRACE_ID)
            .map(|value| value.as_str().to_owned())
    }
}

/// Return the current observation (span) ID from the thread-local OTel
/// context, or `None` if no valid span is active.
pub fn get_current_observation_id() -> Option<String> {
    let cx = Context::current();
    let span = cx.span();
    let sc = span.span_context();
    if sc.is_valid() {
        Some(sc.span_id().to_string())
    } else {
        None
    }
}

/// Execute a closure with propagated trace-level attributes.
///
/// All spans created within `f()` will inherit the attributes from `attrs`.
/// Attributes are applied to each span's OTel attributes so the Langfuse
/// server can pick them up.
///
/// ```ignore
/// let attrs = PropagateAttributes {
///     user_id: Some("user-123".into()),
///     session_id: Some("sess-abc".into()),
///     ..Default::default()
/// };
/// propagate_attributes(&attrs, || {
///     // All spans created here inherit user_id and session_id
/// });
/// ```
pub fn propagate_attributes(attrs: &PropagateAttributes, f: impl FnOnce()) {
    // Store the attributes in a thread-local so span constructors can read them.
    let _guard = PropagatedAttrsGuard::install(attrs);
    if attrs.as_baggage {
        propagate_as_baggage(attrs, f);
    } else {
        f();
    }
}

/// Execute a closure with Langfuse context stored in W3C baggage.
///
/// Existing baggage entries are preserved. Langfuse values are written with the
/// following keys:
///
/// - `langfuse.trace_id`
/// - `langfuse.user_id`
/// - `langfuse.session_id`
/// - `langfuse.tags` (JSON-encoded string array)
pub fn propagate_as_baggage(attrs: &PropagateAttributes, f: impl FnOnce()) {
    let cx = Context::current();

    let mut baggage: opentelemetry::baggage::Baggage = cx
        .baggage()
        .iter()
        .map(|(key, (value, metadata))| (key.clone(), (value.clone(), metadata.clone())))
        .collect();

    let span_ref = cx.span();
    let sc = span_ref.span_context();
    if sc.is_valid() {
        let _ = baggage.insert(BAGGAGE_TRACE_ID, sc.trace_id().to_string());
    }

    if let Some(ref user_id) = attrs.user_id {
        let _ = baggage.insert(BAGGAGE_USER_ID, user_id.clone());
    }
    if let Some(ref session_id) = attrs.session_id {
        let _ = baggage.insert(BAGGAGE_SESSION_ID, session_id.clone());
    }
    if let Some(ref tags) = attrs.tags
        && let Ok(tags_json) = serde_json::to_string(tags)
    {
        let _ = baggage.insert(BAGGAGE_TAGS, tags_json);
    }

    let cx_with_baggage = cx.with_baggage(baggage);
    let _guard = cx_with_baggage.attach();
    f();
}

/// Read Langfuse propagation attributes from current W3C baggage.
///
/// This is primarily useful in incoming request handlers after framework-level
/// extraction has attached a remote context.
#[must_use]
pub fn read_propagated_attributes_from_baggage() -> PropagateAttributes {
    read_propagated_attributes_from_context(&Context::current())
}

/// Apply any propagated attributes from the current context to the given span.
pub(crate) fn apply_propagated_attributes(span: &LangfuseSpan) {
    let explicit_attrs = PROPAGATED_ATTRS.with(|cell| cell.borrow().clone());
    if let Some(attrs) = resolve_propagated_attributes(explicit_attrs) {
        if let Some(ref user_id) = attrs.user_id {
            span.set_string_attribute(attributes::LANGFUSE_USER_ID, user_id);
        }
        if let Some(ref session_id) = attrs.session_id {
            span.set_string_attribute(attributes::LANGFUSE_SESSION_ID, session_id);
        }
        if let Some(ref metadata) = attrs.metadata {
            span.set_json_attribute(attributes::LANGFUSE_METADATA, metadata);
        }
        if let Some(ref version) = attrs.version {
            span.set_string_attribute(attributes::LANGFUSE_VERSION, version);
        }
        if let Some(ref tags) = attrs.tags {
            let tag_refs: Vec<&str> = tags.iter().map(String::as_str).collect();
            span.set_json_attribute(attributes::LANGFUSE_TAGS, &tag_refs);
        }
        if let Some(ref trace_name) = attrs.trace_name {
            span.set_string_attribute("langfuse.trace.name", trace_name);
        }
    }
}

fn resolve_propagated_attributes(
    explicit_attrs: Option<PropagateAttributes>,
) -> Option<PropagateAttributes> {
    let from_baggage = read_propagated_attributes_from_context(&Context::current());
    let mut merged = explicit_attrs.unwrap_or_default();

    if merged.user_id.is_none() {
        merged.user_id = from_baggage.user_id;
    }
    if merged.session_id.is_none() {
        merged.session_id = from_baggage.session_id;
    }
    if merged.tags.is_none() {
        merged.tags = from_baggage.tags;
    }

    if has_propagated_values(&merged) {
        Some(merged)
    } else {
        None
    }
}

fn read_propagated_attributes_from_context(cx: &Context) -> PropagateAttributes {
    let baggage = cx.baggage();

    let user_id = baggage
        .get(BAGGAGE_USER_ID)
        .map(|value| value.as_str().to_owned());
    let session_id = baggage
        .get(BAGGAGE_SESSION_ID)
        .map(|value| value.as_str().to_owned());
    let tags = baggage
        .get(BAGGAGE_TAGS)
        .and_then(|value| serde_json::from_str::<Vec<String>>(value.as_str()).ok())
        .filter(|tags| !tags.is_empty());

    PropagateAttributes {
        user_id,
        session_id,
        tags,
        ..Default::default()
    }
}

struct PropagatedAttrsGuard {
    prev: Option<PropagateAttributes>,
}

impl PropagatedAttrsGuard {
    fn install(attrs: &PropagateAttributes) -> Self {
        let prev = PROPAGATED_ATTRS.with(|cell| cell.replace(Some(attrs.clone())));
        Self { prev }
    }
}

impl Drop for PropagatedAttrsGuard {
    fn drop(&mut self) {
        PROPAGATED_ATTRS.with(|cell| {
            let _ = cell.replace(self.prev.clone());
        });
    }
}

const fn has_propagated_values(attrs: &PropagateAttributes) -> bool {
    attrs.user_id.is_some()
        || attrs.session_id.is_some()
        || attrs.metadata.is_some()
        || attrs.version.is_some()
        || attrs.tags.is_some()
        || attrs.trace_name.is_some()
}

std::thread_local! {
    static PROPAGATED_ATTRS: std::cell::RefCell<Option<PropagateAttributes>> = const { std::cell::RefCell::new(None) };
}
