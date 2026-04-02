//! Thread-safe batch queue for score events.

use std::sync::{Arc, Mutex};

use langfuse_core::types::ScoreBody;

/// A thread-safe batch queue for score events.
///
/// Buffers [`ScoreBody`] items until the queue reaches `max_size`,
/// at which point [`push`](BatchQueue::push) signals that a flush is needed.
pub struct BatchQueue {
    buffer: Arc<Mutex<Vec<ScoreBody>>>,
    max_size: usize,
}

impl BatchQueue {
    /// Create a new `BatchQueue` with the given maximum buffer size.
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            max_size,
        }
    }

    /// Add a score to the queue.
    ///
    /// Returns `true` if the queue has reached `max_size` (flush needed).
    pub fn push(&self, score: ScoreBody) -> bool {
        let mut buf = self.buffer.lock().expect("BatchQueue lock poisoned");
        buf.push(score);
        buf.len() >= self.max_size
    }

    /// Drain all buffered scores, returning them and leaving the buffer empty.
    pub fn drain(&self) -> Vec<ScoreBody> {
        let mut buf = self.buffer.lock().expect("BatchQueue lock poisoned");
        std::mem::take(&mut *buf)
    }

    /// Number of buffered scores.
    pub fn len(&self) -> usize {
        self.buffer.lock().expect("BatchQueue lock poisoned").len()
    }

    /// Returns `true` if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer
            .lock()
            .expect("BatchQueue lock poisoned")
            .is_empty()
    }
}
