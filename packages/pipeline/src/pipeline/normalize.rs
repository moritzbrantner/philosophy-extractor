use crate::model::{PipelineDiagnostic, PropositionCandidate, SourceFragment};
use std::collections::HashSet;

use super::{PipelineConfig, normalize_canonical_text, proposition_id_for};

pub fn normalize_candidates(
    fragments: &[SourceFragment],
    candidates: Vec<PropositionCandidate>,
    config: &PipelineConfig,
    diagnostics: &mut Vec<PipelineDiagnostic>,
) -> Vec<PropositionCandidate> {
    let known_fragments = fragments
        .iter()
        .map(|fragment| fragment.id.as_str())
        .collect::<HashSet<_>>();
    let mut normalized = Vec::new();

    for mut candidate in candidates {
        candidate.canonical_text = normalize_canonical_text(&candidate.canonical_text);
        if candidate.canonical_text.is_empty() {
            diagnostics.push(PipelineDiagnostic {
                code: "empty_candidate".to_string(),
                severity: "warning".to_string(),
                message: "A candidate had no usable canonical text.".to_string(),
                target_id: None,
            });
            continue;
        }

        candidate.id = proposition_id_for(&candidate.canonical_text);
        candidate.confidence = super::clamp_confidence(candidate.confidence);
        candidate
            .source_fragment_ids
            .retain(|id| known_fragments.contains(id.as_str()));
        if candidate.source_fragment_ids.is_empty()
            && let Some(first) = fragments.first()
        {
            candidate.source_fragment_ids.push(first.id.clone());
        }

        if candidate.confidence < config.min_confidence {
            diagnostics.push(PipelineDiagnostic {
                code: "low_confidence_candidate".to_string(),
                severity: "info".to_string(),
                message: format!(
                    "Candidate '{}' was below the configured confidence threshold.",
                    candidate.id
                ),
                target_id: Some(candidate.id),
            });
            continue;
        }

        normalized.push(candidate);
    }

    let mut normalized = super::rank::conservative_deduplicate(normalized, diagnostics);
    normalized.sort_by(|left, right| {
        left.source_fragment_ids
            .first()
            .cmp(&right.source_fragment_ids.first())
            .then_with(|| left.id.cmp(&right.id))
    });

    normalized
}
