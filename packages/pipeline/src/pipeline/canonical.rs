use crate::model::{ExtractionDocument, RelationCandidate};

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
