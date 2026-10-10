# Project Rules

## Proof-seeking boundary

- This repository produces source-grounded proposition and formalization candidates; it does not certify philosophical truth through prose or model agreement.
- Treat LLM output as candidate generation. A claimed consequence becomes authoritative only when a downstream mechanically checkable artifact verifies the encoded premises, definitions, and inference steps.
- Preserve the boundary between source evidence, proposition extraction, formalization, proof search, and proof verification.
- Use Curry–Howard as the target intuition where the formalization fits dependent type theory, but do not treat failure to discover a proof as evidence of negation.


## Authority boundaries

Machine-readable in `.repository.toml` (`[architecture]`); keep both lists identical. Check with `coding-tooling repository contract --root . --json`.

- Owns: `philosophy-extractor/claim-candidates`, `philosophy-extractor/argument-reconstruction`, `philosophy-extractor/philosophical-assessment`, `philosophy-extractor/formalization-candidates`, `philosophy-extractor/worldview-compatibility`
- Adapts: `nlp-stack/text-core`, `nlp-stack/text-embeddings`, `nlp-stack/text-linguistics`, `nlp-stack/retrieval-semantics`, `moenarch-foundation/corpus-core`, `document-search/source-span-export`
- Non-authoritative: `nlp-stack/text-core`, `nlp-stack/text-embeddings`, `nlp-stack/text-linguistics`, `nlp-stack/retrieval-semantics`, `moenarch-foundation/corpus-core`, `document-search/corpus-ingestion`, `youtube-corpus/corpus-persistence`, `media/asr`, `media/ocr`, `media/scene-detection`

## Engineering and dependency rules

- Use `bun` for frontend package management, scripts, and dependency execution.
- Format frontend code with `oxfmt`.
- Write frontend code in TypeScript, not JavaScript.
- Use React for frontend UI.
- Use Tailwind CSS for frontend styling.
- Use React Query for frontend server state and data fetching.

- Consume `nlp-stack` through committed exact-revision git dependencies on the public repository (owner decision, #8): all `moenarch-text-*` workspace dependencies share one full `rev`. Update that `rev` only to a pushed, reviewed nlp-stack commit, and regenerate `Cargo.lock` in the same change. Do not publish crates merely to unblock development; releases come later.
- No standing source patches: `.coding-tooling.source-deps.json` declares none until coding-tooling can patch git dependencies (moritzbrantner/coding-tooling#313). Do not hand-write `[patch]` sections.
