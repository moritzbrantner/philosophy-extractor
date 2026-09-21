pub use philosophy_extractor_schema as model;

pub mod pipeline;

pub use model::{
    ArtifactRef, ExtractionDocument, ExtractionResponse, MediaEvidenceBatchV1,
    PhilosophyCorpusInputV1, PipelineDiagnostic, PipelineStage, SourceFragment,
    SourceSpanExtractionResponse, UnifiedWorldviewV10,
};
pub use pipeline::{
    ClassificationBackendConfig, EmbeddingBackendConfig, NlpMode, PhilosophyExtractor,
    PipelineConfig, TermExtractionBackendConfig,
};
