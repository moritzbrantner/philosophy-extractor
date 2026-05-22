use crate::model::{ExtractionDocument, SourceFragment};

use super::source_document_id_for;

pub fn segment(document: &ExtractionDocument) -> Vec<SourceFragment> {
    let document_id = source_document_id_for(document);
    sentence_fragments(&document.text)
        .into_iter()
        .enumerate()
        .map(|(index, text)| SourceFragment {
            id: format!("frag_{}_{}", document_id, index + 1),
            document_id: document_id.clone(),
            locator: Some(format!("sentence {}", index + 1)),
            text: Some(text),
        })
        .collect()
}

fn sentence_fragments(text: &str) -> Vec<String> {
    let mut fragments = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?' | '\n') {
            push_current(&mut fragments, &mut current);
        }
    }
    push_current(&mut fragments, &mut current);

    fragments
}

fn push_current(fragments: &mut Vec<String>, current: &mut String) {
    let value = current.trim();
    if !value.is_empty() {
        fragments.push(value.to_string());
    }
    current.clear();
}
