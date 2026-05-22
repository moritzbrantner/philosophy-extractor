pub use philosophy_extractor_schema as model;

pub mod pipeline;

pub use model::{
    ArtifactRef, ExtractionDocument, ExtractionResponse, PipelineDiagnostic, PipelineStage,
    SourceFragment, UnifiedWorldviewV10,
};
pub use pipeline::{PhilosophyExtractor, PipelineConfig};
