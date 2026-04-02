//! Chat prompt client with `{{variable}}` template compilation across messages.

use std::collections::HashMap;

use langfuse_core::error::LangfuseError;
use langfuse_core::types::ChatMessage;

use crate::prompts::text::compile_template;

/// A compiled chat prompt. Holds a list of messages with `{{variable}}` support.
#[derive(Debug, Clone)]
pub struct ChatPromptClient {
    /// Prompt name.
    pub name: String,
    /// Prompt version.
    pub version: i32,
    /// Chat messages, each potentially containing `{{variable}}` placeholders in `content`.
    pub messages: Vec<ChatMessage>,
    /// Arbitrary configuration attached to the prompt.
    pub config: serde_json::Value,
    /// Labels associated with this prompt version.
    pub labels: Vec<String>,
    /// Tags for categorisation.
    pub tags: Vec<String>,
    /// Whether this prompt was served from an expired cache entry (fallback).
    pub is_fallback: bool,
}

impl ChatPromptClient {
    /// Compile all messages by replacing `{{variable}}` placeholders in each message's content.
    ///
    /// Returns [`LangfuseError::PromptCompilation`] if any message references a variable
    /// that is not present in the map.
    pub fn compile(
        &self,
        variables: &HashMap<String, String>,
    ) -> Result<Vec<ChatMessage>, LangfuseError> {
        self.messages
            .iter()
            .map(|msg| {
                let compiled_content = compile_template(&msg.content, variables)?;
                Ok(ChatMessage {
                    role: msg.role.clone(),
                    content: compiled_content,
                })
            })
            .collect()
    }
}
