# Local NLP stack development

The workspace consumes `nlp-stack` through exact-revision git dependencies on the public repository (owner decision in #8). All four `moenarch-text-*` entries in the root `Cargo.toml` share one full commit `rev`, currently `057689ca1df031d677f4afd9d65eed181c6fca06`. A fresh clone therefore builds and tests with plain Cargo (`cargo test --locked --workspace`), without a sibling checkout, credentials or source mode.

To consume newer NLP behavior:

1. Land and push the change in `nlp-stack`.
2. Update the shared `rev` of all four entries to that commit.
3. Run `cargo update -p moenarch-text-core -p moenarch-text-embeddings -p moenarch-text-linguistics -p moenarch-text-retrieval` (or plain `cargo metadata`) and commit the resulting `Cargo.lock` with the manifest change.
4. Run `cargo test --locked --workspace`.

Testing against an *unpushed* sibling `../nlp-stack` change is not supported yet: `coding-tooling source-deps` only generates `[patch.crates-io]`, which does not apply to git dependencies (moritzbrantner/coding-tooling#313). `.coding-tooling.source-deps.json` therefore declares no patches. Do not hand-write `[patch]` sections or commit sibling paths.

Publishing crates and switching back to registry requirements is a later release task.
