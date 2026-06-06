use crate::model::{
    CandidateIndexEntry, CandidateScoreBreakdown, ClaimKind, ClaimRole, Metadata, Passage,
    PipelineDiagnostic, PropositionCandidate, RelationCandidate, RelationKind, RoleAssignment,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::clamp_confidence;
use super::extract::{has_negation, philosophical_domain};

const VARIANT_LEXICAL_THRESHOLD: f64 = 0.72;

pub fn normalized_fingerprint(text: &str) -> String {
    content_tokens(text)
        .into_iter()
        .filter(|token| !FINGERPRINT_STOPWORDS.contains(&token.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn score_candidates(
    candidates: &[PropositionCandidate],
    passages: &[Passage],
    roles: &[RoleAssignment],
    relations: &[RelationCandidate],
) -> Vec<CandidateIndexEntry> {
    let passage_by_id = passages
        .iter()
        .map(|passage| (passage.id.as_str(), passage))
        .collect::<HashMap<_, _>>();
    let role_by_id = roles
        .iter()
        .map(|role| (role.claim_id.as_str(), role.primary_role))
        .collect::<HashMap<_, _>>();
    let relation_counts = relation_counts(relations);
    let variants_by_id = variants_by_id(relations);

    candidates
        .iter()
        .map(|candidate| {
            let source_passages = candidate
                .source_fragment_ids
                .iter()
                .filter_map(|id| passage_by_id.get(id.as_str()).copied())
                .collect::<Vec<_>>();
            let role = role_by_id
                .get(candidate.id.as_str())
                .copied()
                .unwrap_or(ClaimRole::Unknown);
            let philosophical_score = philosophical_score(candidate);
            let source_quality_score = source_quality_score(&source_passages);
            let specificity_score = specificity_score(&candidate.canonical_text);
            let role_score = role_score(role);
            let relation_score =
                relation_score(*relation_counts.get(candidate.id.as_str()).unwrap_or(&0));
            let mut reasons = score_reasons(
                candidate,
                role,
                philosophical_score,
                source_quality_score,
                specificity_score,
                relation_score,
            );
            let final_rank_score = clamp_confidence(
                0.30 * candidate.confidence
                    + 0.25 * philosophical_score
                    + 0.15 * specificity_score
                    + 0.10 * source_quality_score
                    + 0.10 * role_score
                    + 0.10 * relation_score,
            );
            reasons.push(format!("final_rank_score:{final_rank_score:.3}"));

            let first_passage = source_passages
                .iter()
                .min_by_key(|passage| passage.sequence)
                .copied();
            let duplicate_source_ids = metadata_string_array(candidate, "duplicateSourceIds");
            CandidateIndexEntry {
                proposition_id: candidate.id.clone(),
                canonical_text: candidate.canonical_text.clone(),
                claim_kind: candidate.claim_kind,
                assertion_status: candidate.assertion_status,
                confidence: candidate.confidence,
                score_breakdown: CandidateScoreBreakdown {
                    extraction_confidence: candidate.confidence,
                    philosophical_score,
                    source_quality_score,
                    specificity_score,
                    role_score,
                    relation_score,
                    final_rank_score,
                    reasons,
                },
                source_fragment_ids: candidate.source_fragment_ids.clone(),
                first_source_sequence: first_passage
                    .map(|passage| passage.sequence)
                    .unwrap_or(usize::MAX),
                first_paragraph_index: first_passage
                    .map(|passage| passage.paragraph_index)
                    .unwrap_or(usize::MAX),
                domain: candidate
                    .metadata
                    .get("domain")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| {
                        philosophical_domain(&candidate.canonical_text).map(str::to_string)
                    }),
                has_negation: has_negation(&candidate.canonical_text),
                normalized_fingerprint: normalized_fingerprint(&candidate.canonical_text),
                duplicate_source_ids,
                variant_of_ids: variants_by_id
                    .get(candidate.id.as_str())
                    .cloned()
                    .unwrap_or_default(),
            }
        })
        .collect()
}

pub fn rank_candidates(
    mut candidates: Vec<PropositionCandidate>,
    index: &[CandidateIndexEntry],
) -> Vec<PropositionCandidate> {
    let rank_by_id = index
        .iter()
        .map(|entry| (entry.proposition_id.as_str(), entry))
        .collect::<HashMap<_, _>>();

    candidates.sort_by(|left, right| {
        let left_entry = rank_by_id.get(left.id.as_str());
        let right_entry = rank_by_id.get(right.id.as_str());
        let left_rank = left_entry
            .map(|entry| entry.score_breakdown.final_rank_score)
            .unwrap_or(left.confidence);
        let right_rank = right_entry
            .map(|entry| entry.score_breakdown.final_rank_score)
            .unwrap_or(right.confidence);
        right_rank
            .total_cmp(&left_rank)
            .then_with(|| right.confidence.total_cmp(&left.confidence))
            .then_with(|| {
                left_entry
                    .map(|entry| entry.first_source_sequence)
                    .unwrap_or(usize::MAX)
                    .cmp(
                        &right_entry
                            .map(|entry| entry.first_source_sequence)
                            .unwrap_or(usize::MAX),
                    )
            })
            .then_with(|| left.id.cmp(&right.id))
    });

    candidates
}

pub fn conservative_deduplicate(
    candidates: Vec<PropositionCandidate>,
    diagnostics: &mut Vec<PipelineDiagnostic>,
) -> Vec<PropositionCandidate> {
    let mut merged = Vec::<PropositionCandidate>::new();
    for candidate in candidates {
        let duplicate_index = merged.iter().position(|existing| {
            existing.canonical_text == candidate.canonical_text
                || (normalized_fingerprint(&existing.canonical_text)
                    == normalized_fingerprint(&candidate.canonical_text)
                    && compatible_for_merge(existing, &candidate))
        });

        if let Some(index) = duplicate_index {
            let is_fingerprint_duplicate = merged[index].canonical_text != candidate.canonical_text;
            let duplicate_id = candidate.id.clone();
            merge_candidate(&mut merged[index], candidate);
            if is_fingerprint_duplicate {
                diagnostics.push(PipelineDiagnostic {
                    code: "fingerprint_duplicate_merged".to_string(),
                    severity: "info".to_string(),
                    message: format!(
                        "Candidate '{}' was merged into '{}' because their normalized fingerprints matched.",
                        duplicate_id, merged[index].id
                    ),
                    target_id: Some(merged[index].id.clone()),
                });
            }
            continue;
        }

        merged.push(candidate);
    }
    merged
}

pub fn infer_variant_relations(candidates: &[PropositionCandidate]) -> Vec<RelationCandidate> {
    let mut relations = Vec::new();
    for (left_index, left) in candidates.iter().enumerate() {
        for right in candidates.iter().skip(left_index + 1) {
            if !compatible_for_variant(left, right) {
                continue;
            }
            let overlap = lexical_overlap(&left.canonical_text, &right.canonical_text);
            if overlap < VARIANT_LEXICAL_THRESHOLD {
                continue;
            }
            relations.push(variant_relation(left, right, "lexical", overlap));
        }
    }
    relations
}

pub fn add_rank_metadata(candidates: &mut [PropositionCandidate], index: &[CandidateIndexEntry]) {
    let index_by_id = index
        .iter()
        .map(|entry| (entry.proposition_id.as_str(), entry))
        .collect::<HashMap<_, _>>();
    for candidate in candidates {
        let Some(entry) = index_by_id.get(candidate.id.as_str()) else {
            continue;
        };
        candidate.metadata.insert(
            "philosophicalScore".to_string(),
            Value::from(entry.score_breakdown.philosophical_score),
        );
        candidate.metadata.insert(
            "sourceQualityScore".to_string(),
            Value::from(entry.score_breakdown.source_quality_score),
        );
        candidate.metadata.insert(
            "specificityScore".to_string(),
            Value::from(entry.score_breakdown.specificity_score),
        );
        candidate.metadata.insert(
            "finalRankScore".to_string(),
            Value::from(entry.score_breakdown.final_rank_score),
        );
        candidate.metadata.insert(
            "scoreReasons".to_string(),
            Value::Array(
                entry
                    .score_breakdown
                    .reasons
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
        candidate.metadata.insert(
            "normalizedFingerprint".to_string(),
            Value::String(entry.normalized_fingerprint.clone()),
        );
        if !entry.duplicate_source_ids.is_empty() {
            candidate.metadata.insert(
                "duplicateSourceIds".to_string(),
                Value::Array(
                    entry
                        .duplicate_source_ids
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect(),
                ),
            );
        }
        if !entry.variant_of_ids.is_empty() {
            candidate.metadata.insert(
                "variantOfIds".to_string(),
                Value::Array(
                    entry
                        .variant_of_ids
                        .iter()
                        .cloned()
                        .map(Value::String)
                        .collect(),
                ),
            );
        }
    }
}

fn merge_candidate(existing: &mut PropositionCandidate, candidate: PropositionCandidate) {
    let mut duplicate_ids = metadata_string_array(existing, "duplicateSourceIds");
    duplicate_ids.push(candidate.id.clone());
    duplicate_ids.extend(metadata_string_array(&candidate, "duplicateSourceIds"));
    duplicate_ids.sort();
    duplicate_ids.dedup();

    for source_id in candidate.source_fragment_ids {
        if !existing.source_fragment_ids.contains(&source_id) {
            existing.source_fragment_ids.push(source_id);
        }
    }

    if candidate.confidence > existing.confidence {
        existing.confidence = candidate.confidence;
        existing.assertion_status = candidate.assertion_status;
    }

    existing.metadata.insert(
        "duplicateSourceIds".to_string(),
        Value::Array(duplicate_ids.into_iter().map(Value::String).collect()),
    );
}

fn compatible_for_merge(left: &PropositionCandidate, right: &PropositionCandidate) -> bool {
    left.claim_kind == right.claim_kind
        && has_negation(&left.canonical_text) == has_negation(&right.canonical_text)
}

fn compatible_for_variant(left: &PropositionCandidate, right: &PropositionCandidate) -> bool {
    left.id != right.id
        && has_negation(&left.canonical_text) == has_negation(&right.canonical_text)
        && compatible_claim_kind(left.claim_kind, right.claim_kind)
        && normalized_fingerprint(&left.canonical_text)
            != normalized_fingerprint(&right.canonical_text)
}

fn compatible_claim_kind(left: ClaimKind, right: ClaimKind) -> bool {
    left == right || left == ClaimKind::Atomic || right == ClaimKind::Atomic
}

fn philosophical_score(candidate: &PropositionCandidate) -> f64 {
    let mut score: f64 = 0.25;
    let lower = candidate.canonical_text.to_ascii_lowercase();
    if philosophical_domain(&candidate.canonical_text).is_some()
        || candidate
            .metadata
            .get("domain")
            .and_then(Value::as_str)
            .is_some()
    {
        score += 0.35;
    }
    match candidate.claim_kind {
        ClaimKind::Conditional
        | ClaimKind::Definition
        | ClaimKind::Modal
        | ClaimKind::Normative
        | ClaimKind::Universal => score += 0.25,
        ClaimKind::Atomic => score += 0.08,
        ClaimKind::Unknown => {}
    }
    if contains_any(
        &lower,
        &[
            "soul",
            "virtue",
            "truth",
            "knowledge",
            "justice",
            "good",
            "being",
            "essence",
            "reason",
            "belief",
            "freedom",
            "duty",
            "meaning",
        ],
    ) {
        score += 0.18;
    }
    if contains_any(
        &lower,
        &[
            "meeting",
            "chapter",
            "section",
            "page",
            "wrote",
            "said",
            "visited",
            "arrived",
            "published",
        ],
    ) {
        score -= 0.20;
    }
    clamp_confidence(score)
}

fn source_quality_score(passages: &[&Passage]) -> f64 {
    if passages.is_empty() {
        return 0.2;
    }
    let mut score: f64 = 0.55;
    if passages
        .iter()
        .any(|passage| passage.end_char > passage.start_char && !passage.text.trim().is_empty())
    {
        score += 0.25;
    }
    if passages.iter().any(|passage| {
        let text = passage.text.trim();
        word_count(text) >= 4 && text.chars().any(char::is_alphabetic) && !is_heading_like(text)
    }) {
        score += 0.20;
    }
    clamp_confidence(score)
}

fn specificity_score(text: &str) -> f64 {
    let words = word_count(text);
    let lower = text.to_ascii_lowercase();
    let mut score: f64 = if (5..=35).contains(&words) {
        0.65
    } else if (3..=50).contains(&words) {
        0.45
    } else {
        0.25
    };
    if contains_any(
        &lower,
        &[
            " is ",
            " are ",
            " implies ",
            " requires ",
            " grounds ",
            " causes ",
            " constitutes ",
            " consists in ",
            " depends on ",
            " entails ",
        ],
    ) {
        score += 0.25;
    }
    clamp_confidence(score)
}

fn role_score(role: ClaimRole) -> f64 {
    match role {
        ClaimRole::MainClaim => 1.0,
        ClaimRole::Conclusion => 0.9,
        ClaimRole::Definition => 0.85,
        ClaimRole::Premise | ClaimRole::Objection | ClaimRole::Reply => 0.7,
        ClaimRole::Qualification => 0.55,
        ClaimRole::Background | ClaimRole::Example => 0.35,
        ClaimRole::Unknown => 0.25,
    }
}

fn relation_score(count: usize) -> f64 {
    (count as f64 * 0.25).min(1.0)
}

fn relation_counts(relations: &[RelationCandidate]) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for relation in relations {
        *counts
            .entry(relation.from_proposition_id.as_str())
            .or_insert(0) += 1;
        *counts
            .entry(relation.to_proposition_id.as_str())
            .or_insert(0) += 1;
    }
    counts
}

fn variants_by_id(relations: &[RelationCandidate]) -> BTreeMap<&str, Vec<String>> {
    let mut variants = BTreeMap::<&str, Vec<String>>::new();
    for relation in relations
        .iter()
        .filter(|relation| relation.kind == RelationKind::VariantOf)
    {
        variants
            .entry(relation.from_proposition_id.as_str())
            .or_default()
            .push(relation.to_proposition_id.clone());
        variants
            .entry(relation.to_proposition_id.as_str())
            .or_default()
            .push(relation.from_proposition_id.clone());
    }
    variants
}

fn score_reasons(
    candidate: &PropositionCandidate,
    role: ClaimRole,
    philosophical_score: f64,
    source_quality_score: f64,
    specificity_score: f64,
    relation_score: f64,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if philosophical_score >= 0.65 {
        reasons.push("candidate_ranked_higher_due_to_philosophical_signal".to_string());
    } else if philosophical_score <= 0.35 {
        reasons.push("candidate_ranked_lower_due_to_low_philosophical_score".to_string());
    }
    if matches!(
        role,
        ClaimRole::MainClaim | ClaimRole::Conclusion | ClaimRole::Definition
    ) {
        reasons.push("candidate_ranked_higher_due_to_argument_role".to_string());
    }
    if relation_score >= 0.5 {
        reasons.push("candidate_ranked_higher_due_to_relation_centrality".to_string());
    }
    if source_quality_score >= 0.8 {
        reasons.push("candidate_has_source_offsets".to_string());
    }
    if specificity_score >= 0.8 {
        reasons.push("candidate_has_clear_predicate".to_string());
    }
    if has_negation(&candidate.canonical_text) {
        reasons.push("candidate_has_negation".to_string());
    }
    reasons
}

fn variant_relation(
    from: &PropositionCandidate,
    to: &PropositionCandidate,
    similarity_kind: &str,
    similarity_score: f64,
) -> RelationCandidate {
    let mut metadata = Metadata::new();
    metadata.insert(
        "generatedBy".to_string(),
        Value::String("candidate-quality-ranker".to_string()),
    );
    metadata.insert("reviewRequired".to_string(), Value::Bool(true));
    metadata.insert(
        "similarityKind".to_string(),
        Value::String(similarity_kind.to_string()),
    );
    metadata.insert("similarityScore".to_string(), Value::from(similarity_score));

    RelationCandidate {
        from_proposition_id: from.id.clone(),
        to_proposition_id: to.id.clone(),
        kind: RelationKind::VariantOf,
        confidence: similarity_score,
        note: Some("Similar source-grounded claims are reviewable variants.".to_string()),
        metadata,
    }
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
        .filter(|token| token.len() > 2 && !CONTENT_STOPWORDS.contains(&token.as_str()))
        .collect()
}

fn metadata_string_array(candidate: &PropositionCandidate, key: &str) -> Vec<String> {
    candidate
        .metadata
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn word_count(value: &str) -> usize {
    value
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .count()
}

fn is_heading_like(value: &str) -> bool {
    let trimmed = value.trim_matches(|ch: char| !ch.is_alphanumeric());
    !trimmed.is_empty()
        && trimmed.len() <= 48
        && !trimmed.contains('.')
        && trimmed
            .chars()
            .filter(|ch| ch.is_alphabetic())
            .all(|ch| ch.is_uppercase())
}

const CONTENT_STOPWORDS: &[&str] = &[
    "the", "and", "for", "that", "this", "with", "from", "into", "than", "then", "are", "was",
    "were", "has", "have", "had", "not", "all", "any", "each", "every", "its", "their", "his",
    "her", "one", "two", "who", "what", "when", "where", "why", "how",
];

const FINGERPRINT_STOPWORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "that", "this", "these", "those", "of", "to", "for",
    "in", "on", "at", "by", "with", "from", "as",
];
