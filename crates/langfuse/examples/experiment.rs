//! Experiment runner example showing dataset creation and experiment execution.
//!
//! This example demonstrates:
//! - Creating a dataset
//! - Adding items to the dataset
//! - Running an experiment with a task function and evaluator
//! - Collecting and displaying results
//!
//! Run: LANGFUSE_PUBLIC_KEY=pk-... LANGFUSE_SECRET_KEY=sk-... cargo run --example experiment

use langfuse::{
    Langfuse, LangfuseConfig,
    datasets::experiment::{ExperimentConfig, run_experiment},
    datasets::types::{CreateDatasetBody, CreateDatasetItemBody, DatasetItem},
};
use serde_json::json;
use std::future::Future;
use std::pin::Pin;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Langfuse from environment variables
    let config = LangfuseConfig::from_env()?;
    let langfuse = Langfuse::new(config)?;

    println!("=== Experiment Runner Example ===\n");

    // Example 1: Create a dataset
    println!("Step 1: Creating dataset...");
    let dataset_name = format!("qa-dataset-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
    let create_body = CreateDatasetBody {
        name: dataset_name.clone(),
        description: Some("Example QA dataset for experiments".to_string()),
        metadata: None,
    };

    match langfuse.datasets.create_dataset(create_body).await {
        Ok(dataset) => {
            println!("  ✓ Created dataset: {}", dataset.name);
        }
        Err(e) => {
            println!(
                "  ✗ Error creating dataset: {} (expected if dataset already exists)",
                e
            );
        }
    }

    // Example 2: Add items to the dataset
    println!("\nStep 2: Adding items to dataset...");
    let items = [
        ("What is Rust?", "A systems programming language"),
        ("What is Python?", "A high-level programming language"),
        ("What is Go?", "A compiled programming language"),
    ];

    let mut dataset_items = Vec::new();
    for (i, (question, expected_answer)) in items.iter().enumerate() {
        let item_body = CreateDatasetItemBody {
            dataset_name: dataset_name.clone(),
            input: Some(json!({ "question": question })),
            expected_output: Some(json!({ "answer": expected_answer })),
            metadata: None,
            id: None,
        };

        match langfuse.datasets.create_item(item_body).await {
            Ok(item) => {
                println!("  ✓ Added item {}: {}", i + 1, question);
                dataset_items.push(item);
            }
            Err(e) => {
                println!(
                    "  ✗ Error adding item: {} (expected if dataset doesn't exist)",
                    e
                );
            }
        }
    }

    // Example 3: Run an experiment
    println!("\nStep 3: Running experiment...");
    if !dataset_items.is_empty() {
        let config = ExperimentConfig {
            name: format!(
                "qa-experiment-{}",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            ),
            max_concurrency: 5,
            base_url: String::new(),
            dataset_name: String::new(),
        };

        let results = run_experiment(
            dataset_items,
            config,
            // Task function: process each item
            |item: DatasetItem| {
                Box::pin(async move {
                    // Simulate LLM call
                    let question = item
                        .input
                        .as_ref()
                        .and_then(|q| q.get("question"))
                        .and_then(|q| q.as_str())
                        .unwrap_or("unknown");

                    // Simple mock response
                    let answer = match question {
                        "What is Rust?" => {
                            "A systems programming language focused on safety and performance"
                        }
                        "What is Python?" => "A high-level, interpreted programming language",
                        "What is Go?" => "A compiled language designed for concurrent programming",
                        _ => "I don't know",
                    };

                    json!({ "answer": answer })
                }) as Pin<Box<dyn Future<Output = serde_json::Value> + Send>>
            },
            // Evaluator function: score the output
            |item: &DatasetItem, output: &serde_json::Value| {
                let expected = item
                    .expected_output
                    .as_ref()
                    .and_then(|e| e.get("answer"))
                    .and_then(|a| a.as_str())
                    .unwrap_or("");

                let actual = output.get("answer").and_then(|a| a.as_str()).unwrap_or("");

                // Simple exact match scoring
                let exact_match = if expected == actual { 1.0 } else { 0.0 };

                // Simple substring match scoring
                let substring_match = if actual.contains(expected) { 1.0 } else { 0.0 };

                vec![
                    ("exact_match".to_string(), exact_match),
                    ("substring_match".to_string(), substring_match),
                ]
            },
        )
        .await;

        // Display results
        println!("  ✓ Experiment completed with {} results", results.len());
        for (i, result) in results.iter().enumerate() {
            println!("\n  Result {}:", i + 1);
            println!("    Item ID: {}", result.item_id);
            println!("    Output: {}", result.output);
            for (score_name, score_value) in &result.scores {
                println!("    {}: {:.2}", score_name, score_value);
            }
        }
    } else {
        println!("  ⚠ No dataset items available (dataset may not exist on server)");
    }

    println!("\nExperiment example completed!");
    Ok(())
}
