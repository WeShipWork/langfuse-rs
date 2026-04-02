# TRACING MODULE

OTel-native tracing pipeline that maps Langfuse observations to OpenTelemetry spans. Langfuse-specific data (input/output, model, usage) stored as JSON string attributes on OTel spans, exported via OTLP HTTP to Langfuse's `/api/public/otel` endpoint.

## STRUCTURE

```
tracing/
├── mod.rs           # Module declarations only
├── span.rs          # LangfuseSpan — OTel Context wrapper, all attribute setters
├── generation.rs    # LangfuseGeneration — Deref<Target=LangfuseSpan> + model/usage setters
├── observe.rs       # with_span(), with_generation() — closure-based async API
├── context.rs       # get_current_trace_id(), get_current_observation_id()
├── exporter.rs      # LangfuseTracing — OTLP HTTP exporter + BatchSpanProcessor setup
└── attributes.rs    # Attribute key constants (langfuse.input, langfuse.model, etc.)
```

## WHERE TO LOOK

| Task | File | Notes |
|------|------|-------|
| Add new span attribute | `attributes.rs` + `span.rs` | Add const, add setter method returning `&Self` |
| Add generation-only attribute | `attributes.rs` + `generation.rs` | Generation delegates to `span.set_json_attribute()` |
| Change export config | `exporter.rs` | BatchConfigBuilder, OTLP endpoint, protocol |
| Add new closure helper | `observe.rs` | Follow `with_span` pattern: create → pass to closure → await |
| Read current trace context | `context.rs` | Uses `Context::current().span().span_context()` |
| Change observation type logic | `span.rs` `start_child()` | Tags child with ObservationType via JSON attribute |

## DATA FLOW

```
User code → LangfuseSpan::start(name)
         → OTel global tracer creates span
         → Setters serialize to JSON → KeyValue string attributes
         → span.end() or scope drop
         → BatchSpanProcessor collects
         → OTLP HTTP export to Langfuse /api/public/otel
         → Langfuse server reads langfuse.* attributes
```

## CONVENTIONS

- **All setters return `&Self`** — enables fluent chaining: `span.set_input(&x).set_output(&y).end()`
- **JSON-in-string**: `set_json_attribute()` serializes via `serde_json::to_string()`, stores as OTel `StringValue`. Silently no-ops on serialization failure
- **`set_string_attribute()`** for plain strings (status_message, user_id, session_id, model name)
- **`set_json_attribute()`** for structured data (input, output, metadata, usage, cost)
- **Interior mutability**: OTel `SpanRef` uses internal `Mutex`, so all setters take `&self` (not `&mut self`)
- **Tracer name**: Always `"langfuse"` — `opentelemetry::global::tracer("langfuse")`
- **Generation = Span + Deref**: `LangfuseGeneration` wraps `LangfuseSpan` via `Deref`, adds model/usage/cost setters
- **Context propagation**: Child spans created via `tracer.start_with_context(name, &parent.context)` — automatic parent-child linking

## ANTI-PATTERNS

- **DO NOT use `&mut self`** on setters — OTel spans use interior mutability
- **DO NOT store span references across `.await` points** without the Context — use `with_span` closure API instead
- **DO NOT create tracers with names other than `"langfuse"`** — server-side processing depends on this
- **DO NOT call `opentelemetry::global::set_tracer_provider()`** — `LangfuseTracing` manages the provider; the global tracer is used read-only

## KEY TYPES

| Type | Role |
|------|------|
| `LangfuseSpan` | Wraps `opentelemetry::Context`. All attribute setters, child creation, lifecycle |
| `LangfuseGeneration` | `Deref<Target=LangfuseSpan>`. Adds `set_model`, `set_usage`, `set_cost`, `set_completion_start_time` |
| `LangfuseTracing` | Owns `SdkTracerProvider` with OTLP HTTP exporter + `BatchSpanProcessor` |
| `LangfuseTracingBuilder` | Configures exporter from `LangfuseConfig` (endpoint, auth, batch sizes) |

## ATTRIBUTE KEYS

All defined in `attributes.rs` as `&str` constants prefixed `LANGFUSE_`:

| Constant | Key String | Set By | Format |
|----------|-----------|--------|--------|
| `LANGFUSE_OBSERVATION_TYPE` | `langfuse.observation.type` | Span/Generation constructor | JSON enum |
| `LANGFUSE_INPUT` | `langfuse.input` | `set_input()` | JSON string |
| `LANGFUSE_OUTPUT` | `langfuse.output` | `set_output()` | JSON string |
| `LANGFUSE_METADATA` | `langfuse.metadata` | `set_metadata()` | JSON string |
| `LANGFUSE_MODEL` | `langfuse.model` | `set_model()` | Plain string |
| `LANGFUSE_USAGE` | `langfuse.usage` | `set_usage()` | JSON string |
| `LANGFUSE_COST` | `langfuse.cost` | `set_cost()` | JSON string |
| `LANGFUSE_USER_ID` | `langfuse.user_id` | `set_trace_user_id()` | Plain string |
| `LANGFUSE_SESSION_ID` | `langfuse.session_id` | `set_trace_session_id()` | Plain string |

## NOTES

- Module exposed as `langfuse_tracing` (not `tracing`) via `#[path]` in parent lib.rs to avoid conflict with `tracing` crate
- `with_span` / `with_generation` do NOT auto-call `.end()` — callers should end explicitly or rely on OTel context cleanup
- Exporter uses `Protocol::HttpBinary` (protobuf over HTTP), not gRPC
- Batch config derives from `LangfuseConfig`: `max_queue_size = flush_at * 4`, `max_export_batch_size = flush_at`, `scheduled_delay = flush_interval`
