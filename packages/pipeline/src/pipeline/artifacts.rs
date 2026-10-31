use crate::model::{ArtifactEnvelope, ArtifactRef, PipelineDiagnostic, PipelineStage};
use philosophy_extractor_artifact_store::FileArtifactStore;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::PipelineError;

pub fn persist_stage<T: Serialize>(
    store: Option<&FileArtifactStore>,
    run_id: &str,
    stage: PipelineStage,
    provider: &str,
    provider_version: &str,
    input_artifact_ids: Vec<String>,
    diagnostics: Vec<PipelineDiagnostic>,
    payload: &T,
    artifacts: &mut Vec<ArtifactRef>,
) -> Result<(), PipelineError> {
    persist_stage_with_metadata(
        store,
        run_id,
        stage,
        provider,
        provider_version,
        BTreeMap::new(),
        input_artifact_ids,
        diagnostics,
        payload,
        artifacts,
    )
}

pub fn persist_stage_with_metadata<T: Serialize>(
    store: Option<&FileArtifactStore>,
    run_id: &str,
    stage: PipelineStage,
    provider: &str,
    provider_version: &str,
    metadata: BTreeMap<String, Value>,
    input_artifact_ids: Vec<String>,
    diagnostics: Vec<PipelineDiagnostic>,
    payload: &T,
    artifacts: &mut Vec<ArtifactRef>,
) -> Result<(), PipelineError> {
    if let Some(store) = store {
        let envelope = ArtifactEnvelope {
            run_id: run_id.to_string(),
            stage,
            provider: provider.to_string(),
            provider_version: provider_version.to_string(),
            metadata,
            input_artifact_ids,
            created_at: created_at(),
            diagnostics,
            payload,
        };
        artifacts.push(store.write_stage(run_id, &envelope)?);
    }
    Ok(())
}

pub fn persist_named_stage<T: Serialize>(
    store: Option<&FileArtifactStore>,
    run_id: &str,
    file_name: &str,
    artifact_ref_id: String,
    stage: PipelineStage,
    provider: &str,
    provider_version: &str,
    input_artifact_ids: Vec<String>,
    diagnostics: Vec<PipelineDiagnostic>,
    payload: &T,
    artifacts: &mut Vec<ArtifactRef>,
) -> Result<(), PipelineError> {
    if let Some(store) = store {
        let envelope = ArtifactEnvelope {
            run_id: run_id.to_string(),
            stage,
            provider: provider.to_string(),
            provider_version: provider_version.to_string(),
            metadata: BTreeMap::new(),
            input_artifact_ids,
            created_at: created_at(),
            diagnostics,
            payload,
        };
        artifacts.push(store.write_named_stage(run_id, file_name, artifact_ref_id, &envelope)?);
    }
    Ok(())
}

fn created_at() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("unix:{seconds}")
}

pub fn provider_metadata(
    model_id: &str,
    backend: &str,
    model_bundle_dir: &std::path::Path,
    auto_download: bool,
    task_category: &str,
) -> BTreeMap<String, Value> {
    let mut metadata = BTreeMap::new();
    metadata.insert("modelId".to_string(), Value::String(model_id.to_string()));
    metadata.insert("backend".to_string(), Value::String(backend.to_string()));
    metadata.insert(
        "modelBundleDir".to_string(),
        Value::String(model_bundle_dir.to_string_lossy().into_owned()),
    );
    metadata.insert("autoDownloadModels".to_string(), Value::Bool(auto_download));
    metadata.insert(
        "taskCategory".to_string(),
        Value::String(task_category.to_string()),
    );
    metadata
}
