use philosophy_extractor::model::{ArtifactEnvelope, CandidateIndexEntry, ClaimKind, Passage};
use philosophy_extractor::{
    EmbeddingBackendConfig, ExtractionDocument, NlpMode, PhilosophyExtractor, PipelineConfig,
    PipelineStage, TermExtractionBackendConfig,
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
fn ranks_philosophical_claims_above_generic_sentences() {
    let extractor = PhilosophyExtractor::new(PipelineConfig::default());
    let response = extractor
        .extract(document("The meeting is scheduled. Truth is knowable."))
        .unwrap();

    assert_eq!(
        response
            .candidates
            .first()
            .map(|candidate| candidate.canonical_text.as_str()),
        Some("Truth is knowable.")
    );
}

#[test]
fn merges_fingerprint_duplicates_conservatively() {
    let extractor = PhilosophyExtractor::new(PipelineConfig::default());
    let response = extractor
        .extract(document("Truth is knowable. The truth is knowable."))
        .unwrap();

    assert_eq!(response.worldview.propositions.len(), 1);
    assert_eq!(response.candidates.len(), 1);
    assert_eq!(response.candidates[0].source_fragment_ids.len(), 2);
    assert!(
        response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "fingerprint_duplicate_merged")
    );
}

#[test]
fn does_not_merge_semantic_variants() {
    let extractor = PhilosophyExtractor::new(PipelineConfig::default());
    let response = extractor
        .extract(document(
            "Knowledge concerns truth and reason. Knowledge concerns truth and reason itself.",
        ))
        .unwrap();

    assert_eq!(response.candidates.len(), 2);
    assert!(
        response
            .worldview
            .relations
            .iter()
            .any(|relation| relation.kind == "variant_of")
    );
}

#[test]
fn does_not_merge_opposite_polarity_variants() {
    let extractor = PhilosophyExtractor::new(PipelineConfig::default());
    let response = extractor
        .extract(document("The soul is immortal. The soul is not immortal."))
        .unwrap();

    assert_eq!(response.candidates.len(), 2);
    assert!(
        response
            .worldview
            .relations
            .iter()
            .any(|relation| relation.kind == "contradicts")
    );
    assert!(
        !response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "fingerprint_duplicate_merged")
    );
}

#[test]
fn candidate_index_contains_score_breakdown() {
    let extractor = PhilosophyExtractor::new(PipelineConfig::default());
    let response = extractor
        .extract(document(
            "Knowledge concerns truth. Justice should guide action.",
        ))
        .unwrap();

    assert!(!response.candidate_index.is_empty());
    for entry in &response.candidate_index {
        assert!(entry.score_breakdown.final_rank_score > 0.0);
        assert_ne!(entry.first_source_sequence, usize::MAX);
        assert!(!entry.normalized_fingerprint.is_empty());
        assert!(!entry.score_breakdown.reasons.is_empty());
    }
}

#[test]
fn max_propositions_uses_rank_score_order() {
    let extractor = PhilosophyExtractor::new(PipelineConfig {
        max_propositions: Some(1),
        ..PipelineConfig::default()
    });
    let response = extractor
        .extract(document("The meeting is scheduled. Truth is knowable."))
        .unwrap();

    assert_eq!(response.candidates.len(), 1);
    assert_eq!(response.candidates[0].canonical_text, "Truth is knowable.");
}

#[test]
fn candidate_index_artifact_is_written() {
    let artifact_dir = temp_artifact_dir("candidate-index");
    let extractor = PhilosophyExtractor::new(PipelineConfig {
        artifact_dir: artifact_dir.clone(),
        ..PipelineConfig::default()
    });
    let response = extractor
        .extract(document(
            "Knowledge concerns truth. Justice should guide action.",
        ))
        .unwrap();
    let run_dir = artifact_dir.join(&response.run_id);
    let envelope = serde_json::from_str::<ArtifactEnvelope<Vec<CandidateIndexEntry>>>(
        &std::fs::read_to_string(run_dir.join("09_candidate_index.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(envelope.provider, "local-candidate-quality-ranker");
    assert!(!envelope.payload.is_empty());
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
        "09_candidate_index.json",
        "09_normalized_propositions.json",
        "10_formalizations.json",
        "11_evaluations.json",
        "worldview.json",
    ] {
        assert!(run_dir.join(file).exists(), "missing artifact {file}");
    }
    assert_eq!(response.artifacts.len(), 13);
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

#[test]
fn text_core_segmentation_preserves_source_offsets() {
    let artifact_dir = temp_artifact_dir("segment-offsets");
    let text = "Dr. Smith wrote pi is 3.14. Wait... Really? Yes!";
    let extractor = PhilosophyExtractor::new(PipelineConfig {
        artifact_dir: artifact_dir.clone(),
        stage_through: Some(PipelineStage::Segment),
        ..PipelineConfig::default()
    });
    let response = extractor.extract(document(text)).unwrap();
    let run_dir = artifact_dir.join(&response.run_id);
    let envelope = serde_json::from_str::<ArtifactEnvelope<Vec<Passage>>>(
        &std::fs::read_to_string(run_dir.join("02_passages.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(envelope.provider, "rules+local-small-model");
    assert!(!envelope.payload.is_empty());
    for passage in envelope.payload {
        assert_eq!(&text[passage.start_char..passage.end_char], passage.text);
    }
}

#[test]
fn text_retrieval_embedding_artifact_records_model_provenance() {
    let artifact_dir = temp_artifact_dir("text-retrieval-embed");
    let extractor = PhilosophyExtractor::new(PipelineConfig {
        artifact_dir: artifact_dir.clone(),
        stage_through: Some(PipelineStage::Embed),
        embedding_backend: EmbeddingBackendConfig::TextRetrieval,
        ..PipelineConfig::default()
    });
    let response = extractor
        .extract(document(
            "Knowledge concerns truth. Justice concerns action.",
        ))
        .unwrap();
    let run_dir = artifact_dir.join(&response.run_id);
    let envelope = serde_json::from_str::<ArtifactEnvelope<serde_json::Value>>(
        &std::fs::read_to_string(run_dir.join("03_embeddings.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(envelope.provider, "text-retrieval-feature-extraction");
    assert_eq!(
        envelope
            .metadata
            .get("taskCategory")
            .and_then(|value| value.as_str()),
        Some("feature-extraction")
    );
    assert_eq!(
        envelope
            .metadata
            .get("modelId")
            .and_then(|value| value.as_str()),
        Some("sentence-transformers/all-MiniLM-L6-v2")
    );
}

#[test]
fn local_models_stage_through_cluster_stops_before_ner() {
    let artifact_dir = temp_artifact_dir("local-models-cluster");
    let extractor = PhilosophyExtractor::new(PipelineConfig {
        artifact_dir: artifact_dir.clone(),
        nlp_mode: NlpMode::LocalModels,
        auto_download_models: false,
        stage_through: Some(PipelineStage::Cluster),
        ..PipelineConfig::default()
    });
    let response = extractor
        .extract(document(
            "Knowledge concerns truth. Justice concerns action.",
        ))
        .unwrap();

    let run_dir = artifact_dir.join(&response.run_id);
    assert!(run_dir.join("04_clusters.json").exists());
    assert!(!run_dir.join("07_terms.json").exists());
    assert_eq!(
        response.stages.last().map(|stage| stage.name.as_str()),
        Some("cluster")
    );
}

#[test]
fn text_linguistics_terms_fall_back_without_duplicate_heuristic_labels() {
    let extractor = PhilosophyExtractor::new(PipelineConfig {
        persist_artifacts: false,
        term_extraction_backend: TermExtractionBackendConfig::TextLinguistics,
        auto_download_models: false,
        ..PipelineConfig::default()
    });
    let response = extractor
        .extract(document("Knowledge concerns truth. Knowledge matters."))
        .unwrap();
    let term_stage = response
        .stages
        .iter()
        .find(|stage| stage.name == "extract_terms")
        .unwrap();

    assert!(term_stage.output_count >= 2);
    assert!(
        term_stage
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "ner_provider_unavailable")
    );
}

#[test]
fn classifier_unknown_labels_map_to_safe_fallbacks() {
    assert_eq!(
        philosophy_extractor::pipeline::claim_kind_from_label("surprise"),
        ClaimKind::Unknown
    );
    assert_eq!(
        philosophy_extractor::pipeline::relation_kind_from_label("surprise"),
        None
    );
}

fn temp_artifact_dir(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("philosophy-extractor-{name}-{suffix}"))
}
