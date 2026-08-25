# Project Rules

- Use `bun` for frontend package management, scripts, and dependency execution.
- Format frontend code with `oxfmt`.
- Write frontend code in TypeScript, not JavaScript.
- Use React for frontend UI.
- Use Tailwind CSS for frontend styling.
- Use React Query for frontend server state and data fetching.

- Use managed exact-revision source mode for unreleased `nlp-stack` work; do not create ad hoc Cargo patch files or publish crates merely to unblock development.
- Keep committed package manifests registry-based. The generated `.cargo/config.toml` is local-only and must remain ignored.
- Deactivate source mode before registry-only release verification.
