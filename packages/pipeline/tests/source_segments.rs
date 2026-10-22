//! Issue #18: proposition candidates keep the corpus-core segment ids they came from, plus the
//! media time span of timed segments, so consumers can join them to speaker turns.

use philosophy_extractor::model::{PropositionCandidate, SourceSegmentRef, SourceSpanBatchV1};
use philosophy_extractor::{PhilosophyCorpusInputV1, PhilosophyExtractor, PipelineConfig};
use philosophy_extractor_source_ingestion::source_spans_from_corpus_core_jsonl;

const FIXTURE: &str =
    include_str!("../../source-ingestion/tests/fixtures/corpus_core/youtube-corpus-batch.jsonl");

fn batch() -> SourceSpanBatchV1 {
    source_spans_from_corpus_core_jsonl(FIXTURE).expect("youtube-corpus batch")
}

fn segment_id(batch: &SourceSpanBatchV1, text: &str) -> String {
    batch
        .spans
        .iter()
        .find(|span| span.text == text)
        .unwrap_or_else(|| panic!("no span with text {text:?}"))
        .id
        .clone()
}

fn extract(batch: SourceSpanBatchV1) -> Vec<PropositionCandidate> {
    PhilosophyExtractor::new(PipelineConfig::default())
        .extract_source_spans(PhilosophyCorpusInputV1::new(batch, Vec::new()))
        .expect("span extraction")
        .candidates
}

fn candidate<'a>(candidates: &'a [PropositionCandidate], text: &str) -> &'a PropositionCandidate {
    candidates
        .iter()
        .find(|candidate| candidate.canonical_text.contains(text))
        .unwrap_or_else(|| panic!("no candidate containing {text:?}"))
}

#[test]
fn timed_segments_reach_proposition_candidates() {
    let batch = batch();
    let first = segment_id(&batch, "so the question is what being means");
    assert!(first.starts_with("segment:"));

    let candidates = extract(batch);
    let candidate = candidate(&candidates, "what being means");
    assert_eq!(
        candidate.source_segments,
        vec![SourceSegmentRef {
            segment_id: first.clone(),
            start_seconds: Some(0.0),
            end_seconds: Some(3.125),
        }]
    );

    let json = serde_json::to_value(candidate).unwrap();
    assert_eq!(json["sourceSegments"][0]["segmentId"], first.as_str());
    assert_eq!(json["sourceSegments"][0]["startSeconds"], 0.0);
    assert_eq!(json["sourceSegments"][0]["endSeconds"], 3.125);
}

#[test]
fn a_candidate_from_several_segments_lists_all_of_them() {
    let mut batch = batch();
    let first = segment_id(&batch, "so the question is what being means");
    let repeat = segment_id(&batch, "essence comes first");
    // The speaker repeats the first sentence at 3.125–6.0s; deduplication merges both spans.
    let span = batch
        .spans
        .iter_mut()
        .find(|span| span.id == repeat)
        .unwrap();
    span.text = "so the question is what being means".to_string();

    let candidates = extract(batch);
    let candidate = candidate(&candidates, "what being means");
    assert_eq!(
        candidate.source_segments,
        vec![
            SourceSegmentRef {
                segment_id: first,
                start_seconds: Some(0.0),
                end_seconds: Some(3.125),
            },
            SourceSegmentRef {
                segment_id: repeat,
                start_seconds: Some(3.125),
                end_seconds: Some(6.0),
            },
        ]
    );
}

#[test]
fn spans_outside_corpus_core_carry_no_segments() {
    let mut batch = batch();
    for span in &mut batch.spans {
        span.metadata.remove("corpusCoreSegmentRevision");
    }

    let candidates = extract(batch);
    assert!(!candidates.is_empty());
    for candidate in &candidates {
        assert!(candidate.source_segments.is_empty());
        let json = serde_json::to_value(candidate).unwrap();
        assert!(json.get("sourceSegments").is_none());
    }
}
