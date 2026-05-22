pub use philosophy_extractor_schema as model;

pub mod pipeline;

pub use model::{
    ExtractionDocument, ExtractionResponse, PipelineDiagnostic, SourceFragment, UnifiedWorldviewV10,
};
pub use pipeline::{PhilosophyExtractor, PipelineConfig};
