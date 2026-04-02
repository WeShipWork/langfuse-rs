# PROJECT KNOWLEDGE BASE

**Generated:** 2026-03-01
**Commit:** 6f17939
**Branch:** main

## OVERVIEW

Unofficial Rust SDK for Langfuse (LLM observability). Cargo workspace with 4 crates: core types/API client, main SDK, proc macros, and OpenAI integration. OTel-native tracing via OTLP HTTP, prompt management with TTL cache, score batching, dataset experiments, media handling.

## STRUCTURE

```
langfuse-rs/
├── crates/
│   ├── langfuse-core/       # Foundation: config, errors, types, auto-generated REST client
│   │   ├── build.rs         # Progenitor codegen from openapi.yml → OUT_DIR/codegen.rs
│   │   └── src/
│   │       ├── api/         # include!(codegen.rs) — #[allow(clippy::all)] on module
│   │       ├── config.rs    # LangfuseConfig + builder (230 lines, largest in core)
│   │       ├── error.rs     # LangfuseError, ConfigError — thiserror
│   │       └── types/       # Domain types: media, observation, prompt, score, evaluation
│   ├── langfuse/               # Main SDK package (`langfuse-sdk`)
│   │   ├── src/
│   │   │   ├── client.rs    # Langfuse struct, OnceLock singleton, DashMap named instances
│   │   │   ├── tracing/     # OTel pipeline — see tracing/AGENTS.md
│   │   │   │   ├── embedding.rs      # LangfuseEmbedding wrapper (Deref to LangfuseSpan)
│   │   │   │   ├── context_apis.rs    # Context-aware free functions (update_current_span, score_current_span, etc.)
│   │   │   │   └── stream_wrapper.rs  # ObservingStream, ObservingIterator
│   │   │   ├── prompts/     # PromptManager, TTL cache (DashMap), {{var}} compilation, Prompt enum
│   │   │   ├── scoring/     # ScoreManager, BatchQueue (Arc<Mutex<Vec>>), auto-flush background task
│   │   │   ├── datasets/    # DatasetManager, ExperimentRunner, Evaluator trait
│   │   │   ├── media/       # MediaManager, presigned URL upload/fetch
│   │   │   └── http.rs      # HTTP client builder, retry logic (exponential backoff)
│   │   ├── examples/        # 5 runnable examples (need LANGFUSE_PUBLIC/SECRET_KEY)
│   │   └── tests/           # 9 integration test files
│   ├── langfuse-macros/     # #[observe] proc macro (syn/quote)
│   └── langfuse-openai/     # async-openai wrapper → auto generation spans
└── *.md                     # Research docs (Python/JS SDK inventories, mapping)
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add new domain type | `langfuse-core/src/types/` | Add file, re-export in mod.rs |
| Add new error variant | `langfuse-core/src/error.rs` | Add to `LangfuseError` enum |
| New manager/feature module | `langfuse/src/` | Create module, wire into `client.rs` |
| Modify API client | `langfuse-core/build.rs` + `openapi.yml` | Codegen — don't edit api/mod.rs |
| Tracing/OTel changes | `langfuse/src/tracing/` | See `tracing/AGENTS.md` |
| Config: new field | `langfuse-core/src/config.rs` | Add to struct + builder + `from_env()` |
| OpenAI integration | `langfuse-openai/src/wrapper.rs` | TracedChat, TracedStream |
| Proc macro changes | `langfuse-macros/src/lib.rs` | Single-file crate |
| Test any module | `crates/*/tests/*_test.rs` | Integration tests only (no inline `mod tests`) |
| New observation types | `langfuse/src/tracing/span.rs` | `start_with_type`, convenience child methods (agent, tool, chain, etc.) |
| Context-aware APIs | `langfuse/src/tracing/context_apis.rs` | `update_current_span`, `score_current_span`, `get_current_trace_url` |
| Stream/Iterator observe | `langfuse/src/tracing/stream_wrapper.rs` | `ObservingStream`, `ObservingIterator` |
| Prompt CRUD | `langfuse/src/prompts/manager.rs` | `create_prompt`, `update_prompt`, `get_prompt` |
| Evaluation framework | `langfuse/src/datasets/evaluator.rs` | `Evaluator` trait + closure blanket impl |
| Named instances | `langfuse/src/client.rs` | `init_named`, `get_named`, `try_get_named` |
| Retry/HTTP | `langfuse/src/http.rs` | `build_http_client`, `retry_request` with exponential backoff |

## CODE MAP

**Crate Dependency Chain:**
```
langfuse-core  ←  langfuse-sdk  ←  langfuse-openai
                    ↑
              langfuse-macros
```

**Key Types:**

| Symbol | Crate | Role |
|--------|-------|------|
| `Langfuse` | langfuse | Main client, holds all managers, OnceLock singleton |
| `LangfuseConfig` | core | Builder pattern config, `from_env()` support |
| `LangfuseError` | core | thiserror enum: Config, Api, Auth, Network, Serialization, PromptNotFound, PromptCompilation, Media, Otel |
| `Result<T>` | core | `std::result::Result<T, LangfuseError>` alias |
| `LangfuseSpan` | langfuse | OTel span wrapper with fluent setters |
| `LangfuseGeneration` | langfuse | Deref to LangfuseSpan + model/usage/cost setters |
| `PromptManager` | langfuse | HTTP fetch + DashMap TTL cache |
| `ScoreManager` | langfuse | Batched async flush via BatchQueue |
| `DatasetManager` | langfuse | CRUD + ExperimentRunner (Semaphore bounded) |
| `MediaManager` | langfuse | Two-step presigned URL upload |
| `TracedChat` | openai | async-openai wrapper for auto generation spans |
| `#[observe]` | macros | Attribute macro: name, as_type, capture_input/output |
| `LangfuseEmbedding` | langfuse | Deref to LangfuseSpan + model/usage setters for embeddings |
| `ObservingStream<S>` | langfuse | Stream wrapper that collects items and finalizes span |
| `ObservingIterator<I>` | langfuse | Iterator wrapper with same behavior |
| `Prompt` | langfuse | Text/Chat enum for unified prompt access |
| `Evaluator` | langfuse | Async trait for dataset evaluators (+ closure blanket impl) |
| `Evaluation` | core | Evaluation result struct (name, value, comment, metadata) |
| `ScoreBodyBuilder` | core | Fluent builder for ScoreBody |
| `PropagateAttributes` | core | Trace-level attributes for propagation (user_id, session_id, etc.) |
| `SpanUpdateParams` | core | Bulk update params for spans (output, metadata, level, etc.) |

## CONVENTIONS

- **Edition 2024**, MSRV 1.93.1, resolver 2
- **Warnings = errors**: `RUSTFLAGS="-Dwarnings"` in CI, `cargo clippy -- -D warnings`
- **Error handling**: `thiserror` with `#[from]` conversions. Never `anyhow`. Custom `Result<T>` alias in core
- **Builder pattern**: Manual impl (not derive). `Option<T>` fields, `impl Into<String>` args, `build() -> Result<T, ConfigError>`
- **Module rename**: `tracing/` exposed as `langfuse_tracing` via `#[path]` to avoid conflict with `tracing` crate
- **Serialization**: `serde` derives everywhere. `rename_all = "camelCase"` for API types. `SCREAMING_SNAKE_CASE` for enums. `skip_serializing_if = "Option::is_none"`
- **Concurrency**: `DashMap` for caches, `Arc<Mutex<Vec>>` for batch buffers, `tokio::sync::Semaphore` for bounded parallelism
- **Tests**: Integration tests only (`crates/*/tests/`). No inline `mod tests`. Helper functions defined per-test-file. No mocking libraries — no-op OTel provider in tests
- **Workspace deps**: All shared deps pinned in root `Cargo.toml` `[workspace.dependencies]`
- **Context-aware APIs**: Free functions in `context_apis.rs` extract current OTel context and wrap in LangfuseSpan for mutation without explicit handles
- **Evaluator trait**: Uses `async_trait` — evaluator closures get blanket impl via `Fn(&Value, Option<&Value>) -> Future`
- **Named instances**: `DashMap<String, Langfuse>` alongside default singleton `OnceLock` for multi-project usage
- **HTTP retry**: All REST calls use `retry_request()` with exponential backoff via `backoff` crate (retries 5xx, 429, network errors)
- **Auto-flush**: ScoreManager spawns background tokio task for periodic flush at configured `flush_interval`

## ANTI-PATTERNS (THIS PROJECT)

- **DO NOT edit `crates/langfuse-core/src/api/mod.rs`** — auto-generated via `build.rs` from `openapi.yml`. Modify the spec or build script instead
- **DO NOT use `anyhow`** — all errors go through `LangfuseError` / `ConfigError`
- **DO NOT add `#[cfg(test)] mod tests` inline** — tests live in separate `tests/` directories
- **DO NOT suppress warnings** with `#[allow]` except on auto-generated code (`api` module)
- **Mutex lock poisoning**: `scoring/queue.rs` uses `expect("lock poisoned")` — do not introduce more `.expect()` on locks; prefer `unwrap_or_else(|e| e.into_inner())`
- **Global singleton** (`OnceLock`): `Langfuse::get()` panics if not initialized. Prefer `try_get()` in library code

## UNIQUE STYLES

- **Fluent setter chaining**: Span/Generation methods return `&Self` for chaining (`span.set_input(&x).set_output(&y).end()`)
- **JSON-in-string attributes**: OTel span attributes store serialized JSON as string values (Langfuse server expects this)
- **Two API patterns**: Closure-based (`with_span`/`with_generation`) and manual (`LangfuseSpan::start()` / `.end()`)
- **Progenitor workaround**: `build.rs` strips empty error response schemas before codegen (progenitor panics on mixed response types)
- **OpenAI lint escalation**: `langfuse-openai` crate enables `clippy::pedantic` and `unsafe_code = "warn"` — stricter than other crates

## COMMANDS

```bash
cargo fmt --all -- --check     # Format check
cargo clippy --workspace --all-targets -- -D warnings  # Lint
cargo build --workspace        # Build all crates
cargo test --workspace         # Run all tests
cargo doc --workspace --no-deps  # Build docs

# Run examples (requires env vars)
LANGFUSE_PUBLIC_KEY=pk-lf-... LANGFUSE_SECRET_KEY=sk-lf-... \
  cargo run --example basic_tracing

# Regenerate API client (modify openapi.yml, then)
cargo build -p langfuse-core   # Triggers build.rs
```

## NOTES

- **No feature flags**: All functionality always compiled. Crate boundaries provide modularity
- **No benchmarks or coverage**: CI runs fmt → clippy → build → test → doc only
- **`unsafe` exists only in tests**: `config_test.rs` uses `unsafe { std::env::set_var() }` — no safety comments
- **`async-openai` deprecation**: Tests use `#[allow(deprecated)]` for `function_call` field — migration to `tool_calls` needed when async-openai drops it
- **`async-trait`**: Used for `Evaluator` trait in `datasets/evaluator.rs`
- **`backoff` crate**: Added for exponential backoff retry logic in `http.rs`
- **`futures` crate**: Added for `Stream` trait support in `stream_wrapper.rs`
