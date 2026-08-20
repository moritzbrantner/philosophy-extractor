# Local NLP Stack Development

The committed workspace resolves its NLP dependencies exclusively from
crates.io at these exact releases:

- `moenarch-text-core = 0.1.1`
- `moenarch-text-embeddings = 0.1.1`
- `moenarch-text-linguistics = 0.1.1`
- `moenarch-text-retrieval = 0.1.1`

For temporary co-development against a local `nlp-stack` checkout, supply
uncommitted Cargo patches outside this repository. For example, create a local
Cargo configuration file and pass it explicitly:

```toml
# /path/outside/philosophy-extractor/nlp-stack-patches.toml
[patch.crates-io]
moenarch-text-core = { path = "/path/to/nlp-stack/crates/text/text-core" }
moenarch-text-embeddings = { path = "/path/to/nlp-stack/crates/text/text-embeddings" }
moenarch-text-linguistics = { path = "/path/to/nlp-stack/crates/text/text-linguistics" }
moenarch-text-retrieval = { path = "/path/to/nlp-stack/crates/text/text-retrieval" }
```

Then invoke Cargo with that file:

```sh
cargo --config /path/outside/philosophy-extractor/nlp-stack-patches.toml <command>
```

Do not commit a `[patch.crates-io]` section, a `.cargo/config.toml` patch, or a
path/Git replacement in this repository. Remove the local configuration when
returning to normal registry consumption.
