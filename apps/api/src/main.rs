use philosophy_extractor::{ExtractionDocument, PhilosophyExtractor, PipelineConfig};
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
        let document = ExtractionDocument {
            id: None,
            kind: Some("philosophical_text".to_string()),
            title: None,
            authors: Vec::new(),
            language: None,
            uri: None,
            text: body.to_string(),
        };
        let response = PhilosophyExtractor::new(PipelineConfig::default()).extract(document)?;
        let json = serde_json::to_string(&response)?;
        write_response(&mut stream, 200, &json)?;
        return Ok(());
    }

    write_response(&mut stream, 404, r#"{"error":"not_found"}"#)?;
    Ok(())
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
