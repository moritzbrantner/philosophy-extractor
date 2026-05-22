use philosophy_extractor_schema::{
    ClaimKind, ClaimRole, FormalEvaluation, FormalEvaluationStatus, FormalizationCandidate,
    Passage, PipelineDiagnostic, RelationKind, TermCandidate, TermType,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;
use text_embeddings::{HashedTextEmbedder, TextEmbeddingConfig};
use text_linguistics::{
    EntityRecognitionOptions, EntityType, LinguisticAnalysisOptions, NamedEntity, TextNlpConfig,
    TextNlpPipeline,
};
use thiserror::Error;

pub trait ModelProvider {
    fn name(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct StructuredPrompt {
    pub system: String,
    pub user: String,
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("{0}")]
    Message(String),
}

pub trait EmbeddingProvider {
    fn name(&self) -> &'static str;
    fn model(&self) -> &str;
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, ProviderError>;
}

pub trait TextGenerationProvider {
    fn name(&self) -> &'static str;
    fn model(&self) -> &str;
    fn generate_structured(
        &self,
        prompt: StructuredPrompt,
        schema: Value,
    ) -> Result<Value, ProviderError>;
}

pub trait NerProvider {
    fn name(&self) -> &'static str;
    fn model(&self) -> &str {
        "heuristic"
    }
    fn extract_entities(&self, passages: &[Passage]) -> Result<Vec<TermCandidate>, ProviderError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassificationResult {
    pub label: String,
    pub confidence: f64,
}

pub trait ClaimClassifierProvider {
    fn name(&self) -> &'static str;
    fn model(&self) -> &str;
    fn classify_claim_kind(&self, text: &str) -> Result<ClassificationResult, ProviderError>;
    fn classify_role(&self, text: &str) -> Result<ClassificationResult, ProviderError>;
    fn classify_relation(
        &self,
        left: &str,
        right: &str,
    ) -> Result<ClassificationResult, ProviderError>;
}

pub trait TruthEngineProvider {
    fn name(&self) -> &'static str;
    fn evaluate(
        &self,
        candidates: &[FormalizationCandidate],
    ) -> Result<Vec<FormalEvaluation>, ProviderError>;
}

#[derive(Debug, Clone)]
pub struct DeterministicEmbeddingProvider {
    model: String,
    dimensions: usize,
}

impl DeterministicEmbeddingProvider {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            dimensions: 16,
        }
    }
}

impl EmbeddingProvider for DeterministicEmbeddingProvider {
    fn name(&self) -> &'static str {
        "local-deterministic-embeddings"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        Ok(texts
            .iter()
            .map(|text| hash_embedding(text, self.dimensions))
            .collect())
    }
}

#[derive(Debug, Clone)]
pub struct RustPackagesEmbeddingProvider {
    model: String,
    dimensions: usize,
    embedder: HashedTextEmbedder,
}

impl RustPackagesEmbeddingProvider {
    pub fn new(model: impl Into<String>, dimensions: usize) -> Result<Self, ProviderError> {
        let dimensions = dimensions.max(1);
        let embedder = HashedTextEmbedder::new(
            TextEmbeddingConfig {
                dimensions,
                use_idf: false,
            },
            Default::default(),
        )
        .map_err(|error| ProviderError::Message(error.to_string()))?;
        Ok(Self {
            model: model.into(),
            dimensions,
            embedder,
        })
    }
}

impl EmbeddingProvider for RustPackagesEmbeddingProvider {
    fn name(&self) -> &'static str {
        "text-retrieval-feature-extraction"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        texts
            .iter()
            .map(|text| {
                self.embedder
                    .embed_text(text)
                    .map(|vector| vector.into_values())
                    .map_err(|error| ProviderError::Message(error.to_string()))
            })
            .map(|result| {
                result.map(|mut vector| {
                    vector.truncate(self.dimensions);
                    vector
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiResponsesProvider {
    model: String,
}

impl OpenAiResponsesProvider {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }
}

impl TextGenerationProvider for OpenAiResponsesProvider {
    fn name(&self) -> &'static str {
        "openai-responses"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn generate_structured(
        &self,
        _prompt: StructuredPrompt,
        _schema: Value,
    ) -> Result<Value, ProviderError> {
        Err(ProviderError::Message(
            "OpenAI Responses provider is configured as an integration boundary; offline MVP uses deterministic fallback stages".to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct LocalRuleNerProvider;

impl NerProvider for LocalRuleNerProvider {
    fn name(&self) -> &'static str {
        "local-rule-ner"
    }

    fn extract_entities(&self, _passages: &[Passage]) -> Result<Vec<TermCandidate>, ProviderError> {
        Ok(Vec::new())
    }
}

#[derive(Debug, Clone)]
pub struct TextLinguisticsNerProvider {
    model: String,
    model_bundle_dir: PathBuf,
    auto_download: bool,
}

impl TextLinguisticsNerProvider {
    pub fn new(
        model: impl Into<String>,
        model_bundle_dir: impl Into<PathBuf>,
        auto_download: bool,
    ) -> Self {
        Self {
            model: model.into(),
            model_bundle_dir: model_bundle_dir.into(),
            auto_download,
        }
    }
}

impl NerProvider for TextLinguisticsNerProvider {
    fn name(&self) -> &'static str {
        "text-linguistics-token-classification"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn extract_entities(&self, passages: &[Passage]) -> Result<Vec<TermCandidate>, ProviderError> {
        let mut options = LinguisticAnalysisOptions::heuristic();
        options.entity_recognition = EntityRecognitionOptions::local_model();
        options.entity_recognition.bundle_dir = self.model_bundle_dir.clone();
        options.entity_recognition.auto_download = self.auto_download;
        options.entity_recognition.download_progress = false;
        let pipeline = TextNlpPipeline::new(TextNlpConfig::from_options(options));

        let mut by_label = BTreeMap::<String, TermCandidate>::new();
        for passage in passages {
            let analysis = pipeline
                .analyze_text(&passage.text)
                .map_err(|error| ProviderError::Message(error.to_string()))?;
            for entity in analysis.entities {
                let term = named_entity_to_term_candidate(&entity, &passage.id);
                merge_term(&mut by_label, term);
            }
        }
        Ok(by_label.into_values().collect())
    }
}

pub fn named_entity_to_term_candidate(entity: &NamedEntity, passage_id: &str) -> TermCandidate {
    let normalized = if entity.normalized.trim().is_empty() {
        entity.mention.text.to_ascii_lowercase()
    } else {
        entity.normalized.to_ascii_lowercase()
    };
    TermCandidate {
        id: format!("term_{}", &digest_text(&normalized)[..16]),
        text: entity.mention.text.clone(),
        normalized_label: normalized,
        term_type: term_type_for_entity(entity.entity_type),
        source_passage_ids: vec![passage_id.to_string()],
        confidence: f64::from(entity.confidence),
        requires_review: true,
        model_added: true,
    }
}

fn term_type_for_entity(entity_type: EntityType) -> TermType {
    match entity_type {
        EntityType::Person => TermType::Person,
        EntityType::Work | EntityType::Law => TermType::Work,
        EntityType::Organization => TermType::School,
        EntityType::Misc | EntityType::Product | EntityType::Event => TermType::TechnicalTerm,
        EntityType::Location => TermType::Place,
        EntityType::Date | EntityType::Amount => TermType::Other,
    }
}

fn merge_term(by_label: &mut BTreeMap<String, TermCandidate>, term: TermCandidate) {
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
        })
        .or_insert(term);
}

#[derive(Debug, Clone)]
pub struct HeuristicClaimClassifierProvider {
    model: String,
}

impl HeuristicClaimClassifierProvider {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }
}

impl ClaimClassifierProvider for HeuristicClaimClassifierProvider {
    fn name(&self) -> &'static str {
        "local-heuristic-classifier"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn classify_claim_kind(&self, text: &str) -> Result<ClassificationResult, ProviderError> {
        let lower = text.to_ascii_lowercase();
        let label = if lower.contains(" if ") || lower.starts_with("if ") {
            ClaimKind::Conditional
        } else if lower.contains(" should ") || lower.contains(" ought ") {
            ClaimKind::Normative
        } else if lower.contains(" is ") || lower.contains(" are ") {
            ClaimKind::Atomic
        } else {
            ClaimKind::Unknown
        };
        Ok(ClassificationResult {
            label: claim_kind_label(label).to_string(),
            confidence: 0.56,
        })
    }

    fn classify_role(&self, text: &str) -> Result<ClassificationResult, ProviderError> {
        let lower = text.to_ascii_lowercase();
        let label = if lower.contains("because") || lower.contains("therefore") {
            ClaimRole::Conclusion
        } else if lower.contains("for example") {
            ClaimRole::Example
        } else {
            ClaimRole::Premise
        };
        Ok(ClassificationResult {
            label: claim_role_label(label).to_string(),
            confidence: 0.56,
        })
    }

    fn classify_relation(
        &self,
        left: &str,
        right: &str,
    ) -> Result<ClassificationResult, ProviderError> {
        let label = if left.contains(" not ") != right.contains(" not ") {
            RelationKind::Contradicts
        } else {
            RelationKind::Supports
        };
        Ok(ClassificationResult {
            label: relation_kind_label(label).to_string(),
            confidence: 0.52,
        })
    }
}

fn claim_kind_label(kind: ClaimKind) -> &'static str {
    match kind {
        ClaimKind::Atomic => "atomic",
        ClaimKind::Conditional => "conditional",
        ClaimKind::Definition => "definition",
        ClaimKind::Modal => "modal",
        ClaimKind::Normative => "normative",
        ClaimKind::Universal => "universal",
        ClaimKind::Unknown => "unknown",
    }
}

fn claim_role_label(role: ClaimRole) -> &'static str {
    match role {
        ClaimRole::MainClaim => "main_claim",
        ClaimRole::Premise => "premise",
        ClaimRole::Conclusion => "conclusion",
        ClaimRole::Definition => "definition",
        ClaimRole::Objection => "objection",
        ClaimRole::Reply => "reply",
        ClaimRole::Example => "example",
        ClaimRole::Background => "background",
        ClaimRole::Qualification => "qualification",
        ClaimRole::Unknown => "unknown",
    }
}

fn relation_kind_label(kind: RelationKind) -> &'static str {
    match kind {
        RelationKind::Supports => "supports",
        RelationKind::Contradicts => "contradicts",
        RelationKind::Interprets => "interprets",
        RelationKind::Specializes => "specializes",
        RelationKind::Generalizes => "generalizes",
        RelationKind::VariantOf => "variant_of",
    }
}

#[derive(Debug, Clone)]
pub struct LeanTruthEngineProvider {
    lean_bin: String,
}

impl LeanTruthEngineProvider {
    pub fn new(lean_bin: impl Into<String>) -> Self {
        Self {
            lean_bin: lean_bin.into(),
        }
    }
}

impl TruthEngineProvider for LeanTruthEngineProvider {
    fn name(&self) -> &'static str {
        "lean"
    }

    fn evaluate(
        &self,
        candidates: &[FormalizationCandidate],
    ) -> Result<Vec<FormalEvaluation>, ProviderError> {
        Ok(candidates
            .iter()
            .map(|candidate| FormalEvaluation {
                formalization_id: candidate.id.clone(),
                proposition_id: candidate.proposition_id.clone(),
                status: FormalEvaluationStatus::Unsupported,
                lean_source: Some(format!(
                    "-- {} integration boundary\n#check {}\n",
                    self.lean_bin, candidate.lean_expression
                )),
                diagnostics: vec![PipelineDiagnostic {
                    code: "lean_not_executed".to_string(),
                    severity: "info".to_string(),
                    message:
                        "Lean adapter emitted source but did not execute Lean in offline MVP mode."
                            .to_string(),
                    target_id: Some(candidate.id.clone()),
                }],
            })
            .collect())
    }
}

fn hash_embedding(text: &str, dimensions: usize) -> Vec<f32> {
    let mut vector = vec![0.0; dimensions];
    for token in text
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        let mut hasher = Sha256::new();
        hasher.update(token.to_ascii_lowercase().as_bytes());
        let digest = hasher.finalize();
        let index = usize::from(digest[0]) % dimensions;
        vector[index] += 1.0;
    }

    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value = (*value / norm * 1000.0).round() / 1000.0;
        }
    }
    vector
}

fn digest_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use text_core::TextSpan;
    use text_linguistics::{EntityMentionSpan, EntityType, NamedEntity};

    #[test]
    fn rust_packages_embedding_provider_returns_configured_dimensions() {
        let provider = RustPackagesEmbeddingProvider::new("mock-minilm", 8).unwrap();
        let vectors = provider
            .embed(&["Knowledge concerns truth.".to_string()])
            .unwrap();

        assert_eq!(provider.name(), "text-retrieval-feature-extraction");
        assert_eq!(provider.model(), "mock-minilm");
        assert_eq!(vectors.len(), 1);
        assert_eq!(vectors[0].len(), 8);
    }

    #[test]
    fn named_entity_maps_to_term_candidate() {
        let entity = NamedEntity {
            id: "ent-1".to_string(),
            entity_type: EntityType::Person,
            mention: EntityMentionSpan {
                text: "Socrates".to_string(),
                span: TextSpan {
                    byte_start: 0,
                    byte_end: 8,
                    char_start: 0,
                    char_end: 8,
                },
            },
            normalized: "Socrates".to_string(),
            sentence_index: 0,
            token_start: 0,
            token_end: 1,
            confidence: 0.91,
        };

        let term = named_entity_to_term_candidate(&entity, "frag-doc-1");

        assert_eq!(term.text, "Socrates");
        assert_eq!(term.normalized_label, "socrates");
        assert_eq!(term.term_type, TermType::Person);
        assert_eq!(term.source_passage_ids, vec!["frag-doc-1".to_string()]);
        assert!(term.model_added);
    }
}
