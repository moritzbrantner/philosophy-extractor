use crate::model::{
    Assertion, Facet, Metadata, Proposition, PropositionCandidate, Relation, RelationCandidate,
    SourceDocument, SourceFragment, UnifiedWorldviewV10,
};
use serde_json::Value;

use super::{
    PipelineConfig, digest_text, extract::philosophical_domain, relation_id_for,
    source_document_id_for,
};
use crate::model::ExtractionDocument;

pub fn build_worldview(
    document: &ExtractionDocument,
    fragments: &[SourceFragment],
    candidates: &[PropositionCandidate],
    relations: &[RelationCandidate],
    config: &PipelineConfig,
) -> UnifiedWorldviewV10 {
    let worldview_id = config
        .worldview_id
        .as_ref()
        .filter(|id| !id.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| format!("draft-worldview-{}", &digest_text(&document.text)[..16]));

    UnifiedWorldviewV10 {
        schema_version: "10".to_string(),
        id: worldview_id,
        fragment: "unified_worldview_v2".to_string(),
        label: config
            .label
            .clone()
            .or_else(|| document.title.clone())
            .or_else(|| Some("Draft philosophical worldview".to_string())),
        propositions: candidates.iter().map(proposition_for).collect(),
        relations: relations.iter().map(relation_for).collect(),
        source_documents: vec![source_document_for(document)],
        source_fragments: fragments.to_vec(),
        metadata: worldview_metadata(),
    }
}

fn source_document_for(document: &ExtractionDocument) -> SourceDocument {
    SourceDocument {
        id: source_document_id_for(document),
        kind: document.kind.clone(),
        title: document.title.clone(),
        authors: document.authors.clone(),
        language: document.language.clone(),
        uri: document.uri.clone(),
    }
}

fn proposition_for(candidate: &PropositionCandidate) -> Proposition {
    let mut metadata = candidate.metadata.clone();
    metadata.insert(
        "claimKind".to_string(),
        candidate.claim_kind.as_metadata_value(),
    );
    metadata.insert(
        "generatedBy".to_string(),
        Value::String("philosophy-extractor".to_string()),
    );
    metadata.insert("reviewRequired".to_string(), Value::Bool(true));
    metadata.insert(
        "provider".to_string(),
        Value::String("philosophy-extractor".to_string()),
    );
    metadata.insert("confidence".to_string(), Value::from(candidate.confidence));

    Proposition {
        id: candidate.id.clone(),
        canonical_text: Some(candidate.canonical_text.clone()),
        assertions: vec![Assertion {
            id: format!("assertion_{}", candidate.id),
            status: Some(candidate.assertion_status.as_str().to_string()),
            provenance: Some("generated".to_string()),
            confidence: Some(candidate.confidence),
            source_fragment_ids: candidate.source_fragment_ids.clone(),
        }],
        source_fragment_ids: candidate.source_fragment_ids.clone(),
        facets: facets_for(candidate),
        metadata,
    }
}

fn facets_for(candidate: &PropositionCandidate) -> Vec<Facet> {
    let mut facets = Vec::new();
    facets.push(Facet {
        family: "claim_kind".to_string(),
        value: format!("{:?}", candidate.claim_kind).to_ascii_lowercase(),
        provenance: Some("heuristic".to_string()),
        confidence: Some(candidate.confidence),
    });

    if let Some(domain) = candidate
        .metadata
        .get("domain")
        .and_then(Value::as_str)
        .or_else(|| philosophical_domain(&candidate.canonical_text))
    {
        facets.push(Facet {
            family: "philosophical_domain".to_string(),
            value: domain.to_string(),
            provenance: Some("heuristic".to_string()),
            confidence: Some(candidate.confidence),
        });
    }

    facets
}

fn relation_for(candidate: &RelationCandidate) -> Relation {
    Relation {
        id: relation_id_for(candidate),
        from_proposition_id: candidate.from_proposition_id.clone(),
        to_proposition_id: candidate.to_proposition_id.clone(),
        kind: candidate.kind.as_str().to_string(),
        note: candidate.note.clone(),
    }
}

fn worldview_metadata() -> Metadata {
    let mut metadata = Metadata::new();
    metadata.insert(
        "generatedBy".to_string(),
        Value::String("philosophy-extractor".to_string()),
    );
    metadata.insert("reviewRequired".to_string(), Value::Bool(true));
    metadata.insert(
        "provider".to_string(),
        Value::String("philosophy-extractor".to_string()),
    );
    metadata.insert(
        "source".to_string(),
        Value::String("text_extraction".to_string()),
    );
    metadata.insert(
        "targetContract".to_string(),
        Value::String("truth-engine unified worldview v10".to_string()),
    );
    metadata
}
