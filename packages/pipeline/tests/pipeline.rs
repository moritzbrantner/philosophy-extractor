use philosophy_extractor::{
    ExtractionDocument, PhilosophyExtractor, PipelineConfig, PipelineStage,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[test]
fn writes_all_mvp_artifacts() {
    let artifact_dir = temp_artifact_dir("all");
    let extractor = PhilosophyExtractor::new(PipelineConfig {
        artifact_dir: artifact_dir.clone(),
        ..PipelineConfig::default()
    });
    let response = extractor
        .extract(document(
            "Virtue is knowledge. If virtue is knowledge, teaching matters. Injustice is not good.",
        ))
        .unwrap();

    let run_dir = artifact_dir.join(&response.run_id);
    for file in [
        "manifest.json",
        "01_ingest.json",
        "02_passages.json",
        "03_embeddings.json",
        "04_clusters.json",
        "05_claim_candidates.json",
        "06_claim_roles.json",
        "07_terms.json",
        "08_arguments.json",
        "09_normalized_propositions.json",
        "10_formalizations.json",
        "11_evaluations.json",
        "worldview.json",
    ] {
        assert!(run_dir.join(file).exists(), "missing artifact {file}");
    }
    assert_eq!(response.artifacts.len(), 12);
}

#[test]
fn stage_through_cluster_stops_after_cluster_artifact() {
    let artifact_dir = temp_artifact_dir("cluster");
    let extractor = PhilosophyExtractor::new(PipelineConfig {
        artifact_dir: artifact_dir.clone(),
        stage_through: Some(PipelineStage::Cluster),
        ..PipelineConfig::default()
    });
    let response = extractor
        .extract(document(
            "Knowledge concerns truth. Justice should guide action.",
        ))
        .unwrap();

    let run_dir = artifact_dir.join(&response.run_id);
    assert!(run_dir.join("04_clusters.json").exists());
    assert!(!run_dir.join("05_claim_candidates.json").exists());
    assert_eq!(
        response.stages.last().map(|stage| stage.name.as_str()),
        Some("cluster")
    );
}

fn temp_artifact_dir(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("philosophy-extractor-{name}-{suffix}"))
}
