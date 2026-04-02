use langfuse::datasets::experiment::{ExperimentConfig, ExperimentResult, run_experiment};
use langfuse::datasets::types::{
    CreateDatasetBody, CreateDatasetItemBody, Dataset, DatasetItem, DatasetRun,
};
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

// ── Type serialization tests ─────────────────────────────────────────────

#[test]
fn test_dataset_serializes_camel_case() {
    let ds = Dataset {
        id: "ds-1".to_string(),
        name: "my-dataset".to_string(),
        description: Some("A test dataset".to_string()),
        metadata: None,
    };

    let json = serde_json::to_value(&ds).unwrap();
    // Fields should be camelCase (though these are single-word, verify structure)
    assert_eq!(json["id"], "ds-1");
    assert_eq!(json["name"], "my-dataset");
    assert_eq!(json["description"], "A test dataset");
    // metadata should be absent (skip_serializing_if = None)
    assert!(json.get("metadata").is_none());
}

#[test]
fn test_create_dataset_body() {
    let body = CreateDatasetBody {
        name: "eval-set".to_string(),
        description: None,
        metadata: Some(json!({"version": 1})),
    };

    let json = serde_json::to_value(&body).unwrap();
    assert_eq!(json["name"], "eval-set");
    assert!(json.get("description").is_none());
    assert_eq!(json["metadata"]["version"], 1);
}

#[test]
fn test_dataset_item_serializes() {
    let item = DatasetItem {
        id: "item-1".to_string(),
        dataset_id: "ds-1".to_string(),
        input: Some(json!({"prompt": "hello"})),
        expected_output: Some(json!({"response": "world"})),
        metadata: None,
        source_trace_id: Some("trace-abc".to_string()),
        source_observation_id: None,
        status: "ACTIVE".to_string(),
    };

    let json = serde_json::to_value(&item).unwrap();
    assert_eq!(json["id"], "item-1");
    assert_eq!(json["datasetId"], "ds-1");
    assert_eq!(json["input"]["prompt"], "hello");
    assert_eq!(json["expectedOutput"]["response"], "world");
    assert_eq!(json["sourceTraceId"], "trace-abc");
    assert!(json.get("sourceObservationId").is_none());
    assert!(json.get("metadata").is_none());
    assert_eq!(json["status"], "ACTIVE");
}

#[test]
fn test_dataset_item_deserializes_camel_case() {
    let json_str = r#"{
        "id": "item-2",
        "datasetId": "ds-2",
        "input": {"x": 42},
        "expectedOutput": {"y": 84},
        "status": "ACTIVE"
    }"#;

    let item: DatasetItem = serde_json::from_str(json_str).unwrap();
    assert_eq!(item.id, "item-2");
    assert_eq!(item.dataset_id, "ds-2");
    assert_eq!(item.input.unwrap()["x"], 42);
    assert_eq!(item.expected_output.unwrap()["y"], 84);
    assert!(item.metadata.is_none());
    assert!(item.source_trace_id.is_none());
}

#[test]
fn test_dataset_run_serializes() {
    let run = DatasetRun {
        id: "run-1".to_string(),
        name: "experiment-20250228".to_string(),
        dataset_id: "ds-1".to_string(),
        metadata: Some(json!({"model": "gpt-4"})),
    };

    let json = serde_json::to_value(&run).unwrap();
    assert_eq!(json["id"], "run-1");
    assert_eq!(json["name"], "experiment-20250228");
    assert_eq!(json["datasetId"], "ds-1");
    assert_eq!(json["metadata"]["model"], "gpt-4");
}

#[test]
fn test_create_dataset_item_body() {
    let body = CreateDatasetItemBody {
        dataset_name: "my-dataset".to_string(),
        input: Some(json!({"question": "What is 2+2?"})),
        expected_output: Some(json!(4)),
        metadata: None,
        id: None,
    };

    let json = serde_json::to_value(&body).unwrap();
    assert_eq!(json["datasetName"], "my-dataset");
    assert_eq!(json["input"]["question"], "What is 2+2?");
    assert_eq!(json["expectedOutput"], 4);
    assert!(json.get("metadata").is_none());
    assert!(json.get("id").is_none());
}

// ── Experiment runner tests ──────────────────────────────────────────────

#[tokio::test]
async fn test_run_experiment() {
    let items = vec![
        make_item("1", json!({"x": 1}), json!(2)),
        make_item("2", json!({"x": 2}), json!(4)),
    ];

    let results = run_experiment(
        items,
        ExperimentConfig::default(),
        |item| {
            Box::pin(async move {
                let x = item.input.unwrap()["x"].as_i64().unwrap();
                json!(x * 2)
            })
        },
        |item, output| {
            let expected = item.expected_output.as_ref().unwrap().as_i64().unwrap();
            let actual = output.as_i64().unwrap();
            vec![(
                "accuracy".to_string(),
                if actual == expected { 1.0 } else { 0.0 },
            )]
        },
    )
    .await;

    assert_eq!(results.len(), 2);
    for r in &results {
        assert_eq!(r.scores.len(), 1);
        assert_eq!(r.scores[0].0, "accuracy");
        assert_eq!(r.scores[0].1, 1.0);
    }
}

#[tokio::test]
async fn test_run_experiment_with_failures() {
    let items = vec![
        make_item("1", json!({"x": 3}), json!(6)),
        make_item("2", json!({"x": 5}), json!(10)),
        make_item("3", json!({"x": 7}), json!(15)), // wrong: 7*2=14, not 15
    ];

    let results = run_experiment(
        items,
        ExperimentConfig::default(),
        |item| {
            Box::pin(async move {
                let x = item.input.unwrap()["x"].as_i64().unwrap();
                json!(x * 2)
            })
        },
        |item, output| {
            let expected = item.expected_output.as_ref().unwrap().as_i64().unwrap();
            let actual = output.as_i64().unwrap();
            vec![(
                "accuracy".to_string(),
                if actual == expected { 1.0 } else { 0.0 },
            )]
        },
    )
    .await;

    assert_eq!(results.len(), 3);

    // Sort by item_id for deterministic assertions
    let mut sorted = results.clone();
    sorted.sort_by(|a, b| a.item_id.cmp(&b.item_id));

    assert_eq!(sorted[0].scores[0].1, 1.0); // item 1: correct
    assert_eq!(sorted[1].scores[0].1, 1.0); // item 2: correct
    assert_eq!(sorted[2].scores[0].1, 0.0); // item 3: incorrect
}

#[tokio::test]
async fn test_experiment_concurrency() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let max_concurrent = Arc::new(AtomicUsize::new(0));
    let current_concurrent = Arc::new(AtomicUsize::new(0));

    let items: Vec<DatasetItem> = (0..5)
        .map(|i| make_item(&i.to_string(), json!({"x": i}), json!(i)))
        .collect();

    let max_c = max_concurrent.clone();
    let cur_c = current_concurrent.clone();

    let results = run_experiment(
        items,
        ExperimentConfig {
            name: "concurrency-test".to_string(),
            max_concurrency: 1,
            base_url: String::new(),
            dataset_name: String::new(),
        },
        move |item| {
            let max_c = max_c.clone();
            let cur_c = cur_c.clone();
            Box::pin(async move {
                let prev = cur_c.fetch_add(1, Ordering::SeqCst);
                // Update max observed concurrency
                max_c.fetch_max(prev + 1, Ordering::SeqCst);
                // Small yield to allow other tasks to attempt running
                tokio::task::yield_now().await;
                cur_c.fetch_sub(1, Ordering::SeqCst);
                item.input.unwrap_or(json!(null))
            })
        },
        |_item, _output| vec![("done".to_string(), 1.0)],
    )
    .await;

    assert_eq!(results.len(), 5);
    // With max_concurrency=1, at most 1 task should run at a time
    assert_eq!(max_concurrent.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_experiment_empty_items() {
    let results = run_experiment(
        vec![],
        ExperimentConfig::default(),
        |_item| Box::pin(async move { json!(null) }),
        |_item, _output| vec![],
    )
    .await;

    assert!(results.is_empty());
}

#[test]
fn test_experiment_config_default() {
    let config = ExperimentConfig::default();
    assert!(config.name.starts_with("experiment-"));
    assert_eq!(config.max_concurrency, 10);
}

#[test]
fn test_experiment_result_clone() {
    let result = ExperimentResult {
        item_id: "test-1".to_string(),
        output: json!({"answer": 42}),
        scores: vec![("accuracy".to_string(), 0.95)],
        dataset_run_url: String::new(),
    };

    let cloned = result.clone();
    assert_eq!(cloned.item_id, result.item_id);
    assert_eq!(cloned.output, result.output);
    assert_eq!(cloned.scores, result.scores);
}
