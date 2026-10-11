//! Regression tests for review findings on the corpus-core intake (implementation-authored, not
//! part of the independent acceptance in corpus_core_intake.rs).

use philosophy_extractor_source_ingestion::source_spans_from_corpus_core_jsonl;
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("fixtures/corpus_core/document-search-batch.jsonl");

fn with_first_representation(mutate: impl FnOnce(&mut Value)) -> String {
    let mut done = false;
    let mut mutate = Some(mutate);
    FIXTURE
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut value: Value = serde_json::from_str(line).unwrap();
            if !done && value["record"]["record_type"] == "representation" {
                (mutate.take().unwrap())(&mut value["record"]["record"]);
                done = true;
            }
            serde_json::to_string(&value).unwrap()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn accepts_a_content_reference_that_agrees_with_the_representation_hash() {
    let jsonl = with_first_representation(|record| {
        let hash = record["content_hash"].clone();
        record["content"] = json!({
            "uri": "payload:extracted-text",
            "hash": hash,
            "media_type": "text/plain; charset=utf-8",
            "byte_length": null
        });
    });
    source_spans_from_corpus_core_jsonl(&jsonl).expect("consistent content reference");
}

#[test]
fn rejects_a_content_reference_hash_that_conflicts_with_the_representation_hash() {
    let jsonl = with_first_representation(|record| {
        record["content"] = json!({
            "uri": "payload:extracted-text",
            "hash": "0".repeat(64),
            "media_type": "text/plain; charset=utf-8",
            "byte_length": null
        });
    });
    let error = source_spans_from_corpus_core_jsonl(&jsonl)
        .expect_err("conflicting content hash must be rejected")
        .to_string();
    assert!(error.contains("conflicts"), "{error}");
}
