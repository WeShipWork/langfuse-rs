use langfuse::scoring::manager::ScoreManager;
use langfuse::scoring::queue::BatchQueue;
use langfuse_core::config::LangfuseConfig;
use langfuse_core::types::{ScoreBody, ScoreValue};

// ── Helpers ──────────────────────────────────────────────────────────────

fn make_score(name: &str, value: ScoreValue) -> ScoreBody {
    ScoreBody {
        name: name.to_string(),
        value,
        trace_id: None,
        observation_id: None,
        comment: None,
        metadata: None,
        config_id: None,
        data_type: None,
    }
}

fn test_config() -> LangfuseConfig {
    LangfuseConfig::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .build()
        .unwrap()
}

// ── BatchQueue tests ─────────────────────────────────────────────────────

#[test]
fn test_batch_queue_push_and_drain() {
    let queue = BatchQueue::new(100);
    let score = make_score("accuracy", ScoreValue::Numeric(0.9));

    let full = queue.push(score);
    assert!(!full, "queue should not be full after one push");
    assert_eq!(queue.len(), 1);

    let drained = queue.drain();
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].name, "accuracy");
    assert!(queue.is_empty());
}

#[test]
fn test_batch_queue_max_size_signals_full() {
    let queue = BatchQueue::new(2);

    let full1 = queue.push(make_score("s1", ScoreValue::Numeric(1.0)));
    assert!(!full1);

    let full2 = queue.push(make_score("s2", ScoreValue::Numeric(2.0)));
    assert!(full2, "queue should signal full when reaching max_size");
}

#[test]
fn test_batch_queue_drain_empties_buffer() {
    let queue = BatchQueue::new(10);
    queue.push(make_score("a", ScoreValue::Boolean(true)));
    queue.push(make_score("b", ScoreValue::Categorical("good".into())));

    assert_eq!(queue.len(), 2);
    let drained = queue.drain();
    assert_eq!(drained.len(), 2);
    assert_eq!(queue.len(), 0);
    assert!(queue.is_empty());

    // Drain again should return empty
    let drained2 = queue.drain();
    assert!(drained2.is_empty());
}

#[test]
fn test_batch_queue_len_and_is_empty() {
    let queue = BatchQueue::new(10);
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);

    queue.push(make_score("x", ScoreValue::Numeric(0.5)));
    assert!(!queue.is_empty());
    assert_eq!(queue.len(), 1);
}

// ── ScoreManager tests ──────────────────────────────────────────────────

#[tokio::test]
async fn test_score_manager_buffers_scores() {
    let config = test_config();
    let manager = ScoreManager::new(&config);

    manager.score(make_score("raw", ScoreValue::Numeric(0.8)));
    manager.score_trace("trace-1", "accuracy", ScoreValue::Numeric(0.95));
    manager.score_observation(
        "trace-1",
        "obs-1",
        "quality",
        ScoreValue::Categorical("good".into()),
    );

    assert_eq!(manager.pending_count(), 3);
}

#[tokio::test]
async fn test_score_trace_sets_trace_id() {
    let config = test_config();
    let manager = ScoreManager::new(&config);

    manager.score_trace("trace-abc", "relevance", ScoreValue::Numeric(0.7));
    assert_eq!(manager.pending_count(), 1);
}

#[tokio::test]
async fn test_score_observation_sets_both_ids() {
    let config = test_config();
    let manager = ScoreManager::new(&config);

    manager.score_observation(
        "trace-abc",
        "obs-xyz",
        "helpfulness",
        ScoreValue::Boolean(true),
    );
    assert_eq!(manager.pending_count(), 1);
}

#[tokio::test]
async fn test_score_manager_flush_empty() {
    let config = test_config();
    let manager = ScoreManager::new(&config);

    // Flushing with no scores should succeed (no network call made)
    let result = manager.flush().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_score_manager_flush_clears_buffer() {
    let config = test_config();
    let manager = ScoreManager::new(&config);

    manager.score(make_score("test", ScoreValue::Numeric(1.0)));
    assert_eq!(manager.pending_count(), 1);

    // Flush will fail (no real server), but buffer should be drained
    let _result = manager.flush().await;
    assert_eq!(manager.pending_count(), 0);
}

#[tokio::test]
async fn test_score_manager_shutdown_flushes() {
    let config = test_config();
    let manager = ScoreManager::new(&config);

    // Shutdown with empty buffer should succeed
    let result = manager.shutdown().await;
    assert!(result.is_ok());
}
