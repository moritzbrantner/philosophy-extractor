use crate::model::{
    ClaimCandidate, ClaimKind, ClaimRole, Passage, PropositionCandidate, SourceOffset,
};
use philosophy_extractor_model_providers::{
    ClaimClassifierProvider, HeuristicClaimClassifierProvider,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

use super::artifacts::provider_metadata;
use super::config::{ClassificationBackendConfig, PipelineConfig};
use super::labels::claim_role_from_label;

pub fn claim_candidates_from_propositions(
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
                source_segments: candidate.source_segments.clone(),
                claim_kind: candidate.claim_kind,
                confidence: candidate.confidence,
                requires_review: true,
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct RoleStageOutput {
    pub roles: Vec<crate::model::RoleAssignment>,
    pub provider: String,
    pub provider_version: String,
    pub metadata: BTreeMap<String, Value>,
}

pub fn classify_roles(claims: &[ClaimCandidate], config: &PipelineConfig) -> RoleStageOutput {
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
