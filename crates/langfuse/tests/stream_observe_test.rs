//! Tests for `ObservingStream` and `ObservingIterator` wrappers.
//!
//! These tests run against the default no-op OTel provider — no real exporter
//! is configured, so spans are silently discarded. The goal is to verify that
//! the wrappers correctly yield items, collect serialized output, and finalize
//! the span when the inner stream/iterator is exhausted.

use futures::Stream;
use futures::stream::{self, StreamExt};
use langfuse::langfuse_tracing::span::LangfuseSpan;
use langfuse::langfuse_tracing::stream_wrapper::{ObservingIterator, ObservingStream};
use serde::Serialize;

// =========================================================================
// ObservingStream tests
// =========================================================================

#[tokio::test]
async fn observing_stream_yields_all_items() {
    let span = LangfuseSpan::start("stream-test");
    let inner = stream::iter(vec![1_i32, 2, 3]);
    let mut observed = ObservingStream::new(span, inner);

    let mut items = Vec::new();
    while let Some(item) = observed.next().await {
        items.push(item);
    }

    assert_eq!(items, vec![1, 2, 3]);
}

#[tokio::test]
async fn observing_stream_items_unchanged() {
    let span = LangfuseSpan::start("unchanged-test");
    let inner = stream::iter(vec!["hello".to_string(), "world".to_string()]);
    let observed = ObservingStream::new(span, inner);

    let items: Vec<String> = observed.collect().await;
    assert_eq!(items, vec!["hello", "world"]);
}

#[tokio::test]
async fn observing_stream_empty() {
    let span = LangfuseSpan::start("empty-stream-test");
    let inner = stream::iter(Vec::<i32>::new());
    let observed = ObservingStream::new(span, inner);

    let items: Vec<i32> = observed.collect().await;
    assert!(items.is_empty());
}

#[tokio::test]
async fn observing_stream_with_serializable_structs() {
    #[derive(Debug, Clone, PartialEq, Serialize)]
    struct Chunk {
        text: String,
        index: usize,
    }

    let span = LangfuseSpan::start("struct-stream-test");
    let chunks = vec![
        Chunk {
            text: "hello".into(),
            index: 0,
        },
        Chunk {
            text: "world".into(),
            index: 1,
        },
    ];
    let inner = stream::iter(chunks.clone());
    let observed = ObservingStream::new(span, inner);

    let items: Vec<Chunk> = observed.collect().await;
    assert_eq!(items, chunks);
}

#[tokio::test]
async fn observing_stream_with_transform() {
    let span = LangfuseSpan::start("transform-stream-test");
    let inner = stream::iter(vec!["hello", "world"]);
    let observed = ObservingStream::with_transform(span, inner, |collected| {
        // Join all collected JSON strings with a separator.
        collected.join(" | ")
    });

    let items: Vec<&str> = observed.collect().await;
    assert_eq!(items, vec!["hello", "world"]);
}

#[tokio::test]
async fn observing_stream_size_hint() {
    let span = LangfuseSpan::start("size-hint-test");
    let inner = stream::iter(vec![1_i32, 2, 3]);
    let observed = ObservingStream::new(span, inner);

    let (lower, upper) = observed.size_hint();
    assert_eq!(lower, 3);
    assert_eq!(upper, Some(3));
}

// =========================================================================
// ObservingIterator tests
// =========================================================================

#[test]
fn observing_iterator_yields_all_items() {
    let span = LangfuseSpan::start("iter-test");
    let inner = vec![10_i32, 20, 30].into_iter();
    let observed = ObservingIterator::new(span, inner);

    let items: Vec<i32> = observed.collect();
    assert_eq!(items, vec![10, 20, 30]);
}

#[test]
fn observing_iterator_items_unchanged() {
    let span = LangfuseSpan::start("iter-unchanged-test");
    let inner = vec!["foo".to_string(), "bar".to_string()].into_iter();
    let observed = ObservingIterator::new(span, inner);

    let items: Vec<String> = observed.collect();
    assert_eq!(items, vec!["foo", "bar"]);
}

#[test]
fn observing_iterator_empty() {
    let span = LangfuseSpan::start("empty-iter-test");
    let inner = Vec::<i32>::new().into_iter();
    let observed = ObservingIterator::new(span, inner);

    let items: Vec<i32> = observed.collect();
    assert!(items.is_empty());
}

#[test]
fn observing_iterator_with_serializable_structs() {
    #[derive(Debug, Clone, PartialEq, Serialize)]
    struct Token {
        id: u32,
        text: String,
    }

    let span = LangfuseSpan::start("struct-iter-test");
    let tokens = vec![
        Token {
            id: 1,
            text: "hello".into(),
        },
        Token {
            id: 2,
            text: "world".into(),
        },
    ];
    let inner = tokens.clone().into_iter();
    let observed = ObservingIterator::new(span, inner);

    let items: Vec<Token> = observed.collect();
    assert_eq!(items, tokens);
}

#[test]
fn observing_iterator_with_transform() {
    let span = LangfuseSpan::start("transform-iter-test");
    let inner = vec![1_i32, 2, 3].into_iter();
    let observed = ObservingIterator::with_transform(span, inner, |collected| {
        format!("total items: {}", collected.len())
    });

    let items: Vec<i32> = observed.collect();
    assert_eq!(items, vec![1, 2, 3]);
}

#[test]
fn observing_iterator_size_hint() {
    let span = LangfuseSpan::start("iter-size-hint-test");
    let inner = vec![1_i32, 2, 3].into_iter();
    let observed = ObservingIterator::new(span, inner);

    let (lower, upper) = observed.size_hint();
    assert_eq!(lower, 3);
    assert_eq!(upper, Some(3));
}

#[test]
fn observing_iterator_step_by_step() {
    let span = LangfuseSpan::start("step-iter-test");
    let inner = vec![1_i32, 2, 3].into_iter();
    let mut observed = ObservingIterator::new(span, inner);

    assert_eq!(observed.next(), Some(1));
    assert_eq!(observed.next(), Some(2));
    assert_eq!(observed.next(), Some(3));
    // This call triggers span finalization.
    assert_eq!(observed.next(), None);
    // Subsequent calls should also return None without double-ending the span.
    assert_eq!(observed.next(), None);
}

#[tokio::test]
async fn observing_stream_step_by_step() {
    let span = LangfuseSpan::start("step-stream-test");
    let inner = stream::iter(vec![1_i32, 2]);
    let mut observed = ObservingStream::new(span, inner);

    assert_eq!(observed.next().await, Some(1));
    assert_eq!(observed.next().await, Some(2));
    // This call triggers span finalization.
    assert_eq!(observed.next().await, None);
    // Subsequent calls should also return None without double-ending the span.
    assert_eq!(observed.next().await, None);
}
