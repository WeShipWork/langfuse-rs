//! Tests for the langfuse-openai crate.
//!
//! `async-openai` marks several struct fields as `#[deprecated]` (e.g.
//! `function_call`, `system_fingerprint`). We must set them when constructing
//! test values because the structs lack `Default`. This module-level allow
//! covers all such usages.
#![allow(deprecated)]

use async_openai::types::chat::{
    ChatChoice, ChatChoiceStream, ChatCompletionMessageToolCall,
    ChatCompletionMessageToolCallChunk, ChatCompletionMessageToolCalls,
    ChatCompletionResponseMessage, ChatCompletionStreamResponseDelta, CompletionUsage,
    CreateChatCompletionResponse, CreateChatCompletionStreamResponse, FunctionCall,
    FunctionCallStream, FunctionType, Role,
};
use langfuse_openai::parser::{self, ToolCallAccumulator};

// ---------------------------------------------------------------------------
// Helper constructors (async-openai structs lack Default)
// ---------------------------------------------------------------------------

fn make_response(
    model: &str,
    content: Option<&str>,
    usage: Option<CompletionUsage>,
) -> CreateChatCompletionResponse {
    CreateChatCompletionResponse {
        id: "chatcmpl-test".to_string(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatCompletionResponseMessage {
                content: content.map(String::from),
                refusal: None,
                tool_calls: None,
                annotations: None,
                role: Role::Assistant,
                function_call: None,
                audio: None,
            },
            finish_reason: None,
            logprobs: None,
        }],
        created: 1_700_000_000,
        model: model.to_string(),
        service_tier: None,
        system_fingerprint: None,
        object: "chat.completion".to_string(),
        usage,
    }
}

fn make_usage(prompt: u32, completion: u32, total: u32) -> CompletionUsage {
    CompletionUsage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: total,
        prompt_tokens_details: None,
        completion_tokens_details: None,
    }
}

fn make_stream_chunk(
    content: Option<&str>,
    usage: Option<CompletionUsage>,
) -> CreateChatCompletionStreamResponse {
    CreateChatCompletionStreamResponse {
        id: "chatcmpl-test".to_string(),
        choices: vec![ChatChoiceStream {
            index: 0,
            delta: ChatCompletionStreamResponseDelta {
                content: content.map(String::from),
                function_call: None,
                tool_calls: None,
                role: None,
                refusal: None,
            },
            finish_reason: None,
            logprobs: None,
        }],
        created: 1_700_000_000,
        model: "gpt-4".to_string(),
        service_tier: None,
        system_fingerprint: None,
        object: "chat.completion.chunk".to_string(),
        usage,
    }
}

// ---------------------------------------------------------------------------
// Parser tests
// ---------------------------------------------------------------------------

#[test]
fn test_extract_model() {
    let response = make_response("gpt-4o-mini", Some("Hello!"), None);
    assert_eq!(parser::extract_model(&response), "gpt-4o-mini");
}

#[test]
fn test_extract_model_custom() {
    let response = make_response("ft:gpt-4o:my-org:custom:id", Some("Hi"), None);
    assert_eq!(
        parser::extract_model(&response),
        "ft:gpt-4o:my-org:custom:id"
    );
}

#[test]
fn test_extract_usage() {
    let usage = make_usage(10, 20, 30);
    let response = make_response("gpt-4", Some("Hello"), Some(usage));
    let result = parser::extract_usage(&response).expect("expected usage");
    assert_eq!(result.input, Some(10));
    assert_eq!(result.output, Some(20));
    assert_eq!(result.total, Some(30));
}

#[test]
fn test_extract_usage_none() {
    let response = make_response("gpt-4", Some("Hello"), None);
    assert!(parser::extract_usage(&response).is_none());
}

#[test]
fn test_extract_output_text() {
    let response = make_response("gpt-4", Some("Hello, world!"), None);
    let output = parser::extract_output(&response);
    assert_eq!(
        output,
        serde_json::Value::String("Hello, world!".to_string())
    );
}

#[test]
fn test_extract_output_empty() {
    let response = make_response("gpt-4", None, None);
    let output = parser::extract_output(&response);
    assert_eq!(output, serde_json::Value::Null);
}

#[test]
fn test_extract_output_no_choices() {
    let response = CreateChatCompletionResponse {
        id: "chatcmpl-test".to_string(),
        choices: vec![],
        created: 1_700_000_000,
        model: "gpt-4".to_string(),
        service_tier: None,
        system_fingerprint: None,
        object: "chat.completion".to_string(),
        usage: None,
    };
    let output = parser::extract_output(&response);
    assert_eq!(output, serde_json::Value::Null);
}

#[test]
fn test_extract_stream_chunk_content() {
    let chunk = make_stream_chunk(Some("Hello"), None);
    assert_eq!(
        parser::extract_stream_chunk_content(&chunk),
        Some("Hello".to_string())
    );
}

#[test]
fn test_extract_stream_chunk_content_none() {
    let chunk = make_stream_chunk(None, None);
    assert!(parser::extract_stream_chunk_content(&chunk).is_none());
}

#[test]
fn test_extract_stream_usage() {
    let usage = make_usage(5, 15, 20);
    let chunk = make_stream_chunk(None, Some(usage));
    let result = parser::extract_stream_usage(&chunk).expect("expected usage");
    assert_eq!(result.input, Some(5));
    assert_eq!(result.output, Some(15));
    assert_eq!(result.total, Some(20));
}

#[test]
fn test_extract_stream_usage_none() {
    let chunk = make_stream_chunk(Some("token"), None);
    assert!(parser::extract_stream_usage(&chunk).is_none());
}

// ---------------------------------------------------------------------------
// Wrapper construction test
// ---------------------------------------------------------------------------

#[test]
fn test_observe_openai_creates_traced_chat() {
    // Verify that observe_openai() compiles and produces a TracedChat.
    // We can't call API methods without a real server, but construction must work.
    let client = async_openai::Client::new();
    let _traced = langfuse_openai::observe_openai(&client);
}

// ---------------------------------------------------------------------------
// TracedEmbeddings construction test
// ---------------------------------------------------------------------------

#[test]
fn test_observe_openai_embeddings_creates_traced_embeddings() {
    // Verify that observe_openai_embeddings() compiles and produces a TracedEmbeddings.
    // We can't call API methods without a real server, but construction must work.
    let client = async_openai::Client::new();
    let _traced = langfuse_openai::observe_openai_embeddings(&client);
}

#[test]
fn test_traced_embeddings_new() {
    let client = async_openai::Client::new();
    let _traced = langfuse_openai::TracedEmbeddings::new(client.embeddings());
}

// ---------------------------------------------------------------------------
// Tool calls extraction tests
// ---------------------------------------------------------------------------

fn make_tool_call(id: &str, name: &str, arguments: &str) -> ChatCompletionMessageToolCalls {
    ChatCompletionMessageToolCalls::Function(ChatCompletionMessageToolCall {
        id: id.to_string(),
        function: FunctionCall {
            name: name.to_string(),
            arguments: arguments.to_string(),
        },
    })
}

fn make_response_with_tool_calls(
    tool_calls: Vec<ChatCompletionMessageToolCalls>,
) -> CreateChatCompletionResponse {
    CreateChatCompletionResponse {
        id: "chatcmpl-test".to_string(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatCompletionResponseMessage {
                content: None,
                refusal: None,
                tool_calls: Some(tool_calls),
                annotations: None,
                role: Role::Assistant,
                function_call: None,
                audio: None,
            },
            finish_reason: None,
            logprobs: None,
        }],
        created: 1_700_000_000,
        model: "gpt-4".to_string(),
        service_tier: None,
        system_fingerprint: None,
        object: "chat.completion".to_string(),
        usage: None,
    }
}

#[test]
fn test_extract_tool_calls_single() {
    let tc = make_tool_call(
        "call_abc123",
        "get_weather",
        r#"{"location":"San Francisco"}"#,
    );
    let response = make_response_with_tool_calls(vec![tc]);
    let result = parser::extract_tool_calls(&response).expect("expected tool calls");
    let arr = result.as_array().expect("expected array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "call_abc123");
    assert_eq!(arr[0]["function"]["name"], "get_weather");
    assert_eq!(
        arr[0]["function"]["arguments"],
        r#"{"location":"San Francisco"}"#
    );
}

#[test]
fn test_extract_tool_calls_multiple() {
    let tc1 = make_tool_call("call_1", "get_weather", r#"{"city":"NYC"}"#);
    let tc2 = make_tool_call("call_2", "get_time", r#"{"tz":"EST"}"#);
    let response = make_response_with_tool_calls(vec![tc1, tc2]);
    let result = parser::extract_tool_calls(&response).expect("expected tool calls");
    let arr = result.as_array().expect("expected array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["function"]["name"], "get_weather");
    assert_eq!(arr[1]["function"]["name"], "get_time");
}

#[test]
fn test_extract_tool_calls_none_when_absent() {
    let response = make_response("gpt-4", Some("Hello"), None);
    assert!(parser::extract_tool_calls(&response).is_none());
}

#[test]
fn test_extract_tool_calls_none_when_empty() {
    let response = make_response_with_tool_calls(vec![]);
    // Empty tool_calls should not produce a result (we store None in the builder).
    // But extract_tool_calls checks is_empty() explicitly.
    assert!(parser::extract_tool_calls(&response).is_none());
}

#[test]
fn test_extract_output_with_tool_calls_serializes_message() {
    // When tool calls are present, extract_output serializes the full message.
    let tc = make_tool_call("call_1", "my_func", "{}");
    let response = make_response_with_tool_calls(vec![tc]);
    let output = parser::extract_output(&response);
    // The output should be an object containing tool_calls.
    assert!(output.is_object());
    assert!(output["tool_calls"].is_array());
}

// ---------------------------------------------------------------------------
// Streaming tool call accumulation tests
// ---------------------------------------------------------------------------

fn make_stream_chunk_with_tool_calls(
    tool_calls: Vec<ChatCompletionMessageToolCallChunk>,
) -> CreateChatCompletionStreamResponse {
    CreateChatCompletionStreamResponse {
        id: "chatcmpl-test".to_string(),
        choices: vec![ChatChoiceStream {
            index: 0,
            delta: ChatCompletionStreamResponseDelta {
                content: None,
                function_call: None,
                tool_calls: Some(tool_calls),
                role: None,
                refusal: None,
            },
            finish_reason: None,
            logprobs: None,
        }],
        created: 1_700_000_000,
        model: "gpt-4".to_string(),
        service_tier: None,
        system_fingerprint: None,
        object: "chat.completion.chunk".to_string(),
        usage: None,
    }
}

#[test]
fn test_tool_call_accumulator_single_call() {
    let mut acc = ToolCallAccumulator::new();

    // First chunk: tool call ID + function name start
    let chunk1 = make_stream_chunk_with_tool_calls(vec![ChatCompletionMessageToolCallChunk {
        index: 0,
        id: Some("call_abc".to_string()),
        r#type: Some(FunctionType::Function),
        function: Some(FunctionCallStream {
            name: Some("get_weather".to_string()),
            arguments: Some(String::new()),
        }),
    }]);
    acc.accumulate(&chunk1);

    // Second chunk: argument fragment
    let chunk2 = make_stream_chunk_with_tool_calls(vec![ChatCompletionMessageToolCallChunk {
        index: 0,
        id: None,
        r#type: None,
        function: Some(FunctionCallStream {
            name: None,
            arguments: Some(r#"{"loc"#.to_string()),
        }),
    }]);
    acc.accumulate(&chunk2);

    // Third chunk: remaining arguments
    let chunk3 = make_stream_chunk_with_tool_calls(vec![ChatCompletionMessageToolCallChunk {
        index: 0,
        id: None,
        r#type: None,
        function: Some(FunctionCallStream {
            name: None,
            arguments: Some(r#"ation":"SF"}"#.to_string()),
        }),
    }]);
    acc.accumulate(&chunk3);

    assert!(acc.has_calls());
    let result = acc.finalize();
    let arr = result.as_array().expect("expected array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "call_abc");
    assert_eq!(arr[0]["function"]["name"], "get_weather");
    assert_eq!(arr[0]["function"]["arguments"], r#"{"location":"SF"}"#);
}

#[test]
fn test_tool_call_accumulator_parallel_calls() {
    let mut acc = ToolCallAccumulator::new();

    // OpenAI can send multiple parallel tool calls with different indices.
    // First chunk: two tool calls start simultaneously.
    let chunk = make_stream_chunk_with_tool_calls(vec![
        ChatCompletionMessageToolCallChunk {
            index: 0,
            id: Some("call_1".to_string()),
            r#type: Some(FunctionType::Function),
            function: Some(FunctionCallStream {
                name: Some("func_a".to_string()),
                arguments: Some(r#"{"a":1}"#.to_string()),
            }),
        },
        ChatCompletionMessageToolCallChunk {
            index: 1,
            id: Some("call_2".to_string()),
            r#type: Some(FunctionType::Function),
            function: Some(FunctionCallStream {
                name: Some("func_b".to_string()),
                arguments: Some(r#"{"b":2}"#.to_string()),
            }),
        },
    ]);
    acc.accumulate(&chunk);

    assert!(acc.has_calls());
    let result = acc.finalize();
    let arr = result.as_array().expect("expected array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["function"]["name"], "func_a");
    assert_eq!(arr[1]["function"]["name"], "func_b");
}

#[test]
fn test_tool_call_accumulator_sparse_indices_skip_placeholders() {
    let mut acc = ToolCallAccumulator::new();

    let chunk = make_stream_chunk_with_tool_calls(vec![ChatCompletionMessageToolCallChunk {
        index: 2,
        id: Some("call_3".to_string()),
        r#type: Some(FunctionType::Function),
        function: Some(FunctionCallStream {
            name: Some("func_c".to_string()),
            arguments: Some(r#"{"c":3}"#.to_string()),
        }),
    }]);
    acc.accumulate(&chunk);

    assert!(acc.has_calls());
    let result = acc.finalize();
    let arr = result.as_array().expect("expected array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "call_3");
    assert_eq!(arr[0]["function"]["name"], "func_c");
}

#[test]
fn test_tool_call_accumulator_empty() {
    let acc = ToolCallAccumulator::new();
    assert!(!acc.has_calls());
    let result = acc.finalize();
    assert_eq!(result, serde_json::Value::Array(vec![]));
}

#[test]
fn test_tool_call_accumulator_chunk_without_tool_calls() {
    let mut acc = ToolCallAccumulator::new();
    // Regular content chunk — no tool calls.
    let chunk = make_stream_chunk(Some("Hello"), None);
    acc.accumulate(&chunk);
    assert!(!acc.has_calls());
}
