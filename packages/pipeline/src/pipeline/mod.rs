mod extract;
mod ingest;
mod normalize;
mod rank;
mod relate;
mod segment;
mod worldview;

use crate::model::{
    ArgumentCandidate, ArgumentRoleLink, ArtifactEnvelope, ArtifactRef, CandidateIndexEntry,
    ClaimCandidate, ClaimKind, ClaimRole, ExtractionDocument, ExtractionResponse, FormalEvaluation,
    FormalEvaluationStatus, FormalLogicAst, FormalizationCandidate, FormalizationStatus,
    IngestedDocument, NormalizedProposition, Passage, PassageCluster, PassageEmbedding,
    PipelineDiagnostic, PipelineRun, PipelineRunConfig, PipelineStage, PropositionCandidate,
    PhilosophyCorpusInputV1, RelationCandidate, RelationKind, SourceDocument, SourceFragment,
    SourceLocatorV1, SourceOffset, SourceSpanExtractionResponse, StageSummary, TermCandidate,
    TermType, UnifiedWorldviewV10,
};
use philosophy_extractor_artifact_store::{ArtifactStoreError, FileArtifactStore, artifact_id};
use philosophy_extractor_model_providers::{
    ClaimClassifierProvider, DeterministicEmbeddingProvider, EmbeddingProvider,
    HeuristicClaimClassifierProvider, NerProvider, RustPackagesEmbeddingProvider,
    TextLinguisticsNerProvider,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub worldview_id: Option<String>,
    pub label: Option<String>,
    pub min_confidence: f64,
    pub max_propositions: Option<usize>,
    pub include_relations: bool,
    pub artifact_dir: PathBuf,
    pub persist_artifacts: bool,
    pub openai_model_primary: String,
    pub openai_model_cheap: String,
    pub embedding_model: String,
    pub ner_model: String,
    pub nlp_mode: NlpMode,
    pub model_bundle_dir: PathBuf,
    pub auto_download_models: bool,
    pub embedding_backend: EmbeddingBackendConfig,
    pub term_extraction_backend: TermExtractionBackendConfig,
    pub classification_backend: ClassificationBackendConfig,
    pub lean_bin: String,
    pub stage_through: Option<PipelineStage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NlpMode {
    Heuristic,
    LocalModels,
}

impl NlpMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Heuristic => "heuristic",
            Self::LocalModels => "local-models",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingBackendConfig {
    Deterministic,
    TextRetrieval,
}

impl EmbeddingBackendConfig {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic",
            Self::TextRetrieval => "text-retrieval",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermExtractionBackendConfig {
    Heuristic,
    TextLinguistics,
}

impl TermExtractionBackendConfig {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Heuristic => "heuristic",
            Self::TextLinguistics => "text-linguistics",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationBackendConfig {
    Heuristic,
    TextLinguistics,
}

impl ClassificationBackendConfig {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Heuristic => "heuristic",
            Self::TextLinguistics => "text-linguistics",
        }
    }
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            worldview_id: None,
            label: None,
            min_confidence: 0.35,
            max_propositions: None,
            include_relations: true,
            artifact_dir: std::env::var("PHILOSOPHY_EXTRACTOR_ARTIFACT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(".artifacts")),
            persist_artifacts: true,
            openai_model_primary: std::env::var("PHILOSOPHY_EXTRACTOR_OPENAI_MODEL_PRIMARY")
                .unwrap_or_else(|_| "gpt-5.5".to_string()),
            openai_model_cheap: std::env::var("PHILOSOPHY_EXTRACTOR_OPENAI_MODEL_CHEAP")
                .unwrap_or_else(|_| "gpt-5.5-mini".to_string()),
            embedding_model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            ner_model: "rust-bert-default-ner".to_string(),
            nlp_mode: NlpMode::Heuristic,
            model_bundle_dir: PathBuf::from(".video-analysis-models"),
            auto_download_models: false,
            embedding_backend: EmbeddingBackendConfig::Deterministic,
            term_extraction_backend: TermExtractionBackendConfig::Heuristic,
            classification_backend: ClassificationBackendConfig::Heuristic,
            lean_bin: std::env::var("PHILOSOPHY_EXTRACTOR_LEAN_BIN")
                .unwrap_or_else(|_| "lean".to_string()),
            stage_through: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("input text is empty")]
    EmptyInput,
    #[error("artifact store error: {0}")]
    Artifact(#[from] ArtifactStoreError),
    #[error("embedding provider error: {0}")]
    Embedding(String),
    #[error("invalid corpus input: {0}")]
    InvalidCorpusInput(String),
    #[error("corpus serialization error: {0}")]
    CorpusSerialization(String),
}

#[derive(Debug, Clone)]
pub struct PhilosophyExtractor {
    config: PipelineConfig,
}

impl PhilosophyExtractor {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    pub fn extract(
        &self,
        mut document: ExtractionDocument,
    ) -> Result<ExtractionResponse, PipelineError> {
        let mut diagnostics = Vec::new();
        let mut stages = Vec::new();
        let mut artifacts = Vec::new();
        let run_id = format!("run_{}", &digest_text(&document.text)[..16]);
        let store = self
            .config
            .persist_artifacts
            .then(|| FileArtifactStore::new(&self.config.artifact_dir));
        let stage_through = self
            .config
            .stage_through
            .unwrap_or(PipelineStage::Worldview);

        document = ingest::ingest(document)?;
        let document_id = source_document_id_for(&document);
        let ingested = IngestedDocument {
            document_id: document_id.clone(),
            raw_text_hash: digest_text(&document.text),
            document: document.clone(),
        };
        persist_stage(
            store.as_ref(),
            &run_id,
            PipelineStage::Ingest,
            "rules",
            "mvp-1",
            Vec::new(),
            Vec::new(),
            &ingested,
            &mut artifacts,
        )?;
        stages.push(StageSummary {
            name: "ingest".to_string(),
            output_count: 1,
            diagnostics: Vec::new(),
        });
        if should_stop(stage_through, PipelineStage::Ingest) {
            return self.partial_response(run_id, artifacts, diagnostics, stages, &document);
        }

        let passages = segment::segment_passages(&document);
        let fragments = source_fragments_from_passages(&passages);
        let segment_diagnostics = empty_fragments_diagnostic(&fragments);
        persist_stage(
            store.as_ref(),
            &run_id,
            PipelineStage::Segment,
            "rules+local-small-model",
            "mvp-1",
            vec![artifact_id(&run_id, PipelineStage::Ingest)],
            segment_diagnostics.clone(),
            &passages,
            &mut artifacts,
        )?;
        stages.push(StageSummary {
            name: "segment".to_string(),
            output_count: fragments.len(),
            diagnostics: segment_diagnostics,
        });
        if should_stop(stage_through, PipelineStage::Segment) {
            return self.partial_response(run_id, artifacts, diagnostics, stages, &document);
        }

        let embedding_output = embed_passages(&passages, &self.config)?;
        persist_stage_with_metadata(
            store.as_ref(),
            &run_id,
            PipelineStage::Embed,
            &embedding_output.provider,
            &embedding_output.provider_version,
            embedding_output.metadata.clone(),
            vec![artifact_id(&run_id, PipelineStage::Segment)],
            Vec::new(),
            &embedding_output.embeddings,
            &mut artifacts,
        )?;
        stages.push(StageSummary {
            name: "embed".to_string(),
            output_count: embedding_output.embeddings.len(),
            diagnostics: Vec::new(),
        });
        if should_stop(stage_through, PipelineStage::Embed) {
            return self.partial_response(run_id, artifacts, diagnostics, stages, &document);
        }

        let clusters = cluster_passages(&passages, &embedding_output.embeddings);
        persist_stage(
            store.as_ref(),
            &run_id,
            PipelineStage::Cluster,
            "local-embeddings",
            "cosine-union-find-mvp-1",
            vec![artifact_id(&run_id, PipelineStage::Embed)],
            Vec::new(),
            &clusters,
            &mut artifacts,
        )?;
        stages.push(StageSummary {
            name: "cluster".to_string(),
            output_count: clusters.len(),
            diagnostics: Vec::new(),
        });
        if should_stop(stage_through, PipelineStage::Cluster) {
            return self.partial_response(run_id, artifacts, diagnostics, stages, &document);
        }

        let raw_candidates = extract::extract_candidates(&fragments);
        let claim_candidates = claim_candidates_from_propositions(&raw_candidates, &passages);
        let claim_diagnostics = empty_claim_candidates_diagnostic(&claim_candidates);
        persist_stage(
            store.as_ref(),
            &run_id,
            PipelineStage::ExtractClaims,
            "openai-structured-output-fallback",
            &self.config.openai_model_cheap,
            vec![artifact_id(&run_id, PipelineStage::Cluster)],
            claim_diagnostics.clone(),
            &claim_candidates,
            &mut artifacts,
        )?;
        stages.push(StageSummary {
            name: "extract_claims".to_string(),
            output_count: claim_candidates.len(),
            diagnostics: claim_diagnostics,
        });
        if should_stop(stage_through, PipelineStage::ExtractClaims) {
            return self.partial_response(run_id, artifacts, diagnostics, stages, &document);
        }

        let role_output = classify_roles(&claim_candidates, &self.config);
        persist_stage_with_metadata(
            store.as_ref(),
            &run_id,
            PipelineStage::ClassifyRoles,
            &role_output.provider,
            &role_output.provider_version,
            role_output.metadata.clone(),
            vec![artifact_id(&run_id, PipelineStage::ExtractClaims)],
            Vec::new(),
            &role_output.roles,
            &mut artifacts,
        )?;
        stages.push(StageSummary {
            name: "classify_roles".to_string(),
            output_count: role_output.roles.len(),
            diagnostics: Vec::new(),
        });
        if should_stop(stage_through, PipelineStage::ClassifyRoles) {
            return self.partial_response(run_id, artifacts, diagnostics, stages, &document);
        }

        let term_output = extract_terms(&passages, &self.config);
        persist_stage_with_metadata(
            store.as_ref(),
            &run_id,
            PipelineStage::ExtractTerms,
            &term_output.provider,
            &term_output.provider_version,
            term_output.metadata.clone(),
            vec![artifact_id(&run_id, PipelineStage::ClassifyRoles)],
            term_output.diagnostics.clone(),
            &term_output.terms,
            &mut artifacts,
        )?;
        stages.push(StageSummary {
            name: "extract_terms".to_string(),
            output_count: term_output.terms.len(),
            diagnostics: term_output.diagnostics,
        });
        if should_stop(stage_through, PipelineStage::ExtractTerms) {
            return self.partial_response(run_id, artifacts, diagnostics, stages, &document);
        }

        let arguments = reconstruct_arguments(&clusters, &claim_candidates, &role_output.roles);
        persist_stage(
            store.as_ref(),
            &run_id,
            PipelineStage::ReconstructArguments,
            "openai-argument-reconstructor-fallback",
            &self.config.openai_model_primary,
            vec![artifact_id(&run_id, PipelineStage::ExtractTerms)],
            Vec::new(),
            &arguments,
            &mut artifacts,
        )?;
        stages.push(StageSummary {
            name: "reconstruct_arguments".to_string(),
            output_count: arguments.len(),
            diagnostics: Vec::new(),
        });
        if should_stop(stage_through, PipelineStage::ReconstructArguments) {
            return self.partial_response(run_id, artifacts, diagnostics, stages, &document);
        }

        let mut candidates = normalize::normalize_candidates(
            &fragments,
            raw_candidates,
            &self.config,
            &mut diagnostics,
        );
        let ranking_relations = if self.config.include_relations {
            relation_candidates_for(&candidates, &self.config)
        } else {
            Vec::new()
        };
        let ranking_index = rank::score_candidates(
            &candidates,
            &passages,
            &role_output.roles,
            &ranking_relations,
        );
        rank::add_rank_metadata(&mut candidates, &ranking_index);
        candidates = rank::rank_candidates(candidates, &ranking_index);
        if let Some(max) = self.config.max_propositions {
            candidates.truncate(max);
        }

        let mut relations = if self.config.include_relations {
            relation_candidates_for(&candidates, &self.config)
        } else {
            Vec::new()
        };
        if self.config.include_relations {
            relations.extend(argument_relations(&arguments));
        }
        add_variant_diagnostics(&relations, &mut diagnostics);

        let mut candidate_index =
            rank::score_candidates(&candidates, &passages, &role_output.roles, &relations);
        rank::add_rank_metadata(&mut candidates, &candidate_index);
        candidates = rank::rank_candidates(candidates, &candidate_index);
        candidate_index =
            rank::score_candidates(&candidates, &passages, &role_output.roles, &relations);
        rank::add_rank_metadata(&mut candidates, &candidate_index);
        add_rank_diagnostics(&candidate_index, &mut diagnostics);

        let normalized_propositions = normalized_propositions(&candidates, &role_output.roles);
        persist_stage(
            store.as_ref(),
            &run_id,
            PipelineStage::NormalizePropositions,
            "openai-normalizer-fallback",
            &self.config.openai_model_primary,
            vec![artifact_id(&run_id, PipelineStage::ReconstructArguments)],
            diagnostics.clone(),
            &normalized_propositions,
            &mut artifacts,
        )?;
        stages.push(StageSummary {
            name: "normalize_propositions".to_string(),
            output_count: candidates.len(),
            diagnostics: diagnostics.clone(),
        });
        persist_named_stage(
            store.as_ref(),
            &run_id,
            "09_candidate_index.json",
            format!("{run_id}_candidate_index"),
            PipelineStage::NormalizePropositions,
            "local-candidate-quality-ranker",
            "heuristic-rank-v1",
            vec![artifact_id(&run_id, PipelineStage::NormalizePropositions)],
            Vec::new(),
            &candidate_index,
            &mut artifacts,
        )?;
        if should_stop(stage_through, PipelineStage::NormalizePropositions) {
            return self.partial_response(run_id, artifacts, diagnostics, stages, &document);
        }

        let formalizations = formalize(&normalized_propositions);
        persist_stage(
            store.as_ref(),
            &run_id,
            PipelineStage::Formalize,
            "openai-formalizer+schema-validator-fallback",
            &self.config.openai_model_primary,
            vec![artifact_id(&run_id, PipelineStage::NormalizePropositions)],
            Vec::new(),
            &formalizations,
            &mut artifacts,
        )?;
        stages.push(StageSummary {
            name: "formalize".to_string(),
            output_count: formalizations.len(),
            diagnostics: Vec::new(),
        });
        if should_stop(stage_through, PipelineStage::Formalize) {
            return self.partial_response(run_id, artifacts, diagnostics, stages, &document);
        }

        let evaluations = evaluate_formalizations(&formalizations, &self.config);
        persist_stage(
            store.as_ref(),
            &run_id,
            PipelineStage::Evaluate,
            "lean",
            &self.config.lean_bin,
            vec![artifact_id(&run_id, PipelineStage::Formalize)],
            Vec::new(),
            &evaluations,
            &mut artifacts,
        )?;
        stages.push(StageSummary {
            name: "evaluate".to_string(),
            output_count: evaluations.len(),
            diagnostics: Vec::new(),
        });
        if should_stop(stage_through, PipelineStage::Evaluate) {
            return self.partial_response(run_id, artifacts, diagnostics, stages, &document);
        }

        stages.push(StageSummary {
            name: "relate".to_string(),
            output_count: relations.len(),
            diagnostics: Vec::new(),
        });

        let worldview = worldview::build_worldview(
            &document,
            &fragments,
            &candidates,
            &relations,
            &self.config,
        );
        persist_stage(
            store.as_ref(),
            &run_id,
            PipelineStage::Worldview,
            "truth-engine-worldview-v10",
            "unified_worldview_v2",
            vec![artifact_id(&run_id, PipelineStage::Evaluate)],
            Vec::new(),
            &worldview,
            &mut artifacts,
        )?;
        stages.push(StageSummary {
            name: "worldview".to_string(),
            output_count: worldview.propositions.len(),
            diagnostics: Vec::new(),
        });

        if let Some(store) = &store {
            store.write_manifest(&PipelineRun {
                run_id: run_id.clone(),
                document_id,
                config: pipeline_run_config(&self.config),
                artifacts: artifacts.clone(),
            })?;
        }

        Ok(ExtractionResponse {
            run_id,
            provider: "philosophy-extractor".to_string(),
            worldview,
            artifacts,
            candidate_index,
            candidates,
            diagnostics,
            stages,
        })
    }

    pub fn extract_source_spans(
        &self,
        input: PhilosophyCorpusInputV1,
    ) -> Result<SourceSpanExtractionResponse, PipelineError> {
        philosophy_extractor_source_ingestion::validate_philosophy_corpus_input(&input)
            .map_err(|error| PipelineError::InvalidCorpusInput(error.to_string()))?;

        let sources = input
            .sources
            .sources
            .iter()
            .map(|source| SourceDocument {
                id: source.id.clone(),
                kind: Some(source.kind.clone()),
                title: source.title.clone(),
                authors: source.creators.clone(),
                language: source.language.clone(),
                uri: source.uri.clone(),
            })
            .collect::<Vec<_>>();

        let fragments = input
            .sources
            .spans
            .iter()
            .map(|span| {
                let locator = serde_json::to_string(&span.locator)
                    .map_err(|error| PipelineError::CorpusSerialization(error.to_string()))?;
                Ok(SourceFragment {
                    id: span.id.clone(),
                    document_id: span.source_id.clone(),
                    locator: Some(locator),
                    text: Some(span.text.clone()),
                })
            })
            .collect::<Result<Vec<_>, PipelineError>>()?;

        let mut diagnostics = empty_fragments_diagnostic(&fragments);
        let raw_candidates = extract::extract_candidates(&fragments);
        let mut candidates =
            normalize::normalize_candidates(&fragments, raw_candidates, &self.config, &mut diagnostics);
        add_media_context(&mut candidates, &input);
        if let Some(max) = self.config.max_propositions {
            candidates.truncate(max);
        }

        Ok(SourceSpanExtractionResponse {
            sources,
            fragments,
            candidates,
            media_evidence_revisions: input
                .media_evidence
                .iter()
                .map(|evidence| evidence.revision.clone())
                .collect(),
            diagnostics,
        })
    }

    fn partial_response(
        &self,
        run_id: String,
        artifacts: Vec<ArtifactRef>,
        diagnostics: Vec<PipelineDiagnostic>,
        stages: Vec<StageSummary>,
        document: &ExtractionDocument,
    ) -> Result<ExtractionResponse, PipelineError> {
        Ok(ExtractionResponse {
            run_id,
            provider: "philosophy-extractor".to_string(),
            worldview: empty_worldview(document, &self.config),
            artifacts,
            candidate_index: Vec::new(),
            candidates: Vec::new(),
            diagnostics,
            stages,
        })
    }
}

fn empty_fragments_diagnostic(fragments: &[SourceFragment]) -> Vec<PipelineDiagnostic> {
    if fragments.is_empty() {
        vec![PipelineDiagnostic {
            code: "no_fragments".to_string(),
            severity: "warning".to_string(),
            message: "No textual fragments were produced from the document.".to_string(),
            target_id: None,
        }]
    } else {
        Vec::new()
    }
}

fn empty_claim_candidates_diagnostic(candidates: &[ClaimCandidate]) -> Vec<PipelineDiagnostic> {
    if candidates.is_empty() {
        vec![PipelineDiagnostic {
            code: "no_claim_candidates".to_string(),
            severity: "warning".to_string(),
            message: "No source-grounded claim candidates were found.".to_string(),
            target_id: None,
        }]
    } else {
        Vec::new()
    }
}

fn relation_candidates_for(
    candidates: &[PropositionCandidate],
    config: &PipelineConfig,
) -> Vec<RelationCandidate> {
    let mut relations = relate::infer_relations(candidates);
    relations.extend(rank::infer_variant_relations(candidates));
    if config.embedding_backend == EmbeddingBackendConfig::TextRetrieval
        || config.nlp_mode == NlpMode::LocalModels
    {
        relations = relate::rerank_relations_with_text_retrieval(candidates, relations);
    }
    relations
}

fn add_variant_diagnostics(
    relations: &[RelationCandidate],
    diagnostics: &mut Vec<PipelineDiagnostic>,
) {
    for relation in relations
        .iter()
        .filter(|relation| relation.kind == RelationKind::VariantOf)
    {
        diagnostics.push(PipelineDiagnostic {
            code: "semantic_variant_detected".to_string(),
            severity: "info".to_string(),
            message: format!(
                "Candidates '{}' and '{}' were retained as reviewable semantic variants.",
                relation.from_proposition_id, relation.to_proposition_id
            ),
            target_id: Some(relation.from_proposition_id.clone()),
        });
    }
}

fn add_rank_diagnostics(
    candidate_index: &[CandidateIndexEntry],
    diagnostics: &mut Vec<PipelineDiagnostic>,
) {
    for entry in candidate_index {
        for reason in &entry.score_breakdown.reasons {
            if matches!(
                reason.as_str(),
                "candidate_ranked_lower_due_to_low_philosophical_score"
                    | "candidate_ranked_higher_due_to_argument_role"
                    | "candidate_ranked_higher_due_to_relation_centrality"
            ) {
                diagnostics.push(PipelineDiagnostic {
                    code: reason.clone(),
                    severity: "info".to_string(),
                    message: format!(
                        "Candidate '{}' rank reason: {}.",
                        entry.proposition_id, reason
                    ),
                    target_id: Some(entry.proposition_id.clone()),
                });
            }
        }
    }
}


fn add_media_context(candidates: &mut [PropositionCandidate], input: &PhilosophyCorpusInputV1) {
    let spans_by_id = input
        .sources
        .spans
        .iter()
        .map(|span| (span.id.as_str(), span))
        .collect::<HashMap<_, _>>();
    let evidence_by_video = input
        .media_evidence
        .iter()
        .map(|evidence| (evidence.video.id.as_str(), evidence))
        .collect::<HashMap<_, _>>();

    for candidate in candidates {
        let mut contexts = Vec::new();
        for fragment_id in &candidate.source_fragment_ids {
            let Some(span) = spans_by_id.get(fragment_id.as_str()) else {
                continue;
            };
            let Some(video_id) = span.metadata.get("videoId").and_then(Value::as_str) else {
                continue;
            };
            let Some(evidence) = evidence_by_video.get(video_id) else {
                continue;
            };

            let bounds = timed_locator_bounds(&span.locator);
            let scenes = bounds.map_or_else(Vec::new, |(start, end)| {
                evidence
                    .scenes
                    .iter()
                    .filter(|scene| ranges_overlap(start, end, scene.start_seconds, scene.end_seconds))
                    .map(|scene| {
                        serde_json::json!({
                            "id": scene.id,
                            "sceneIndex": scene.scene_index,
                            "startSeconds": scene.start_seconds,
                            "endSeconds": scene.end_seconds,
                            "processor": scene.provenance.processor,
                            "processorVersion": scene.provenance.processor_version,
                            "model": scene.provenance.model,
                            "modelVersion": scene.provenance.model_version,
                        })
                    })
                    .collect()
            });
            let ocr_tracks = bounds.map_or_else(Vec::new, |(start, end)| {
                evidence
                    .ocr_tracks
                    .iter()
                    .filter_map(|track| {
                        let track_bounds = optional_bounds(track.start_seconds, track.end_seconds)?;
                        ranges_overlap(start, end, track_bounds.0, track_bounds.1).then(|| {
                            serde_json::json!({
                                "id": track.id,
                                "role": track.role,
                                "text": track.text,
                                "startSeconds": track.start_seconds,
                                "endSeconds": track.end_seconds,
                                "processor": track.provenance.processor,
                                "processorVersion": track.provenance.processor_version,
                                "model": track.provenance.model,
                                "modelVersion": track.provenance.model_version,
                            })
                        })
                    })
                    .collect()
            });
            let sponsorblock_segments = bounds.map_or_else(Vec::new, |(start, end)| {
                evidence
                    .sponsorblock
                    .as_ref()
                    .map(|sponsorblock| {
                        sponsorblock
                            .segments
                            .iter()
                            .filter(|segment| {
                                ranges_overlap(
                                    start,
                                    end,
                                    segment.start_seconds,
                                    segment.end_seconds,
                                )
                            })
                            .map(|segment| {
                                serde_json::json!({
                                    "uuid": segment.uuid,
                                    "category": segment.category,
                                    "actionType": segment.action_type,
                                    "startSeconds": segment.start_seconds,
                                    "endSeconds": segment.end_seconds,
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            });

            let mut context = serde_json::json!({
                "sourceFragmentId": span.id,
                "videoId": video_id,
                "mediaEvidenceRevision": evidence.revision,
                "scenes": scenes,
                "ocrTracks": ocr_tracks,
                "sponsorBlockSegments": sponsorblock_segments,
            });
            if let Some(reference) = span.metadata.get("mediaEvidenceRef") {
                context["sourceMediaEvidenceRef"] = reference.clone();
            }
            if let Some(sponsorblock) = &evidence.sponsorblock {
                context["sponsorBlockProvenance"] = serde_json::json!({
                    "snapshotId": sponsorblock.snapshot_id,
                    "responseHash": sponsorblock.response_hash,
                    "dataLicense": sponsorblock.data_license,
                    "attribution": sponsorblock.attribution,
                });
            }
            contexts.push(context);
        }
        if !contexts.is_empty() {
            candidate
                .metadata
                .insert("mediaContext".to_string(), Value::Array(contexts));
        }
    }
}

fn timed_locator_bounds(locator: &SourceLocatorV1) -> Option<(f64, f64)> {
    match locator {
        SourceLocatorV1::Timed {
            start_seconds,
            end_seconds,
            ..
        } => optional_bounds(*start_seconds, *end_seconds),
        SourceLocatorV1::Text { .. } => None,
    }
}

fn optional_bounds(start: Option<f64>, end: Option<f64>) -> Option<(f64, f64)> {
    match (start, end) {
        (Some(start), Some(end)) => Some((start, end)),
        (Some(start), None) => Some((start, start)),
        (None, Some(end)) => Some((end, end)),
        (None, None) => None,
    }
}

fn ranges_overlap(left_start: f64, left_end: f64, right_start: f64, right_end: f64) -> bool {
    left_start <= right_end && right_start <= left_end
}

fn should_stop(stage_through: PipelineStage, current: PipelineStage) -> bool {
    current >= stage_through
}

fn pipeline_run_config(config: &PipelineConfig) -> PipelineRunConfig {
    PipelineRunConfig {
        artifact_dir: config.artifact_dir.clone(),
        openai_model_primary: config.openai_model_primary.clone(),
        openai_model_cheap: config.openai_model_cheap.clone(),
        embedding_model: config.embedding_model.clone(),
        ner_model: config.ner_model.clone(),
        nlp_mode: config.nlp_mode.as_str().to_string(),
        model_bundle_dir: config.model_bundle_dir.clone(),
        auto_download_models: config.auto_download_models,
        embedding_backend: config.embedding_backend.as_str().to_string(),
        term_extraction_backend: config.term_extraction_backend.as_str().to_string(),
        classification_backend: config.classification_backend.as_str().to_string(),
        lean_bin: config.lean_bin.clone(),
        stage_through: config.stage_through,
    }
}

fn persist_stage<T: Serialize>(
    store: Option<&FileArtifactStore>,
    run_id: &str,
    stage: PipelineStage,
    provider: &str,
    provider_version: &str,
    input_artifact_ids: Vec<String>,
    diagnostics: Vec<PipelineDiagnostic>,
    payload: &T,
    artifacts: &mut Vec<ArtifactRef>,
) -> Result<(), PipelineError> {
    persist_stage_with_metadata(
        store,
        run_id,
        stage,
        provider,
        provider_version,
        BTreeMap::new(),
        input_artifact_ids,
        diagnostics,
        payload,
        artifacts,
    )
}

fn persist_stage_with_metadata<T: Serialize>(
    store: Option<&FileArtifactStore>,
    run_id: &str,
    stage: PipelineStage,
    provider: &str,
    provider_version: &str,
    metadata: BTreeMap<String, Value>,
    input_artifact_ids: Vec<String>,
    diagnostics: Vec<PipelineDiagnostic>,
    payload: &T,
    artifacts: &mut Vec<ArtifactRef>,
) -> Result<(), PipelineError> {
    if let Some(store) = store {
        let envelope = ArtifactEnvelope {
            run_id: run_id.to_string(),
            stage,
            provider: provider.to_string(),
            provider_version: provider_version.to_string(),
            metadata,
            input_artifact_ids,
            created_at: created_at(),
            diagnostics,
            payload,
        };
        artifacts.push(store.write_stage(run_id, &envelope)?);
    }
    Ok(())
}

fn persist_named_stage<T: Serialize>(
    store: Option<&FileArtifactStore>,
    run_id: &str,
    file_name: &str,
    artifact_ref_id: String,
    stage: PipelineStage,
    provider: &str,
    provider_version: &str,
    input_artifact_ids: Vec<String>,
    diagnostics: Vec<PipelineDiagnostic>,
    payload: &T,
    artifacts: &mut Vec<ArtifactRef>,
) -> Result<(), PipelineError> {
    if let Some(store) = store {
        let envelope = ArtifactEnvelope {
            run_id: run_id.to_string(),
            stage,
            provider: provider.to_string(),
            provider_version: provider_version.to_string(),
            metadata: BTreeMap::new(),
            input_artifact_ids,
            created_at: created_at(),
            diagnostics,
            payload,
        };
        artifacts.push(store.write_named_stage(run_id, file_name, artifact_ref_id, &envelope)?);
    }
    Ok(())
}

fn created_at() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("unix:{seconds}")
}

fn source_fragments_from_passages(passages: &[Passage]) -> Vec<SourceFragment> {
    passages
        .iter()
        .map(|passage| SourceFragment {
            id: passage.id.clone(),
            document_id: passage.document_id.clone(),
            locator: Some(passage.locator.clone()),
            text: Some(passage.text.clone()),
        })
        .collect()
}

#[derive(Debug, Clone)]
struct EmbeddingStageOutput {
    embeddings: Vec<PassageEmbedding>,
    provider: String,
    provider_version: String,
    metadata: BTreeMap<String, Value>,
}

fn embed_passages(
    passages: &[Passage],
    config: &PipelineConfig,
) -> Result<EmbeddingStageOutput, PipelineError> {
    let use_text_retrieval = config.embedding_backend == EmbeddingBackendConfig::TextRetrieval
        || config.nlp_mode == NlpMode::LocalModels;
    let provider: Box<dyn EmbeddingProvider> = if use_text_retrieval {
        Box::new(
            RustPackagesEmbeddingProvider::new(config.embedding_model.clone(), 128)
                .map_err(|error| PipelineError::Embedding(error.to_string()))?,
        )
    } else {
        Box::new(DeterministicEmbeddingProvider::new(
            config.embedding_model.clone(),
        ))
    };
    let texts = passages
        .iter()
        .map(|passage| passage.text.clone())
        .collect::<Vec<_>>();
    let vectors = provider
        .embed(&texts)
        .map_err(|error| PipelineError::Embedding(error.to_string()))?;
    let embeddings = passages
        .iter()
        .zip(vectors)
        .map(|(passage, vector)| PassageEmbedding {
            passage_id: passage.id.clone(),
            model: provider.model().to_string(),
            dimensions: vector.len(),
            vector,
        })
        .collect();
    Ok(EmbeddingStageOutput {
        embeddings,
        provider: provider.name().to_string(),
        provider_version: provider.model().to_string(),
        metadata: provider_metadata(
            provider.model(),
            if use_text_retrieval {
                "text-retrieval"
            } else {
                "deterministic"
            },
            &config.model_bundle_dir,
            config.auto_download_models,
            if use_text_retrieval {
                "feature-extraction"
            } else {
                "deterministic-feature-extraction"
            },
        ),
    })
}

fn cluster_passages(passages: &[Passage], embeddings: &[PassageEmbedding]) -> Vec<PassageCluster> {
    if passages.is_empty() {
        return Vec::new();
    }
    let mut union_find = UnionFind::new(passages.len());
    let index_by_id = passages
        .iter()
        .enumerate()
        .map(|(index, passage)| (passage.id.as_str(), index))
        .collect::<HashMap<_, _>>();

    for (left_index, left) in embeddings.iter().enumerate() {
        for right in embeddings.iter().skip(left_index + 1) {
            let Some(&right_index) = index_by_id.get(right.passage_id.as_str()) else {
                continue;
            };
            let similarity = cosine(&left.vector, &right.vector);
            let same_paragraph =
                passages[left_index].paragraph_index == passages[right_index].paragraph_index;
            if similarity >= 0.74 || (same_paragraph && similarity >= 0.58) {
                union_find.union(left_index, right_index);
            }
        }
    }

    let mut grouped: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for index in 0..passages.len() {
        grouped
            .entry(union_find.find(index))
            .or_default()
            .push(index);
    }

    grouped
        .into_values()
        .enumerate()
        .map(|(cluster_index, indices)| {
            let representative = indices[0];
            let passage_ids = indices
                .iter()
                .map(|&index| passages[index].id.clone())
                .collect::<Vec<_>>();
            PassageCluster {
                id: format!("cluster_{}", cluster_index + 1),
                passage_ids,
                centroid_vector_id: format!("centroid_{}", cluster_index + 1),
                representative_passage_id: passages[representative].id.clone(),
                confidence: if indices.len() > 1 { 0.72 } else { 0.5 },
            }
        })
        .collect()
}

fn claim_candidates_from_propositions(
    candidates: &[PropositionCandidate],
    passages: &[Passage],
) -> Vec<ClaimCandidate> {
    let passages_by_id = passages
        .iter()
        .map(|passage| (passage.id.as_str(), passage))
        .collect::<HashMap<_, _>>();
    candidates
        .iter()
        .filter_map(|candidate| {
            let source_passage_ids = candidate.source_fragment_ids.clone();
            if source_passage_ids.is_empty() {
                return None;
            }
            let source_offsets = source_passage_ids
                .iter()
                .filter_map(|id| passages_by_id.get(id.as_str()))
                .map(|passage| SourceOffset {
                    passage_id: passage.id.clone(),
                    start_char: passage.start_char,
                    end_char: passage.end_char,
                })
                .collect::<Vec<_>>();
            if source_offsets.is_empty() {
                return None;
            }
            Some(ClaimCandidate {
                id: candidate.id.clone(),
                raw_text: candidate.canonical_text.clone(),
                canonical_hint: Some(candidate.canonical_text.clone()),
                source_passage_ids,
                source_offsets,
                claim_kind: candidate.claim_kind,
                confidence: candidate.confidence,
                requires_review: true,
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
struct RoleStageOutput {
    roles: Vec<crate::model::RoleAssignment>,
    provider: String,
    provider_version: String,
    metadata: BTreeMap<String, Value>,
}

fn classify_roles(claims: &[ClaimCandidate], config: &PipelineConfig) -> RoleStageOutput {
    let provider = HeuristicClaimClassifierProvider::new(match config.classification_backend {
        ClassificationBackendConfig::Heuristic => "heuristic-role-classifier",
        ClassificationBackendConfig::TextLinguistics => &config.openai_model_primary,
    });
    let roles = claims
        .iter()
        .enumerate()
        .map(|(index, claim)| {
            let fallback_role = match claim.claim_kind {
                ClaimKind::Definition => ClaimRole::Definition,
                ClaimKind::Conditional if index + 1 == claims.len() => ClaimRole::Conclusion,
                ClaimKind::Conditional => ClaimRole::Premise,
                ClaimKind::Normative => ClaimRole::MainClaim,
                ClaimKind::Unknown => ClaimRole::Unknown,
                _ if index + 1 == claims.len() && claims.len() > 1 => ClaimRole::Conclusion,
                _ => ClaimRole::Premise,
            };
            let classified_role =
                if config.classification_backend == ClassificationBackendConfig::TextLinguistics {
                    provider
                        .classify_role(&claim.raw_text)
                        .ok()
                        .map(|classification| claim_role_from_label(&classification.label))
                        .filter(|role| *role != ClaimRole::Unknown)
                        .unwrap_or(fallback_role)
                } else {
                    fallback_role
                };
            crate::model::RoleAssignment {
                claim_id: claim.id.clone(),
                primary_role: classified_role,
                secondary_roles: Vec::new(),
                confidence: 0.64,
                rationale: Some(
                    "Deterministic MVP role assignment from claim kind and order.".to_string(),
                ),
            }
        })
        .collect();
    RoleStageOutput {
        roles,
        provider: provider.name().to_string(),
        provider_version: provider.model().to_string(),
        metadata: provider_metadata(
            provider.model(),
            config.classification_backend.as_str(),
            &config.model_bundle_dir,
            config.auto_download_models,
            "text-classification",
        ),
    }
}

#[derive(Debug, Clone)]
struct TermStageOutput {
    terms: Vec<TermCandidate>,
    provider: String,
    provider_version: String,
    metadata: BTreeMap<String, Value>,
    diagnostics: Vec<PipelineDiagnostic>,
}

fn extract_terms(passages: &[Passage], config: &PipelineConfig) -> TermStageOutput {
    let mut diagnostics = Vec::new();
    let heuristic_terms = extract_terms_heuristic(passages);
    let use_text_linguistics = config.term_extraction_backend
        == TermExtractionBackendConfig::TextLinguistics
        || config.nlp_mode == NlpMode::LocalModels;
    if !use_text_linguistics {
        return TermStageOutput {
            terms: heuristic_terms,
            provider: "local-heuristic-terms".to_string(),
            provider_version: "heuristic-concept-v1".to_string(),
            metadata: provider_metadata(
                "heuristic-concept-v1",
                "heuristic",
                &config.model_bundle_dir,
                config.auto_download_models,
                "token-classification",
            ),
            diagnostics,
        };
    }

    let provider = TextLinguisticsNerProvider::new(
        config.ner_model.clone(),
        config.model_bundle_dir.clone(),
        config.auto_download_models,
    );
    let model_terms = match provider.extract_entities(passages) {
        Ok(terms) => terms,
        Err(error) => {
            diagnostics.push(PipelineDiagnostic {
                code: "ner_provider_unavailable".to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "text-linguistics NER provider was unavailable; heuristic terms were used: {error}"
                ),
                target_id: None,
            });
            Vec::new()
        }
    };
    TermStageOutput {
        terms: merge_terms(heuristic_terms, model_terms),
        provider: provider.name().to_string(),
        provider_version: provider.model().to_string(),
        metadata: provider_metadata(
            provider.model(),
            "text-linguistics",
            &config.model_bundle_dir,
            config.auto_download_models,
            "token-classification",
        ),
        diagnostics,
    }
}

fn extract_terms_heuristic(passages: &[Passage]) -> Vec<TermCandidate> {
    let lexicon = [
        "knowledge",
        "truth",
        "justice",
        "virtue",
        "soul",
        "being",
        "form",
        "reason",
        "substance",
        "essence",
    ];
    let mut by_label: BTreeMap<String, TermCandidate> = BTreeMap::new();
    for passage in passages {
        for token in passage
            .text
            .split(|ch: char| !ch.is_alphanumeric() && ch != '\'')
            .filter(|token| token.len() > 2)
        {
            let normalized = token
                .trim_matches(|ch: char| !ch.is_alphanumeric())
                .to_ascii_lowercase();
            if normalized.is_empty() {
                continue;
            }
            let is_capitalized = token.chars().next().is_some_and(char::is_uppercase);
            let is_lexicon = lexicon.contains(&normalized.as_str());
            if !is_capitalized && !is_lexicon {
                continue;
            }
            by_label
                .entry(normalized.clone())
                .and_modify(|term| {
                    if !term.source_passage_ids.contains(&passage.id) {
                        term.source_passage_ids.push(passage.id.clone());
                    }
                })
                .or_insert_with(|| TermCandidate {
                    id: format!("term_{}", &digest_text(&normalized)[..16]),
                    text: token.to_string(),
                    normalized_label: normalized,
                    term_type: if is_lexicon {
                        TermType::Concept
                    } else {
                        TermType::Other
                    },
                    source_passage_ids: vec![passage.id.clone()],
                    confidence: if is_lexicon { 0.78 } else { 0.52 },
                    requires_review: !is_lexicon,
                    model_added: false,
                });
        }
    }
    by_label.into_values().collect()
}

fn merge_terms(
    heuristic_terms: Vec<TermCandidate>,
    model_terms: Vec<TermCandidate>,
) -> Vec<TermCandidate> {
    let mut by_label = BTreeMap::<String, TermCandidate>::new();
    for term in heuristic_terms.into_iter().chain(model_terms) {
        by_label
            .entry(term.normalized_label.clone())
            .and_modify(|existing| {
                for passage_id in &term.source_passage_ids {
                    if !existing.source_passage_ids.contains(passage_id) {
                        existing.source_passage_ids.push(passage_id.clone());
                    }
                }
                existing.confidence = existing.confidence.max(term.confidence);
                existing.model_added |= term.model_added;
                if existing.term_type == TermType::Other && term.term_type != TermType::Other {
                    existing.term_type = term.term_type;
                }
            })
            .or_insert(term);
    }
    by_label.into_values().collect()
}

pub fn claim_kind_from_label(label: &str) -> ClaimKind {
    match normalize_label(label).as_str() {
        "atomic" => ClaimKind::Atomic,
        "conditional" => ClaimKind::Conditional,
        "definition" => ClaimKind::Definition,
        "modal" => ClaimKind::Modal,
        "normative" => ClaimKind::Normative,
        "universal" => ClaimKind::Universal,
        _ => ClaimKind::Unknown,
    }
}

pub fn claim_role_from_label(label: &str) -> ClaimRole {
    match normalize_label(label).as_str() {
        "main_claim" | "mainclaim" => ClaimRole::MainClaim,
        "premise" => ClaimRole::Premise,
        "conclusion" => ClaimRole::Conclusion,
        "definition" => ClaimRole::Definition,
        "objection" => ClaimRole::Objection,
        "reply" => ClaimRole::Reply,
        "example" => ClaimRole::Example,
        "background" => ClaimRole::Background,
        "qualification" => ClaimRole::Qualification,
        _ => ClaimRole::Unknown,
    }
}

pub fn relation_kind_from_label(label: &str) -> Option<RelationKind> {
    match normalize_label(label).as_str() {
        "supports" => Some(RelationKind::Supports),
        "contradicts" => Some(RelationKind::Contradicts),
        "generalizes" => Some(RelationKind::Generalizes),
        "specializes" => Some(RelationKind::Specializes),
        "variant_of" | "variantof" => Some(RelationKind::VariantOf),
        _ => None,
    }
}

fn normalize_label(label: &str) -> String {
    label.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

fn provider_metadata(
    model_id: &str,
    backend: &str,
    model_bundle_dir: &std::path::Path,
    auto_download: bool,
    task_category: &str,
) -> BTreeMap<String, Value> {
    let mut metadata = BTreeMap::new();
    metadata.insert("modelId".to_string(), Value::String(model_id.to_string()));
    metadata.insert("backend".to_string(), Value::String(backend.to_string()));
    metadata.insert(
        "modelBundleDir".to_string(),
        Value::String(model_bundle_dir.to_string_lossy().into_owned()),
    );
    metadata.insert("autoDownloadModels".to_string(), Value::Bool(auto_download));
    metadata.insert(
        "taskCategory".to_string(),
        Value::String(task_category.to_string()),
    );
    metadata
}

fn reconstruct_arguments(
    clusters: &[PassageCluster],
    claims: &[ClaimCandidate],
    roles: &[crate::model::RoleAssignment],
) -> Vec<ArgumentCandidate> {
    let role_by_claim = roles
        .iter()
        .map(|role| (role.claim_id.as_str(), role.primary_role))
        .collect::<HashMap<_, _>>();
    let mut arguments = Vec::new();
    for cluster in clusters {
        let cluster_claims = claims
            .iter()
            .filter(|claim| {
                claim
                    .source_passage_ids
                    .iter()
                    .any(|id| cluster.passage_ids.contains(id))
            })
            .collect::<Vec<_>>();
        if cluster_claims.len() < 2 {
            continue;
        }
        let conclusion = cluster_claims
            .iter()
            .find(|claim| {
                matches!(
                    role_by_claim.get(claim.id.as_str()),
                    Some(ClaimRole::Conclusion | ClaimRole::MainClaim)
                )
            })
            .copied()
            .unwrap_or(cluster_claims[cluster_claims.len() - 1]);
        let premise_claim_ids = cluster_claims
            .iter()
            .filter(|claim| claim.id != conclusion.id)
            .map(|claim| claim.id.clone())
            .collect::<Vec<_>>();
        if premise_claim_ids.is_empty() {
            continue;
        }
        let support_relations = premise_claim_ids
            .iter()
            .map(|premise_id| ArgumentRoleLink {
                from_claim_id: premise_id.clone(),
                to_claim_id: conclusion.id.clone(),
                relation: RelationKind::Supports,
                confidence: 0.6,
            })
            .collect::<Vec<_>>();
        arguments.push(ArgumentCandidate {
            id: format!(
                "arg_{}",
                &digest_text(&format!("{}{}", cluster.id, conclusion.id))[..16]
            ),
            cluster_id: cluster.id.clone(),
            premise_claim_ids,
            conclusion_claim_id: conclusion.id.clone(),
            objection_claim_ids: Vec::new(),
            support_relations,
            attack_relations: Vec::new(),
            confidence: 0.58,
            rationale: Some(
                "Deterministic MVP argument reconstruction within passage cluster.".to_string(),
            ),
        });
    }
    arguments
}

fn normalized_propositions(
    candidates: &[PropositionCandidate],
    roles: &[crate::model::RoleAssignment],
) -> Vec<NormalizedProposition> {
    let role_by_claim = roles
        .iter()
        .map(|role| (role.claim_id.as_str(), role.primary_role))
        .collect::<HashMap<_, _>>();
    let mut by_text: BTreeMap<String, NormalizedProposition> = BTreeMap::new();
    for candidate in candidates {
        by_text
            .entry(candidate.canonical_text.clone())
            .and_modify(|existing| {
                existing.source_claim_ids.push(candidate.id.clone());
                for passage_id in &candidate.source_fragment_ids {
                    if !existing.source_passage_ids.contains(passage_id) {
                        existing.source_passage_ids.push(passage_id.clone());
                    }
                }
                existing.confidence = existing.confidence.max(candidate.confidence);
            })
            .or_insert_with(|| NormalizedProposition {
                id: candidate.id.clone(),
                canonical_text: candidate.canonical_text.clone(),
                source_claim_ids: vec![candidate.id.clone()],
                source_passage_ids: candidate.source_fragment_ids.clone(),
                claim_kind: candidate.claim_kind,
                role: role_by_claim
                    .get(candidate.id.as_str())
                    .copied()
                    .unwrap_or(ClaimRole::Unknown),
                confidence: candidate.confidence,
            });
    }
    by_text.into_values().collect()
}

fn formalize(propositions: &[NormalizedProposition]) -> Vec<FormalizationCandidate> {
    propositions
        .iter()
        .map(|proposition| {
            let predicate = sanitize_identifier(&proposition.id);
            let ast = if proposition
                .canonical_text
                .to_ascii_lowercase()
                .contains(" not ")
            {
                FormalLogicAst::Not {
                    value: Box::new(FormalLogicAst::Atomic {
                        predicate,
                        args: Vec::new(),
                    }),
                }
            } else {
                FormalLogicAst::Atomic {
                    predicate,
                    args: Vec::new(),
                }
            };
            let lean_expression = lean_expression_for_ast(&ast);
            FormalizationCandidate {
                id: format!("formal_{}", proposition.id),
                proposition_id: proposition.id.clone(),
                ast,
                lean_expression,
                status: FormalizationStatus::SchemaValid,
                confidence: 0.5,
                diagnostics: Vec::new(),
            }
        })
        .collect()
}

fn evaluate_formalizations(
    formalizations: &[FormalizationCandidate],
    config: &PipelineConfig,
) -> Vec<FormalEvaluation> {
    formalizations
        .iter()
        .map(|candidate| FormalEvaluation {
            formalization_id: candidate.id.clone(),
            proposition_id: candidate.proposition_id.clone(),
            status: FormalEvaluationStatus::Unsupported,
            lean_source: Some(format!(
                "-- Lean binary: {}\ndef {} : Prop := {}\n#check {}\n",
                config.lean_bin,
                sanitize_identifier(&candidate.id),
                candidate.lean_expression,
                sanitize_identifier(&candidate.id)
            )),
            diagnostics: vec![PipelineDiagnostic {
                code: "unsupported_goal".to_string(),
                severity: "info".to_string(),
                message: "Lean MVP generated typecheckable source; proof search is unsupported for this candidate.".to_string(),
                target_id: Some(candidate.id.clone()),
            }],
        })
        .collect()
}

fn argument_relations(arguments: &[ArgumentCandidate]) -> Vec<RelationCandidate> {
    let mut relations = Vec::new();
    for argument in arguments {
        for support in &argument.support_relations {
            let mut metadata = BTreeMap::new();
            metadata.insert("argumentId".to_string(), Value::String(argument.id.clone()));
            metadata.insert("reviewRequired".to_string(), Value::Bool(true));
            relations.push(RelationCandidate {
                from_proposition_id: support.from_claim_id.clone(),
                to_proposition_id: support.to_claim_id.clone(),
                kind: support.relation,
                confidence: support.confidence,
                note: Some("Argument reconstruction linked premise to conclusion.".to_string()),
                metadata,
            });
        }
    }
    relations
}

fn empty_worldview(document: &ExtractionDocument, config: &PipelineConfig) -> UnifiedWorldviewV10 {
    worldview::build_worldview(document, &[], &[], &[], config)
}

fn cosine(left: &[f32], right: &[f32]) -> f64 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let dot = left
        .iter()
        .zip(right)
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum::<f64>();
    let left_norm = left
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt();
    let right_norm = right
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm * right_norm)
    }
}

fn sanitize_identifier(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    if output
        .chars()
        .next()
        .is_none_or(|ch| !ch.is_ascii_alphabetic())
    {
        output.insert(0, 'p');
    }
    output
}

fn lean_expression_for_ast(ast: &FormalLogicAst) -> String {
    match ast {
        FormalLogicAst::Atomic { .. } => "True".to_string(),
        FormalLogicAst::Not { value } => format!("Not ({})", lean_expression_for_ast(value)),
        FormalLogicAst::And { values } => values
            .iter()
            .map(lean_expression_for_ast)
            .collect::<Vec<_>>()
            .join(" /\\ "),
        FormalLogicAst::Or { values } => values
            .iter()
            .map(lean_expression_for_ast)
            .collect::<Vec<_>>()
            .join(" \\/ "),
        FormalLogicAst::Implies { left, right } => {
            format!(
                "({}) -> ({})",
                lean_expression_for_ast(left),
                lean_expression_for_ast(right)
            )
        }
        FormalLogicAst::Iff { left, right } => {
            format!(
                "Iff ({}) ({})",
                lean_expression_for_ast(left),
                lean_expression_for_ast(right)
            )
        }
        FormalLogicAst::ForAll { variable, body } => {
            format!(
                "forall {variable} : Prop, {}",
                lean_expression_for_ast(body)
            )
        }
        FormalLogicAst::Exists { variable, body } => {
            format!(
                "exists {variable} : Prop, {}",
                lean_expression_for_ast(body)
            )
        }
    }
}

#[derive(Debug, Clone)]
struct UnionFind {
    parents: Vec<usize>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        Self {
            parents: (0..size).collect(),
        }
    }

    fn find(&mut self, index: usize) -> usize {
        let parent = self.parents[index];
        if parent == index {
            index
        } else {
            let root = self.find(parent);
            self.parents[index] = root;
            root
        }
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root != right_root {
            self.parents[right_root] = left_root;
        }
    }
}

pub(crate) fn clamp_confidence(value: f64) -> f64 {
    (value.clamp(0.0, 1.0) * 100.0).round() / 100.0
}

pub(crate) fn digest_text(value: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(crate) fn proposition_id_for(canonical_text: &str) -> String {
    format!("p_{}", &digest_text(canonical_text)[..24])
}

pub(crate) fn source_document_id_for(document: &ExtractionDocument) -> String {
    document
        .id
        .as_ref()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("doc_{}", &digest_text(&document.text)[..16]))
}

pub(crate) fn normalize_canonical_text(value: &str) -> String {
    let mut text = value.trim().to_string();
    while let Some(stripped) = text.strip_prefix(['-', '*']) {
        text = stripped.trim_start().to_string();
    }
    let mut collapsed = String::new();
    let mut last_was_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                collapsed.push(' ');
            }
            last_was_space = true;
        } else {
            collapsed.push(ch);
            last_was_space = false;
        }
    }

    let mut normalized = collapsed.trim().to_string();
    if normalized.is_empty() {
        return normalized;
    }
    if let Some(first) = normalized.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    if !matches!(normalized.chars().last(), Some('.') | Some('!') | Some('?')) {
        normalized.push('.');
    }
    normalized
}

pub(crate) fn relation_id_for(relation: &RelationCandidate) -> String {
    let readable = format!(
        "relation_{}_{}_{}",
        relation.from_proposition_id,
        relation.kind.as_str(),
        relation.to_proposition_id
    );
    if readable.len() <= 120 {
        readable
    } else {
        format!("relation_{}", &digest_text(&readable)[..32])
    }
}
