//! Acceptance tests for issue #8: intake of document-search's `moenarch_corpus_core_batch` v1
//! export through the real `moenarch-corpus-core` types into the existing source-span input.
//!
//! Contract under test:
//! `philosophy_extractor_source_ingestion::source_spans_from_corpus_core_jsonl(&str)
//!     -> Result<SourceSpanBatchV1, SourceSpanValidationError>`
//!
//! The fixture was produced by document-search's real exporter (see fixtures/corpus_core/README.md).
//! Every expected value below was derived independently with Python (hashlib, Unicode-scalar
//! slicing, UTF-8 byte offsets, corpus-core's length-delimited id derivation), never by running
//! the intake code.
//!
//! Mapping (one source-span source per corpus-core asset that owns text segments):
//! - producer: the batch header producer (`name`, `revision`).
//! - source `id` = corpus-core asset id; `kind` = asset kind as serialized by corpus-core;
//!   `uri` = asset content uri; `title` = source metadata `title`; `contentHash` and `revision` =
//!   `sha256:` + content hash of the extracted-text representation the segments index into;
//!   metadata keeps `corpusCoreSourceId`, `corpusCoreRepresentationId`, `assetContentHash`
//!   (`sha256:`-prefixed), `extractionProducer`, `extractionProducerVersion`.
//! - span `id` = corpus-core segment id; `sourceId` = asset id; `sequence` = zero-based position
//!   of the segment within its asset ordered by character range; `text` = representation text
//!   sliced by the Unicode-scalar `character_range`; `contentHash` = `sha256:` + SHA-256 of that
//!   text; locator `text` with UTF-8 `byteStart`/`byteEnd` in the representation text,
//!   `paragraphOrdinal` and `sourceSelector` from segment metadata, `headingPath` = the JSON array
//!   in segment metadata with `null` entries dropped; metadata keeps `characterStart` and
//!   `characterEnd` (the original corpus-core scalar range).
//! - order: sources in asset file order, spans grouped by source in source order then sequence.

use philosophy_extractor_schema::{PhilosophyCorpusInputV1, SourceLocatorV1, SourceSpanBatchV1};
use philosophy_extractor_source_ingestion::{
    source_spans_from_corpus_core_jsonl, validate_philosophy_corpus_input,
    validate_source_span_batch,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("fixtures/corpus_core/document-search-batch.jsonl");

const ETHICS_ASSET: &str = "asset:1130de43d60efbf8f61bfdc49b19451ed761d244ddf6dfaa9bd8761e349f15b5";
const MEDITATION_ASSET: &str =
    "asset:1dd96d5d7f36b7910a8d5167f08b5c48934a33ce9bcfff08fbcc8ea1c53e4c02";
const WEB_ASSET: &str = "asset:834dbe5f424c10c50b519c241bff22d763920ddcca50580a866d31b593c36003";
const MEDITATION_REPRESENTATION: &str =
    "representation:bfe5e6e54224057810c157a67ada0669611b84447bef05cff521dfa808b931c0";
const ETHICS_REPRESENTATION: &str =
    "representation:e6229248f7e571fdf582803a9102b1fe906f720fa56eeece2a674898731ae74d";
const MEDITATION_SEGMENT_1: &str =
    "segment:19218c8c57af1fd044c77a3ff21b8c7249f02eeb90353bb9e7198353315ec134";

struct ExpectedSource {
    id: &'static str,
    kind: &'static str,
    uri: &'static str,
    title: &'static str,
    corpus_source_id: &'static str,
    representation_id: &'static str,
    representation_hash: &'static str,
    asset_hash: &'static str,
}

struct ExpectedSpan {
    id: &'static str,
    source_id: &'static str,
    sequence: u64,
    text: &'static str,
    content_hash: &'static str,
    byte_start: usize,
    byte_end: usize,
    char_start: u64,
    char_end: u64,
    paragraph_ordinal: usize,
    source_selector: &'static str,
    heading_path: &'static [&'static str],
}

const EXPECTED_SOURCES: [ExpectedSource; 3] = [
    ExpectedSource {
        id: ETHICS_ASSET,
        kind: "text",
        uri: "ethics.txt",
        title: "ethics",
        corpus_source_id: "source:747a508aa301ae998bdf40d634807a143bb47353c1fcd3121f38bf5e52be48af",
        representation_id: ETHICS_REPRESENTATION,
        representation_hash: "afabd667c85fe7db1549e7fa80096a9ae29d7a5e93a861ae88c232c896211e70",
        asset_hash: "22f928dddaa82c885d0ea6d1c1e052373edfffd9071261ee442f6d44dbcade23",
    },
    ExpectedSource {
        id: MEDITATION_ASSET,
        kind: "text",
        uri: "meditation.md",
        title: "meditation",
        corpus_source_id: "source:1741566f80a0c2716c38f4a70df51387e8d5521fef2248a3b01fa50a179e713d",
        representation_id: MEDITATION_REPRESENTATION,
        representation_hash: "4f174ba3b3a7a55505b30ea55a430b248eb646c5f518cdb31f01c58520f3c91d",
        asset_hash: "1a15e389cec0391d26530fa4a1d5a9590bb59fb24b02448f8eef7d9181dd661a",
    },
    ExpectedSource {
        id: WEB_ASSET,
        kind: "web_page",
        uri: "https://example.test/essay",
        title: "Remote Essay",
        corpus_source_id: "source:474bf16046a6c338affc60eac0d392c4cb4320d606fee7caab448b4a79b18c50",
        representation_id: "representation:dc1992e89cb326eff9a755237d145f0afdf2afa06d539da13362a814da6f449d",
        representation_hash: "058c130dd87eae7a58f49aba4c539c8278f3b837b1177efce4315bbbbf617a64",
        asset_hash: "1b6a90a365cb90ec462500111ce4480c4da8e48194eb440f815b77bdb55df779",
    },
];

const EXPECTED_SPANS: [ExpectedSpan; 7] = [
    ExpectedSpan {
        id: "segment:3bb622591f64567489aeb0af42f40d88b75b0f86876f471456fc458a4b893454",
        source_id: ETHICS_ASSET,
        sequence: 0,
        text: "Virtue is a mean between two vices.",
        content_hash: "20c4991d89e91befc7c7d295a72d8f00c0ecf38eddc2095896909526ea867c7e",
        byte_start: 0,
        byte_end: 35,
        char_start: 0,
        char_end: 35,
        paragraph_ordinal: 0,
        source_selector: "p",
        heading_path: &[],
    },
    ExpectedSpan {
        id: "segment:5481becbfff056c552f0ace8842f2c9aea1f081b1e07214dfafd62ce31418c97",
        source_id: ETHICS_ASSET,
        sequence: 1,
        text: "Happiness is activity in accordance with virtue.",
        content_hash: "98e09f904134415a603658463bc634e9c5ceb00d0e344a7e751eed710f200328",
        byte_start: 37,
        byte_end: 85,
        char_start: 37,
        char_end: 85,
        paragraph_ordinal: 1,
        source_selector: "p",
        heading_path: &[],
    },
    ExpectedSpan {
        id: "segment:832a68cdc002fc69c60c00910fdd50d6c0a2bcf15631a11b3b726ab38e2d1551",
        source_id: MEDITATION_ASSET,
        sequence: 0,
        text: "Méditation 𝄞",
        content_hash: "d2a481f91c9234ec700745b9c629828468210e74b4d06171ab77d0a4f8612135",
        byte_start: 0,
        byte_end: 16,
        char_start: 0,
        char_end: 12,
        paragraph_ordinal: 0,
        source_selector: "h1",
        heading_path: &["Méditation 𝄞"],
    },
    ExpectedSpan {
        id: MEDITATION_SEGMENT_1,
        source_id: MEDITATION_ASSET,
        sequence: 1,
        text: "I think, therefore I am — cogito 😀 ergo sum.",
        content_hash: "32d2a09017ffa8eed88d9bfd946c90a5a3659afe50e27b9a78d96c6382979b91",
        byte_start: 18,
        byte_end: 67,
        char_start: 14,
        char_end: 58,
        paragraph_ordinal: 1,
        source_selector: "p",
        heading_path: &["Méditation 𝄞"],
    },
    ExpectedSpan {
        id: "segment:aaf3e88a754831eb342e4b52f239561bff9eeb8eda70c6b7863159ef8b0f2cf6",
        source_id: MEDITATION_ASSET,
        sequence: 2,
        text: "Doubt is the beginning of wisdom.",
        content_hash: "6a90c6df3eb32162091afb0c0e0ea5670d4fa0e59ae233140dd83698231a1cbe",
        byte_start: 69,
        byte_end: 102,
        char_start: 60,
        char_end: 93,
        paragraph_ordinal: 2,
        source_selector: "p",
        heading_path: &["Méditation 𝄞"],
    },
    ExpectedSpan {
        id: "segment:14226abbf8eb1fdccd7ea5b00268bd17ef87fb8ff861cddb188dbc4e013ff041",
        source_id: WEB_ASSET,
        sequence: 0,
        text: "On Truth",
        content_hash: "f32c24dc6f91fb30440653a3ddc721ddb2857abd87c9b08586c983ceb64e4cdd",
        byte_start: 0,
        byte_end: 8,
        char_start: 0,
        char_end: 8,
        paragraph_ordinal: 0,
        source_selector: "h2",
        heading_path: &["On Truth"],
    },
    ExpectedSpan {
        id: "segment:c4531ac2e3937d05edd5dd2c2706e26486abcafc2b38b176c9211ab88c2e2870",
        source_id: WEB_ASSET,
        sequence: 1,
        text: "Truth is correspondence ✓.",
        content_hash: "86080d9aaafa982fd63a4ce7905dfdcb14c4a05c1618f2e0fb34f4f1f663754e",
        byte_start: 10,
        byte_end: 38,
        char_start: 10,
        char_end: 36,
        paragraph_ordinal: 1,
        source_selector: "p",
        heading_path: &["On Truth"],
    },
];

fn sha(hex: &str) -> String {
    format!("sha256:{hex}")
}

fn intake(jsonl: &str) -> SourceSpanBatchV1 {
    source_spans_from_corpus_core_jsonl(jsonl).expect("fixture must be accepted")
}

// ---------------------------------------------------------------------------
// Fixture mutation helpers for negative cases.
// ---------------------------------------------------------------------------

fn lines() -> Vec<Value> {
    FIXTURE
        .lines()
        .map(|line| serde_json::from_str(line).expect("fixture line is JSON"))
        .collect()
}

fn render(lines: &[Value]) -> String {
    let mut text = lines
        .iter()
        .map(|line| serde_json::to_string(line).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    text.push('\n');
    text
}

/// Mutates the record (the inner `record.record` object) with the given id.
fn with_record(id: &str, mutate: impl FnOnce(&mut Value)) -> String {
    let mut lines = lines();
    let record = lines
        .iter_mut()
        .filter_map(|line| line.get_mut("record"))
        .map(|record| &mut record["record"])
        .find(|record| record["id"] == id)
        .unwrap_or_else(|| panic!("fixture has record {id}"));
    mutate(record);
    render(&lines)
}

fn rejected(jsonl: &str) -> String {
    match source_spans_from_corpus_core_jsonl(jsonl) {
        Ok(batch) => panic!("expected rejection, got {} spans", batch.spans.len()),
        Err(error) => error.to_string(),
    }
}

fn assert_mentions(message: &str, needles: &[&str]) {
    let lower = message.to_lowercase();
    for needle in needles {
        assert!(
            lower.contains(&needle.to_lowercase()),
            "error {message:?} should mention {needle:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Positive mapping.
// ---------------------------------------------------------------------------

#[test]
fn maps_the_document_search_export_into_source_spans_preserving_identity() {
    let batch = intake(FIXTURE);

    assert_eq!(batch.producer.name, "document-search");
    assert_eq!(batch.producer.revision, "git:28463de");

    assert_eq!(batch.sources.len(), EXPECTED_SOURCES.len());
    for (source, expected) in batch.sources.iter().zip(&EXPECTED_SOURCES) {
        assert_eq!(source.id, expected.id);
        assert_eq!(source.kind, expected.kind, "kind of {}", expected.id);
        assert_eq!(source.uri.as_deref(), Some(expected.uri));
        assert_eq!(source.title.as_deref(), Some(expected.title));
        assert!(source.creators.is_empty());
        assert_eq!(source.language, None);
        assert_eq!(source.content_hash, sha(expected.representation_hash));
        assert_eq!(source.revision, sha(expected.representation_hash));
        for (key, value) in [
            ("corpusCoreSourceId", expected.corpus_source_id.to_string()),
            (
                "corpusCoreRepresentationId",
                expected.representation_id.to_string(),
            ),
            ("assetContentHash", sha(expected.asset_hash)),
            (
                "extractionProducer",
                "document-search/html-extraction".to_string(),
            ),
            ("extractionProducerVersion", "git:28463de".to_string()),
        ] {
            assert_eq!(
                source.metadata.get(key),
                Some(&Value::String(value)),
                "source {} metadata {key}",
                expected.id
            );
        }
    }

    assert_eq!(batch.spans.len(), EXPECTED_SPANS.len());
    for (span, expected) in batch.spans.iter().zip(&EXPECTED_SPANS) {
        assert_eq!(span.id, expected.id);
        assert_eq!(span.source_id, expected.source_id, "span {}", expected.id);
        assert_eq!(span.sequence, expected.sequence, "span {}", expected.id);
        assert_eq!(span.text, expected.text);
        assert_eq!(span.content_hash, sha(expected.content_hash));
        assert_eq!(span.language, None);
        assert_eq!(
            span.locator,
            SourceLocatorV1::Text {
                byte_start: expected.byte_start,
                byte_end: expected.byte_end,
                paragraph_ordinal: Some(expected.paragraph_ordinal),
                page: None,
                section: None,
                source_selector: Some(expected.source_selector.to_string()),
                heading_path: expected
                    .heading_path
                    .iter()
                    .map(|entry| entry.to_string())
                    .collect(),
            },
            "locator of {}",
            expected.id
        );
        assert_eq!(
            span.metadata.get("characterStart"),
            Some(&json!(expected.char_start))
        );
        assert_eq!(
            span.metadata.get("characterEnd"),
            Some(&json!(expected.char_end))
        );
    }
}

#[test]
fn mapped_batch_is_valid_interpretation_input() {
    let batch = intake(FIXTURE);
    validate_source_span_batch(&batch).unwrap();
    validate_philosophy_corpus_input(&PhilosophyCorpusInputV1::new(batch, Vec::new())).unwrap();
}

#[test]
fn byte_locators_address_the_span_text_inside_the_representation_text() {
    // Independent of the expected table: the representation payload text from the fixture,
    // sliced by the produced UTF-8 byte locator, must equal the span text.
    let payloads = lines()
        .into_iter()
        .filter_map(|line| line.get("payload").cloned())
        .map(|payload| {
            (
                payload["hash"].as_str().unwrap().to_string(),
                payload["text"].as_str().unwrap().to_string(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let batch = intake(FIXTURE);
    for span in &batch.spans {
        let source = batch
            .sources
            .iter()
            .find(|s| s.id == span.source_id)
            .unwrap();
        let text = &payloads[source.content_hash.strip_prefix("sha256:").unwrap()];
        let SourceLocatorV1::Text {
            byte_start,
            byte_end,
            ..
        } = span.locator
        else {
            panic!("text locator expected");
        };
        assert_eq!(&text[byte_start..byte_end], span.text, "span {}", span.id);
    }
}

#[test]
fn intake_is_deterministic() {
    let first = intake(FIXTURE);
    let second = intake(FIXTURE);
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
}

#[test]
fn payload_and_segment_line_order_do_not_change_the_output() {
    // Reverse the payload lines and the order of segment lines within the file: sources keep
    // asset file order and spans are ordered by character range, so output is identical.
    let original = lines();
    let header = original[0].clone();
    let mut non_segments = Vec::new();
    let mut segments = Vec::new();
    let mut payloads = Vec::new();
    for line in original.into_iter().skip(1) {
        if line.get("payload").is_some() {
            payloads.push(line);
        } else if line["record"]["record_type"] == "segment" {
            segments.push(line);
        } else {
            non_segments.push(line);
        }
    }
    segments.reverse();
    payloads.reverse();
    let mut shuffled = vec![header];
    shuffled.extend(non_segments);
    shuffled.extend(segments);
    shuffled.extend(payloads);

    assert_eq!(intake(&render(&shuffled)), intake(FIXTURE));
}

// ---------------------------------------------------------------------------
// Negative cases: each must be rejected with an error that names the problem.
// ---------------------------------------------------------------------------

#[test]
fn rejects_unknown_schema_version() {
    let mut lines = lines();
    lines[0]["schema_version"] = json!(2);
    let message = rejected(&render(&lines));
    assert_mentions(&message, &["unsupported", "2"]);
}

#[test]
fn rejects_unknown_schema_name() {
    let mut lines = lines();
    lines[0]["schema"] = json!("source_span_interchange");
    let message = rejected(&render(&lines));
    assert_mentions(&message, &["unsupported"]);
}

#[test]
fn rejects_a_missing_producer_revision() {
    let mut lines = lines();
    lines[0]["producer"]["revision"] = json!("  ");
    let message = rejected(&render(&lines));
    assert_mentions(&message, &["revision"]);
}

#[test]
fn rejects_malformed_lines() {
    assert!(source_spans_from_corpus_core_jsonl("").is_err());
    let mut lines = lines();
    lines[1]["payload"] = json!({"hash": "0".repeat(64), "media_type": "text/plain", "text": ""});
    rejected(&render(&lines));
    rejected(&format!("{FIXTURE}not json\n"));
}

#[test]
fn rejects_a_segment_with_a_dangling_asset_reference() {
    let jsonl = with_record(MEDITATION_SEGMENT_1, |segment| {
        segment["asset_id"] = json!("asset:missing");
    });
    assert_mentions(&rejected(&jsonl), &["unknown", "asset:missing"]);
}

#[test]
fn rejects_an_asset_with_a_dangling_source_reference() {
    let jsonl = with_record(MEDITATION_ASSET, |asset| {
        asset["source_id"] = json!("source:missing");
    });
    assert_mentions(&rejected(&jsonl), &["unknown", "source:missing"]);
}

#[test]
fn rejects_a_segment_with_a_dangling_representation_reference() {
    let jsonl = with_record(MEDITATION_SEGMENT_1, |segment| {
        segment["locator"]["selectors"]["representation_id"] = json!("representation:missing");
    });
    assert_mentions(&rejected(&jsonl), &["unknown", "representation:missing"]);
}

#[test]
fn rejects_a_representation_with_a_dangling_owner() {
    let jsonl = with_record(MEDITATION_REPRESENTATION, |representation| {
        representation["owner"]["id"] = json!("asset:missing");
    });
    assert_mentions(&rejected(&jsonl), &["unknown", "asset:missing"]);
}

#[test]
fn rejects_a_segment_pointing_at_another_assets_representation() {
    let jsonl = with_record(MEDITATION_SEGMENT_1, |segment| {
        segment["locator"]["selectors"]["representation_id"] = json!(ETHICS_REPRESENTATION);
    });
    assert_mentions(&rejected(&jsonl), &[MEDITATION_SEGMENT_1]);
}

#[test]
fn rejects_duplicate_record_ids() {
    let mut lines = lines();
    let duplicate = lines
        .iter()
        .find(|line| line["record"]["record"]["id"] == MEDITATION_SEGMENT_1)
        .unwrap()
        .clone();
    lines.insert(1, duplicate);
    assert_mentions(
        &rejected(&render(&lines)),
        &["duplicate", MEDITATION_SEGMENT_1],
    );
}

#[test]
fn rejects_a_span_beyond_the_representation_text() {
    // The meditation representation text is 93 Unicode scalars long.
    let jsonl = with_record(MEDITATION_SEGMENT_1, |segment| {
        segment["locator"]["character_range"] = json!({"start": 60, "end": 94});
    });
    assert_mentions(&rejected(&jsonl), &[MEDITATION_SEGMENT_1, "range"]);
}

#[test]
fn rejects_a_reversed_span() {
    let jsonl = with_record(MEDITATION_SEGMENT_1, |segment| {
        segment["locator"]["character_range"] = json!({"start": 58, "end": 14});
    });
    assert_mentions(&rejected(&jsonl), &[MEDITATION_SEGMENT_1, "range"]);
}

#[test]
fn rejects_a_payload_whose_text_does_not_match_its_hash() {
    let mut lines = lines();
    let payload = lines
        .iter_mut()
        .filter_map(|line| line.get_mut("payload"))
        .find(|payload| {
            payload["hash"] == "4f174ba3b3a7a55505b30ea55a430b248eb646c5f518cdb31f01c58520f3c91d"
        })
        .unwrap();
    payload["text"] = json!(
        "Méditation 𝄞\n\nI think, therefore I am — cogito 😀 ergo sum.\n\nDoubt is the end of wisdom."
    );
    assert_mentions(&rejected(&render(&lines)), &["hash"]);
}

#[test]
fn rejects_a_non_canonical_content_hash() {
    let jsonl = with_record(MEDITATION_REPRESENTATION, |representation| {
        representation["content_hash"] =
            json!("4F174BA3B3A7A55505B30EA55A430B248EB646C5F518CDB31F01C58520F3C91D");
    });
    assert_mentions(&rejected(&jsonl), &["hash"]);
}

#[test]
fn rejects_segments_whose_representation_text_is_not_carried() {
    // A valid-looking hash with no payload: the span text cannot be recovered or verified.
    let jsonl = with_record(MEDITATION_REPRESENTATION, |representation| {
        representation["content_hash"] = json!("0".repeat(64));
    });
    assert_mentions(&rejected(&jsonl), &[MEDITATION_REPRESENTATION]);
}

#[test]
fn rejects_values_the_corpus_core_serde_model_rejects() {
    // corpus-core's `AssetKind` is a closed enum (unknown families use `{"other": ...}`) and its
    // metadata is string-to-string; a loose local mirror would accept both of these.
    let unknown_kind = with_record(MEDITATION_ASSET, |asset| asset["kind"] = json!("hologram"));
    rejected(&unknown_kind);
    let non_string_metadata = with_record(MEDITATION_ASSET, |asset| {
        asset["metadata"]["content_basis"] = json!(5)
    });
    rejected(&non_string_metadata);
    let mut lines = lines();
    lines[1]["record"]["record_type"] = json!("collection");
    rejected(&render(&lines));
}

// ---------------------------------------------------------------------------
// Architecture guard: corpus identity comes from moenarch-corpus-core, not a local mirror.
// ---------------------------------------------------------------------------

const CORPUS_CORE_TYPES: [&str; 12] = [
    "Source",
    "Asset",
    "Segment",
    "Representation",
    "SegmentLocator",
    "ContentRef",
    "ContentHash",
    "Provenance",
    "CorpusRecord",
    "CorpusItemRef",
    "SourceKind",
    "AssetKind",
];

fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_local_mirror_of_corpus_core_types() {
    let packages = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for package in ["source-ingestion", "schema"] {
        rust_files(&packages.join(package).join("src"), &mut files);
    }
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for line in text.lines() {
            let tokens = line
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .collect::<Vec<_>>();
            for pair in tokens.windows(2) {
                if matches!(pair[0], "struct" | "enum" | "type") {
                    assert!(
                        !CORPUS_CORE_TYPES.contains(&pair[1]),
                        "{} defines `{} {}`; use moenarch-corpus-core's type instead",
                        file.display(),
                        pair[0],
                        pair[1]
                    );
                }
            }
        }
    }

    let mut ingestion_src = Vec::new();
    rust_files(
        &packages.join("source-ingestion").join("src"),
        &mut ingestion_src,
    );
    let uses_corpus_core = ingestion_src.iter().any(|file| {
        let text = std::fs::read_to_string(file).unwrap();
        text.contains("corpus_core::") && text.contains("CorpusRecord")
    });
    assert!(
        uses_corpus_core,
        "source-ingestion must deserialize into corpus_core::CorpusRecord"
    );
}

#[test]
fn corpus_core_is_an_exact_rev_git_dependency_on_public_foundation() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let declaration = manifest
        .lines()
        .find(|line| {
            line.contains("\"moenarch-corpus-core\"") || line.starts_with("moenarch-corpus-core")
        })
        .expect("workspace declares moenarch-corpus-core");
    assert!(
        declaration.contains("git = \"https://github.com/moritzbrantner/moenarch-foundation\""),
        "{declaration}"
    );
    assert!(
        !declaration.contains("path ="),
        "no path dependency: {declaration}"
    );
    assert!(
        !declaration.contains("version ="),
        "no registry version: {declaration}"
    );
    assert!(
        !declaration.contains("branch =") && !declaration.contains("tag ="),
        "{declaration}"
    );
    let rev = declaration
        .split("rev = \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("exact rev");
    assert!(
        rev.len() == 40
            && rev
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "rev must be a full commit sha: {rev}"
    );

    let ingestion = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    assert!(
        ingestion.contains("corpus-core"),
        "source-ingestion depends on corpus-core"
    );

    let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();
    let expected_source = format!(
        "source = \"git+https://github.com/moritzbrantner/moenarch-foundation?rev={rev}#{rev}\""
    );
    let block = lock
        .split("[[package]]")
        .find(|block| block.contains("name = \"moenarch-corpus-core\""))
        .expect("Cargo.lock resolves moenarch-corpus-core");
    assert!(block.contains(&expected_source), "lock entry: {block}");
    assert_eq!(
        lock.matches("name = \"moenarch-corpus-core\"").count(),
        1,
        "exactly one resolved corpus-core"
    );
}
