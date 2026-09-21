use philosophy_extractor_schema::{
    ExtractionDocument, SOURCE_SPAN_INTERCHANGE_SCHEMA, SOURCE_SPAN_INTERCHANGE_VERSION_V1,
    SourceLocatorV1, SourceSpanBatchV1,
};
use std::collections::HashSet;
use std::fmt;

pub fn plain_text_document(text: impl Into<String>) -> ExtractionDocument {
    ExtractionDocument {
        id: None,
        kind: Some("philosophical_text".to_string()),
        title: None,
        authors: Vec::new(),
        language: None,
        uri: None,
        text: text.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpanValidationError {
    message: String,
}

impl SourceSpanValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SourceSpanValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SourceSpanValidationError {}

pub fn validate_source_span_batch(
    batch: &SourceSpanBatchV1,
) -> Result<(), SourceSpanValidationError> {
    if batch.schema != SOURCE_SPAN_INTERCHANGE_SCHEMA
        || batch.schema_version != SOURCE_SPAN_INTERCHANGE_VERSION_V1
    {
        return Err(SourceSpanValidationError::new(format!(
            "unsupported source-span contract {}@{}",
            batch.schema, batch.schema_version
        )));
    }
    if batch.producer.name.trim().is_empty() || batch.producer.revision.trim().is_empty() {
        return Err(SourceSpanValidationError::new(
            "source-span producer name and revision are required",
        ));
    }

    let mut source_ids = HashSet::new();
    for source in &batch.sources {
        if source.id.trim().is_empty() || source.kind.trim().is_empty() {
            return Err(SourceSpanValidationError::new(
                "source id and kind are required",
            ));
        }
        if !source_ids.insert(source.id.as_str()) {
            return Err(SourceSpanValidationError::new(format!(
                "duplicate source id {}",
                source.id
            )));
        }
        if source.revision.trim().is_empty() {
            return Err(SourceSpanValidationError::new(format!(
                "source {} is missing an exact revision",
                source.id
            )));
        }
        validate_sha256(&source.content_hash, &source.id)?;
    }

    let mut span_ids = HashSet::new();
    let mut sequences = HashSet::new();
    for span in &batch.spans {
        if span.id.trim().is_empty() {
            return Err(SourceSpanValidationError::new("span id is required"));
        }
        if !span_ids.insert(span.id.as_str()) {
            return Err(SourceSpanValidationError::new(format!(
                "duplicate span id {}",
                span.id
            )));
        }
        if !source_ids.contains(span.source_id.as_str()) {
            return Err(SourceSpanValidationError::new(format!(
                "span {} references unknown source {}",
                span.id, span.source_id
            )));
        }
        if !sequences.insert((span.source_id.as_str(), span.sequence)) {
            return Err(SourceSpanValidationError::new(format!(
                "source {} has duplicate span sequence {}",
                span.source_id, span.sequence
            )));
        }
        if span.text.trim().is_empty() {
            return Err(SourceSpanValidationError::new(format!(
                "span {} has empty text",
                span.id
            )));
        }
        validate_sha256(&span.content_hash, &span.id)?;
        validate_locator(&span.id, &span.locator)?;
    }

    Ok(())
}

fn validate_sha256(value: &str, target_id: &str) -> Result<(), SourceSpanValidationError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(SourceSpanValidationError::new(format!(
            "{target_id} content hash must use sha256:<hex>"
        )));
    };
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(SourceSpanValidationError::new(format!(
            "{target_id} content hash is not a complete SHA-256 digest"
        )));
    }
    Ok(())
}

fn validate_locator(
    span_id: &str,
    locator: &SourceLocatorV1,
) -> Result<(), SourceSpanValidationError> {
    match locator {
        SourceLocatorV1::Text {
            byte_start,
            byte_end,
            ..
        } if byte_start >= byte_end => Err(SourceSpanValidationError::new(format!(
            "span {span_id} has an invalid UTF-8 byte range"
        ))),
        SourceLocatorV1::Timed {
            start_seconds,
            end_seconds,
            ..
        } => {
            if start_seconds.is_some_and(|value| !value.is_finite() || value < 0.0)
                || end_seconds.is_some_and(|value| !value.is_finite() || value < 0.0)
                || start_seconds
                    .zip(*end_seconds)
                    .is_some_and(|(start, end)| end < start)
            {
                return Err(SourceSpanValidationError::new(format!(
                    "span {span_id} has an invalid timed range"
                )));
            }
            Ok(())
        }
        SourceLocatorV1::Text { .. } => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use philosophy_extractor_schema::{
        Metadata, SourceProducerV1, SourceRecordV1, SourceSpanRecordV1,
    };

    fn hash(ch: char) -> String {
        format!("sha256:{}", ch.to_string().repeat(64))
    }

    fn valid_batch() -> SourceSpanBatchV1 {
        SourceSpanBatchV1::new(
            SourceProducerV1 {
                name: "document-search".to_string(),
                revision: "abc123".to_string(),
            },
            vec![SourceRecordV1 {
                id: "doc-1".to_string(),
                kind: "document".to_string(),
                revision: hash('a'),
                uri: Some("https://example.test/book".to_string()),
                title: Some("Example".to_string()),
                creators: Vec::new(),
                language: Some("en".to_string()),
                content_hash: hash('a'),
                metadata: Metadata::new(),
            }],
            vec![SourceSpanRecordV1 {
                id: "doc-1:p0".to_string(),
                source_id: "doc-1".to_string(),
                sequence: 0,
                text: "Knowledge concerns truth.".to_string(),
                content_hash: hash('b'),
                language: Some("en".to_string()),
                locator: SourceLocatorV1::Text {
                    byte_start: 0,
                    byte_end: 25,
                    paragraph_ordinal: Some(0),
                    source_selector: None,
                    heading_path: Vec::new(),
                },
                metadata: Metadata::new(),
            }],
        )
    }

    #[test]
    fn accepts_a_provenance_complete_v1_batch() {
        validate_source_span_batch(&valid_batch()).unwrap();
    }

    #[test]
    fn rejects_spans_that_reference_missing_sources() {
        let mut batch = valid_batch();
        batch.spans[0].source_id = "missing".to_string();

        let error = validate_source_span_batch(&batch).unwrap_err();
        assert!(error.to_string().contains("unknown source"));
    }

    #[test]
    fn rejects_ambiguous_or_unverifiable_provenance() {
        let mut batch = valid_batch();
        batch.spans[0].content_hash = "not-a-hash".to_string();

        let error = validate_source_span_batch(&batch).unwrap_err();
        assert!(error.to_string().contains("sha256"));
    }

    #[test]
    fn rejects_reversed_timed_ranges() {
        let mut batch = valid_batch();
        batch.spans[0].locator = SourceLocatorV1::Timed {
            segment_index: 0,
            start_seconds: Some(12.0),
            end_seconds: Some(11.5),
        };

        let error = validate_source_span_batch(&batch).unwrap_err();
        assert!(error.to_string().contains("timed range"));
    }
}
