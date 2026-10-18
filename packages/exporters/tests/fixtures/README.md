# Belief Lab reference export fixture

`document-source-spans.json` is generated through the real corpus-core intake from
`packages/source-ingestion/tests/fixtures/corpus_core/document-search-batch.jsonl`.
That corpus was produced by document-search at
`28463defb66c6ac709a693530d9e4bd55ed242a4`; its existing fixture README records the
source and generation command. It contains synthetic markdown, plain-text and
HTML documents. No real person's philosophical belief is asserted.

The reference exporter implementation is pinned at `git:88cd7c05f2130d2056209f7db751774a581fcf72`.
The invocation records the exporter separately from original upstream processors:

```sh
cargo run -q -p philosophy-extractor-exporters --example export_belief_sources -- \
  packages/source-ingestion/tests/fixtures/corpus_core/document-search-batch.jsonl \
  git:88cd7c05f2130d2056209f7db751774a581fcf72 --source-spans \
  > packages/exporters/tests/fixtures/document-source-spans.json

cargo run -q -p philosophy-extractor-exporters --example export_belief_sources -- \
  packages/source-ingestion/tests/fixtures/corpus_core/document-search-batch.jsonl \
  git:88cd7c05f2130d2056209f7db751774a581fcf72 \
  > /tmp/philosophy-belief-export.json

python3 -I packages/exporters/tests/fixtures/derive_expected.py
cmp /tmp/philosophy-belief-export.json packages/exporters/tests/fixtures/belief-evidence.json
```

The Python oracle never calls Rust. It independently checks the native corpus-core
record hashes and processor receipts against the original JSONL, then derives the
reference envelope, evidence IDs, correlation families and batch checksum. The
Rust integration test separately checks the real intake against the saved input
and the public exporter against this expected output.

Source descriptors reference original corpus-core **representations** as metadata.
Document passages reference original **segments** as `source_text`, with the
representation descriptor as their imported parent. Each revision is SHA-256 of
the canonical original native record, including its locators and provenance, not
a hash of a release label or a redefined corpus record. Canonical JSON sorts object
keys, uses compact UTF-8 JSON and preserves array order. Correlation families identify
the same repository/scope/record across revisions; evidence IDs bind the complete
reference, including its producer and imported parents.

The producer artifacts retain full text, locators and upstream input lineage.
Only references leave this adapter. It exports neither philosophical candidates nor
extraction confidence, worldview assertions, person-confidence or conviction.
Older Belief Lab consumers without the `source_text` class reject the new class;
the matching consumer update belongs to belief-lab#11.
