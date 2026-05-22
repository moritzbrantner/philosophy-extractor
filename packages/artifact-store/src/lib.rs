use philosophy_extractor_schema::{ArtifactEnvelope, ArtifactRef, PipelineRun, PipelineStage};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub trait ArtifactStore {
    fn put(&self, key: &str, bytes: &[u8]) -> Result<(), ArtifactStoreError>;
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, ArtifactStoreError>;
}

#[derive(Debug, Error)]
pub enum ArtifactStoreError {
    #[error("failed to create artifact directory '{path}': {source}")]
    CreateDir {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to serialize artifact '{path}': {source}")]
    Serialize {
        path: String,
        source: serde_json::Error,
    },
    #[error("failed to write artifact '{path}': {source}")]
    Write {
        path: String,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone)]
pub struct FileArtifactStore {
    root: PathBuf,
}

impl FileArtifactStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.root.join(run_id)
    }

    pub fn write_stage<T: Serialize>(
        &self,
        run_id: &str,
        envelope: &ArtifactEnvelope<T>,
    ) -> Result<ArtifactRef, ArtifactStoreError> {
        let run_dir = self.ensure_run_dir(run_id)?;
        let path = run_dir.join(envelope.stage.artifact_file());
        write_json(&path, envelope)?;
        Ok(ArtifactRef {
            id: artifact_id(run_id, envelope.stage),
            stage: envelope.stage,
            path: path.to_string_lossy().into_owned(),
            provider: envelope.provider.clone(),
            provider_version: envelope.provider_version.clone(),
            metadata: envelope.metadata.clone(),
        })
    }

    pub fn write_manifest(&self, run: &PipelineRun) -> Result<(), ArtifactStoreError> {
        let run_dir = self.ensure_run_dir(&run.run_id)?;
        write_json(&run_dir.join("manifest.json"), run)
    }

    fn ensure_run_dir(&self, run_id: &str) -> Result<PathBuf, ArtifactStoreError> {
        let path = self.run_dir(run_id);
        fs::create_dir_all(&path).map_err(|source| ArtifactStoreError::CreateDir {
            path: path.to_string_lossy().into_owned(),
            source,
        })?;
        Ok(path)
    }
}

pub fn artifact_id(run_id: &str, stage: PipelineStage) -> String {
    format!("{}_{}", run_id, stage.as_str())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), ArtifactStoreError> {
    let json =
        serde_json::to_string_pretty(value).map_err(|source| ArtifactStoreError::Serialize {
            path: path.to_string_lossy().into_owned(),
            source,
        })?;
    fs::write(path, json).map_err(|source| ArtifactStoreError::Write {
        path: path.to_string_lossy().into_owned(),
        source,
    })
}
