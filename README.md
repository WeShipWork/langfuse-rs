<p align="center">
  <img src="assets/langfuse-sdk-logo.png" alt="langfuse-rs logo" width="400">
</p>

# langfuse-rs

[![CI](https://github.com/weshipwork/langfuse-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/weshipwork/langfuse-rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/langfuse-sdk.svg)](https://crates.io/crates/langfuse-sdk)
[![docs.rs](https://docs.rs/langfuse-sdk/badge.svg)](https://docs.rs/langfuse-sdk)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
![MSRV](https://img.shields.io/badge/MSRV-1.93.1-blue.svg)

Unofficial Rust SDK for [Langfuse](https://langfuse.com) — open-source LLM observability, prompt management, and evaluation.

## Features

- OTel-native tracing (spans, generations) via OTLP HTTP export to Langfuse's `/api/public/otel` endpoint
- Prompt management with local caching (TTL-based) and `{{variable}}` compilation
- Score management with batched async flush
- Dataset and experiment management with bounded concurrency
- Media handling (upload, fetch, data URI parsing)
- OpenAI integration (`langfuse-openai`) — automatic generation spans for `async-openai` chat completions
- `#[observe]` proc macro for zero-boilerplate tracing
- Closure-based API (`with_span`, `with_generation`) for scoped instrumentation
- Auto-generated REST API client from Langfuse's OpenAPI spec

## Crate Structure

| Package | Import path | Description |
|---------|-------------|-------------|
| `langfuse-sdk` | `langfuse` | Main SDK — tracing, prompts, scores, datasets, media |
| `langfuse-core` | `langfuse_core` | Core types, config, errors, auto-generated API client |
| `langfuse-macros` | `langfuse_macros` | `#[observe]` proc macro |
| `langfuse-openai` | `langfuse_openai` | OpenAI integration wrapping `async-openai` |

## Quick Start

```toml
[dependencies]
langfuse-sdk = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

> **Note on naming:** `langfuse-rs` is already taken on crates.io, so this project publishes the main package as `langfuse-sdk` while keeping the Rust import path as `use langfuse::*`. This keeps your code clean while avoiding the registry name conflict.

```rust
use langfuse::{Langfuse, LangfuseConfig, with_span, with_generation};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LangfuseConfig::builder()
        .public_key("pk-lf-...")
        .secret_key("sk-lf-...")
        .build()?;
    let langfuse = Langfuse::new(config)?;

    with_span("my-pipeline", || {
        with_generation("llm-call", || {
            // Your LLM call here
            "Hello from Langfuse!"
        })
    });

    langfuse.shutdown().await?;
    Ok(())
}
```

## Environment Variables

| Variable | Required | Default |
|----------|----------|---------|
| `LANGFUSE_PUBLIC_KEY` | Yes | |
| `LANGFUSE_SECRET_KEY` | Yes | |
| `LANGFUSE_BASE_URL` | No | `https://cloud.langfuse.com` |
| `LANGFUSE_TIMEOUT` | No | `5s` |
| `LANGFUSE_SAMPLE_RATE` | No | `1.0` |

Load config directly from the environment:

```rust
let config = LangfuseConfig::from_env()?;
```

## OpenAI Integration

Add the integration crate:

```toml
[dependencies]
langfuse-openai = "0.1"
async-openai = "0.27"
```

Wrap your existing client to get automatic generation spans:

```rust
use langfuse_openai::observe_openai;
use async_openai::Client;

let client = Client::new();
let traced = observe_openai(&client);
let response = traced.create(request).await?;
// Automatic generation span with model, usage, input/output
```

## `#[observe]` Macro

Add the macro feature or use the `langfuse-macros` crate directly:

```rust
use langfuse::observe;

#[observe(name = "my-function", capture_input, capture_output)]
async fn my_llm_call(prompt: &str) -> String {
    // Automatically creates a span with input/output capture
    format!("Response to: {prompt}")
}
```

## Examples

Runnable examples live in `crates/langfuse/examples/`:

| Example | Description |
|---------|-------------|
| `basic_tracing.rs` | Manual spans, closures, `#[observe]` macro |
| `prompt_management.rs` | Fetch, cache, and compile prompts |
| `experiment.rs` | Dataset creation and experiment runner |
| `openai_integration.rs` | Traced OpenAI chat completions |

Run an example:

```sh
LANGFUSE_PUBLIC_KEY=pk-lf-... LANGFUSE_SECRET_KEY=sk-lf-... \
  cargo run --example basic_tracing
```

## Requirements

- Rust 1.93.1+ (MSRV), edition 2024
- Tokio async runtime

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.

## Contributing

Contributions welcome! Please open an issue first to discuss what you'd like to change.

Repository note: internal planning and agent workflow artifacts (for example `openspec/`, `docs/`, and selected `.claude/` paths) are not part of the tracked source tree.

---

*This is an unofficial community SDK and is not maintained by Langfuse GmbH.*
