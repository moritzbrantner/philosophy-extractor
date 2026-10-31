use super::{PhilosophyExtractor, PipelineConfig};
use crate::model::{
    MediaBoundingBoxV1, MediaEvidenceBatchV1, MediaEvidenceProducerV1, MediaEvidenceVideoV1,
    OcrTrackEvidenceV1, PhilosophyCorpusInputV1, ProcessingEvidenceV1, SourceLocatorV1,
    SourceProducerV1, SourceRecordV1, SourceSpanBatchV1, SourceSpanRecordV1,
    SponsorBlockEvidenceV1, SponsorBlockSegmentEvidenceV1,
};
use serde_json::Value;
use std::collections::BTreeMap;

fn hash(ch: char) -> String {
    format!("sha256:{}", ch.to_string().repeat(64))
}

fn processing(run_id: &str) -> ProcessingEvidenceV1 {
    ProcessingEvidenceV1 {
        run_id: run_id.to_string(),
        processor: "fixture".to_string(),
        processor_version: "1".to_string(),
        model: "fixture-model".to_string(),
        model_version: "1".to_string(),
        input_hash: hash('e'),
        config_hash: hash('f'),
        processing_config: serde_json::json!({}),
    }
}

#[test]
fn span_native_extraction_preserves_and_aligns_media_evidence() {
    let mut source_metadata = BTreeMap::new();
    source_metadata.insert("videoId".to_string(), Value::String("video-1".to_string()));
    let sources = SourceSpanBatchV1::new(
        SourceProducerV1 {
            name: "youtube-corpus".to_string(),
            revision: "git:producer".to_string(),
        },
        vec![SourceRecordV1 {
            id: "stream-1".to_string(),
            kind: "youtube_transcript".to_string(),
            revision: hash('a'),
            uri: Some("https://youtube.test/watch?v=fixture".to_string()),
            title: Some("Fixture lecture".to_string()),
            creators: vec!["Lecturer".to_string()],
            language: Some("en".to_string()),
            content_hash: hash('b'),
            metadata: BTreeMap::new(),
        }],
        vec![SourceSpanRecordV1 {
            id: "segment-1".to_string(),
            source_id: "stream-1".to_string(),
            sequence: 0,
            text: "Every change requires an actual cause.".to_string(),
            content_hash: hash('c'),
            language: Some("en".to_string()),
            locator: SourceLocatorV1::Timed {
                segment_index: 0,
                start_seconds: Some(1.0),
                end_seconds: Some(5.0),
            },
            metadata: source_metadata,
        }],
    );

    let media = MediaEvidenceBatchV1 {
        schema: crate::model::MEDIA_EVIDENCE_SCHEMA.to_string(),
        schema_version: crate::model::MEDIA_EVIDENCE_VERSION_V1,
        producer: MediaEvidenceProducerV1 {
            name: "youtube-corpus".to_string(),
            revision: "git:evidence".to_string(),
        },
        video: MediaEvidenceVideoV1 {
            id: "video-1".to_string(),
            youtube_id: Some("fixture".to_string()),
            source_url: "https://youtube.test/watch?v=fixture".to_string(),
            title: Some("Fixture lecture".to_string()),
        },
        revision: hash('d'),
        scenes: vec![crate::model::SceneEvidenceV1 {
            id: "scene-1".to_string(),
            scene_index: 0,
            start_frame: 0,
            end_frame: 300,
            start_seconds: 0.0,
            end_seconds: 10.0,
            metadata: serde_json::json!({}),
            provenance: processing("scene-run"),
        }],
        ocr_observations: Vec::new(),
        ocr_tracks: vec![OcrTrackEvidenceV1 {
            id: "ocr-1".to_string(),
            text: "Act and potency".to_string(),
            role: "presentation_slide".to_string(),
            language: Some("en".to_string()),
            sample_count: 1,
            start_frame: Some(60),
            end_frame: Some(120),
            start_seconds: Some(2.0),
            end_seconds: Some(4.0),
            region: Some(MediaBoundingBoxV1 {
                x: 10,
                y: 10,
                width: 100,
                height: 30,
            }),
            metadata: serde_json::json!({}),
            observation_ids: Vec::new(),
            scene_ids: vec!["scene-1".to_string()],
            provenance: processing("ocr-run"),
        }],
        sponsorblock: Some(SponsorBlockEvidenceV1 {
            snapshot_id: "snapshot-1".to_string(),
            response_hash: hash('9'),
            data_license: "CC BY-NC-SA 4.0".to_string(),
            attribution: "SponsorBlock".to_string(),
            categories: vec!["intro".to_string()],
            segments: vec![SponsorBlockSegmentEvidenceV1 {
                uuid: "sb-1".to_string(),
                category: "intro".to_string(),
                action_type: None,
                start_seconds: 0.0,
                end_seconds: 2.0,
                video_duration: Some(600.0),
                metadata: serde_json::json!({}),
            }],
        }),
    };

    let extractor = PhilosophyExtractor::new(PipelineConfig {
        persist_artifacts: false,
        ..PipelineConfig::default()
    });
    let response = extractor
        .extract_source_spans(PhilosophyCorpusInputV1::new(sources, vec![media]))
        .unwrap();

    assert_eq!(response.fragments[0].id, "segment-1");
    assert_eq!(response.media_evidence_revisions, vec![hash('d')]);
    let candidate = response.candidates.first().expect("candidate");
    let contexts = candidate.metadata["mediaContext"].as_array().unwrap();
    assert_eq!(contexts[0]["scenes"][0]["id"], "scene-1");
    assert_eq!(contexts[0]["ocrTracks"][0]["id"], "ocr-1");
    assert_eq!(contexts[0]["sponsorBlockSegments"][0]["category"], "intro");
    assert_eq!(
        contexts[0]["sponsorBlockProvenance"]["dataLicense"],
        "CC BY-NC-SA 4.0"
    );
}
