//! Integration tests for the evaluation framework (Group 5).

use langfuse::datasets::evaluator::Evaluator;
use langfuse::datasets::experiment::{
    ExperimentConfig, ExperimentResult, format_experiment_summary, run_experiment,
    run_experiment_with_evaluators,
};
use langfuse::datasets::manager::{BatchedEvaluationConfig, DatasetManager};
use langfuse::datasets::types::DatasetItem;
use langfuse_core::config::LangfuseConfig;
use langfuse_core::error::LangfuseError;
use langfuse_core::types::{Evaluation, ScoreValue};
use serde_json::json;

// ── Helpers ──────────────────────────────────────────────────────────────

fn make_item(id: &str, input: serde_json::Value, expected: serde_json::Value) -> DatasetItem {
    DatasetItem {
        id: id.to_string(),
        dataset_id: "ds-test".to_string(),
        input: Some(input),
        expected_output: Some(expected),
        metadata: None,
        source_trace_id: None,
        source_observation_id: None,
        status: "ACTIVE".to_string(),
    }
}

fn test_config() -> LangfuseConfig {
    LangfuseConfig::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .build()
        .unwrap()
}

// ── 5.1: Evaluation struct tests ─────────────────────────────────────────

#[test]
fn test_evaluation_construction() {
    let eval = Evaluation {
        name: "accuracy".to_string(),
        value: ScoreValue::Numeric(0.95),
        comment: Some("High accuracy".to_string()),
        metadata: Some(json!({"model": "gpt-4"})),
        data_type: Some("NUMERIC".to_string()),
    };

    assert_eq!(eval.name, "accuracy");
    assert_eq!(eval.comment.as_deref(), Some("High accuracy"));
    assert_eq!(eval.data_type.as_deref(), Some("NUMERIC"));
}

#[test]
fn test_evaluation_serialization() {
    let eval = Evaluation {
        name: "relevance".to_string(),
        value: ScoreValue::Numeric(0.8),
        comment: None,
        metadata: None,
        data_type: None,
    };

    let json = serde_json::to_value(&eval).unwrap();
    assert_eq!(json["name"], "relevance");
    assert_eq!(json["value"], 0.8);
    // Optional fields should be absent when None
    assert!(json.get("comment").is_none());
    assert!(json.get("metadata").is_none());
    assert!(json.get("data_type").is_none());
}

#[test]
fn test_evaluation_deserialization() {
    let json_str = r#"{
        "name": "quality",
        "value": true,
        "comment": "Good quality"
    }"#;

    let eval: Evaluation = serde_json::from_str(json_str).unwrap();
    assert_eq!(eval.name, "quality");
    assert_eq!(eval.value, ScoreValue::Boolean(true));
    assert_eq!(eval.comment.as_deref(), Some("Good quality"));
    assert!(eval.metadata.is_none());
    assert!(eval.data_type.is_none());
}

#[test]
fn test_evaluation_with_categorical_value() {
    let eval = Evaluation {
        name: "sentiment".to_string(),
        value: ScoreValue::Categorical("positive".to_string()),
        comment: None,
        metadata: None,
        data_type: Some("CATEGORICAL".to_string()),
    };

    let json = serde_json::to_value(&eval).unwrap();
    assert_eq!(json["value"], "positive");
    assert_eq!(json["data_type"], "CATEGORICAL");
}

// ── 5.2/5.3: Evaluator trait with closure ────────────────────────────────

#[tokio::test]
async fn test_evaluator_closure() {
    let evaluator = |output: &serde_json::Value,
                     expected: Option<&serde_json::Value>|
     -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<Evaluation>, LangfuseError>> + Send>,
    > {
        let matches = expected.is_some_and(|exp| exp == output);
        Box::pin(async move {
            Ok(vec![Evaluation {
                name: "exact_match".to_string(),
                value: ScoreValue::Boolean(matches),
                comment: None,
                metadata: None,
                data_type: None,
            }])
        })
    };

    let output = json!(42);
    let expected = json!(42);
    let results = evaluator.evaluate(&output, Some(&expected)).await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "exact_match");
    assert_eq!(results[0].value, ScoreValue::Boolean(true));
}

#[tokio::test]
async fn test_evaluator_closure_no_expected() {
    let evaluator = |_output: &serde_json::Value,
                     expected: Option<&serde_json::Value>|
     -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<Evaluation>, LangfuseError>> + Send>,
    > {
        let has_expected = expected.is_some();
        Box::pin(async move {
            Ok(vec![Evaluation {
                name: "has_expected".to_string(),
                value: ScoreValue::Boolean(has_expected),
                comment: None,
                metadata: None,
                data_type: None,
            }])
        })
    };

    let output = json!("hello");
    let results = evaluator.evaluate(&output, None).await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].value, ScoreValue::Boolean(false));
}

// ── 5.4: run_experiment_with_evaluators ──────────────────────────────────

#[tokio::test]
async fn test_run_experiment_with_evaluators() {
    let items = vec![
        make_item("1", json!({"x": 2}), json!(4)),
        make_item("2", json!({"x": 3}), json!(6)),
    ];

    struct MatchEvaluator;

    #[async_trait::async_trait]
    impl Evaluator for MatchEvaluator {
        async fn evaluate(
            &self,
            output: &serde_json::Value,
            expected: Option<&serde_json::Value>,
        ) -> Result<Vec<Evaluation>, LangfuseError> {
            let matches = expected.is_some_and(|exp| exp == output);
            Ok(vec![Evaluation {
                name: "match".to_string(),
                value: ScoreValue::Numeric(if matches { 1.0 } else { 0.0 }),
                comment: None,
                metadata: None,
                data_type: None,
            }])
        }
    }

    let evaluators: Vec<Box<dyn Evaluator>> = vec![Box::new(MatchEvaluator)];

    let results = run_experiment_with_evaluators(
        items,
        ExperimentConfig::default(),
        |item| {
            Box::pin(async move {
                let x = item.input.unwrap()["x"].as_i64().unwrap();
                json!(x * 2)
            })
        },
        evaluators,
    )
    .await;

    assert_eq!(results.len(), 2);
    for r in &results {
        assert_eq!(r.scores.len(), 1);
        assert_eq!(r.scores[0].0, "match");
        assert_eq!(r.scores[0].1, 1.0);
    }
}

#[tokio::test]
async fn test_run_experiment_with_multiple_evaluators() {
    let items = vec![make_item("1", json!(10), json!(10))];

    struct AlwaysOneEvaluator {
        metric_name: String,
    }

    #[async_trait::async_trait]
    impl Evaluator for AlwaysOneEvaluator {
        async fn evaluate(
            &self,
            _output: &serde_json::Value,
            _expected: Option<&serde_json::Value>,
        ) -> Result<Vec<Evaluation>, LangfuseError> {
            Ok(vec![Evaluation {
                name: self.metric_name.clone(),
                value: ScoreValue::Numeric(1.0),
                comment: None,
                metadata: None,
                data_type: None,
            }])
        }
    }

    let evaluators: Vec<Box<dyn Evaluator>> = vec![
        Box::new(AlwaysOneEvaluator {
            metric_name: "metric_a".to_string(),
        }),
        Box::new(AlwaysOneEvaluator {
            metric_name: "metric_b".to_string(),
        }),
    ];

    let results = run_experiment_with_evaluators(
        items,
        ExperimentConfig::default(),
        |item| Box::pin(async move { item.input.unwrap_or(json!(null)) }),
        evaluators,
    )
    .await;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].scores.len(), 2);

    let score_names: Vec<&str> = results[0].scores.iter().map(|(n, _)| n.as_str()).collect();
    assert!(score_names.contains(&"metric_a"));
    assert!(score_names.contains(&"metric_b"));
}

// ── 5.5: ExperimentResult::format() ──────────────────────────────────────

#[test]
fn test_experiment_result_format() {
    let result = ExperimentResult {
        item_id: "item-1".to_string(),
        output: json!(42),
        scores: vec![
            ("accuracy".to_string(), 0.95),
            ("relevance".to_string(), 0.8),
        ],
        dataset_run_url: "https://langfuse.com/datasets/test/runs/run-1".to_string(),
    };

    let formatted = result.format();
    assert!(formatted.contains("Item: item-1"));
    assert!(formatted.contains("accuracy: 0.95"));
    assert!(formatted.contains("relevance: 0.8"));
    assert!(formatted.contains("Run URL: https://langfuse.com/datasets/test/runs/run-1"));
}

#[test]
fn test_experiment_result_format_no_scores() {
    let result = ExperimentResult {
        item_id: "item-2".to_string(),
        output: json!(null),
        scores: vec![],
        dataset_run_url: String::new(),
    };

    let formatted = result.format();
    assert!(formatted.contains("Item: item-2"));
    assert!(formatted.contains("(none)"));
}

#[test]
fn test_format_experiment_summary() {
    let results = vec![
        ExperimentResult {
            item_id: "1".to_string(),
            output: json!(1),
            scores: vec![("accuracy".to_string(), 1.0), ("speed".to_string(), 0.5)],
            dataset_run_url: String::new(),
        },
        ExperimentResult {
            item_id: "2".to_string(),
            output: json!(2),
            scores: vec![("accuracy".to_string(), 0.8), ("speed".to_string(), 0.9)],
            dataset_run_url: String::new(),
        },
    ];

    let summary = format_experiment_summary(&results);
    assert!(summary.contains("2 items"));
    assert!(summary.contains("accuracy"));
    assert!(summary.contains("speed"));
    // Average accuracy = (1.0 + 0.8) / 2 = 0.9
    assert!(summary.contains("0.9000"));
}

#[test]
fn test_format_experiment_summary_empty() {
    let summary = format_experiment_summary(&[]);
    assert!(summary.contains("0 items"));
    assert!(summary.contains("No results"));
}

// ── 5.6: dataset_run_url ─────────────────────────────────────────────────

#[test]
fn test_experiment_config_dataset_run_url() {
    let config = ExperimentConfig {
        name: "run-001".to_string(),
        max_concurrency: 5,
        base_url: "https://cloud.langfuse.com".to_string(),
        dataset_name: "my-dataset".to_string(),
    };

    let url = config.dataset_run_url();
    assert_eq!(
        url,
        "https://cloud.langfuse.com/datasets/my-dataset/runs/run-001"
    );
}

#[test]
fn test_experiment_config_dataset_run_url_empty() {
    let config = ExperimentConfig::default();
    let url = config.dataset_run_url();
    assert!(url.is_empty());
}

#[tokio::test]
async fn test_run_experiment_populates_dataset_run_url() {
    let items = vec![make_item("1", json!(1), json!(1))];

    let config = ExperimentConfig {
        name: "test-run".to_string(),
        max_concurrency: 1,
        base_url: "https://example.com".to_string(),
        dataset_name: "ds-1".to_string(),
    };

    let results = run_experiment(
        items,
        config,
        |item| Box::pin(async move { item.input.unwrap_or(json!(null)) }),
        |_item, _output| vec![],
    )
    .await;

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].dataset_run_url,
        "https://example.com/datasets/ds-1/runs/test-run"
    );
}

// ── 5.7: delete_run returns error without real server ────────────────────

#[tokio::test]
async fn test_delete_run_returns_error_without_server() {
    let config = test_config();
    let manager = DatasetManager::new(&config);

    let result = manager.delete_run("test-dataset", "test-run").await;
    // Should fail with a network error since there's no real server
    assert!(result.is_err());
}

// ── 5.8/5.9: Batched evaluation config ──────────────────────────────────

#[test]
fn test_batched_evaluation_config_default() {
    let config = BatchedEvaluationConfig::default();
    assert_eq!(config.max_concurrency, 10);
    assert_eq!(config.page_size, 50);
    assert_eq!(config.max_retries, 3);
    assert!(config.start_after.is_none());
    assert!(config.run_name.starts_with("batch-eval-"));
}

#[test]
fn test_batched_evaluation_config_with_start_after() {
    let config = BatchedEvaluationConfig {
        start_after: Some("item-050".to_string()),
        ..BatchedEvaluationConfig::default()
    };
    assert_eq!(config.start_after.as_deref(), Some("item-050"));
}

#[tokio::test]
async fn test_batched_evaluation_returns_error_without_server() {
    let config = test_config();
    let manager = DatasetManager::new(&config);

    let result = manager
        .run_batched_evaluation(
            "test-dataset",
            BatchedEvaluationConfig::default(),
            |item| Box::pin(async move { item.input.unwrap_or(json!(null)) }),
            vec![],
        )
        .await;

    // Should fail with a network error since there's no real server
    assert!(result.is_err());
}

// ── Display impl ─────────────────────────────────────────────────────────

#[test]
fn test_experiment_result_display() {
    let result = ExperimentResult {
        item_id: "item-display".to_string(),
        output: json!("test"),
        scores: vec![("score".to_string(), 1.0)],
        dataset_run_url: String::new(),
    };

    let display = format!("{result}");
    assert!(display.contains("Item: item-display"));
    assert!(display.contains("score: 1"));
}
