use std::time::Duration;

use langfuse::prompts::cache::PromptCache;
use langfuse::prompts::chat::ChatPromptClient;
use langfuse::prompts::text::TextPromptClient;
use langfuse::prompts::types::Prompt;
use langfuse_core::types::ChatMessage;

// ── Helpers ──────────────────────────────────────────────────────────────

fn make_text(name: &str, version: i32) -> TextPromptClient {
    TextPromptClient {
        name: name.into(),
        version,
        template: "Hello {{name}}!".into(),
        config: serde_json::Value::Null,
        labels: vec!["production".into()],
        tags: vec!["test".into()],
        is_fallback: false,
    }
}

fn make_chat(name: &str, version: i32) -> ChatPromptClient {
    ChatPromptClient {
        name: name.into(),
        version,
        messages: vec![ChatMessage {
            role: "system".into(),
            content: "You are a {{role}} assistant.".into(),
        }],
        config: serde_json::Value::Null,
        labels: vec![],
        tags: vec![],
        is_fallback: false,
    }
}

// ── Prompt enum construction and matching ────────────────────────────────

#[test]
fn test_prompt_enum_text_variant() {
    let text = make_text("my-prompt", 1);
    let prompt = Prompt::Text(text);

    assert!(prompt.is_text());
    assert!(!prompt.is_chat());
    assert_eq!(prompt.name(), "my-prompt");
    assert_eq!(prompt.version(), 1);
    assert!(!prompt.is_fallback());
}

#[test]
fn test_prompt_enum_chat_variant() {
    let chat = make_chat("chat-prompt", 3);
    let prompt = Prompt::Chat(chat);

    assert!(!prompt.is_text());
    assert!(prompt.is_chat());
    assert_eq!(prompt.name(), "chat-prompt");
    assert_eq!(prompt.version(), 3);
    assert!(!prompt.is_fallback());
}

#[test]
fn test_prompt_as_text() {
    let text = make_text("t", 1);
    let prompt = Prompt::Text(text.clone());

    let inner = prompt.as_text().expect("should be text");
    assert_eq!(inner.name, "t");
    assert_eq!(inner.template, text.template);

    assert!(prompt.as_chat().is_none());
}

#[test]
fn test_prompt_as_chat() {
    let chat = make_chat("c", 2);
    let prompt = Prompt::Chat(chat.clone());

    let inner = prompt.as_chat().expect("should be chat");
    assert_eq!(inner.name, "c");
    assert_eq!(inner.messages.len(), chat.messages.len());

    assert!(prompt.as_text().is_none());
}

// ── is_fallback field ────────────────────────────────────────────────────

#[test]
fn test_is_fallback_defaults_to_false() {
    let text = make_text("p", 1);
    assert!(!text.is_fallback);

    let chat = make_chat("p", 1);
    assert!(!chat.is_fallback);
}

#[test]
fn test_is_fallback_can_be_set_true() {
    let mut text = make_text("p", 1);
    text.is_fallback = true;
    assert!(text.is_fallback);

    let prompt = Prompt::Text(text);
    assert!(prompt.is_fallback());
}

// ── Cache invalidation by prefix ─────────────────────────────────────────

#[test]
fn test_cache_invalidate_by_prefix() {
    let cache = PromptCache::new(Duration::from_secs(60));

    cache.put_text("my-prompt:1", make_text("my-prompt", 1));
    cache.put_text("my-prompt:2", make_text("my-prompt", 2));
    cache.put_text("my-prompt:latest", make_text("my-prompt", 3));
    cache.put_text("other-prompt:1", make_text("other-prompt", 1));

    cache.invalidate_by_prefix("my-prompt:");

    assert!(cache.get_text("my-prompt:1").is_none());
    assert!(cache.get_text("my-prompt:2").is_none());
    assert!(cache.get_text("my-prompt:latest").is_none());
    // Other prompt should be unaffected.
    assert!(cache.get_text("other-prompt:1").is_some());
}

#[test]
fn test_cache_invalidate_by_prefix_chat() {
    let cache = PromptCache::new(Duration::from_secs(60));

    cache.put_chat("chat:1", make_chat("chat", 1));
    cache.put_chat("chat:latest", make_chat("chat", 2));
    cache.put_text("chat:1", make_text("chat", 1));

    cache.invalidate_by_prefix("chat:");

    assert!(cache.get_chat("chat:1").is_none());
    assert!(cache.get_chat("chat:latest").is_none());
    assert!(cache.get_text("chat:1").is_none());
}

// ── Expired cache retrieval (fallback support) ───────────────────────────

#[test]
fn test_get_text_expired_returns_entry_after_ttl() {
    let cache = PromptCache::new(Duration::from_millis(1));
    cache.put_text("p:1", make_text("p", 1));

    // Wait for expiry.
    std::thread::sleep(Duration::from_millis(10));

    // Expired get should return the entry even though TTL has passed.
    let expired = cache.get_text_expired("p:1");
    assert!(expired.is_some());
    assert_eq!(expired.unwrap().name, "p");

    // Normal get should return None (expired) and evict the entry.
    assert!(cache.get_text("p:1").is_none());
}

#[test]
fn test_get_chat_expired_returns_entry_after_ttl() {
    let cache = PromptCache::new(Duration::from_millis(1));
    cache.put_chat("c:1", make_chat("c", 1));

    std::thread::sleep(Duration::from_millis(10));

    // Expired get should return the entry even though TTL has passed.
    let expired = cache.get_chat_expired("c:1");
    assert!(expired.is_some());
    assert_eq!(expired.unwrap().name, "c");

    // Normal get should return None (expired) and evict the entry.
    assert!(cache.get_chat("c:1").is_none());
}

#[test]
fn test_get_text_expired_returns_none_when_no_entry() {
    let cache = PromptCache::new(Duration::from_secs(60));
    assert!(cache.get_text_expired("nonexistent").is_none());
}

#[test]
fn test_get_chat_expired_returns_none_when_no_entry() {
    let cache = PromptCache::new(Duration::from_secs(60));
    assert!(cache.get_chat_expired("nonexistent").is_none());
}

// ── Prompt enum with fallback flag ───────────────────────────────────────

#[test]
fn test_prompt_enum_reflects_fallback_from_text() {
    let mut text = make_text("fb", 1);
    text.is_fallback = true;
    let prompt = Prompt::Text(text);
    assert!(prompt.is_fallback());
}

#[test]
fn test_prompt_enum_reflects_fallback_from_chat() {
    let mut chat = make_chat("fb", 1);
    chat.is_fallback = true;
    let prompt = Prompt::Chat(chat);
    assert!(prompt.is_fallback());
}

// ── get_text_expired does not remove entry from cache ────────────────────

#[test]
fn test_get_text_expired_does_not_evict() {
    let cache = PromptCache::new(Duration::from_millis(1));
    cache.put_text("p:1", make_text("p", 1));

    std::thread::sleep(Duration::from_millis(10));

    // First expired get.
    assert!(cache.get_text_expired("p:1").is_some());
    // Second expired get should also work (entry not evicted).
    assert!(cache.get_text_expired("p:1").is_some());
}

// ── Note: get_text evicts expired entries, so expired get after normal get may fail ──

#[test]
fn test_normal_get_evicts_then_expired_get_returns_none() {
    let cache = PromptCache::new(Duration::from_millis(1));
    cache.put_text("p:1", make_text("p", 1));

    std::thread::sleep(Duration::from_millis(10));

    // Normal get evicts the expired entry.
    assert!(cache.get_text("p:1").is_none());
    // Now expired get also returns None because the entry was removed.
    assert!(cache.get_text_expired("p:1").is_none());
}
