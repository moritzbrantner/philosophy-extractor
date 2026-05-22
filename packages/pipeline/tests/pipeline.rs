use philosophy_extractor::{ExtractionDocument, PhilosophyExtractor, PipelineConfig};

fn document(text: &str) -> ExtractionDocument {
    ExtractionDocument {
        id: Some("doc-test".to_string()),
        kind: Some("philosophical_text".to_string()),
        title: Some("Test text".to_string()),
        authors: vec!["Example Author".to_string()],
        language: Some("en".to_string()),
        uri: None,
        text: text.to_string(),
    }
}

#[test]
fn emits_truth_engine_worldview_v10() {
    let extractor = PhilosophyExtractor::new(PipelineConfig::default());
    let response = extractor
        .extract(document(
            "Knowledge concerns truth. Justice should harmonize the soul. If virtue is knowledge, teaching matters.",
        ))
        .unwrap();

    assert_eq!(response.worldview.schema_version, "10");
    assert_eq!(response.worldview.fragment, "unified_worldview_v2");
    assert_eq!(response.worldview.source_documents[0].id, "doc-test");
    assert_eq!(response.worldview.source_fragments.len(), 3);
    assert_eq!(response.worldview.propositions.len(), 3);
    assert!(response.worldview.propositions.iter().any(|proposition| {
        proposition
            .metadata
            .get("claimKind")
            .and_then(|value| value.as_str())
            == Some("normative")
    }));
}

#[test]
fn deduplicates_canonical_propositions() {
    let extractor = PhilosophyExtractor::new(PipelineConfig::default());
    let response = extractor
        .extract(document("Truth is knowable. truth is knowable."))
        .unwrap();

    assert_eq!(response.worldview.propositions.len(), 1);
}

#[test]
fn infers_basic_contradictions() {
    let extractor = PhilosophyExtractor::new(PipelineConfig::default());
    let response = extractor
        .extract(document(
            "The soul is immortal. The soul is not immortal. Knowledge concerns truth.",
        ))
        .unwrap();

    assert!(
        response
            .worldview
            .relations
            .iter()
            .any(|relation| relation.kind == "contradicts")
    );
}

#[test]
fn honors_max_propositions() {
    let extractor = PhilosophyExtractor::new(PipelineConfig {
        max_propositions: Some(2),
        ..PipelineConfig::default()
    });
    let response = extractor
        .extract(document(
            "Reality is intelligible. Knowledge concerns truth. Justice should guide action.",
        ))
        .unwrap();

    assert_eq!(response.worldview.propositions.len(), 2);
}
