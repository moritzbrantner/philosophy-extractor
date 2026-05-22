use philosophy_extractor_schema::{
    FormalEvaluation, FormalEvaluationStatus, FormalizationCandidate, Passage, PipelineDiagnostic,
    TermCandidate,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
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
    fn extract_entities(&self, passages: &[Passage]) -> Result<Vec<TermCandidate>, ProviderError>;
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
