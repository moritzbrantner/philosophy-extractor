use philosophy_extractor::{
    ExtractionDocument, PhilosophyExtractor, PipelineConfig, PipelineStage,
};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind_addr =
        std::env::var("PHILOSOPHY_EXTRACTOR_API_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = TcpListener::bind(&bind_addr)?;
    eprintln!("philosophy-extractor-api listening on {bind_addr}");

    for stream in listener.incoming() {
        let stream = stream?;
        handle_connection(stream)?;
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0; 64 * 1024];
    let bytes_read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();

    if request.starts_with("GET /health ") {
        write_response(&mut stream, 200, r#"{"status":"ok"}"#)?;
        return Ok(());
    }

    if request.starts_with("POST /extract ") {
        let (document, config) = parse_extract_request(body)?;
        let response = PhilosophyExtractor::new(config).extract(document)?;
        let json = serde_json::to_string(&response)?;
        write_response(&mut stream, 200, &json)?;
        return Ok(());
    }

    write_response(&mut stream, 404, r#"{"error":"not_found"}"#)?;
    Ok(())
}

fn parse_extract_request(
    body: &str,
) -> Result<(ExtractionDocument, PipelineConfig), Box<dyn std::error::Error>> {
    if body.trim_start().starts_with('{') {
        let value = serde_json::from_str::<Value>(body)?;
        let document = serde_json::from_value::<ExtractionDocument>(
            value
                .get("document")
                .cloned()
                .unwrap_or_else(|| Value::String(body.to_string())),
        )
        .or_else(|_| {
            Ok::<ExtractionDocument, serde_json::Error>(ExtractionDocument {
                id: None,
                kind: Some("philosophical_text".to_string()),
                title: None,
                authors: Vec::new(),
                language: None,
                uri: None,
                text: value
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            })
        })?;
        let mut config = PipelineConfig::default();
        if let Some(config_value) = value.get("config") {
            if let Some(stage) = config_value.get("stageThrough").and_then(Value::as_str) {
                config.stage_through = Some(parse_stage(stage)?);
            }
            if let Some(persist) = config_value
                .get("persistArtifacts")
                .and_then(Value::as_bool)
            {
                config.persist_artifacts = persist;
            }
            if let Some(path) = config_value.get("artifactsDir").and_then(Value::as_str) {
                config.artifact_dir = path.into();
            }
        }
        return Ok((document, config));
    }

    Ok((
        ExtractionDocument {
            id: None,
            kind: Some("philosophical_text".to_string()),
            title: None,
            authors: Vec::new(),
            language: None,
            uri: None,
            text: body.to_string(),
        },
        PipelineConfig::default(),
    ))
}

fn parse_stage(value: &str) -> Result<PipelineStage, Box<dyn std::error::Error>> {
    let stage = match value {
        "ingest" => PipelineStage::Ingest,
        "segment" => PipelineStage::Segment,
        "embed" => PipelineStage::Embed,
        "cluster" => PipelineStage::Cluster,
        "extract_claims" | "extract-claims" => PipelineStage::ExtractClaims,
        "classify_roles" | "classify-roles" => PipelineStage::ClassifyRoles,
        "extract_terms" | "extract-terms" => PipelineStage::ExtractTerms,
        "reconstruct_arguments" | "reconstruct-arguments" => PipelineStage::ReconstructArguments,
        "normalize_propositions" | "normalize-propositions" => PipelineStage::NormalizePropositions,
        "formalize" => PipelineStage::Formalize,
        "evaluate" => PipelineStage::Evaluate,
        "worldview" => PipelineStage::Worldview,
        _ => return Err(format!("unknown stage '{value}'").into()),
    };
    Ok(stage)
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_extract_request() {
        let (document, config) = parse_extract_request(
            r#"{
                "document": {
                    "id": "doc-json",
                    "kind": "philosophical_text",
                    "title": "Fragment",
                    "authors": ["Example"],
                    "language": "en",
                    "text": "Knowledge concerns truth."
                },
                "config": {
                    "stageThrough": "cluster",
                    "persistArtifacts": false
                }
            }"#,
        )
        .unwrap();

        assert_eq!(document.id.as_deref(), Some("doc-json"));
        assert_eq!(config.stage_through, Some(PipelineStage::Cluster));
        assert!(!config.persist_artifacts);
    }

    #[test]
    fn parses_raw_text_extract_request() {
        let (document, config) = parse_extract_request("Knowledge concerns truth.").unwrap();

        assert_eq!(document.text, "Knowledge concerns truth.");
        assert_eq!(config.stage_through, None);
    }
}
