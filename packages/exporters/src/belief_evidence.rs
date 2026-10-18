//! Reference export from validated source-span inputs, never interpretation results.
use philosophy_extractor_schema::{Metadata, SourceLocatorV1, SourceSpanBatchV1};
use philosophy_extractor_source_ingestion::validate_source_span_batch;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum ReferenceExportError {
    #[error("{0}")]
    InvalidInput(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn invalid(message: impl Into<String>) -> ReferenceExportError {
    ReferenceExportError::InvalidInput(message.into())
}

fn immutable(field: &str, value: &str) -> Result<(), ReferenceExportError> {
    let prefixed = ["sha256:", "git:", "commit:"]
        .iter()
        .any(|prefix| value.starts_with(prefix) && value.len() > prefix.len());
    let hex = matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit());
    if prefixed || hex {
        Ok(())
    } else {
        Err(invalid(format!(
            "{field} lacks an immutable revision ({value:?}); restore the original processor receipt or reprocess the source; changing the exporter revision cannot repair historical provenance"
        )))
    }
}

fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn json_hash(value: &Value) -> Result<String, ReferenceExportError> {
    Ok(hash(&serde_json::to_vec(value)?))
}

fn required<'a>(
    metadata: &'a Metadata,
    key: &str,
    record: &str,
) -> Result<&'a str, ReferenceExportError> {
    metadata.get(key).and_then(Value::as_str).filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid(format!("record {record} lacks its {key} receipt; restore the original producer artifact or reprocess the source")))
}

fn producer(
    metadata: &Metadata,
    prefix: &str,
    record: &str,
) -> Result<Value, ReferenceExportError> {
    let name = required(metadata, &format!("{prefix}Producer"), record)?;
    let revision = required(metadata, &format!("{prefix}ProducerVersion"), record)?;
    immutable(
        &format!("record {record} {prefix}ProducerVersion"),
        revision,
    )?;
    let mut output = json!({"name":name,"revision":revision});
    if let Some(config) = metadata.get(&format!("{prefix}ConfigHash")) {
        let config = config
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid(format!("record {record} has invalid {prefix}ConfigHash")))?;
        output["configHash"] = json!(config);
    }
    Ok(output)
}

fn record(
    (repository, scope, id, revision): (&str, &str, &str, &str),
    class: &str,
    producer: &Value,
    parents: Vec<String>,
) -> Result<Value, ReferenceExportError> {
    let mut output = json!({
        "class":class,
        "familyId": format!("family:source-record:{}", json_hash(&json!([repository,scope,id]))?),
        "source":{"repository":repository,"scopeId":scope,"recordId":id,"revision":revision},
        "producer":producer,
    });
    if !parents.is_empty() {
        output["parentEvidenceIds"] = json!(parents);
    }
    // Bind the complete immutable record; changed input revisions get a new id
    // while their family continues to identify correlated versions of one unit.
    output["id"] = json!(format!(
        "evidence:philosophy-source:{}",
        json_hash(&output)?
    ));
    Ok(output)
}

/// Export source descriptors and document passages. Candidates, philosophical
/// assessments and confidence scores are deliberately absent from this interface.
/// Text and locators remain in the original source-span artifact, addressed by
/// repository/scope/record and the canonical record revision.
pub fn export_belief_evidence(
    input: &SourceSpanBatchV1,
    exporter_revision: &str,
) -> Result<String, ReferenceExportError> {
    validate_source_span_batch(input).map_err(|error| invalid(error.to_string()))?;
    immutable("exporter.revision", exporter_revision)?;
    immutable("source-span producer.revision", &input.producer.revision)?;
    if input.spans.is_empty() {
        return Err(invalid(
            "source-span input has no document passages to export",
        ));
    }
    let mut sources = BTreeMap::new();
    for source in &input.sources {
        if !matches!(source.kind.as_str(), "text" | "web_page") {
            return Err(invalid(format!(
                "source {} kind {:?} is not supported by the document-text adapter; use the original transcript/OCR producer export instead",
                source.id, source.kind
            )));
        }
        immutable(&format!("source {} revision", source.id), &source.revision)?;
        let scope = source
            .metadata
            .get("corpusCoreSourceId")
            .and_then(Value::as_str)
            .unwrap_or(&source.id);
        if scope.trim().is_empty() {
            return Err(invalid(format!(
                "source {} has an empty corpus scope",
                source.id
            )));
        }
        let receipt = producer(&source.metadata, "extraction", &source.id)?;
        let representation_id =
            required(&source.metadata, "corpusCoreRepresentationId", &source.id)?;
        let representation_revision = required(
            &source.metadata,
            "corpusCoreRepresentationRevision",
            &source.id,
        )?;
        immutable("representation record revision", representation_revision)?;
        let descriptor = record(
            (
                &input.producer.name,
                scope,
                representation_id,
                representation_revision,
            ),
            "metadata",
            &receipt,
            Vec::new(),
        )?;
        sources.insert(source.id.as_str(), (scope, receipt, descriptor));
    }
    let mut evidence = sources
        .values()
        .map(|(_, _, descriptor)| descriptor.clone())
        .collect::<Vec<_>>();
    for span in &input.spans {
        if !matches!(span.locator, SourceLocatorV1::Text { .. }) {
            return Err(invalid(format!(
                "span {} has a timed locator; use the transcript producer export instead of labeling it document text",
                span.id
            )));
        }
        if hash(span.text.as_bytes()) != span.content_hash {
            return Err(invalid(format!(
                "span {} text does not match its recorded content hash",
                span.id
            )));
        }
        let (scope, _, descriptor) = &sources[span.source_id.as_str()];
        let expected_input =
            json!([{"record_type":"representation", "id":descriptor["source"]["recordId"]}]);
        if span.metadata.get("spanProducerInputs") != Some(&expected_input) {
            return Err(invalid(format!(
                "span {} has missing or unsupported input lineage; its processor receipt must reference exactly the exported representation",
                span.id
            )));
        }
        let receipt = producer(&span.metadata, "span", &span.id)?;
        let revision = required(&span.metadata, "corpusCoreSegmentRevision", &span.id)?;
        immutable("segment record revision", revision)?;
        evidence.push(record(
            (&input.producer.name, scope, &span.id, revision),
            "source_text",
            &receipt,
            vec![
                descriptor["id"]
                    .as_str()
                    .ok_or_else(|| invalid("source descriptor id is missing"))?
                    .to_string(),
            ],
        )?);
    }
    evidence.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    let revision = json_hash(&json!(evidence))?;
    let output = json!({"schema":"belief_evidence_interchange","schemaVersion":1,
        "exporter":{"name":"philosophy-extractor","revision":exporter_revision},
        "revision":revision,"evidence":evidence});
    Ok(format!("{}\n", serde_json::to_string_pretty(&output)?))
}
