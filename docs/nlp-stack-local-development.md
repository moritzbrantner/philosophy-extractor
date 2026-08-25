# Local NLP stack development

The committed workspace keeps exact crates.io requirements for its four NLP dependencies, but ordinary feature work does not require publishing an intermediate `nlp-stack` release.

The committed `.coding-tooling.source-deps.json` pins all four packages to one exact `nlp-stack` revision. Run:

```sh
bash scripts/source-deps activate
bash scripts/source-deps status
```

`coding-tooling` generates the ignored `.cargo/config.toml`. When a sibling `../nlp-stack` checkout exists, activation verifies that its Git `HEAD` exactly matches the declared revision before using its crate paths. Otherwise the exact Git revision is used when the private repository is accessible.

All four source crates must continue to declare versions compatible with the committed `=0.1.1` requirements. A mismatch fails Cargo resolution; do not loosen the consumer requirements merely to make source mode work. Update all four declaration entries to the same reviewed revision whenever the validated NLP head changes.

Run normal Cargo checks while source mode is active. The resolved packages should have local or exact Git source provenance rather than crates.io provenance. Treat unused-patch warnings or revision mismatches as failed setup.

Before registry-only verification, deactivate the managed override:

```sh
bash scripts/source-deps deactivate
```

Do not commit the generated Cargo configuration, sibling paths, moving Git dependencies, or lockfile changes caused only by switching modes. Version bumps, crates.io publication, and registry-only cutover belong to a dedicated release task after the source graph has been proven.
