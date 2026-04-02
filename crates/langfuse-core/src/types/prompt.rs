use serde::{Deserialize, Serialize};

/// The type of prompt template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptType {
    /// A plain text prompt template.
    Text,
    /// A chat prompt template (list of messages).
    Chat,
}

/// A chat message in a chat prompt template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message sender (e.g. "system", "user", "assistant").
    pub role: String,
    /// The message content.
    pub content: String,
}
