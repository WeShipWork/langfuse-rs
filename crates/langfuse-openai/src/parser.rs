//! Response parser for async-openai types.
//!
//! Extracts model name, token usage, and output content from `OpenAI` chat
//! completion responses and stream chunks.

use async_openai::types::chat::{CreateChatCompletionResponse, CreateChatCompletionStreamResponse};
use langfuse_core::types::UsageDetails;
use std::collections::BTreeMap;

/// Extract the model name from a chat completion response.
#[must_use]
pub fn extract_model(response: &CreateChatCompletionResponse) -> String {
    response.model.clone()
}

/// Map `OpenAI`'s `CompletionUsage` to Langfuse's [`UsageDetails`].
///
/// Returns `None` when the response carries no usage information.
#[must_use]
pub fn extract_usage(response: &CreateChatCompletionResponse) -> Option<UsageDetails> {
    response.usage.as_ref().map(|u| UsageDetails {
        input: Some(u64::from(u.prompt_tokens)),
        output: Some(u64::from(u.completion_tokens)),
        total: Some(u64::from(u.total_tokens)),
    })
}

/// Extract the assistant's output from the first choice.
///
/// If the message contains tool calls the entire message is serialized as JSON.
/// Otherwise the plain text content is returned (or `null` when absent).
#[must_use]
pub fn extract_output(response: &CreateChatCompletionResponse) -> serde_json::Value {
    let Some(choice) = response.choices.first() else {
        return serde_json::Value::Null;
    };

    let message = &choice.message;

    // When tool calls are present, serialize the full message so callers can
    // inspect both the text and the tool invocations.
    if message.tool_calls.as_ref().is_some_and(|tc| !tc.is_empty()) {
        return serde_json::to_value(message).unwrap_or(serde_json::Value::Null);
    }

    match &message.content {
        Some(text) => serde_json::Value::String(text.clone()),
        None => serde_json::Value::Null,
    }
}

/// Extract the delta content from a single stream chunk.
///
/// Returns `None` when the chunk carries no content delta (e.g. role-only or
/// final usage chunk).
#[must_use]
pub fn extract_stream_chunk_content(chunk: &CreateChatCompletionStreamResponse) -> Option<String> {
    chunk.choices.first().and_then(|c| c.delta.content.clone())
}

/// Extract usage information from a stream chunk.
///
/// `OpenAI` sends usage in the final chunk when
/// `stream_options.include_usage = true`.
#[must_use]
pub fn extract_stream_usage(chunk: &CreateChatCompletionStreamResponse) -> Option<UsageDetails> {
    chunk.usage.as_ref().map(|u| UsageDetails {
        input: Some(u64::from(u.prompt_tokens)),
        output: Some(u64::from(u.completion_tokens)),
        total: Some(u64::from(u.total_tokens)),
    })
}

/// Extract tool calls from a non-streaming chat completion response.
///
/// Returns a JSON array of objects with `id`, `type`, `function.name`, and
/// `function.arguments` for each tool call. Returns `None` if no tool calls
/// are present.
#[must_use]
pub fn extract_tool_calls(response: &CreateChatCompletionResponse) -> Option<serde_json::Value> {
    let choice = response.choices.first()?;
    let tool_calls = choice.message.tool_calls.as_ref()?;
    if tool_calls.is_empty() {
        return None;
    }
    serde_json::to_value(tool_calls).ok()
}

/// Accumulated state for tool call deltas during streaming.
///
/// `OpenAI` streams tool calls as incremental chunks identified by `index`.
/// Each chunk may carry partial `id`, `type`, `function.name`, or
/// `function.arguments` that must be concatenated.
#[derive(Debug, Default, Clone)]
pub struct ToolCallAccumulator {
    calls: BTreeMap<usize, AccumulatedToolCall>,
}

#[derive(Debug, Default, Clone)]
struct AccumulatedToolCall {
    id: String,
    r#type: String,
    function_name: String,
    function_arguments: String,
}

impl AccumulatedToolCall {
    fn is_empty(&self) -> bool {
        self.id.is_empty()
            && self.r#type.is_empty()
            && self.function_name.is_empty()
            && self.function_arguments.is_empty()
    }
}

impl ToolCallAccumulator {
    /// Create a new empty accumulator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge a stream chunk's tool call deltas into the accumulated state.
    ///
    /// Each delta carries an `index` that identifies which tool call it belongs
    /// to. The accumulator stores entries by index so sparse updates do not
    /// create empty placeholder calls.
    pub fn accumulate(&mut self, chunk: &CreateChatCompletionStreamResponse) {
        let Some(choice) = chunk.choices.first() else {
            return;
        };
        let Some(tool_calls) = &choice.delta.tool_calls else {
            return;
        };
        for tc in tool_calls {
            let idx = tc.index as usize;
            let entry = self.calls.entry(idx).or_default();
            if let Some(id) = &tc.id {
                entry.id.push_str(id);
            }
            if let Some(t) = &tc.r#type {
                entry.r#type = serde_json::to_value(t)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default();
            }
            if let Some(func) = &tc.function {
                if let Some(name) = &func.name {
                    entry.function_name.push_str(name);
                }
                if let Some(args) = &func.arguments {
                    entry.function_arguments.push_str(args);
                }
            }
        }
    }

    /// Returns `true` when at least one tool call has been accumulated.
    #[must_use]
    pub fn has_calls(&self) -> bool {
        self.calls.values().any(|call| !call.is_empty())
    }

    /// Finalize the accumulated tool calls into a JSON array.
    ///
    /// Each element mirrors the non-streaming `ChatCompletionMessageToolCall`
    /// structure: `{ "id", "type", "function": { "name", "arguments" } }`.
    #[must_use]
    pub fn finalize(&self) -> serde_json::Value {
        let arr: Vec<serde_json::Value> = self
            .calls
            .values()
            .filter(|call| !call.is_empty())
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "type": c.r#type,
                    "function": {
                        "name": c.function_name,
                        "arguments": c.function_arguments
                    }
                })
            })
            .collect();
        serde_json::Value::Array(arr)
    }
}
