use crate::model::{Passage, PassageEmbedding};
use philosophy_extractor_model_providers::{
    DeterministicEmbeddingProvider, EmbeddingProvider, RustPackagesEmbeddingProvider,
};
use serde_json::Value;
use std::collections::BTreeMap;

use super::PipelineError;
use super::artifacts::provider_metadata;
use super::config::{EmbeddingBackendConfig, NlpMode, PipelineConfig};

#[derive(Debug, Clone)]
pub struct EmbeddingStageOutput {
    pub embeddings: Vec<PassageEmbedding>,
    pub provider: String,
    pub provider_version: String,
    pub metadata: BTreeMap<String, Value>,
}

pub fn embed_passages(
    passages: &[Passage],
    config: &PipelineConfig,
) -> Result<EmbeddingStageOutput, PipelineError> {
    let use_text_retrieval = config.embedding_backend == EmbeddingBackendConfig::TextRetrieval
        || config.nlp_mode == NlpMode::LocalModels;
    let provider: Box<dyn EmbeddingProvider> = if use_text_retrieval {
        Box::new(
            RustPackagesEmbeddingProvider::new(config.embedding_model.clone(), 128)
                .map_err(|error| PipelineError::Embedding(error.to_string()))?,
        )
    } else {
        Box::new(DeterministicEmbeddingProvider::new(
            config.embedding_model.clone(),
        ))
    };
    let texts = passages
        .iter()
        .map(|passage| passage.text.clone())
        .collect::<Vec<_>>();
    let vectors = provider
        .embed(&texts)
        .map_err(|error| PipelineError::Embedding(error.to_string()))?;
    let embeddings = passages
        .iter()
        .zip(vectors)
        .map(|(passage, vector)| PassageEmbedding {
            passage_id: passage.id.clone(),
            model: provider.model().to_string(),
            dimensions: vector.len(),
            vector,
        })
        .collect();
    Ok(EmbeddingStageOutput {
        embeddings,
        provider: provider.name().to_string(),
        provider_version: provider.model().to_string(),
        metadata: provider_metadata(
            provider.model(),
            if use_text_retrieval {
                "text-retrieval"
            } else {
                "deterministic"
            },
            &config.model_bundle_dir,
            config.auto_download_models,
            if use_text_retrieval {
                "feature-extraction"
            } else {
                "deterministic-feature-extraction"
            },
        ),
    })
}
