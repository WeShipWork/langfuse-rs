//! Prompt management example showing fetching, caching, and compiling prompts.
//!
//! This example demonstrates:
//! - Fetching text and chat prompts from the Langfuse API
//! - Automatic caching with TTL
//! - Compiling prompts with variables
//! - Error handling for missing prompts
//!
//! Run: LANGFUSE_PUBLIC_KEY=pk-... LANGFUSE_SECRET_KEY=sk-... cargo run --example prompt_management

use langfuse::{Langfuse, LangfuseConfig};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Langfuse from environment variables
    let config = LangfuseConfig::from_env()?;
    let langfuse = Langfuse::new(config)?;

    println!("=== Prompt Management Example ===\n");

    // Example 1: Fetch a text prompt by name (latest version)
    println!("Example 1: Fetching text prompt (latest)");
    match langfuse
        .prompts
        .get_text_prompt("greeting", None, None)
        .await
    {
        Ok(prompt) => {
            println!("  ✓ Fetched prompt: {}", prompt.name);
            println!("  Version: {}", prompt.version);

            // Compile the prompt with variables
            let mut vars = HashMap::new();
            vars.insert("name".to_string(), "Alice".to_string());
            vars.insert("language".to_string(), "Rust".to_string());
            match prompt.compile(&vars) {
                Ok(compiled) => println!("  Compiled: {}", compiled),
                Err(e) => println!("  Compilation error: {}", e),
            }
        }
        Err(e) => {
            println!(
                "  ✗ Error: {} (expected if prompt doesn't exist on server)",
                e
            );
        }
    }

    // Example 2: Fetch a text prompt by specific version
    println!("\nExample 2: Fetching text prompt (specific version)");
    match langfuse
        .prompts
        .get_text_prompt("greeting", Some(2), None)
        .await
    {
        Ok(prompt) => {
            println!("  ✓ Fetched prompt version: {}", prompt.version);
        }
        Err(e) => {
            println!("  ✗ Error: {} (expected if version doesn't exist)", e);
        }
    }

    // Example 3: Fetch a text prompt by label
    println!("\nExample 3: Fetching text prompt (by label)");
    match langfuse
        .prompts
        .get_text_prompt("greeting", None, Some("production"))
        .await
    {
        Ok(prompt) => {
            println!("  ✓ Fetched prompt with label 'production'");
            println!("  Version: {}", prompt.version);
        }
        Err(e) => {
            println!("  ✗ Error: {} (expected if label doesn't exist)", e);
        }
    }

    // Example 4: Fetch a chat prompt
    println!("\nExample 4: Fetching chat prompt");
    match langfuse
        .prompts
        .get_chat_prompt("qa-assistant", None, None)
        .await
    {
        Ok(prompt) => {
            println!("  ✓ Fetched chat prompt: {}", prompt.name);
            println!("  Messages: {}", prompt.messages.len());

            // Compile the chat prompt with variables
            let mut vars = HashMap::new();
            vars.insert("question".to_string(), "What is Rust?".to_string());
            vars.insert(
                "context".to_string(),
                "A systems programming language".to_string(),
            );
            match prompt.compile(&vars) {
                Ok(compiled) => println!("  Compiled {} messages", compiled.len()),
                Err(e) => println!("  Compilation error: {}", e),
            }
        }
        Err(e) => {
            println!(
                "  ✗ Error: {} (expected if prompt doesn't exist on server)",
                e
            );
        }
    }

    // Example 5: Caching demonstration
    println!("\nExample 5: Caching demonstration");
    println!("  Fetching 'greeting' prompt twice (second should be cached)...");

    let start = std::time::Instant::now();
    let _prompt1 = langfuse
        .prompts
        .get_text_prompt("greeting", None, None)
        .await;
    let first_duration = start.elapsed();
    println!("  First fetch: {:?}", first_duration);

    let start = std::time::Instant::now();
    let _prompt2 = langfuse
        .prompts
        .get_text_prompt("greeting", None, None)
        .await;
    let second_duration = start.elapsed();
    println!(
        "  Second fetch: {:?} (should be faster if cached)",
        second_duration
    );

    println!("\nPrompt management example completed!");
    Ok(())
}
