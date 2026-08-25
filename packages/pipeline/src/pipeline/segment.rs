use crate::model::{ExtractionDocument, Passage};

use super::source_document_id_for;
use text_core::{
    TextProcessingOptions, split_paragraphs, split_sentence_spans_with_abbreviations,
};

const PHILOSOPHY_ABBREVIATIONS: &[&str] =
    &["Phys.", "Metaph.", "Eth.", "Rep.", "Bk.", "Ch."];

pub fn segment_passages(document: &ExtractionDocument) -> Vec<Passage> {
    let document_id = source_document_id_for(document);
    sentence_spans(&document.text)
        .into_iter()
        .enumerate()
        .map(|(index, span)| Passage {
            id: format!("frag_{}_{}", document_id, index + 1),
            document_id: document_id.clone(),
            paragraph_index: span.paragraph_index,
            sequence: index + 1,
            locator: format!(
                "paragraph {}, sentence {}",
                span.paragraph_index + 1,
                span.sequence
            ),
            start_char: span.start_char,
            end_char: span.end_char,
            text: span.text,
        })
        .collect()
}

#[derive(Debug, Clone)]
struct SentenceSpan {
    paragraph_index: usize,
    sequence: usize,
    start_char: usize,
    end_char: usize,
    text: String,
}

fn sentence_spans(text: &str) -> Vec<SentenceSpan> {
    let options = TextProcessingOptions::default();
    let paragraphs = split_paragraphs(text);
    let mut sentence_counts_by_paragraph = Vec::<usize>::new();

    let spans =
        split_sentence_spans_with_abbreviations(text, &options, PHILOSOPHY_ABBREVIATIONS)
        .into_iter()
        .map(|sentence| {
            let paragraph_index = paragraphs
                .iter()
                .position(|paragraph| {
                    sentence.span.byte_start >= paragraph.span.byte_start
                        && sentence.span.byte_start < paragraph.span.byte_end
                })
                .unwrap_or(0);
            if sentence_counts_by_paragraph.len() <= paragraph_index {
                sentence_counts_by_paragraph.resize(paragraph_index + 1, 0);
            }
            sentence_counts_by_paragraph[paragraph_index] += 1;
            SentenceSpan {
                paragraph_index,
                sequence: sentence_counts_by_paragraph[paragraph_index],
                start_char: sentence.span.byte_start,
                end_char: sentence.span.byte_end,
                text: sentence.text,
            }
        })
        .collect();

    merge_tiny_spans(spans)
}

fn merge_tiny_spans(spans: Vec<SentenceSpan>) -> Vec<SentenceSpan> {
    let mut merged: Vec<SentenceSpan> = Vec::new();
    for span in spans {
        if let Some(previous) = merged.last_mut()
            && previous.paragraph_index == span.paragraph_index
            && (word_count(&previous.text) < 3 || word_count(&span.text) < 3)
        {
            previous.end_char = span.end_char;
            previous.text = format!("{} {}", previous.text.trim_end(), span.text.trim_start());
            continue;
        }
        merged.push(span);
    }
    merged
}

fn word_count(value: &str) -> usize {
    value.split_whitespace().count()
}
