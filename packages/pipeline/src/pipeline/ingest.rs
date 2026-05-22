use crate::model::ExtractionDocument;

use super::PipelineError;

pub fn ingest(mut document: ExtractionDocument) -> Result<ExtractionDocument, PipelineError> {
    document.text = document.text.replace("\r\n", "\n").replace('\r', "\n");
    document.text = document.text.trim().to_string();
    if document.text.is_empty() {
        return Err(PipelineError::EmptyInput);
    }
    if document.kind.as_deref().is_none_or(str::is_empty) {
        document.kind = Some("philosophical_text".to_string());
    }
    Ok(document)
}
