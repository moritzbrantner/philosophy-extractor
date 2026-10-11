//! Intake of `moenarch_corpus_core_batch` v1 JSONL (document-search's corpus-core export) into the
//! source-span interpretation input.
//!
//! Corpus identity (sources, assets, representations, segments, hashes, provenance) is
//! deserialized into the real `moenarch-corpus-core` types; this module only owns the batch line
//! envelope and the mapping into [`SourceSpanBatchV1`]. Serde on corpus-core's newtypes is
//! transparent, so every identifier, hash and range is re-validated through corpus-core's own
//! constructors before it is trusted.

use crate::{SourceSpanValidationError, validate_source_span_batch};
use corpus_core::{
    AssetId, ContentHash, CorpusItemRef, CorpusRecord, RepresentationId, RepresentationKind,
    SegmentId, SourceId, Span,
};
use philosophy_extractor_schema::{
    Metadata, SourceLocatorV1, SourceProducerV1, SourceRecordV1, SourceSpanBatchV1,
    SourceSpanRecordV1,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Batch schema name written by document-search's corpus-core exporter.
pub const CORPUS_CORE_BATCH_SCHEMA: &str = "moenarch_corpus_core_batch";
/// The only supported batch schema version.
pub const CORPUS_CORE_BATCH_VERSION_V1: u64 = 1;

const CHARACTER_UNIT_SELECTOR: &str = "character_unit";
const UNICODE_SCALAR: &str = "unicode_scalar";
const REPRESENTATION_SELECTOR: &str = "representation_id";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchHeader {
    schema: String,
    schema_version: u64,
    producer: BatchProducer,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchProducer {
    name: String,
    revision: String,
}

/// One body line: a corpus-core record or the text payload of a representation.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum BatchLine {
    Record(Box<CorpusRecord>),
    Payload(TextPayload),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextPayload {
    hash: String,
    #[allow(dead_code)]
    media_type: Option<String>,
    text: String,
}

fn error(message: impl Into<String>) -> SourceSpanValidationError {
    SourceSpanValidationError::new(message)
}

/// Reads a `moenarch_corpus_core_batch` v1 JSONL export and maps its text segments into a
/// validated [`SourceSpanBatchV1`].
///
/// One source-span source is produced per corpus-core asset that owns text segments (asset file
/// order); its spans are the asset's segments ordered by Unicode-scalar character range. Span
/// text is recovered from the representation payload carried in the batch and verified against
/// the representation content hash.
pub fn source_spans_from_corpus_core_jsonl(
    jsonl: &str,
) -> Result<SourceSpanBatchV1, SourceSpanValidationError> {
    let mut lines = jsonl
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty());

    let Some((header_index, header_line)) = lines.next() else {
        return Err(error(
            "corpus-core batch is empty; a header line is required",
        ));
    };
    let header: BatchHeader = serde_json::from_str(header_line).map_err(|cause| {
        error(format!(
            "corpus-core batch line {} is not a valid batch header: {cause}",
            header_index + 1
        ))
    })?;
    if header.schema != CORPUS_CORE_BATCH_SCHEMA
        || header.schema_version != CORPUS_CORE_BATCH_VERSION_V1
    {
        return Err(error(format!(
            "unsupported corpus-core batch {}@{}",
            header.schema, header.schema_version
        )));
    }
    if header.producer.name.trim().is_empty() || header.producer.revision.trim().is_empty() {
        return Err(error(
            "corpus-core batch producer name and revision are required",
        ));
    }

    let mut records = Vec::new();
    let mut payloads = BTreeMap::new();
    for (index, line) in lines {
        let parsed: BatchLine = serde_json::from_str(line).map_err(|cause| {
            error(format!(
                "corpus-core batch line {} is not a corpus-core record or payload: {cause}",
                index + 1
            ))
        })?;
        match parsed {
            BatchLine::Record(record) => records.push(*record),
            BatchLine::Payload(payload) => {
                let hash = parse_hash(payload.hash, "payload")?;
                if ContentHash::from_bytes(payload.text.as_bytes()) != hash {
                    return Err(error(format!(
                        "payload text does not match its content hash {hash}"
                    )));
                }
                if payloads.insert(hash.clone(), payload.text).is_some() {
                    return Err(error(format!("duplicate payload for content hash {hash}")));
                }
            }
        }
    }

    let index = CorpusIndex::build(&records)?;
    let batch = index.map(
        SourceProducerV1 {
            name: header.producer.name,
            revision: header.producer.revision,
        },
        &payloads,
    )?;
    validate_source_span_batch(&batch)?;
    Ok(batch)
}

fn parse_hash(
    value: impl Into<String>,
    owner: &str,
) -> Result<ContentHash, SourceSpanValidationError> {
    ContentHash::parse(value)
        .map_err(|cause| error(format!("{owner} has an invalid content hash: {cause}")))
}

fn revalidate_hash(hash: &ContentHash, owner: &str) -> Result<(), SourceSpanValidationError> {
    parse_hash(hash.as_str(), owner).map(|_| ())
}

fn revalidate_span(span: &Span, owner: &str, name: &str) -> Result<(), SourceSpanValidationError> {
    Span::new(span.start, span.end)
        .map(|_| ())
        .map_err(|cause| error(format!("{owner} has an invalid {name}: {cause}")))
}

fn revalidate_id(value: &str, kind: &str) -> Result<(), SourceSpanValidationError> {
    if value.trim().is_empty() {
        return Err(error(format!(
            "corpus-core {kind} identifier must not be empty"
        )));
    }
    Ok(())
}

fn item_id(item: &CorpusItemRef) -> &str {
    match item {
        CorpusItemRef::Source(id) => id.as_str(),
        CorpusItemRef::Asset(id) => id.as_str(),
        CorpusItemRef::Segment(id) => id.as_str(),
        CorpusItemRef::Representation(id) => id.as_str(),
    }
}

/// Records of one batch keyed by typed identity, in file order per record family.
struct CorpusIndex<'a> {
    sources: BTreeMap<&'a SourceId, &'a corpus_core::Source>,
    assets: Vec<&'a corpus_core::Asset>,
    asset_ids: BTreeSet<&'a AssetId>,
    representations: BTreeMap<&'a RepresentationId, &'a corpus_core::Representation>,
    segments: Vec<&'a corpus_core::Segment>,
    segment_ids: BTreeSet<&'a SegmentId>,
}

impl<'a> CorpusIndex<'a> {
    fn build(records: &'a [CorpusRecord]) -> Result<Self, SourceSpanValidationError> {
        let mut index = Self {
            sources: BTreeMap::new(),
            assets: Vec::new(),
            asset_ids: BTreeSet::new(),
            representations: BTreeMap::new(),
            segments: Vec::new(),
            segment_ids: BTreeSet::new(),
        };
        let mut seen = BTreeSet::new();
        for record in records {
            let item = record.item_ref();
            if !seen.insert(item.clone()) {
                return Err(error(format!(
                    "duplicate corpus-core record {}",
                    item_id(&item)
                )));
            }
            match record {
                CorpusRecord::Source(source) => {
                    revalidate_id(source.id.as_str(), "source")?;
                    index.sources.insert(&source.id, source);
                }
                CorpusRecord::Asset(asset) => {
                    revalidate_id(asset.id.as_str(), "asset")?;
                    revalidate_id(asset.source_id.as_str(), "source")?;
                    if let Some(hash) = &asset.content.hash {
                        revalidate_hash(hash, &format!("asset {}", asset.id))?;
                    }
                    index.asset_ids.insert(&asset.id);
                    index.assets.push(asset);
                }
                CorpusRecord::Representation(representation) => {
                    let owner = format!("representation {}", representation.id);
                    revalidate_id(representation.id.as_str(), "representation")?;
                    revalidate_id(item_id(&representation.owner), "owner")?;
                    revalidate_hash(&representation.content_hash, &owner)?;
                    if let Some(hash) = representation
                        .content
                        .as_ref()
                        .and_then(|content| content.hash.as_ref())
                    {
                        revalidate_hash(hash, &owner)?;
                        // The content reference must describe the same payload as the record.
                        if hash != &representation.content_hash {
                            return Err(error(format!(
                                "{owner} content hash {} conflicts with its content reference hash {hash}",
                                representation.content_hash
                            )));
                        }
                    }
                    if let Some(hash) = &representation.provenance.parameters_hash {
                        revalidate_hash(hash, &owner)?;
                    }
                    index
                        .representations
                        .insert(&representation.id, representation);
                }
                CorpusRecord::Segment(segment) => {
                    let owner = format!("segment {}", segment.id);
                    revalidate_id(segment.id.as_str(), "segment")?;
                    revalidate_id(segment.asset_id.as_str(), "asset")?;
                    let locator = &segment.locator;
                    for (name, span) in [
                        ("byte range", locator.byte_range),
                        ("character range", locator.character_range),
                        ("time range", locator.time_range_ms),
                        ("frame range", locator.frame_range),
                    ] {
                        if let Some(span) = span {
                            revalidate_span(&span, &owner, name)?;
                        }
                    }
                    if let Some(hash) = segment
                        .provenance
                        .as_ref()
                        .and_then(|provenance| provenance.parameters_hash.as_ref())
                    {
                        revalidate_hash(hash, &owner)?;
                    }
                    index.segment_ids.insert(&segment.id);
                    index.segments.push(segment);
                }
            }
        }
        index.check_references(records)?;
        Ok(index)
    }

    fn contains(&self, item: &CorpusItemRef) -> bool {
        match item {
            CorpusItemRef::Source(id) => self.sources.contains_key(id),
            CorpusItemRef::Asset(id) => self.asset_ids.contains(id),
            CorpusItemRef::Segment(id) => self.segment_ids.contains(id),
            CorpusItemRef::Representation(id) => self.representations.contains_key(id),
        }
    }

    fn check_references(&self, records: &[CorpusRecord]) -> Result<(), SourceSpanValidationError> {
        for asset in &self.assets {
            if !self.sources.contains_key(&asset.source_id) {
                return Err(error(format!(
                    "asset {} references unknown source {}",
                    asset.id, asset.source_id
                )));
            }
        }
        for segment in &self.segments {
            if !self.asset_ids.contains(&segment.asset_id) {
                return Err(error(format!(
                    "segment {} references unknown asset {}",
                    segment.id, segment.asset_id
                )));
            }
        }
        for representation in self.representations.values() {
            if !self.contains(&representation.owner) {
                return Err(error(format!(
                    "representation {} references unknown owner {}",
                    representation.id,
                    item_id(&representation.owner)
                )));
            }
        }
        for record in records {
            let provenance = match record {
                CorpusRecord::Representation(representation) => Some(&representation.provenance),
                CorpusRecord::Segment(segment) => segment.provenance.as_ref(),
                CorpusRecord::Source(_) | CorpusRecord::Asset(_) => None,
            };
            for input in provenance
                .into_iter()
                .flat_map(|provenance| &provenance.inputs)
            {
                if !self.contains(input) {
                    return Err(error(format!(
                        "{} provenance references unknown input {}",
                        item_id(&record.item_ref()),
                        item_id(input)
                    )));
                }
            }
        }
        Ok(())
    }

    /// Resolves the representation a text segment indexes into and checks that it belongs to
    /// the segment's asset and carries text.
    fn text_representation(
        &self,
        segment: &corpus_core::Segment,
    ) -> Result<&'a corpus_core::Representation, SourceSpanValidationError> {
        let selectors = &segment.locator.selectors;
        if selectors.get(CHARACTER_UNIT_SELECTOR).map(String::as_str) != Some(UNICODE_SCALAR) {
            return Err(error(format!(
                "segment {} must declare {CHARACTER_UNIT_SELECTOR} {UNICODE_SCALAR} for its character range",
                segment.id
            )));
        }
        let Some(representation_id) = selectors.get(REPRESENTATION_SELECTOR) else {
            return Err(error(format!(
                "segment {} does not name the representation its character range indexes",
                segment.id
            )));
        };
        let representation = self
            .representations
            .iter()
            .find(|(id, _)| id.as_str() == representation_id)
            .map(|(_, representation)| *representation)
            .ok_or_else(|| {
                error(format!(
                    "segment {} references unknown representation {representation_id}",
                    segment.id
                ))
            })?;
        if representation.owner != CorpusItemRef::Asset(segment.asset_id.clone()) {
            return Err(error(format!(
                "segment {} indexes representation {} that is not owned by its asset {}",
                segment.id, representation.id, segment.asset_id
            )));
        }
        if !matches!(
            representation.kind,
            RepresentationKind::ExtractedText
                | RepresentationKind::OcrText
                | RepresentationKind::Transcript
        ) {
            return Err(error(format!(
                "segment {} indexes representation {} that is not a text representation",
                segment.id, representation.id
            )));
        }
        Ok(representation)
    }

    fn map(
        &self,
        producer: SourceProducerV1,
        payloads: &BTreeMap<ContentHash, String>,
    ) -> Result<SourceSpanBatchV1, SourceSpanValidationError> {
        // Group text segments by asset, each with its resolved representation.
        let mut by_asset: BTreeMap<&AssetId, Vec<(&corpus_core::Segment, Span)>> = BTreeMap::new();
        let mut asset_representation: BTreeMap<&AssetId, &corpus_core::Representation> =
            BTreeMap::new();
        for segment in &self.segments {
            let Some(range) = segment.locator.character_range else {
                return Err(error(format!(
                    "segment {} has no character range; only text segments can become source spans",
                    segment.id
                )));
            };
            let representation = self.text_representation(segment)?;
            let previous = asset_representation.insert(&segment.asset_id, representation);
            if previous.is_some_and(|previous| previous.id != representation.id) {
                return Err(error(format!(
                    "asset {} has segments over more than one representation (segment {})",
                    segment.asset_id, segment.id
                )));
            }
            by_asset
                .entry(&segment.asset_id)
                .or_default()
                .push((segment, range));
        }

        let mut sources = Vec::new();
        let mut spans = Vec::new();
        for asset in &self.assets {
            let Some(mut segments) = by_asset.remove(&asset.id) else {
                continue;
            };
            let representation = asset_representation[&asset.id];
            let Some(text) = payloads.get(&representation.content_hash) else {
                return Err(error(format!(
                    "representation {} text is not carried in the batch (no payload for {})",
                    representation.id, representation.content_hash
                )));
            };
            let source = self.sources[&asset.source_id];
            let revision = format!("sha256:{}", representation.content_hash);

            let mut metadata = Metadata::new();
            metadata.insert(
                "corpusCoreSourceId".to_string(),
                Value::String(source.id.to_string()),
            );
            metadata.insert(
                "corpusCoreRepresentationId".to_string(),
                Value::String(representation.id.to_string()),
            );
            if let Some(hash) = &asset.content.hash {
                metadata.insert(
                    "assetContentHash".to_string(),
                    Value::String(format!("sha256:{hash}")),
                );
            }
            metadata.insert(
                "extractionProducer".to_string(),
                Value::String(representation.provenance.producer.clone()),
            );
            if let Some(version) = &representation.provenance.producer_version {
                metadata.insert(
                    "extractionProducerVersion".to_string(),
                    Value::String(version.clone()),
                );
            }

            sources.push(SourceRecordV1 {
                id: asset.id.to_string(),
                kind: asset_kind(&asset.kind),
                revision: revision.clone(),
                uri: Some(asset.content.uri.clone()),
                title: source.metadata.get("title").cloned(),
                creators: Vec::new(),
                language: None,
                content_hash: revision,
                metadata,
            });

            let scalar_offsets = text
                .char_indices()
                .map(|(offset, _)| offset)
                .chain(std::iter::once(text.len()))
                .collect::<Vec<_>>();
            segments.sort_by(|(left, left_range), (right, right_range)| {
                (left_range.start, left_range.end, &left.id).cmp(&(
                    right_range.start,
                    right_range.end,
                    &right.id,
                ))
            });
            for (sequence, (segment, range)) in segments.into_iter().enumerate() {
                spans.push(map_segment(
                    segment,
                    range,
                    &asset.id,
                    sequence as u64,
                    text,
                    &scalar_offsets,
                    representation,
                )?);
            }
        }

        Ok(SourceSpanBatchV1::new(producer, sources, spans))
    }
}

fn asset_kind(kind: &corpus_core::AssetKind) -> String {
    match kind {
        corpus_core::AssetKind::Other(name) => format!("other:{name}"),
        // Unit variants serialize as their snake_case name in corpus-core.
        known => serde_json::to_value(known)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_default(),
    }
}

fn map_segment(
    segment: &corpus_core::Segment,
    range: Span,
    asset_id: &AssetId,
    sequence: u64,
    text: &str,
    scalar_offsets: &[usize],
    representation: &corpus_core::Representation,
) -> Result<SourceSpanRecordV1, SourceSpanValidationError> {
    let scalars = scalar_offsets.len() - 1;
    let in_bounds = usize::try_from(range.end).is_ok_and(|end| end <= scalars);
    if !in_bounds || range.is_empty() {
        return Err(error(format!(
            "segment {} character range {}..{} is outside representation {} ({scalars} Unicode scalars) or empty",
            segment.id, range.start, range.end, representation.id
        )));
    }
    let byte_start = scalar_offsets[range.start as usize];
    let byte_end = scalar_offsets[range.end as usize];
    let span_text = text[byte_start..byte_end].to_string();

    let segment_metadata = &segment.metadata;
    let paragraph_ordinal = segment_metadata
        .get("paragraph_ordinal")
        .map(|value| {
            value.parse::<usize>().map_err(|_| {
                error(format!(
                    "segment {} has a non-numeric paragraph_ordinal {value:?}",
                    segment.id
                ))
            })
        })
        .transpose()?;
    let heading_path = segment_metadata
        .get("heading_path")
        .map(|value| heading_path(&segment.id, value))
        .transpose()?
        .unwrap_or_default();

    let mut metadata = Metadata::new();
    metadata.insert("characterStart".to_string(), Value::from(range.start));
    metadata.insert("characterEnd".to_string(), Value::from(range.end));
    if let Some(page_index) = segment.locator.page_index {
        metadata.insert("pageIndex".to_string(), Value::from(page_index));
    }

    Ok(SourceSpanRecordV1 {
        id: segment.id.to_string(),
        source_id: asset_id.to_string(),
        sequence,
        content_hash: format!("sha256:{}", ContentHash::from_bytes(span_text.as_bytes())),
        text: span_text,
        language: None,
        locator: SourceLocatorV1::Text {
            byte_start,
            byte_end,
            paragraph_ordinal,
            page: None,
            section: None,
            source_selector: segment_metadata.get("source_selector").cloned(),
            heading_path,
        },
        metadata,
    })
}

/// Parses the JSON-array heading path document-search stores in segment metadata. `null`
/// entries (headings above the first emitted level) carry no text and are dropped.
fn heading_path(
    segment_id: &SegmentId,
    value: &str,
) -> Result<Vec<String>, SourceSpanValidationError> {
    let invalid = || {
        error(format!(
            "segment {segment_id} heading_path must be a JSON array of strings"
        ))
    };
    let entries: Vec<Value> = serde_json::from_str(value).map_err(|_| invalid())?;
    entries
        .into_iter()
        .filter(|entry| !entry.is_null())
        .map(|entry| match entry {
            Value::String(heading) => Ok(heading),
            _ => Err(invalid()),
        })
        .collect()
}
