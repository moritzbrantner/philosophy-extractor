# Local NLP Stack Development

The committed workspace resolves its NLP dependencies exclusively from
crates.io at these exact releases:

- `moenarch-text-core = 0.1.1`
- `moenarch-text-embeddings = 0.1.1`
- `moenarch-text-linguistics = 0.1.1`
- `moenarch-text-retrieval = 0.1.1`

For temporary co-development against a local `nlp-stack` checkout, supply
uncommitted Cargo patches outside this repository. All four patched crates
must also declare version `0.1.1`, so they satisfy the committed exact
requirements. If the checkout has advanced to another version, stop and use a
compatible revision of `nlp-stack`; do not loosen the consumer requirements.
This workflow deliberately tests local changes against the released `0.1.1`
contract.

Before configuring the patches, inspect the local workspace:

```sh
cargo metadata \
  --manifest-path /path/to/nlp-stack/Cargo.toml \
  --no-deps \
  --format-version 1
```

Confirm that `moenarch-text-core`, `moenarch-text-embeddings`,
`moenarch-text-linguistics`, and `moenarch-text-retrieval` all report
version `0.1.1`. Treat any mismatch as a failed local setup.

Create a local Cargo configuration file and pass it explicitly:

```toml
# /path/outside/philosophy-extractor/nlp-stack-patches.toml
[patch.crates-io]
moenarch-text-core = { path = "/path/to/nlp-stack/crates/text/text-core" }
moenarch-text-embeddings = { path = "/path/to/nlp-stack/crates/text/text-embeddings" }
moenarch-text-linguistics = { path = "/path/to/nlp-stack/crates/text/text-linguistics" }
moenarch-text-retrieval = { path = "/path/to/nlp-stack/crates/text/text-retrieval" }
```

Before building or testing, resolve the consumer graph with the patch
configuration:

```sh
cargo --config /path/outside/philosophy-extractor/nlp-stack-patches.toml metadata --format-version 1
```

Verify that all four package entries have `"source": null` and
`manifest_path` values inside the local `nlp-stack` checkout. Treat any
"Patch ... was not used" warning as a failed local setup. Then invoke other
Cargo commands with the same `--config` argument.

Do not commit a `[patch.crates-io]` section, a `.cargo/config.toml` patch, a
path/Git replacement, or its lockfile update. Remove all local overrides when
returning to normal registry consumption.
