use crate::model::ExtractionDocument;

use super::PipelineError;

pub fn ingest(mut document: ExtractionDocument) -> Result<ExtractionDocument, PipelineError> {
    document.text = document.text.replace("\r\n", "\n").replace('\r', "\n");
    document.text = collapse_blank_lines(document.text.trim());
    if document.text.is_empty() {
        return Err(PipelineError::EmptyInput);
    }
    if document.kind.as_deref().is_none_or(str::is_empty) {
        document.kind = Some("philosophical_text".to_string());
    }
    Ok(document)
}

fn collapse_blank_lines(value: &str) -> String {
    let mut output = String::new();
    let mut blank_lines = 0;
    for line in value.lines() {
        if line.trim().is_empty() {
            blank_lines += 1;
            if blank_lines <= 1 {
                output.push('\n');
            }
        } else {
            blank_lines = 0;
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(line.trim_end());
        }
    }
    output.trim().to_string()
}
