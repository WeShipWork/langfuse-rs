//! Stream and iterator wrappers that collect items and finalize the span on completion.
//!
//! [`ObservingStream`] wraps a [`futures::Stream`] and [`ObservingIterator`] wraps
//! a standard [`Iterator`]. Both serialize each yielded item to JSON, collect the
//! serialized representations, and when the inner stream/iterator is exhausted,
//! set the collected output on the associated [`LangfuseSpan`] and end it.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use pin_project_lite::pin_project;
use serde::Serialize;

use super::span::LangfuseSpan;

/// A boxed transform function that converts collected JSON strings into a single output string.
type TransformFn = Box<dyn Fn(&[String]) -> String + Send + Sync>;
pin_project! {
    /// A [`Stream`] wrapper that observes each yielded item and finalizes the
    /// associated [`LangfuseSpan`] when the stream completes.
    ///
    /// Each item is serialized to JSON and collected. When the inner stream
    /// yields `None`, the collected items are set as the span output (as a JSON
    /// array of strings) and the span is ended.
    ///
    /// ```ignore
    /// use langfuse::{LangfuseSpan, ObservingStream};
    /// use futures::StreamExt;
    ///
    /// let span = LangfuseSpan::start("stream-op");
    /// let stream = ObservingStream::new(span, my_stream);
    /// while let Some(item) = stream.next().await { /* ... */ }
    /// ```
    ///
    /// # Type Parameters
    ///
    /// - `S`: The inner stream type. Its `Item` must implement [`Serialize`].
    pub struct ObservingStream<S: Stream> {
        #[pin]
        inner: S,
        span: Option<LangfuseSpan>,
        collected: Vec<String>,
        transform: Option<TransformFn>,
    }
}

impl<S: Stream> ObservingStream<S> {
    /// Create a new `ObservingStream` wrapping the given stream and span.
    ///
    /// Items yielded by the inner stream are serialized to JSON and collected.
    /// When the stream completes, the collected items are set as the span output.
    #[must_use]
    pub fn new(span: LangfuseSpan, inner: S) -> Self {
        Self {
            inner,
            span: Some(span),
            collected: Vec::new(),
            transform: None,
        }
    }

    /// Create a new `ObservingStream` with a custom transform function.
    ///
    /// Instead of setting the collected JSON strings directly as output, the
    /// transform function is called with the collected strings and its return
    /// value is used as the span output.
    #[must_use]
    pub fn with_transform(
        span: LangfuseSpan,
        inner: S,
        transform: impl Fn(&[String]) -> String + Send + Sync + 'static,
    ) -> Self {
        Self {
            inner,
            span: Some(span),
            collected: Vec::new(),
            transform: Some(Box::new(transform)),
        }
    }
}

impl<S: Stream> std::fmt::Debug for ObservingStream<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObservingStream")
            .field("collected_count", &self.collected.len())
            .finish()
    }
}

impl<S> Stream for ObservingStream<S>
where
    S: Stream,
    S::Item: Serialize,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.inner.poll_next(cx) {
            Poll::Ready(Some(item)) => {
                if let Ok(json) = serde_json::to_string(&item) {
                    this.collected.push(json);
                }
                Poll::Ready(Some(item))
            }
            Poll::Ready(None) => {
                if let Some(span) = this.span.take() {
                    if let Some(transform) = this.transform.as_ref() {
                        let output = transform(this.collected);
                        span.set_output(&output);
                    } else {
                        let output = serde_json::json!(this.collected);
                        span.set_output(&output);
                    }
                    span.end();
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

// ---------------------------------------------------------------------------
// ObservingIterator
// ---------------------------------------------------------------------------

/// An [`Iterator`] wrapper that observes each yielded item and finalizes the
/// associated [`LangfuseSpan`] when the iterator is exhausted.
///
/// Each item is serialized to JSON and collected. When the inner iterator
/// returns `None`, the collected items are set as the span output (as a JSON
/// array of strings) and the span is ended.
///
/// ```ignore
/// let span = LangfuseSpan::start("iter-op");
/// let iter = ObservingIterator::new(span, my_iter);
/// for item in iter { /* ... */ }
/// ```
///
/// # Type Parameters
///
/// - `I`: The inner iterator type. Its `Item` must implement [`Serialize`].
pub struct ObservingIterator<I: Iterator> {
    inner: I,
    span: Option<LangfuseSpan>,
    collected: Vec<String>,
    transform: Option<TransformFn>,
}

impl<I: Iterator> ObservingIterator<I> {
    /// Create a new `ObservingIterator` wrapping the given iterator and span.
    #[must_use]
    pub fn new(span: LangfuseSpan, inner: I) -> Self {
        Self {
            inner,
            span: Some(span),
            collected: Vec::new(),
            transform: None,
        }
    }

    /// Create a new `ObservingIterator` with a custom transform function.
    #[must_use]
    pub fn with_transform(
        span: LangfuseSpan,
        inner: I,
        transform: impl Fn(&[String]) -> String + Send + Sync + 'static,
    ) -> Self {
        Self {
            inner,
            span: Some(span),
            collected: Vec::new(),
            transform: Some(Box::new(transform)),
        }
    }
}

impl<I: Iterator> std::fmt::Debug for ObservingIterator<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObservingIterator")
            .field("collected_count", &self.collected.len())
            .finish()
    }
}

impl<I> Iterator for ObservingIterator<I>
where
    I: Iterator,
    I::Item: Serialize,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(item) => {
                if let Ok(json) = serde_json::to_string(&item) {
                    self.collected.push(json);
                }
                Some(item)
            }
            None => {
                if let Some(span) = self.span.take() {
                    if let Some(transform) = self.transform.as_ref() {
                        let output = transform(&self.collected);
                        span.set_output(&output);
                    } else {
                        let output = serde_json::json!(self.collected);
                        span.set_output(&output);
                    }
                    span.end();
                }
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
