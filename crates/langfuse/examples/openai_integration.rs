//! `OpenAI` integration example showing automatic instrumentation of `OpenAI` API calls.
//!
//! This example demonstrates:
//! - Initializing Langfuse
//! - Wrapping an `OpenAI` client with `observe_openai`
//! - Automatic generation span creation for chat completions
//!
//! Note: This example compiles but requires valid API keys to actually execute.
//!
//! Run: OPENAI_API_KEY=sk-... LANGFUSE_PUBLIC_KEY=pk-... LANGFUSE_SECRET_KEY=sk-... cargo run --example openai_integration

use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent, CreateChatCompletionRequestArgs,
};
use langfuse::{Langfuse, LangfuseConfig};
use langfuse_openai::observe_openai;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OpenAI Integration Example ===\n");

    // Initialize Langfuse from environment variables
    let config = LangfuseConfig::from_env()?;
    let langfuse = Langfuse::new(config)?;
    println!("✓ Langfuse initialized");

    // Initialize the OpenAI client (reads OPENAI_API_KEY from env)
    let openai_client = async_openai::Client::new();
    println!("✓ OpenAI client initialized");

    // Wrap the client with Langfuse tracing — every call through `traced`
    // automatically creates a generation span in Langfuse.
    let traced = observe_openai(&openai_client);

    // Build a chat completion request
    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o-mini")
        .messages(vec![ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(
                    "What is Rust? Answer in one sentence.".to_string(),
                ),
                name: None,
            },
        )])
        .build()?;

    // This call is automatically traced — model, usage, input, and output
    // are recorded as a Langfuse generation span.
    let response = traced.create(request).await?;

    let content = response
        .choices
        .first()
        .and_then(|c| c.message.content.as_deref())
        .unwrap_or("<no content>");
    println!("\nResponse: {content}");

    if let Some(usage) = &response.usage {
        println!(
            "Usage: {} prompt + {} completion = {} total tokens",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    // Flush Langfuse to ensure spans are sent
    langfuse.flush().await?;

    println!("\n✓ Generation span recorded in Langfuse!");
    Ok(())
}
