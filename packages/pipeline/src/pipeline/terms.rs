use crate::model::{Passage, PipelineDiagnostic, TermCandidate, TermType};
use philosophy_extractor_model_providers::{NerProvider, TextLinguisticsNerProvider};
use serde_json::Value;
use std::collections::BTreeMap;

use super::artifacts::provider_metadata;
use super::config::{NlpMode, PipelineConfig, TermExtractionBackendConfig};
use super::digest_text;

#[derive(Debug, Clone)]
pub struct TermStageOutput {
    pub terms: Vec<TermCandidate>,
    pub provider: String,
    pub provider_version: String,
    pub metadata: BTreeMap<String, Value>,
    pub diagnostics: Vec<PipelineDiagnostic>,
}

pub fn extract_terms(passages: &[Passage], config: &PipelineConfig) -> TermStageOutput {
    let mut diagnostics = Vec::new();
    let heuristic_terms = extract_terms_heuristic(passages);
    let use_text_linguistics = config.term_extraction_backend
        == TermExtractionBackendConfig::TextLinguistics
        || config.nlp_mode == NlpMode::LocalModels;
    if !use_text_linguistics {
        return TermStageOutput {
            terms: heuristic_terms,
            provider: "local-heuristic-terms".to_string(),
            provider_version: "heuristic-concept-v1".to_string(),
            metadata: provider_metadata(
                "heuristic-concept-v1",
                "heuristic",
                &config.model_bundle_dir,
                config.auto_download_models,
                "token-classification",
            ),
            diagnostics,
        };
    }

    let provider = TextLinguisticsNerProvider::new(
        config.ner_model.clone(),
        config.model_bundle_dir.clone(),
        config.auto_download_models,
    );
    let model_terms = match provider.extract_entities(passages) {
        Ok(terms) => terms,
        Err(error) => {
            diagnostics.push(PipelineDiagnostic {
                code: "ner_provider_unavailable".to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "text-linguistics NER provider was unavailable; heuristic terms were used: {error}"
                ),
                target_id: None,
            });
            Vec::new()
        }
    };
    TermStageOutput {
        terms: merge_terms(heuristic_terms, model_terms),
        provider: provider.name().to_string(),
        provider_version: provider.model().to_string(),
        metadata: provider_metadata(
            provider.model(),
            "text-linguistics",
            &config.model_bundle_dir,
            config.auto_download_models,
            "token-classification",
        ),
        diagnostics,
    }
}

fn extract_terms_heuristic(passages: &[Passage]) -> Vec<TermCandidate> {
    let lexicon = [
        "knowledge",
        "truth",
        "justice",
        "virtue",
        "soul",
        "being",
        "form",
        "reason",
        "substance",
        "essence",
    ];
    let mut by_label: BTreeMap<String, TermCandidate> = BTreeMap::new();
    for passage in passages {
        for token in passage
            .text
            .split(|ch: char| !ch.is_alphanumeric() && ch != '\'')
            .filter(|token| token.len() > 2)
        {
            let normalized = token
                .trim_matches(|ch: char| !ch.is_alphanumeric())
                .to_ascii_lowercase();
            if normalized.is_empty() {
                continue;
            }
            let is_capitalized = token.chars().next().is_some_and(char::is_uppercase);
            let is_lexicon = lexicon.contains(&normalized.as_str());
            if !is_capitalized && !is_lexicon {
                continue;
            }
            by_label
                .entry(normalized.clone())
                .and_modify(|term| {
                    if !term.source_passage_ids.contains(&passage.id) {
                        term.source_passage_ids.push(passage.id.clone());
                    }
                })
                .or_insert_with(|| TermCandidate {
                    id: format!("term_{}", &digest_text(&normalized)[..16]),
                    text: token.to_string(),
                    normalized_label: normalized,
                    term_type: if is_lexicon {
                        TermType::Concept
                    } else {
                        TermType::Other
                    },
                    source_passage_ids: vec![passage.id.clone()],
                    confidence: if is_lexicon { 0.78 } else { 0.52 },
                    requires_review: !is_lexicon,
                    model_added: false,
                });
        }
    }
    by_label.into_values().collect()
}

fn merge_terms(
    heuristic_terms: Vec<TermCandidate>,
    model_terms: Vec<TermCandidate>,
) -> Vec<TermCandidate> {
    let mut by_label = BTreeMap::<String, TermCandidate>::new();
    for term in heuristic_terms.into_iter().chain(model_terms) {
        by_label
            .entry(term.normalized_label.clone())
            .and_modify(|existing| {
                for passage_id in &term.source_passage_ids {
                    if !existing.source_passage_ids.contains(passage_id) {
                        existing.source_passage_ids.push(passage_id.clone());
                    }
                }
                existing.confidence = existing.confidence.max(term.confidence);
                existing.model_added |= term.model_added;
                if existing.term_type == TermType::Other && term.term_type != TermType::Other {
                    existing.term_type = term.term_type;
                }
            })
            .or_insert(term);
    }
    by_label.into_values().collect()
}
