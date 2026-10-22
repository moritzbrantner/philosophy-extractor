# corpus-core batch fixtures (issue #8)

`document-search-batch.jsonl` is a `moenarch_corpus_core_batch` v1 export produced by
document-search's real exporter (`createCorpusCoreBatch` + `serializeCorpusCoreBatch` in
`src/storage/corpusCoreExport.ts`), not written by hand.

- document-search commit: `28463defb66c6ac709a693530d9e4bd55ed242a4` (origin/main, 2026-10-11)
- Generator: `generate-document-search-batch.ts` (this directory). It only calls document-search's
  public import path (`parseUploadedDocument`, `extractHtmlDocument`) and the exporter.
- Command used:

  ```sh
  D=$(mktemp -d)   # a scratch checkout; document-search itself is not modified
  git -C ~/privat/document-search archive 28463defb66c6ac709a693530d9e4bd55ed242a4 | tar -x -C "$D"
  (cd "$D" && bun install --frozen-lockfile)
  bun run packages/source-ingestion/tests/fixtures/corpus_core/generate-document-search-batch.ts "$D" \
    > packages/source-ingestion/tests/fixtures/corpus_core/document-search-batch.jsonl
  ```

  Running it twice produced byte-identical output.

Corpus: three documents (markdown upload with multi-byte text, plain-text upload, and a URL HTML
document stored before originals were kept, so its asset carries a hash but no payload).

Expected values in `tests/corpus_core_intake.rs` were derived independently with Python
(`hashlib`, scalar slicing, UTF-8 byte offsets, corpus-core's length-delimited id derivation), not
by running the intake code. Negative cases are built in the test by mutating this fixture.

## youtube-corpus

`youtube-corpus-batch.jsonl` is youtube-corpus's committed expected corpus-core export
(`tests/fixtures/exports/expected/corpus-core.jsonl` at youtube-corpus commit `bafc7e27465ffb7fd16c2e763234eb623fe04eeb`), copied
unchanged. Transcript, subtitle, audio and video segments become `timed` locators (seconds from
`time_range_ms`, `segment_index` from segment metadata) with `videoId`; their text position stays in
the `byteStart`/`byteEnd`/`characterStart`/`characterEnd` metadata. Checked by
`tests/youtube_corpus_intake.rs`.
