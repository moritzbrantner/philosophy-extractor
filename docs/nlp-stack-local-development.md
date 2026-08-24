# Local NLP Stack Development

The committed workspace resolves its NLP dependencies exclusively from
crates.io at these exact releases:

- `moenarch-text-core = 0.1.1`
- `moenarch-text-embeddings = 0.1.1`
- `moenarch-text-linguistics = 0.1.1`
- `moenarch-text-retrieval = 0.1.1`

For temporary co-development against a local `nlp-stack` checkout, supply
uncommitted Cargo patches outside this repository. A patched crate's manifest
version must satisfy the corresponding requirement in `Cargo.toml`. The
committed `=0.1.1` requirements therefore patch only local crates that also
declare version `0.1.1`. If the checkout has advanced, temporarily change the
four requirements in `Cargo.toml` to match its crate versions; do not commit
those changes or the resulting lockfile update.

Create a local Cargo configuration file and pass it explicitly:

```toml
# /path/outside/philosophy-extractor/nlp-stack-patches.toml
[patch.crates-io]
moenarch-text-core = { path = "/path/to/nlp-stack/crates/text/text-core" }
moenarch-text-embeddings = { path = "/path/to/nlp-stack/crates/text/text-embeddings" }
moenarch-text-linguistics = { path = "/path/to/nlp-stack/crates/text/text-linguistics" }
moenarch-text-retrieval = { path = "/path/to/nlp-stack/crates/text/text-retrieval" }
```

Before building or testing, resolve the graph with the patch configuration:

```sh
cargo --config /path/outside/philosophy-extractor/nlp-stack-patches.toml metadata --format-version 1
```

Verify that all four package entries have `"source": null` and
`manifest_path` values inside the local `nlp-stack` checkout. Treat any
"Patch ... was not used" warning as a failed local setup. Then invoke other
Cargo commands with the same `--config` argument.

Do not commit a `[patch.crates-io]` section, a `.cargo/config.toml` patch, a
path/Git replacement, temporary version changes, or their lockfile update.
Remove or revert all local overrides when returning to normal registry
consumption.
