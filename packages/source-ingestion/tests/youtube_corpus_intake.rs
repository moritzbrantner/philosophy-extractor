//! Issue #8: youtube-corpus's `moenarch_corpus_core_batch` v1 export goes through the same
//! corpus-core intake as document-search, keeping transcript timing as timed locators.
//!
//! The fixture is youtube-corpus's committed expected export
//! (`tests/fixtures/exports/expected/corpus-core.jsonl`), copied unchanged.

use philosophy_extractor_schema::{SourceLocatorV1, SourceSpanRecordV1};
use philosophy_extractor_source_ingestion::source_spans_from_corpus_core_jsonl;
use serde_json::Value;

const FIXTURE: &str = include_str!("fixtures/corpus_core/youtube-corpus-batch.jsonl");
const VIDEO_ID: &str = "11111111-1111-4111-8111-111111111111";

fn span<'a>(spans: &'a [SourceSpanRecordV1], text: &str) -> &'a SourceSpanRecordV1 {
    spans
        .iter()
        .find(|span| span.text == text)
        .unwrap_or_else(|| panic!("no span with text {text:?}"))
}

#[test]
fn transcript_segments_keep_their_timing_and_video() {
    let batch = source_spans_from_corpus_core_jsonl(FIXTURE).expect("youtube-corpus batch");

    let first = span(&batch.spans, "so the question is what being means");
    assert_eq!(
        first.locator,
        SourceLocatorV1::Timed {
            segment_index: 0,
            start_seconds: Some(0.0),
            end_seconds: Some(3.125),
        }
    );
    assert_eq!(first.language.as_deref(), Some("en"));
    assert_eq!(first.metadata["videoId"], Value::from(VIDEO_ID));
    // The producer's own per-segment text hash matches the recovered span text.
    assert_eq!(
        first.content_hash,
        "sha256:6928f1c1f8acfe5cc2b8e7ad15b286e1e2d908fd7447b6e8471492bac3ace413"
    );
    assert_eq!(first.metadata["byteStart"], Value::from(0));
    assert_eq!(first.metadata["byteEnd"], Value::from(35));

    let second = span(&batch.spans, "essence comes first");
    assert_eq!(
        second.locator,
        SourceLocatorV1::Timed {
            segment_index: 1,
            start_seconds: Some(3.125),
            end_seconds: Some(6.0),
        }
    );

    let source = batch
        .sources
        .iter()
        .find(|source| source.id == first.source_id)
        .unwrap();
    assert_eq!(source.language.as_deref(), Some("en"));
    assert_eq!(
        source.title.as_deref(),
        Some("Aquinas on Being — Lecture 1")
    );
}

#[test]
fn untimed_transcript_segments_stay_timed_without_seconds() {
    let batch = source_spans_from_corpus_core_jsonl(FIXTURE).unwrap();

    let open_ended = span(&batch.spans, "and existence follows.");
    assert_eq!(
        open_ended.locator,
        SourceLocatorV1::Timed {
            segment_index: 2,
            start_seconds: None,
            end_seconds: None,
        }
    );
}

#[test]
fn every_youtube_span_is_timed() {
    let batch = source_spans_from_corpus_core_jsonl(FIXTURE).unwrap();

    assert!(!batch.spans.is_empty());
    for span in &batch.spans {
        assert!(
            matches!(span.locator, SourceLocatorV1::Timed { .. }),
            "span {} ({:?}) is not timed",
            span.id,
            span.text
        );
        assert!(
            span.metadata["videoId"].is_string(),
            "span {} has no video",
            span.id
        );
    }
    let seminar = batch
        .sources
        .iter()
        .find(|source| source.title.as_deref() == Some("Seminar (local recording)"))
        .expect("local recording source");
    let seminar_spans = batch
        .spans
        .iter()
        .filter(|span| span.source_id == seminar.id)
        .collect::<Vec<_>>();
    assert!(!seminar_spans.is_empty());
    for span in seminar_spans {
        assert_eq!(
            span.metadata["videoId"],
            Value::from("22222222-2222-4222-8222-222222222222")
        );
    }
}
