# Project Rules

## Proof-seeking boundary

- This repository produces source-grounded proposition and formalization candidates; it does not certify philosophical truth through prose or model agreement.
- Treat LLM output as candidate generation. A claimed consequence becomes authoritative only when a downstream mechanically checkable artifact verifies the encoded premises, definitions, and inference steps.
- Preserve the boundary between source evidence, proposition extraction, formalization, proof search, and proof verification.
- Use Curry–Howard as the target intuition where the formalization fits dependent type theory, but do not treat failure to discover a proof as evidence of negation.


- Use `bun` for frontend package management, scripts, and dependency execution.
- Format frontend code with `oxfmt`.
- Write frontend code in TypeScript, not JavaScript.
- Use React for frontend UI.
- Use Tailwind CSS for frontend styling.
- Use React Query for frontend server state and data fetching.

- Use managed exact-revision source mode for unreleased `nlp-stack` work; do not create ad hoc Cargo patch files or publish crates merely to unblock development.
- Keep committed package manifests registry-based. The generated `.cargo/config.toml` is local-only and must remain ignored.
- Deactivate source mode before registry-only release verification.
