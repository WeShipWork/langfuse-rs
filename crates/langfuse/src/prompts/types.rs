//! Unified prompt type wrapping text and chat variants.

use crate::prompts::chat::ChatPromptClient;
use crate::prompts::text::TextPromptClient;

/// Unified prompt type — either Text or Chat.
///
/// Returned by [`PromptManager::get_prompt`](crate::prompts::manager::PromptManager::get_prompt).
/// Use [`is_text`](Prompt::is_text) / [`is_chat`](Prompt::is_chat) or pattern
/// matching to determine the variant.
#[derive(Debug, Clone)]
pub enum Prompt {
    /// A text prompt with `{{variable}}` template support.
    Text(TextPromptClient),
    /// A chat prompt with a list of messages.
    Chat(ChatPromptClient),
}

impl Prompt {
    /// Returns `true` if this is a text prompt.
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Returns `true` if this is a chat prompt.
    pub fn is_chat(&self) -> bool {
        matches!(self, Self::Chat(_))
    }

    /// Returns a reference to the inner [`TextPromptClient`] if this is a text prompt.
    pub fn as_text(&self) -> Option<&TextPromptClient> {
        match self {
            Self::Text(t) => Some(t),
            Self::Chat(_) => None,
        }
    }

    /// Returns a reference to the inner [`ChatPromptClient`] if this is a chat prompt.
    pub fn as_chat(&self) -> Option<&ChatPromptClient> {
        match self {
            Self::Text(_) => None,
            Self::Chat(c) => Some(c),
        }
    }

    /// Returns the prompt name regardless of variant.
    pub fn name(&self) -> &str {
        match self {
            Self::Text(t) => &t.name,
            Self::Chat(c) => &c.name,
        }
    }

    /// Returns the prompt version regardless of variant.
    pub fn version(&self) -> i32 {
        match self {
            Self::Text(t) => t.version,
            Self::Chat(c) => c.version,
        }
    }

    /// Returns whether this prompt was served from an expired cache entry (fallback).
    pub fn is_fallback(&self) -> bool {
        match self {
            Self::Text(t) => t.is_fallback,
            Self::Chat(c) => c.is_fallback,
        }
    }
}
