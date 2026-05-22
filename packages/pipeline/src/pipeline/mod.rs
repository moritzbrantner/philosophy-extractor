mod extract;
mod ingest;
mod normalize;
mod relate;
mod segment;
mod worldview;

use crate::model::{
    ExtractionDocument, ExtractionResponse, PipelineDiagnostic, PropositionCandidate,
    RelationCandidate, SourceFragment, StageSummary,
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub worldview_id: Option<String>,
    pub label: Option<String>,
    pub min_confidence: f64,
    pub max_propositions: Option<usize>,
    pub include_relations: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            worldview_id: None,
            label: None,
            min_confidence: 0.35,
            max_propositions: None,
            include_relations: true,
        }
    }
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("input text is empty")]
    EmptyInput,
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

        document = ingest::ingest(document)?;
        stages.push(StageSummary {
            name: "ingest".to_string(),
            output_count: 1,
            diagnostics: Vec::new(),
        });

        let fragments = segment::segment(&document);
        stages.push(StageSummary {
            name: "segment".to_string(),
            output_count: fragments.len(),
            diagnostics: empty_fragments_diagnostic(&fragments),
        });

        let raw_candidates = extract::extract_candidates(&fragments);
        stages.push(StageSummary {
            name: "extract".to_string(),
            output_count: raw_candidates.len(),
            diagnostics: empty_candidates_diagnostic(&raw_candidates),
        });

        let candidates = normalize::normalize_candidates(
            &fragments,
            raw_candidates,
            &self.config,
            &mut diagnostics,
        );
        stages.push(StageSummary {
            name: "normalize".to_string(),
            output_count: candidates.len(),
            diagnostics: diagnostics.clone(),
        });

        let relations = if self.config.include_relations {
            relate::infer_relations(&candidates)
        } else {
            Vec::new()
        };
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
        stages.push(StageSummary {
            name: "worldview".to_string(),
            output_count: worldview.propositions.len(),
            diagnostics: Vec::new(),
        });

        Ok(ExtractionResponse {
            provider: "philosophy-extractor".to_string(),
            worldview,
            candidates,
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

fn empty_candidates_diagnostic(candidates: &[PropositionCandidate]) -> Vec<PipelineDiagnostic> {
    if candidates.is_empty() {
        vec![PipelineDiagnostic {
            code: "no_candidates".to_string(),
            severity: "warning".to_string(),
            message: "No declarative proposition candidates were found.".to_string(),
            target_id: None,
        }]
    } else {
        Vec::new()
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
