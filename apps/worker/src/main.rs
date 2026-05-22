use clap::Parser;
use philosophy_extractor::{ExtractionDocument, PhilosophyExtractor, PipelineConfig};
use std::{fs, io::Read, path::PathBuf};

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Extract propositions and a Truth Engine worldview from philosophical text."
)]
struct Cli {
    /// Input text file. Reads stdin when omitted.
    input: Option<PathBuf>,

    /// Write JSON to a file instead of stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Source document id.
    #[arg(long)]
    document_id: Option<String>,

    /// Source title.
    #[arg(long)]
    title: Option<String>,

    /// Source author. May be repeated.
    #[arg(long = "author")]
    authors: Vec<String>,

    /// Source language code.
    #[arg(long)]
    language: Option<String>,

    /// Source URI.
    #[arg(long)]
    uri: Option<String>,

    /// Truth Engine worldview id.
    #[arg(long)]
    worldview_id: Option<String>,

    /// Truth Engine worldview label.
    #[arg(long)]
    label: Option<String>,

    /// Minimum proposition confidence.
    #[arg(long, default_value_t = 0.35)]
    min_confidence: f64,

    /// Maximum number of propositions to emit.
    #[arg(long)]
    max_propositions: Option<usize>,

    /// Disable relation inference.
    #[arg(long)]
    no_relations: bool,

    /// Emit only the schemaVersion 10 worldview object.
    #[arg(long)]
    worldview_only: bool,

    /// Pretty-print JSON.
    #[arg(long)]
    pretty: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let text = read_input(cli.input.as_ref())?;
    let document = ExtractionDocument {
        id: cli.document_id,
        kind: Some("philosophical_text".to_string()),
        title: cli.title,
        authors: cli.authors,
        language: cli.language,
        uri: cli.uri,
        text,
    };
    let extractor = PhilosophyExtractor::new(PipelineConfig {
        worldview_id: cli.worldview_id,
        label: cli.label,
        min_confidence: cli.min_confidence,
        max_propositions: cli.max_propositions,
        include_relations: !cli.no_relations,
    });
    let response = extractor.extract(document)?;

    let json = if cli.worldview_only {
        serialize(&response.worldview, cli.pretty)?
    } else {
        serialize(&response, cli.pretty)?
    };

    if let Some(output) = cli.output {
        fs::write(output, json)?;
    } else {
        println!("{json}");
    }

    Ok(())
}

fn read_input(path: Option<&PathBuf>) -> Result<String, Box<dyn std::error::Error>> {
    match path {
        Some(path) => Ok(fs::read_to_string(path)?),
        None => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            Ok(input)
        }
    }
}

fn serialize<T: serde::Serialize>(value: &T, pretty: bool) -> Result<String, serde_json::Error> {
    if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
}
