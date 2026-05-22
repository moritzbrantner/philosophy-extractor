use crate::model::{ExtractionDocument, Passage};

use super::source_document_id_for;

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
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut current_start = None;
    let mut paragraph_index = 0;
    let mut sentence_in_paragraph = 1;
    let mut previous_was_newline = false;

    for (index, ch) in text.char_indices() {
        if current_start.is_none() && !ch.is_whitespace() {
            current_start = Some(index);
        }
        current.push(ch);
        if matches!(ch, '.' | '!' | '?' | '\n') {
            push_current(
                &mut spans,
                &mut current,
                &mut current_start,
                paragraph_index,
                sentence_in_paragraph,
                index + ch.len_utf8(),
            );
            sentence_in_paragraph += 1;
        }
        if ch == '\n' && previous_was_newline {
            paragraph_index += 1;
            sentence_in_paragraph = 1;
        }
        previous_was_newline = ch == '\n';
    }
    push_current(
        &mut spans,
        &mut current,
        &mut current_start,
        paragraph_index,
        sentence_in_paragraph,
        text.len(),
    );

    merge_tiny_spans(spans)
}

fn push_current(
    spans: &mut Vec<SentenceSpan>,
    current: &mut String,
    current_start: &mut Option<usize>,
    paragraph_index: usize,
    sequence: usize,
    end_char: usize,
) {
    let value = current.trim();
    if !value.is_empty() {
        spans.push(SentenceSpan {
            paragraph_index,
            sequence,
            start_char: current_start.unwrap_or(0),
            end_char,
            text: value.to_string(),
        });
    }
    current.clear();
    *current_start = None;
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
