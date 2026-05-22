use crate::model::{AssertionStatus, ClaimKind, Metadata, PropositionCandidate, SourceFragment};
use serde_json::Value;

use super::{clamp_confidence, normalize_canonical_text, proposition_id_for};

pub fn extract_candidates(fragments: &[SourceFragment]) -> Vec<PropositionCandidate> {
    fragments
        .iter()
        .filter_map(candidate_for_fragment)
        .collect()
}

fn candidate_for_fragment(fragment: &SourceFragment) -> Option<PropositionCandidate> {
    let text = normalize_canonical_text(fragment.text.as_deref().unwrap_or_default());
    if text.is_empty()
        || word_count(&text) < 3
        || is_question(&text)
        || is_likely_instruction(&text)
    {
        return None;
    }

    let claim_kind = claim_kind(&text);
    let confidence = confidence_for(&text, claim_kind);
    let mut metadata = Metadata::new();
    metadata.insert("negated".to_string(), Value::Bool(has_negation(&text)));
    metadata.insert(
        "extractor".to_string(),
        Value::String("heuristic-rust-v1".to_string()),
    );
    if let Some(domain) = philosophical_domain(&text) {
        metadata.insert("domain".to_string(), Value::String(domain.to_string()));
    }

    Some(PropositionCandidate {
        id: proposition_id_for(&text),
        canonical_text: text,
        claim_kind,
        assertion_status: if confidence >= 0.55 {
            AssertionStatus::Accepted
        } else {
            AssertionStatus::Exploring
        },
        confidence,
        source_fragment_ids: vec![fragment.id.clone()],
        metadata,
    })
}

fn word_count(value: &str) -> usize {
    value
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .count()
}

fn is_question(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value.trim_end().ends_with('?')
        || [
            "who ", "what ", "when ", "where ", "why ", "how ", "does ", "do ", "did ", "is ",
            "are ",
        ]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

fn is_likely_instruction(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "please ",
        "write ",
        "list ",
        "explain ",
        "describe ",
        "compare ",
        "tell ",
        "show ",
        "give ",
        "make ",
        "create ",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
}

fn claim_kind(value: &str) -> ClaimKind {
    let lower = value.to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "if ",
            " if ",
            "when ",
            "whenever ",
            "provided that",
            "only if",
            "implies",
        ],
    ) {
        ClaimKind::Conditional
    } else if contains_any(
        &lower,
        &[
            " is defined as ",
            "means ",
            "consists in ",
            "is the same as ",
        ],
    ) {
        ClaimKind::Definition
    } else if contains_any(
        &lower,
        &[
            " should ",
            " must ",
            " ought ",
            "required",
            "forbidden",
            "permitted",
            "obligated",
            "obligation",
        ],
    ) {
        ClaimKind::Normative
    } else if contains_any(
        &lower,
        &[
            "necessarily",
            "possible",
            "possibly",
            "impossible",
            " can ",
            " could ",
            " may ",
            " might ",
            "contingent",
        ],
    ) {
        ClaimKind::Modal
    } else if contains_any(
        &lower,
        &[
            "all ", " every ", "each ", " each ", "any ", " any ", "none ", " none ", "no ", " no ",
        ],
    ) {
        ClaimKind::Universal
    } else {
        ClaimKind::Atomic
    }
}

fn confidence_for(value: &str, claim_kind: ClaimKind) -> f64 {
    let lower = value.to_ascii_lowercase();
    let mut confidence: f64 = 0.5;
    if value.ends_with('.') || value.ends_with('!') {
        confidence += 0.08;
    }
    if word_count(value) >= 5 {
        confidence += 0.06;
    }
    if claim_kind != ClaimKind::Unknown {
        confidence += 0.04;
    }
    if contains_any(
        &lower,
        &[
            " is ",
            " are ",
            " was ",
            " were ",
            " has ",
            " have ",
            " causes ",
            " implies ",
            " requires ",
            " supports ",
            " contradicts ",
        ],
    ) {
        confidence += 0.07;
    }
    if contains_any(&lower, &["maybe", "perhaps", "arguably", "unclear"]) {
        confidence -= 0.12;
    }
    clamp_confidence(confidence.min(0.85))
}

pub(crate) fn has_negation(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    contains_any(
        &lower,
        &[
            " no ",
            " not ",
            " never ",
            " none ",
            " cannot ",
            " can't ",
            " without ",
            " false ",
        ],
    ) || lower.starts_with("no ")
}

pub(crate) fn philosophical_domain(value: &str) -> Option<&'static str> {
    let lower = value.to_ascii_lowercase();
    if contains_any(
        &lower,
        &["being", "reality", "substance", "form", "essence", "exist"],
    ) {
        Some("metaphysics")
    } else if contains_any(
        &lower,
        &["knowledge", "truth", "belief", "justification", "reason"],
    ) {
        Some("epistemology")
    } else if contains_any(
        &lower,
        &["good", "justice", "virtue", "ought", "should", "duty"],
    ) {
        Some("ethics")
    } else if contains_any(&lower, &["state", "law", "rule", "political", "citizen"]) {
        Some("political_philosophy")
    } else if contains_any(
        &lower,
        &["valid", "implies", "contradiction", "necessary", "possible"],
    ) {
        Some("logic")
    } else {
        None
    }
}

pub(crate) fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}
