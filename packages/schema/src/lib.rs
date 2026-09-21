use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub type Metadata = BTreeMap<String, Value>;

pub const SOURCE_SPAN_INTERCHANGE_SCHEMA: &str = "source_span_interchange";
pub const SOURCE_SPAN_INTERCHANGE_VERSION_V1: u32 = 1;


pub const MEDIA_EVIDENCE_SCHEMA: &str = "media_evidence";
pub const MEDIA_EVIDENCE_VERSION_V1: u32 = 1;
pub const PHILOSOPHY_CORPUS_INPUT_SCHEMA: &str = "philosophy_corpus_input";
pub const PHILOSOPHY_CORPUS_INPUT_VERSION_V1: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PhilosophyCorpusInputV1 {
    pub schema: String,
    pub schema_version: u32,
    pub sources: SourceSpanBatchV1,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub media_evidence: Vec<MediaEvidenceBatchV1>,
}

impl PhilosophyCorpusInputV1 {
    pub fn new(sources: SourceSpanBatchV1, media_evidence: Vec<MediaEvidenceBatchV1>) -> Self {
        Self {
            schema: PHILOSOPHY_CORPUS_INPUT_SCHEMA.to_string(),
            schema_version: PHILOSOPHY_CORPUS_INPUT_VERSION_V1,
            sources,
            media_evidence,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaEvidenceBatchV1 {
    pub schema: String,
    pub schema_version: u32,
    pub producer: MediaEvidenceProducerV1,
    pub video: MediaEvidenceVideoV1,
    pub revision: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenes: Vec<SceneEvidenceV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ocr_observations: Vec<OcrObservationEvidenceV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ocr_tracks: Vec<OcrTrackEvidenceV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sponsorblock: Option<SponsorBlockEvidenceV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MediaEvidenceProducerV1 {
    pub name: String,
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaEvidenceVideoV1 {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub youtube_id: Option<String>,
    pub source_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingEvidenceV1 {
    pub run_id: String,
    pub processor: String,
    pub processor_version: String,
    pub model: String,
    pub model_version: String,
    pub input_hash: String,
    pub config_hash: String,
    pub processing_config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SceneEvidenceV1 {
    pub id: String,
    pub scene_index: u64,
    pub start_frame: u64,
    pub end_frame: u64,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub metadata: Value,
    pub provenance: ProcessingEvidenceV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrObservationEvidenceV1 {
    pub id: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<MediaBoundingBoxV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    pub attributes: Value,
    pub provenance: ProcessingEvidenceV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MediaBoundingBoxV1 {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrTrackEvidenceV1 {
    pub id: String,
    pub text: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub sample_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_frame: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_frame: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<MediaBoundingBoxV1>,
    pub metadata: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observation_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scene_ids: Vec<String>,
    pub provenance: ProcessingEvidenceV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SponsorBlockEvidenceV1 {
    pub snapshot_id: String,
    pub response_hash: String,
    pub data_license: String,
    pub attribution: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<SponsorBlockSegmentEvidenceV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SponsorBlockSegmentEvidenceV1 {
    pub uuid: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<String>,
    pub start_seconds: f64,
    pub end_seconds: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_duration: Option<f64>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceSpanBatchV1 {
    pub schema: String,
    pub schema_version: u32,
    pub producer: SourceProducerV1,
    pub sources: Vec<SourceRecordV1>,
    pub spans: Vec<SourceSpanRecordV1>,
}

impl SourceSpanBatchV1 {
    pub fn new(
        producer: SourceProducerV1,
        sources: Vec<SourceRecordV1>,
        spans: Vec<SourceSpanRecordV1>,
    ) -> Self {
        Self {
            schema: SOURCE_SPAN_INTERCHANGE_SCHEMA.to_string(),
            schema_version: SOURCE_SPAN_INTERCHANGE_VERSION_V1,
            producer,
            sources,
            spans,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceProducerV1 {
    pub name: String,
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceRecordV1 {
    pub id: String,
    pub kind: String,
    pub revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceSpanRecordV1 {
    pub id: String,
    pub source_id: String,
    pub sequence: u64,
    pub text: String,
    pub content_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub locator: SourceLocatorV1,
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SourceLocatorV1 {
    Text {
        byte_start: usize,
        byte_end: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        paragraph_ordinal: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        page: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        section: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source_selector: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        heading_path: Vec<String>,
    },
    Timed {
        segment_index: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_seconds: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_seconds: Option<f64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceDocument {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceFragment {
    pub id: String,
    pub document_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStage {
    Ingest,
    Segment,
    Embed,
    Cluster,
    ExtractClaims,
    ClassifyRoles,
    ExtractTerms,
    ReconstructArguments,
    NormalizePropositions,
    Formalize,
    Evaluate,
    Worldview,
}

impl PipelineStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ingest => "ingest",
            Self::Segment => "segment",
            Self::Embed => "embed",
            Self::Cluster => "cluster",
            Self::ExtractClaims => "extract_claims",
            Self::ClassifyRoles => "classify_roles",
            Self::ExtractTerms => "extract_terms",
            Self::ReconstructArguments => "reconstruct_arguments",
            Self::NormalizePropositions => "normalize_propositions",
            Self::Formalize => "formalize",
            Self::Evaluate => "evaluate",
            Self::Worldview => "worldview",
        }
    }

    pub fn artifact_file(self) -> &'static str {
        match self {
            Self::Ingest => "01_ingest.json",
            Self::Segment => "02_passages.json",
            Self::Embed => "03_embeddings.json",
            Self::Cluster => "04_clusters.json",
            Self::ExtractClaims => "05_claim_candidates.json",
            Self::ClassifyRoles => "06_claim_roles.json",
            Self::ExtractTerms => "07_terms.json",
            Self::ReconstructArguments => "08_arguments.json",
            Self::NormalizePropositions => "09_normalized_propositions.json",
            Self::Formalize => "10_formalizations.json",
            Self::Evaluate => "11_evaluations.json",
            Self::Worldview => "worldview.json",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRef {
    pub id: String,
    pub stage: PipelineStage,
    pub path: String,
    pub provider: String,
    pub provider_version: String,
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineRunConfig {
    pub artifact_dir: PathBuf,
    pub openai_model_primary: String,
    pub openai_model_cheap: String,
    pub embedding_model: String,
    pub ner_model: String,
    pub nlp_mode: String,
    pub model_bundle_dir: PathBuf,
    pub auto_download_models: bool,
    pub embedding_backend: String,
    pub term_extraction_backend: String,
    pub classification_backend: String,
    pub lean_bin: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_through: Option<PipelineStage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineRun {
    pub run_id: String,
    pub document_id: String,
    pub config: PipelineRunConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ArtifactRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactEnvelope<T> {
    pub run_id: String,
    pub stage: PipelineStage,
    pub provider: String,
    pub provider_version: String,
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_artifact_ids: Vec<String>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<PipelineDiagnostic>,
    pub payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IngestedDocument {
    pub document_id: String,
    pub raw_text_hash: String,
    pub document: ExtractionDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Passage {
    pub id: String,
    pub document_id: String,
    pub paragraph_index: usize,
    pub sequence: usize,
    pub locator: String,
    pub start_char: usize,
    pub end_char: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PassageEmbedding {
    pub passage_id: String,
    pub model: String,
    pub dimensions: usize,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PassageCluster {
    pub id: String,
    pub passage_ids: Vec<String>,
    pub centroid_vector_id: String,
    pub representative_passage_id: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceOffset {
    pub passage_id: String,
    pub start_char: usize,
    pub end_char: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimCandidate {
    pub id: String,
    pub raw_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_hint: Option<String>,
    pub source_passage_ids: Vec<String>,
    pub source_offsets: Vec<SourceOffset>,
    pub claim_kind: ClaimKind,
    pub confidence: f64,
    pub requires_review: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimRole {
    MainClaim,
    Premise,
    Conclusion,
    Definition,
    Objection,
    Reply,
    Example,
    Background,
    Qualification,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoleAssignment {
    pub claim_id: String,
    pub primary_role: ClaimRole,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary_roles: Vec<ClaimRole>,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TermType {
    Person,
    Work,
    School,
    Concept,
    TechnicalTerm,
    Place,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TermCandidate {
    pub id: String,
    pub text: String,
    pub normalized_label: String,
    pub term_type: TermType,
    pub source_passage_ids: Vec<String>,
    pub confidence: f64,
    pub requires_review: bool,
    pub model_added: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArgumentRoleLink {
    pub from_claim_id: String,
    pub to_claim_id: String,
    pub relation: RelationKind,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArgumentCandidate {
    pub id: String,
    pub cluster_id: String,
    pub premise_claim_ids: Vec<String>,
    pub conclusion_claim_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objection_claim_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub support_relations: Vec<ArgumentRoleLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attack_relations: Vec<ArgumentRoleLink>,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedProposition {
    pub id: String,
    pub canonical_text: String,
    pub source_claim_ids: Vec<String>,
    pub source_passage_ids: Vec<String>,
    pub claim_kind: ClaimKind,
    pub role: ClaimRole,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FormalLogicAst {
    Atomic {
        predicate: String,
        args: Vec<String>,
    },
    Not {
        value: Box<FormalLogicAst>,
    },
    And {
        values: Vec<FormalLogicAst>,
    },
    Or {
        values: Vec<FormalLogicAst>,
    },
    Implies {
        left: Box<FormalLogicAst>,
        right: Box<FormalLogicAst>,
    },
    Iff {
        left: Box<FormalLogicAst>,
        right: Box<FormalLogicAst>,
    },
    ForAll {
        variable: String,
        body: Box<FormalLogicAst>,
    },
    Exists {
        variable: String,
        body: Box<FormalLogicAst>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FormalizationStatus {
    SchemaValid,
    SchemaInvalid,
    NeedsReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FormalizationCandidate {
    pub id: String,
    pub proposition_id: String,
    pub ast: FormalLogicAst,
    pub lean_expression: String,
    pub status: FormalizationStatus,
    pub confidence: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<PipelineDiagnostic>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FormalEvaluationStatus {
    Typechecked,
    Proved,
    Disproved,
    Unsupported,
    EngineError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FormalEvaluation {
    pub formalization_id: String,
    pub proposition_id: String,
    pub status: FormalEvaluationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lean_source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<PipelineDiagnostic>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    Atomic,
    Conditional,
    Definition,
    Modal,
    Normative,
    Universal,
    Unknown,
}

impl ClaimKind {
    pub fn as_metadata_value(self) -> Value {
        let value = match self {
            Self::Atomic => "atomic",
            Self::Conditional => "conditional",
            Self::Definition => "definition",
            Self::Modal => "modal",
            Self::Normative => "normative",
            Self::Universal => "universal",
            Self::Unknown => "unknown",
        };
        Value::String(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssertionStatus {
    Accepted,
    Exploring,
}

impl AssertionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Exploring => "exploring",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    Supports,
    Contradicts,
    Interprets,
    Specializes,
    Generalizes,
    VariantOf,
}

impl RelationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Supports => "supports",
            Self::Contradicts => "contradicts",
            Self::Interprets => "interprets",
            Self::Specializes => "specializes",
            Self::Generalizes => "generalizes",
            Self::VariantOf => "variant_of",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropositionCandidate {
    pub id: String,
    pub canonical_text: String,
    pub claim_kind: ClaimKind,
    pub assertion_status: AssertionStatus,
    pub confidence: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_fragment_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CandidateScoreBreakdown {
    pub extraction_confidence: f64,
    pub philosophical_score: f64,
    pub source_quality_score: f64,
    pub specificity_score: f64,
    pub role_score: f64,
    pub relation_score: f64,
    pub final_rank_score: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CandidateIndexEntry {
    pub proposition_id: String,
    pub canonical_text: String,
    pub claim_kind: ClaimKind,
    pub assertion_status: AssertionStatus,
    pub confidence: f64,
    pub score_breakdown: CandidateScoreBreakdown,
    pub source_fragment_ids: Vec<String>,
    pub first_source_sequence: usize,
    pub first_paragraph_index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    pub has_negation: bool,
    pub normalized_fingerprint: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub duplicate_source_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variant_of_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RelationCandidate {
    pub from_proposition_id: String,
    pub to_proposition_id: String,
    pub kind: RelationKind,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Facet {
    pub family: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Assertion {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_fragment_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Proposition {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertions: Vec<Assertion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_fragment_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facets: Vec<Facet>,
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
    pub id: String,
    pub from_proposition_id: String,
    pub to_proposition_id: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedWorldviewV10 {
    pub schema_version: String,
    pub id: String,
    pub fragment: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub propositions: Vec<Proposition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<Relation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_documents: Vec<SourceDocument>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_fragments: Vec<SourceFragment>,
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StageSummary {
    pub name: String,
    pub output_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<PipelineDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionResponse {
    pub run_id: String,
    pub provider: String,
    pub worldview: UnifiedWorldviewV10,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ArtifactRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidate_index: Vec<CandidateIndexEntry>,
    pub candidates: Vec<PropositionCandidate>,
    pub diagnostics: Vec<PipelineDiagnostic>,
    pub stages: Vec<StageSummary>,
}

#[cfg(test)]
mod source_span_interchange_tests {
    use super::*;

    fn hash(ch: char) -> String {
        format!("sha256:{}", ch.to_string().repeat(64))
    }

    #[test]
    fn v1_serialization_matches_the_cross_repository_json_shape() {
        let batch = SourceSpanBatchV1::new(
            SourceProducerV1 {
                name: "youtube-corpus".to_string(),
                revision: "git:abc123".to_string(),
            },
            vec![SourceRecordV1 {
                id: "stream-1".to_string(),
                kind: "youtube_transcript".to_string(),
                revision: hash('a'),
                uri: Some("https://example.test/video".to_string()),
                title: Some("Lecture".to_string()),
                creators: Vec::new(),
                language: Some("en".to_string()),
                content_hash: hash('a'),
                metadata: Metadata::new(),
            }],
            vec![SourceSpanRecordV1 {
                id: "segment-1".to_string(),
                source_id: "stream-1".to_string(),
                sequence: 0,
                text: "Knowledge concerns truth.".to_string(),
                content_hash: hash('b'),
                language: Some("en".to_string()),
                locator: SourceLocatorV1::Timed {
                    segment_index: 0,
                    start_seconds: Some(1.25),
                    end_seconds: Some(3.5),
                },
                metadata: Metadata::new(),
            }],
        );

        let value = serde_json::to_value(batch).unwrap();
        assert_eq!(value["schema"], SOURCE_SPAN_INTERCHANGE_SCHEMA);
        assert_eq!(value["schemaVersion"], SOURCE_SPAN_INTERCHANGE_VERSION_V1);
        assert_eq!(value["spans"][0]["sourceId"], "stream-1");
        assert_eq!(value["spans"][0]["contentHash"], hash('b'));
        assert_eq!(value["spans"][0]["locator"]["kind"], "timed");
        assert_eq!(value["spans"][0]["locator"]["segmentIndex"], 0);
        assert_eq!(value["spans"][0]["locator"]["startSeconds"], 1.25);
        assert_eq!(value["spans"][0]["locator"]["endSeconds"], 3.5);
    }
}
