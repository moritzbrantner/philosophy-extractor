# Philosophy Extractor

`philosophy-extractor` is a Rust workspace for turning philosophical source text into proposition candidates and a normalized Truth Engine worldview payload.

It targets Truth Engine's unified worldview contract:

- `schemaVersion: "10"`
- `fragment: "unified_worldview_v2"`
- proposition assertions with source fragment provenance
- optional inferred `supports`, `contradicts`, and `generalizes` relations

## Pipeline Stages

1. `ingest`: normalize document text and metadata.
2. `segment`: split the document into source-grounded passages with stable ids and offsets.
3. `embed`: create local passage embeddings.
4. `cluster`: group related passages by embedding similarity.
5. `extract_claims`: identify source-grounded philosophical claim candidates.
6. `classify_roles`: assign argument roles to claims.
7. `extract_terms`: extract local term candidates and mark them for review.
8. `reconstruct_arguments`: link premises, conclusions, support, and attack candidates.
9. `normalize_propositions`: canonicalize proposition text, conservatively deduplicate, score, rank, and filter by confidence.
10. `formalize`: create schema-validated formalization candidates.
11. `evaluate`: emit Lean-oriented evaluation artifacts.
12. `worldview`: emit a Truth Engine-compatible unified worldview.

The default CLI output includes the worldview, candidates, candidate index, diagnostics, artifact references, and per-stage summaries. Use `--worldview-only` when writing a payload intended for direct Truth Engine import.

Every run persists reviewable JSON artifacts by default under `.artifacts/{run_id}`:

```text
manifest.json
01_ingest.json
02_passages.json
03_embeddings.json
04_clusters.json
05_claim_candidates.json
06_claim_roles.json
07_terms.json
08_arguments.json
09_candidate_index.json
09_normalized_propositions.json
10_formalizations.json
11_evaluations.json
worldview.json
```

The MVP is offline-capable. Provider boundaries are in place for OpenAI Responses, local embeddings/NER, and Lean, while the default testable implementation uses deterministic local fallbacks so `cargo test` does not require network access, model downloads, or Lean.

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

Stop after clustering and write only artifacts through `04_clusters.json`:

```bash
cargo run -p philosophy-extractor-worker -- path/to/text.txt \
  --stage-through cluster \
  --artifacts-dir .artifacts \
  --pretty
```

Configure provider model ids:

```bash
cargo run -p philosophy-extractor-worker -- path/to/text.txt \
  --openai-model-primary gpt-5.5 \
  --openai-model-cheap gpt-5.5-mini \
  --embedding-model sentence-transformers/all-MiniLM-L6-v2 \
  --ner-model rust-bert-default-ner \
  --lean-bin lean
```

Equivalent environment defaults are supported:

```text
PHILOSOPHY_EXTRACTOR_ARTIFACT_DIR
PHILOSOPHY_EXTRACTOR_OPENAI_MODEL_PRIMARY
PHILOSOPHY_EXTRACTOR_OPENAI_MODEL_CHEAP
PHILOSOPHY_EXTRACTOR_LEAN_BIN
OPENAI_API_KEY
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

Extract from a JSON request:

```bash
curl -X POST http://localhost:8080/extract \
  -H 'content-type: application/json' \
  --data '{
    "document": {
      "id": "doc-example",
      "title": "Fragment",
      "authors": ["Example Author"],
      "language": "en",
      "text": "Knowledge concerns truth. Justice should harmonize the soul."
    },
    "config": {
      "stageThrough": "evaluate",
      "persistArtifacts": true
    }
  }'
```

The existing Compose setup can also run the API:

```bash
docker compose up --build api
```

## Notes

The current extractor is deterministic and heuristic. It is designed as a reviewable first pass: generated propositions are marked with `reviewRequired: true`, source fragment ids are preserved, candidate index entries expose score breakdowns, and relation notes explain why each relation was inferred. Exact and normalized-fingerprint duplicates are merged conservatively; semantic near-duplicates are retained as reviewable `variant_of` relations.
