use crate::model::{ClaimKind, Metadata, PropositionCandidate, RelationCandidate, RelationKind};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use text_embeddings::{HashedTextEmbedder, TextEmbeddingConfig};
use text_retrieval::{HybridConfig, IngestionOptions, RetrievalIndex, SearchDocument, SearchQuery};

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

pub fn rerank_relations_with_text_retrieval(
    candidates: &[PropositionCandidate],
    relations: Vec<RelationCandidate>,
) -> Vec<RelationCandidate> {
    if candidates.len() < 2 || relations.is_empty() {
        return relations;
    }
    let Ok(embedder) = HashedTextEmbedder::new(
        TextEmbeddingConfig {
            dimensions: 128,
            use_idf: false,
        },
        Default::default(),
    ) else {
        return relations;
    };
    let mut index = RetrievalIndex::new(embedder);
    let documents = candidates
        .iter()
        .map(|candidate| {
            SearchDocument::new(candidate.id.clone(), candidate.canonical_text.clone())
        })
        .collect::<Vec<_>>();
    if index
        .ingest_documents(
            &documents,
            &IngestionOptions {
                chunk_tokens: 128,
                chunk_overlap_tokens: 0,
                store_raw_text: true,
            },
        )
        .is_err()
    {
        return relations;
    }

    let text_by_id = candidates
        .iter()
        .map(|candidate| (candidate.id.as_str(), candidate.canonical_text.as_str()))
        .collect::<BTreeMap<_, _>>();

    relations
        .into_iter()
        .map(|mut relation| {
            let Some(query_text) = text_by_id.get(relation.from_proposition_id.as_str()) else {
                return relation;
            };
            let query = SearchQuery::hybrid(
                *query_text,
                candidates.len().min(8),
                HybridConfig {
                    semantic_weight: 0.75,
                    lexical_weight: 0.25,
                    rerank_window: candidates.len().min(16).max(1),
                },
            );
            if let Ok(results) = index.search(&query)
                && let Some(result) = results
                    .iter()
                    .find(|result| result.document_id == relation.to_proposition_id)
            {
                let retrieval_confidence = f64::from(result.score.clamp(0.0, 1.0));
                relation.confidence = relation.confidence.max(retrieval_confidence);
                relation.metadata.insert(
                    "rankingBackend".to_string(),
                    Value::String("text-retrieval-hybrid".to_string()),
                );
                relation.metadata.insert(
                    "semanticScore".to_string(),
                    Value::from(f64::from(result.semantic_score)),
                );
                relation.metadata.insert(
                    "lexicalScore".to_string(),
                    Value::from(f64::from(result.lexical_score)),
                );
            }
            relation
        })
        .collect()
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
