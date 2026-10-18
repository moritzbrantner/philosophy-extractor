//! Export real corpus-core document inputs through the public intake and reference adapter.
use philosophy_extractor_exporters::export_belief_evidence;
use philosophy_extractor_source_ingestion::source_spans_from_corpus_core_jsonl;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let input = args
        .next()
        .ok_or("usage: export_belief_sources <corpus-core.jsonl> <immutable-exporter-revision>")?;
    let revision = args
        .next()
        .ok_or("an immutable exporter revision is required")?;
    let source_spans = match args.next().as_deref() {
        None => false,
        Some("--source-spans") => true,
        Some(_) => return Err("unexpected extra argument".into()),
    };
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let sources = source_spans_from_corpus_core_jsonl(&std::fs::read_to_string(input)?)?;
    if source_spans {
        println!("{}", serde_json::to_string_pretty(&sources)?);
    } else {
        print!("{}", export_belief_evidence(&sources, &revision)?);
    }
    Ok(())
}
