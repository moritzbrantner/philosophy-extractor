# Philosophy Extractor Roadmap

`philosophy-extractor` should become the philosophical interpretation layer over general-purpose corpus systems. It should not own document search, YouTube ingest, or the final worldview ontology.

## Architectural direction

The long-term flow is:

```text
source corpora
  -> document-search / youtube-corpus / future adapters
  -> stable source spans with provenance
  -> philosophy-extractor
  -> source-grounded statement candidates + typed assessments
  -> worldview-lab experiments
  -> curated downstream knowledge / truth systems
```

The extractor should preserve enough evidence that later worldview representations can be rebuilt without re-ingesting the original source.

## Near-term milestones

1. [ ] **Define a thin source/span interoperability contract.**
   - Represent source identity separately from source spans.
   - Preserve stable source ids, source revision, exact locator, text, content hash, language, and source metadata.
   - Support document locators such as page/section/paragraph and time-based locators such as YouTube transcript timestamps.
   - Keep this contract independent of any final philosophical ontology.

2. [ ] **Add corpus adapters without importing corpus authority.**
   - Consume deterministic exports from `document-search`.
   - Consume deterministic transcript-span exports from `youtube-corpus`.
   - Keep ingestion/search/transcript ownership in the source repositories.

3. [ ] **Add a Jev decision-provider adapter in shadow mode.**
   - Treat Jev as a typed probabilistic judgment layer, not as an authority over extracted truth.
   - Start with narrow decisions such as philosophical relevance, statement/argument role, domain, context dependence, source support, and formalization readiness.
   - Persist model/provider version, question definition, returned probabilities/confidence, and the exact source span assessed.
   - Keep deterministic local fixtures and a provider boundary so tests do not require network access.

4. [ ] **Build a corpus-scale screening funnel.**
   - Run cheap philosophical-relevance screening before expensive extraction.
   - Route only promising spans to deeper statement extraction and assessment.
   - Escalate low-confidence or interpretation-sensitive cases rather than forcing a label.
   - Measure throughput, cost, calibration, and false-negative risk on representative corpora.

5. [ ] **Separate extracted statements from assessments.**
   - Store the source-grounded statement candidate as its own artifact.
   - Store Jev/LLM/human/formal assessments as append-only evidence about that candidate.
   - Do not mutate a candidate into a canonical proposition merely because a classifier is confident.
   - Preserve verbatim supporting spans and hashes for auditability.

6. [ ] **Export experimental statement candidates to `worldview-lab`.**
   - Let `worldview-lab` experiment with proposition identity, commitments, contexts, modalities, argument structures, and relations.
   - Avoid freezing those choices into `philosophy-extractor` until multiple downstream experiments demonstrate a stable need.
   - Keep the existing Truth Engine/worldview export as a downstream compatibility path rather than the only internal representation.

7. [ ] **Establish evaluation corpora and admission thresholds.**
   - Create hand-reviewed fixtures spanning books, papers, and transcripts.
   - Track philosophical-relevance recall separately from extraction precision.
   - Benchmark Jev decisions against stronger reasoning models and human review where appropriate.
   - Record threshold policies in code so autonomous routing remains inspectable.

## Permanent boundaries

- Provenance is required for every extracted statement and every automated assessment.
- `document-search` owns document corpus/search concerns; `youtube-corpus` owns YouTube/transcript corpus concerns.
- `philosophy-extractor` owns philosophical extraction and assessment orchestration.
- `worldview-lab` owns experiments in philosophical representation and semantics.
- Typed model output is evidence, not proof. Formal verification and curated downstream admission remain separate concerns.
- The final philosophical JSON representation is intentionally not fixed by this roadmap.
