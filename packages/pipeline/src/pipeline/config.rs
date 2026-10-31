use crate::model::PipelineRunConfig;
use crate::model::PipelineStage;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub worldview_id: Option<String>,
    pub label: Option<String>,
    pub min_confidence: f64,
    pub max_propositions: Option<usize>,
    pub include_relations: bool,
    pub artifact_dir: PathBuf,
    pub persist_artifacts: bool,
    pub openai_model_primary: String,
    pub openai_model_cheap: String,
    pub embedding_model: String,
    pub ner_model: String,
    pub nlp_mode: NlpMode,
    pub model_bundle_dir: PathBuf,
    pub auto_download_models: bool,
    pub embedding_backend: EmbeddingBackendConfig,
    pub term_extraction_backend: TermExtractionBackendConfig,
    pub classification_backend: ClassificationBackendConfig,
    pub lean_bin: String,
    pub stage_through: Option<PipelineStage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NlpMode {
    Heuristic,
    LocalModels,
}

impl NlpMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Heuristic => "heuristic",
            Self::LocalModels => "local-models",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingBackendConfig {
    Deterministic,
    TextRetrieval,
}

impl EmbeddingBackendConfig {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic",
            Self::TextRetrieval => "text-retrieval",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermExtractionBackendConfig {
    Heuristic,
    TextLinguistics,
}

impl TermExtractionBackendConfig {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Heuristic => "heuristic",
            Self::TextLinguistics => "text-linguistics",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationBackendConfig {
    Heuristic,
    TextLinguistics,
}

impl ClassificationBackendConfig {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Heuristic => "heuristic",
            Self::TextLinguistics => "text-linguistics",
        }
    }
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            worldview_id: None,
            label: None,
            min_confidence: 0.35,
            max_propositions: None,
            include_relations: true,
            artifact_dir: std::env::var("PHILOSOPHY_EXTRACTOR_ARTIFACT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(".artifacts")),
            persist_artifacts: true,
            openai_model_primary: std::env::var("PHILOSOPHY_EXTRACTOR_OPENAI_MODEL_PRIMARY")
                .unwrap_or_else(|_| "gpt-5.5".to_string()),
            openai_model_cheap: std::env::var("PHILOSOPHY_EXTRACTOR_OPENAI_MODEL_CHEAP")
                .unwrap_or_else(|_| "gpt-5.5-mini".to_string()),
            embedding_model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            ner_model: "rust-bert-default-ner".to_string(),
            nlp_mode: NlpMode::Heuristic,
            model_bundle_dir: PathBuf::from(".video-analysis-models"),
            auto_download_models: false,
            embedding_backend: EmbeddingBackendConfig::Deterministic,
            term_extraction_backend: TermExtractionBackendConfig::Heuristic,
            classification_backend: ClassificationBackendConfig::Heuristic,
            lean_bin: std::env::var("PHILOSOPHY_EXTRACTOR_LEAN_BIN")
                .unwrap_or_else(|_| "lean".to_string()),
            stage_through: None,
        }
    }
}

pub fn pipeline_run_config(config: &PipelineConfig) -> PipelineRunConfig {
    PipelineRunConfig {
        artifact_dir: config.artifact_dir.clone(),
        openai_model_primary: config.openai_model_primary.clone(),
        openai_model_cheap: config.openai_model_cheap.clone(),
        embedding_model: config.embedding_model.clone(),
        ner_model: config.ner_model.clone(),
        nlp_mode: config.nlp_mode.as_str().to_string(),
        model_bundle_dir: config.model_bundle_dir.clone(),
        auto_download_models: config.auto_download_models,
        embedding_backend: config.embedding_backend.as_str().to_string(),
        term_extraction_backend: config.term_extraction_backend.as_str().to_string(),
        classification_backend: config.classification_backend.as_str().to_string(),
        lean_bin: config.lean_bin.clone(),
        stage_through: config.stage_through,
    }
}
