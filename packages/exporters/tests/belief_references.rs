use philosophy_extractor_exporters::export_belief_evidence;
use philosophy_extractor_schema::{SourceLocatorV1, SourceSpanBatchV1};
use philosophy_extractor_source_ingestion::source_spans_from_corpus_core_jsonl;
use serde_json::{Value, json};

const INPUT: &str = include_str!("fixtures/document-source-spans.json");
const CORPUS: &str =
    include_str!("../../source-ingestion/tests/fixtures/corpus_core/document-search-batch.jsonl");
const EXPECTED: &str = include_str!("fixtures/belief-evidence.json");
const REVISION: &str = include_str!("fixtures/exporter-revision.txt");

fn input() -> SourceSpanBatchV1 {
    serde_json::from_str(INPUT).unwrap()
}
fn export(input: &SourceSpanBatchV1) -> String {
    export_belief_evidence(input, REVISION.trim()).unwrap()
}

#[test]
fn real_corpus_intake_matches_recorded_input_and_independent_expected_export() {
    let input = source_spans_from_corpus_core_jsonl(CORPUS).unwrap();
    assert_eq!(input, self::input());
    assert_eq!(export(&input), EXPECTED);
    let output: Value = serde_json::from_str(EXPECTED).unwrap();
    assert_eq!(output["evidence"].as_array().unwrap().len(), 10);
    for record in output["evidence"].as_array().unwrap() {
        assert!(record.get("text").is_none());
        assert!(record.get("score").is_none());
        assert!(record.get("candidates").is_none());
        assert_eq!(record["source"]["repository"], "document-search");
        assert_eq!(record["producer"]["revision"], "git:28463de");
    }
}

#[test]
fn order_and_repeated_export_do_not_change_bytes() {
    let mut input = input();
    assert_eq!(export(&input), export(&input));
    input.sources.reverse();
    input.spans.reverse();
    assert_eq!(export(&input), EXPECTED);
}

#[test]
fn refuses_insufficient_receipts_and_never_uses_the_exporter_to_repair_them() {
    for (key, value) in [
        ("extractionProducerVersion", Some(json!("0.1.0"))),
        ("extractionProducerVersion", None),
        ("extractionProducer", None),
    ] {
        let mut input = input();
        if let Some(value) = value {
            input.sources[0].metadata.insert(key.into(), value);
        } else {
            input.sources[0].metadata.remove(key);
        }
        assert!(
            export_belief_evidence(&input, "git:new-exporter-cannot-repair-old-records").is_err()
        );
    }
    let mut input = input();
    input.spans[0].metadata.remove("spanProducerVersion");
    assert!(export_belief_evidence(&input, REVISION.trim()).is_err());
    input = self::input();
    input.spans[0].metadata.remove("corpusCoreSegmentRevision");
    assert!(export_belief_evidence(&input, REVISION.trim()).is_err());
    assert!(export_belief_evidence(&self::input(), "0.1.0").is_err());
}

#[test]
fn text_integrity_unknown_source_types_and_incomplete_lineage_fail_explicitly() {
    let mut input = input();
    input.spans[0].text.push_str(" forged");
    assert!(
        export_belief_evidence(&input, REVISION.trim())
            .unwrap_err()
            .to_string()
            .contains("content hash")
    );
    input = self::input();
    input.sources[0].kind = "worldview".into();
    assert!(
        export_belief_evidence(&input, REVISION.trim())
            .unwrap_err()
            .to_string()
            .contains("not supported")
    );
    input = self::input();
    input.spans[0]
        .metadata
        .insert("spanProducerInputs".into(), json!([]));
    assert!(
        export_belief_evidence(&input, REVISION.trim())
            .unwrap_err()
            .to_string()
            .contains("lineage")
    );
    input = self::input();
    input.spans[0].locator = SourceLocatorV1::Timed {
        segment_index: 0,
        start_seconds: Some(0.),
        end_seconds: Some(1.),
    };
    assert!(
        export_belief_evidence(&input, REVISION.trim())
            .unwrap_err()
            .to_string()
            .contains("timed locator")
    );
}

#[test]
fn preserves_real_processor_configuration_instead_of_extraction_confidence() {
    let original: Value = serde_json::from_str(CORPUS.lines().nth(4).unwrap()).unwrap();
    let target_id = original["record"]["record"]["id"].as_str().unwrap();
    let hash = "1".repeat(64);
    let changed = CORPUS
        .lines()
        .map(|line| {
            let mut value: Value = serde_json::from_str(line).unwrap();
            if value["record"]["record"]["id"] == target_id {
                value["record"]["record"]["provenance"]["parameters_hash"] = json!(hash);
            }
            serde_json::to_string(&value).unwrap()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let sources = source_spans_from_corpus_core_jsonl(&changed).unwrap();
    let result: Value = serde_json::from_str(&export(&sources)).unwrap();
    let record = result["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["source"]["recordId"] == target_id)
        .unwrap();
    assert_eq!(record["producer"]["configHash"], hash);
    assert!(record.get("score").is_none());
    let before: Value = serde_json::from_str(EXPECTED).unwrap();
    for record in before["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|record| record["source"]["recordId"] != target_id)
    {
        assert!(result["evidence"].as_array().unwrap().contains(record));
    }
}
