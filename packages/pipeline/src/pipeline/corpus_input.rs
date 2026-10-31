use crate::model::{
    PhilosophyCorpusInputV1, PropositionCandidate, SourceLocatorV1, SourceSegmentRef,
};
use serde_json::Value;
use std::collections::HashMap;

/// Records the corpus-core segments each candidate came from. Corpus-core intake keeps the
/// segment id as the span id and marks the span with `corpusCoreSegmentRevision`; spans from
/// other producers carry no segment reference.
pub fn add_source_segments(
    candidates: &mut [PropositionCandidate],
    input: &PhilosophyCorpusInputV1,
) {
    let spans_by_id = input
        .sources
        .spans
        .iter()
        .map(|span| (span.id.as_str(), span))
        .collect::<HashMap<_, _>>();
    for candidate in candidates {
        candidate.source_segments = candidate
            .source_fragment_ids
            .iter()
            .filter_map(|fragment_id| spans_by_id.get(fragment_id.as_str()))
            .filter(|span| span.metadata.contains_key("corpusCoreSegmentRevision"))
            .map(|span| {
                let (start_seconds, end_seconds) = match span.locator {
                    SourceLocatorV1::Timed {
                        start_seconds,
                        end_seconds,
                        ..
                    } => (start_seconds, end_seconds),
                    SourceLocatorV1::Text { .. } => (None, None),
                };
                SourceSegmentRef {
                    segment_id: span.id.clone(),
                    start_seconds,
                    end_seconds,
                }
            })
            .collect();
    }
}

pub fn add_media_context(candidates: &mut [PropositionCandidate], input: &PhilosophyCorpusInputV1) {
    let spans_by_id = input
        .sources
        .spans
        .iter()
        .map(|span| (span.id.as_str(), span))
        .collect::<HashMap<_, _>>();
    let evidence_by_video = input
        .media_evidence
        .iter()
        .map(|evidence| (evidence.video.id.as_str(), evidence))
        .collect::<HashMap<_, _>>();

    for candidate in candidates {
        let mut contexts = Vec::new();
        for fragment_id in &candidate.source_fragment_ids {
            let Some(span) = spans_by_id.get(fragment_id.as_str()) else {
                continue;
            };
            let Some(video_id) = span.metadata.get("videoId").and_then(Value::as_str) else {
                continue;
            };
            let Some(evidence) = evidence_by_video.get(video_id) else {
                continue;
            };

            let bounds = timed_locator_bounds(&span.locator);
            let scenes = bounds.map_or_else(Vec::new, |(start, end)| {
                evidence
                    .scenes
                    .iter()
                    .filter(|scene| {
                        ranges_overlap(start, end, scene.start_seconds, scene.end_seconds)
                    })
                    .map(|scene| {
                        serde_json::json!({
                            "id": scene.id,
                            "sceneIndex": scene.scene_index,
                            "startSeconds": scene.start_seconds,
                            "endSeconds": scene.end_seconds,
                            "processor": scene.provenance.processor,
                            "processorVersion": scene.provenance.processor_version,
                            "model": scene.provenance.model,
                            "modelVersion": scene.provenance.model_version,
                        })
                    })
                    .collect()
            });
            let ocr_tracks = bounds.map_or_else(Vec::new, |(start, end)| {
                evidence
                    .ocr_tracks
                    .iter()
                    .filter_map(|track| {
                        let track_bounds = optional_bounds(track.start_seconds, track.end_seconds)?;
                        ranges_overlap(start, end, track_bounds.0, track_bounds.1).then(|| {
                            serde_json::json!({
                                "id": track.id,
                                "role": track.role,
                                "text": track.text,
                                "startSeconds": track.start_seconds,
                                "endSeconds": track.end_seconds,
                                "processor": track.provenance.processor,
                                "processorVersion": track.provenance.processor_version,
                                "model": track.provenance.model,
                                "modelVersion": track.provenance.model_version,
                            })
                        })
                    })
                    .collect()
            });
            let sponsorblock_segments = bounds.map_or_else(Vec::new, |(start, end)| {
                evidence
                    .sponsorblock
                    .as_ref()
                    .map(|sponsorblock| {
                        sponsorblock
                            .segments
                            .iter()
                            .filter(|segment| {
                                ranges_overlap(
                                    start,
                                    end,
                                    segment.start_seconds,
                                    segment.end_seconds,
                                )
                            })
                            .map(|segment| {
                                serde_json::json!({
                                    "uuid": segment.uuid,
                                    "category": segment.category,
                                    "actionType": segment.action_type,
                                    "startSeconds": segment.start_seconds,
                                    "endSeconds": segment.end_seconds,
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            });

            let mut context = serde_json::json!({
                "sourceFragmentId": span.id,
                "videoId": video_id,
                "mediaEvidenceRevision": evidence.revision,
                "scenes": scenes,
                "ocrTracks": ocr_tracks,
                "sponsorBlockSegments": sponsorblock_segments,
            });
            if let Some(reference) = span.metadata.get("mediaEvidenceRef") {
                context["sourceMediaEvidenceRef"] = reference.clone();
            }
            if let Some(sponsorblock) = &evidence.sponsorblock {
                context["sponsorBlockProvenance"] = serde_json::json!({
                    "snapshotId": sponsorblock.snapshot_id,
                    "responseHash": sponsorblock.response_hash,
                    "dataLicense": sponsorblock.data_license,
                    "attribution": sponsorblock.attribution,
                });
            }
            contexts.push(context);
        }
        if !contexts.is_empty() {
            candidate
                .metadata
                .insert("mediaContext".to_string(), Value::Array(contexts));
        }
    }
}

fn timed_locator_bounds(locator: &SourceLocatorV1) -> Option<(f64, f64)> {
    match locator {
        SourceLocatorV1::Timed {
            start_seconds,
            end_seconds,
            ..
        } => optional_bounds(*start_seconds, *end_seconds),
        SourceLocatorV1::Text { .. } => None,
    }
}

fn optional_bounds(start: Option<f64>, end: Option<f64>) -> Option<(f64, f64)> {
    match (start, end) {
        (Some(start), Some(end)) => Some((start, end)),
        (Some(start), None) => Some((start, start)),
        (None, Some(end)) => Some((end, end)),
        (None, None) => None,
    }
}

fn ranges_overlap(left_start: f64, left_end: f64, right_start: f64, right_end: f64) -> bool {
    left_start <= right_end && right_start <= left_end
}
