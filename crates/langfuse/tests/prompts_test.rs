use std::collections::HashMap;
use std::time::Duration;

use langfuse::prompts::cache::PromptCache;
use langfuse::prompts::chat::ChatPromptClient;
use langfuse::prompts::text::TextPromptClient;
use langfuse_core::types::ChatMessage;

// ── Helpers ──────────────────────────────────────────────────────────────

fn text_prompt(template: &str) -> TextPromptClient {
    TextPromptClient {
        name: "test".into(),
        version: 1,
        template: template.into(),
        config: serde_json::Value::Null,
        labels: vec![],
        tags: vec![],
        is_fallback: false,
    }
}

fn chat_prompt(messages: Vec<(&str, &str)>) -> ChatPromptClient {
    ChatPromptClient {
        name: "test-chat".into(),
        version: 1,
        messages: messages
            .into_iter()
            .map(|(role, content)| ChatMessage {
                role: role.into(),
                content: content.into(),
            })
            .collect(),
        config: serde_json::Value::Null,
        labels: vec![],
        tags: vec![],
        is_fallback: false,
    }
}

// ── TextPromptClient::compile ────────────────────────────────────────────

#[test]
fn test_text_prompt_compile_simple() {
    let prompt = text_prompt("Hello {{name}}!");
    let mut vars = HashMap::new();
    vars.insert("name".into(), "World".into());
    let result = prompt.compile(&vars).unwrap();
    assert_eq!(result, "Hello World!");
}

#[test]
fn test_text_prompt_compile_missing_var() {
    let prompt = text_prompt("Hello {{name}}!");
    let vars = HashMap::new();
    let result = prompt.compile(&vars);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("name"),
        "error should mention the missing variable 'name', got: {err}"
    );
}

#[test]
fn test_text_prompt_compile_multiple_vars() {
    let prompt = text_prompt("Dear {{title}} {{last_name}}, your order #{{order_id}} is ready.");
    let mut vars = HashMap::new();
    vars.insert("title".into(), "Dr.".into());
    vars.insert("last_name".into(), "Smith".into());
    vars.insert("order_id".into(), "42".into());
    let result = prompt.compile(&vars).unwrap();
    assert_eq!(result, "Dear Dr. Smith, your order #42 is ready.");
}

#[test]
fn test_text_prompt_compile_no_placeholders() {
    let prompt = text_prompt("No variables here.");
    let vars = HashMap::new();
    let result = prompt.compile(&vars).unwrap();
    assert_eq!(result, "No variables here.");
}

#[test]
fn test_text_prompt_compile_whitespace_in_braces() {
    let prompt = text_prompt("Hello {{ name }}!");
    let mut vars = HashMap::new();
    vars.insert("name".into(), "World".into());
    let result = prompt.compile(&vars).unwrap();
    assert_eq!(result, "Hello World!");
}

#[test]
fn test_text_prompt_compile_repeated_var() {
    let prompt = text_prompt("{{x}} and {{x}} again");
    let mut vars = HashMap::new();
    vars.insert("x".into(), "val".into());
    let result = prompt.compile(&vars).unwrap();
    assert_eq!(result, "val and val again");
}

// ── ChatPromptClient::compile ────────────────────────────────────────────

#[test]
fn test_chat_prompt_compile() {
    let prompt = chat_prompt(vec![
        ("system", "You are a {{role}} assistant."),
        ("user", "Help me with {{topic}}."),
    ]);
    let mut vars = HashMap::new();
    vars.insert("role".into(), "helpful".into());
    vars.insert("topic".into(), "Rust".into());
    let result = prompt.compile(&vars).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].role, "system");
    assert_eq!(result[0].content, "You are a helpful assistant.");
    assert_eq!(result[1].role, "user");
    assert_eq!(result[1].content, "Help me with Rust.");
}

#[test]
fn test_chat_prompt_compile_missing_var() {
    let prompt = chat_prompt(vec![("user", "Hello {{name}}")]);
    let vars = HashMap::new();
    let result = prompt.compile(&vars);
    assert!(result.is_err());
}

// ── PromptCache ──────────────────────────────────────────────────────────

#[test]
fn test_cache_put_and_get() {
    let cache = PromptCache::new(Duration::from_secs(60));
    let prompt = text_prompt("cached");
    cache.put_text("test:1", prompt);
    let cached = cache.get_text("test:1");
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().template, "cached");
}

#[test]
fn test_cache_expired_entry() {
    let cache = PromptCache::new(Duration::from_millis(1));
    let prompt = text_prompt("expires");
    cache.put_text("test:1", prompt);
    std::thread::sleep(Duration::from_millis(10));
    let cached = cache.get_text("test:1");
    assert!(cached.is_none());
}

#[test]
fn test_cache_clear() {
    let cache = PromptCache::new(Duration::from_secs(60));
    cache.put_text("test:1", text_prompt("a"));
    cache.put_chat("chat:1", chat_prompt(vec![("user", "hi")]));
    cache.clear();
    assert!(cache.get_text("test:1").is_none());
    assert!(cache.get_chat("chat:1").is_none());
}

#[test]
fn test_cache_miss() {
    let cache = PromptCache::new(Duration::from_secs(60));
    assert!(cache.get_text("nonexistent").is_none());
    assert!(cache.get_chat("nonexistent").is_none());
}

#[test]
fn test_cache_chat_put_and_get() {
    let cache = PromptCache::new(Duration::from_secs(60));
    let prompt = chat_prompt(vec![("system", "hello")]);
    cache.put_chat("chat:1", prompt);
    let cached = cache.get_chat("chat:1");
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().messages[0].content, "hello");
}
