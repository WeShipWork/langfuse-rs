//! Thread-safe prompt cache with per-entry TTL.

use std::time::{Duration, Instant};

use dashmap::DashMap;

use crate::prompts::chat::ChatPromptClient;
use crate::prompts::text::TextPromptClient;

/// Internal cache entry wrapping a value with insertion time and TTL.
struct CacheEntry<T> {
    value: T,
    inserted_at: Instant,
    ttl: Duration,
}

impl<T> CacheEntry<T> {
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() >= self.ttl
    }
}

/// Thread-safe prompt cache backed by [`DashMap`].
///
/// Each entry carries its own TTL (defaulting to `default_ttl`). Expired entries are
/// lazily evicted on access.
pub struct PromptCache {
    text_entries: DashMap<String, CacheEntry<TextPromptClient>>,
    chat_entries: DashMap<String, CacheEntry<ChatPromptClient>>,
    default_ttl: Duration,
}

impl PromptCache {
    /// Create a new cache with the given default TTL for entries.
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            text_entries: DashMap::new(),
            chat_entries: DashMap::new(),
            default_ttl,
        }
    }

    // ── Text prompts ──────────────────────────────────────────────────

    /// Retrieve a cached text prompt if it exists and has not expired.
    ///
    /// Cache key format: `"{name}:{version}"` or `"{name}:latest"`.
    pub fn get_text(&self, key: &str) -> Option<TextPromptClient> {
        let entry = self.text_entries.get(key)?;
        if entry.is_expired() {
            drop(entry);
            self.text_entries.remove(key);
            return None;
        }
        Some(entry.value.clone())
    }

    /// Insert (or replace) a text prompt in the cache using the default TTL.
    pub fn put_text(&self, key: &str, prompt: TextPromptClient) {
        self.text_entries.insert(
            key.to_owned(),
            CacheEntry {
                value: prompt,
                inserted_at: Instant::now(),
                ttl: self.default_ttl,
            },
        );
    }

    /// Retrieve a cached text prompt even if it has expired.
    ///
    /// Used for fallback behavior: when the API is unreachable, an expired cached
    /// entry is better than no entry at all.
    pub fn get_text_expired(&self, key: &str) -> Option<TextPromptClient> {
        self.text_entries.get(key).map(|entry| entry.value.clone())
    }

    // ── Chat prompts ──────────────────────────────────────────────────

    /// Retrieve a cached chat prompt if it exists and has not expired.
    pub fn get_chat(&self, key: &str) -> Option<ChatPromptClient> {
        let entry = self.chat_entries.get(key)?;
        if entry.is_expired() {
            drop(entry);
            self.chat_entries.remove(key);
            return None;
        }
        Some(entry.value.clone())
    }

    /// Insert (or replace) a chat prompt in the cache using the default TTL.
    pub fn put_chat(&self, key: &str, prompt: ChatPromptClient) {
        self.chat_entries.insert(
            key.to_owned(),
            CacheEntry {
                value: prompt,
                inserted_at: Instant::now(),
                ttl: self.default_ttl,
            },
        );
    }

    /// Retrieve a cached chat prompt even if it has expired.
    ///
    /// Used for fallback behavior: when the API is unreachable, an expired cached
    /// entry is better than no entry at all.
    pub fn get_chat_expired(&self, key: &str) -> Option<ChatPromptClient> {
        self.chat_entries.get(key).map(|entry| entry.value.clone())
    }

    // ── Maintenance ───────────────────────────────────────────────────

    /// Remove all entries from the cache.
    pub fn clear(&self) {
        self.text_entries.clear();
        self.chat_entries.clear();
    }

    /// Remove all entries whose key starts with the given prefix.
    ///
    /// Used to invalidate all cached versions/labels for a prompt name after
    /// a create or update operation.
    pub fn invalidate_by_prefix(&self, prefix: &str) {
        self.text_entries.retain(|k, _| !k.starts_with(prefix));
        self.chat_entries.retain(|k, _| !k.starts_with(prefix));
    }
}
