# Source/span interchange v1

This contract is the provenance-preserving input boundary between corpus owners such as `document-search` / `youtube-corpus` and `philosophy-extractor`.

It intentionally contains no philosophical ontology.

## Envelope

Every export has:

- `schema: "source_span_interchange"`
- `schemaVersion: 1`
- a producer `name` and exact producer `revision`
- zero or more `sources`
- zero or more `spans`

The producer revision identifies the exact exporter implementation (for example an exact Git/build revision supplied by the caller). It is separate from both the source `revision` and the source `contentHash`.

## Source records

A source record owns:

- a stable `id`
- a coarse `kind`
- an exact `revision`
- optional URI/title/creators/language
- a SHA-256 `contentHash`
- source-specific metadata

For v1, content hashes use `sha256:<64 lowercase hex digits>`. `contentHash` fingerprints the verbatim source text bytes. `revision` fingerprints the exact structured representation consumed downstream, including span identity/order and locators. Changing paragraph structure, headings, transcript timing, transcript source, or other provenance-relevant structure therefore changes `revision` even when the verbatim text and `contentHash` are unchanged.

A `document-search` source is one extracted document. A `youtube-corpus` source is one transcript stream, so manual captions, automatic captions, and ASR remain distinguishable even when they belong to the same video.

## Span records

A span record owns:

- a stable `id`
- the owning `sourceId`
- a source-local `sequence`
- verbatim `text`
- a SHA-256 `contentHash`
- optional language
- an exact locator
- source-specific metadata

Span ids must be unique within a batch. The pair `(sourceId, sequence)` must also be unique.

## Locators

### Text

Text locators use canonical UTF-8 half-open byte offsets:

```json
{
  "kind": "text",
  "byteStart": 0,
  "byteEnd": 42,
  "paragraphOrdinal": 0,
  "page": 12,
  "section": "II.3",
  "sourceSelector": "#argument",
  "headingPath": ["Book II", "Chapter 3"]
}
```

Only `byteStart` and `byteEnd` are required. Page, section, paragraph ordinal, selector, and heading path are optional structural aids.

### Timed

Timed locators preserve source-local ordering even when timestamps are unavailable:

```json
{
  "kind": "timed",
  "segmentIndex": 17,
  "startSeconds": 83.4,
  "endSeconds": 88.1
}
```

`segmentIndex` is required. Timestamps are optional, finite, non-negative, and may not be reversed.

## Authority and validation

- Corpus owners remain authoritative for source text, transcript selection, locators, and source metadata.
- `philosophy-extractor` validates the interchange contract but does not rewrite source truth.
- Exporters fail closed when they cannot reconstruct an exact locator.
- Model judgments and philosophical statements are derived artifacts and never mutate these source/span records.
- Future schema versions must be explicit; v1 consumers must reject unknown schema/version combinations instead of guessing.
