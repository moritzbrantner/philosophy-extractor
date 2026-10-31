use crate::model::{
    ArtifactRef, CandidateIndexEntry, ClaimCandidate, ExtractionDocument, ExtractionResponse,
    IngestedDocument, Passage, PhilosophyCorpusInputV1, PipelineDiagnostic, PipelineRun,
    PipelineStage, PropositionCandidate, RelationCandidate, RelationKind, SourceDocument,
    SourceFragment, SourceSpanExtractionResponse, StageSummary, UnifiedWorldviewV10,
};
use philosophy_extractor_artifact_store::{FileArtifactStore, artifact_id};

use super::arguments::{argument_relations, reconstruct_arguments};
use super::artifacts::{persist_named_stage, persist_stage, persist_stage_with_metadata};
use super::claims::{claim_candidates_from_propositions, classify_roles};
use super::cluster::cluster_passages;
use super::config::{EmbeddingBackendConfig, NlpMode, PipelineConfig, pipeline_run_config};
use super::corpus_input::{add_media_context, add_source_segments};
use super::embed::embed_passages;
use super::formalize::{evaluate_formalizations, formalize, normalized_propositions};
use super::terms::extract_terms;
use super::{
    PipelineError, digest_text, extract, ingest, normalize, rank, relate, segment,
    source_document_id_for, worldview,
};

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
        let mut candidates = normalize::normalize_candidates(
            &fragments,
            raw_candidates,
            &self.config,
            &mut diagnostics,
        );
        add_source_segments(&mut candidates, &input);
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

fn should_stop(stage_through: PipelineStage, current: PipelineStage) -> bool {
    current >= stage_through
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

fn empty_worldview(document: &ExtractionDocument, config: &PipelineConfig) -> UnifiedWorldviewV10 {
    worldview::build_worldview(document, &[], &[], &[], config)
}
