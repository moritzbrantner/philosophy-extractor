# Philosophy Extractor

`philosophy-extractor` is a Rust workspace for turning philosophical source text into proposition candidates and a normalized Truth Engine worldview payload.

It targets Truth Engine's unified worldview contract:

- `schemaVersion: "10"`
- `fragment: "unified_worldview_v2"`
- proposition assertions with source fragment provenance
- optional inferred `supports`, `contradicts`, and `generalizes` relations

## Pipeline Stages

1. `ingest`: normalize document text and metadata.
2. `segment`: split the document into source fragments with stable ids.
3. `extract`: identify declarative philosophical proposition candidates.
4. `normalize`: canonicalize proposition text, deduplicate, and filter by confidence.
5. `relate`: infer lightweight semantic relations between accepted propositions.
6. `worldview`: emit a Truth Engine-compatible unified worldview.

The default CLI output includes the worldview, candidates, diagnostics, and per-stage summaries. Use `--worldview-only` when writing a payload intended for direct Truth Engine import.

## Workspace Layout

- `apps/api`: HTTP API service boundary.
- `apps/worker`: CLI/batch worker executable.
- `apps/review-ui`: placeholder for the human review interface.
- `packages/schema`: shared extraction and worldview data contracts.
- `packages/pipeline`: deterministic extraction pipeline.
- `packages/prompts`: prompt assets for model-assisted extraction.
- `packages/model-providers`: provider abstractions for model-backed stages.
- `packages/artifact-store`: artifact persistence boundary.
- `packages/source-ingestion`: source document ingestion helpers.
- `packages/exporters`: worldview export helpers.
- `fixtures`: author-specific source fixtures.

## Usage

```bash
cargo run -p philosophy-extractor-worker -- path/to/text.txt --title "Nicomachean Ethics" --author Aristotle --pretty
```

Write only the normalized worldview:

```bash
cargo run -p philosophy-extractor-worker -- path/to/text.txt \
  --worldview-id aristotle-draft \
  --label "Aristotle draft worldview" \
  --worldview-only \
  --pretty \
  --output aristotle-worldview.json
```

Read from stdin:

```bash
printf 'Knowledge concerns truth. Justice should harmonize the soul.' \
  | cargo run -p philosophy-extractor-worker -- --title "Fragment" --pretty
```

## Docker

Build the API image:

```bash
docker build -t philosophy-extractor .
```

Run the API:

```bash
docker run --rm -p 8080:8080 philosophy-extractor
```

Check the health endpoint:

```bash
curl http://localhost:8080/health
```

Extract from text:

```bash
curl -X POST http://localhost:8080/extract \
  --data 'Knowledge concerns truth. Justice should harmonize the soul.'
```

The existing Compose setup can also run the API:

```bash
docker compose up --build api
```

## Notes

The current extractor is deterministic and heuristic. It is designed as a reviewable first pass: generated propositions are marked with `reviewRequired: true`, source fragment ids are preserved, and relation notes explain why each relation was inferred.
