use philosophy_extractor_schema::ExtractionDocument;

pub fn plain_text_document(text: impl Into<String>) -> ExtractionDocument {
    ExtractionDocument {
        id: None,
        kind: Some("philosophical_text".to_string()),
        title: None,
        authors: Vec::new(),
        language: None,
        uri: None,
        text: text.into(),
    }
}
