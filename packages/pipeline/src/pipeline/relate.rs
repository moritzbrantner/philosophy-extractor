use crate::model::{ClaimKind, Metadata, PropositionCandidate, RelationCandidate, RelationKind};
use serde_json::Value;
use std::collections::BTreeSet;

pub fn infer_relations(candidates: &[PropositionCandidate]) -> Vec<RelationCandidate> {
    let mut relations = Vec::new();

    for (left_index, left) in candidates.iter().enumerate() {
        for right in candidates.iter().skip(left_index + 1) {
            let overlap = lexical_overlap(&left.canonical_text, &right.canonical_text);
            if overlap < 0.28 {
                continue;
            }

            let left_negated = metadata_bool(left, "negated");
            let right_negated = metadata_bool(right, "negated");
            if left_negated != right_negated && overlap >= 0.34 {
                relations.push(relation(
                    left,
                    right,
                    RelationKind::Contradicts,
                    0.72,
                    "Shared vocabulary with opposite polarity.",
                ));
            } else if left.claim_kind == ClaimKind::Universal
                && right.claim_kind != ClaimKind::Universal
            {
                relations.push(relation(
                    left,
                    right,
                    RelationKind::Generalizes,
                    0.68,
                    "Universal claim appears to generalize a narrower claim.",
                ));
            } else if right.claim_kind == ClaimKind::Universal
                && left.claim_kind != ClaimKind::Universal
            {
                relations.push(relation(
                    right,
                    left,
                    RelationKind::Generalizes,
                    0.68,
                    "Universal claim appears to generalize a narrower claim.",
                ));
            } else if left.claim_kind == ClaimKind::Conditional
                || right.claim_kind == ClaimKind::Conditional
            {
                relations.push(relation(
                    left,
                    right,
                    RelationKind::Supports,
                    0.66,
                    "Conditional and nearby vocabulary suggest a support relation.",
                ));
            }
        }
    }

    relations.sort_by(|left, right| {
        left.from_proposition_id
            .cmp(&right.from_proposition_id)
            .then_with(|| left.to_proposition_id.cmp(&right.to_proposition_id))
            .then_with(|| left.kind.as_str().cmp(right.kind.as_str()))
    });
    relations
}

fn relation(
    from: &PropositionCandidate,
    to: &PropositionCandidate,
    kind: RelationKind,
    confidence: f64,
    note: &str,
) -> RelationCandidate {
    let mut metadata = Metadata::new();
    metadata.insert(
        "generatedBy".to_string(),
        Value::String("philosophy-extractor".to_string()),
    );
    metadata.insert("reviewRequired".to_string(), Value::Bool(true));
    metadata.insert("confidence".to_string(), Value::from(confidence));

    RelationCandidate {
        from_proposition_id: from.id.clone(),
        to_proposition_id: to.id.clone(),
        kind,
        confidence,
        note: Some(note.to_string()),
        metadata,
    }
}

fn metadata_bool(candidate: &PropositionCandidate, key: &str) -> bool {
    candidate
        .metadata
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn lexical_overlap(left: &str, right: &str) -> f64 {
    let left_tokens = content_tokens(left);
    let right_tokens = content_tokens(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }

    let intersection = left_tokens.intersection(&right_tokens).count() as f64;
    let union = left_tokens.union(&right_tokens).count() as f64;
    intersection / union
}

fn content_tokens(value: &str) -> BTreeSet<String> {
    value
        .split(|ch: char| !ch.is_alphanumeric())
        .map(str::to_ascii_lowercase)
        .filter(|token| token.len() > 2 && !STOPWORDS.contains(&token.as_str()))
        .collect()
}

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "that", "this", "with", "from", "into", "than", "then", "are", "was",
    "were", "has", "have", "had", "not", "all", "any", "each", "every", "its", "their", "his",
    "her", "one", "two",
];
