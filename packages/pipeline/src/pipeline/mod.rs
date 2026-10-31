mod arguments;
mod artifacts;
mod canonical;
mod claims;
mod cluster;
mod config;
mod corpus_input;
mod embed;
mod extract;
mod extractor;
mod formalize;
mod ingest;
mod labels;
mod normalize;
mod rank;
mod relate;
mod segment;
mod terms;
mod worldview;

use philosophy_extractor_artifact_store::ArtifactStoreError;
use thiserror::Error;

pub(crate) use canonical::{
    clamp_confidence, digest_text, normalize_canonical_text, proposition_id_for, relation_id_for,
    source_document_id_for,
};
pub use config::{
    ClassificationBackendConfig, EmbeddingBackendConfig, NlpMode, PipelineConfig,
    TermExtractionBackendConfig,
};
pub use extractor::PhilosophyExtractor;
pub use labels::{claim_kind_from_label, claim_role_from_label, relation_kind_from_label};

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("input text is empty")]
    EmptyInput,
    #[error("artifact store error: {0}")]
    Artifact(#[from] ArtifactStoreError),
    #[error("embedding provider error: {0}")]
    Embedding(String),
    #[error("invalid corpus input: {0}")]
    InvalidCorpusInput(String),
    #[error("corpus serialization error: {0}")]
    CorpusSerialization(String),
}

#[cfg(test)]
mod corpus_input_tests;
