#!/usr/bin/env python3
"""Fail unless every nlp-stack dependency is one exact public git revision (owner decision, #8).

Checks the parsed manifest and the resolved lock, not raw text: the four workspace
`moenarch-text-*` dependencies must be git dependencies on the public nlp-stack repository
with one full commit `rev` (no version/branch/tag/path), and every resolved `moenarch-text-*`
package in Cargo.lock must come from exactly that git source, with no registry or second copy.
"""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NLP_GIT = "https://github.com/moritzbrantner/nlp-stack"
EXPECTED = {
    "text-core": "moenarch-text-core",
    "text-embeddings": "moenarch-text-embeddings",
    "text-linguistics": "moenarch-text-linguistics",
    "text-retrieval": "moenarch-text-retrieval",
}
FULL_REV = re.compile(r"^[0-9a-f]{40}$")


def main() -> int:
    errors: list[str] = []
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())
    dependencies = manifest.get("workspace", {}).get("dependencies", {})
    nlp_keys = {
        key
        for key, spec in dependencies.items()
        if isinstance(spec, dict) and str(spec.get("package", "")).startswith("moenarch-text-")
    }
    if nlp_keys != set(EXPECTED):
        errors.append(f"expected nlp-stack dependencies {sorted(EXPECTED)}, found {sorted(nlp_keys)}")

    revs: set[str] = set()
    for key, package in EXPECTED.items():
        spec = dependencies.get(key)
        if not isinstance(spec, dict):
            errors.append(f"{key}: missing or not a table")
            continue
        if spec.get("package") != package:
            errors.append(f"{key}: package must be {package}")
        if spec.get("git") != NLP_GIT:
            errors.append(f"{key}: git must be {NLP_GIT}")
        rev = spec.get("rev")
        if not isinstance(rev, str) or not FULL_REV.fullmatch(rev):
            errors.append(f"{key}: rev must be a full 40-hex commit")
        else:
            revs.add(rev)
        extra = set(spec) - {"package", "git", "rev", "features", "default-features", "optional"}
        if extra:
            errors.append(f"{key}: unexpected keys {sorted(extra)}")
    if len(revs) != 1:
        errors.append(f"all nlp-stack dependencies must share one rev, found {sorted(revs)}")

    if len(revs) == 1:
        (rev,) = revs
        expected_source = f"git+{NLP_GIT}?rev={rev}#{rev}"
        lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
        seen: dict[str, list[str]] = {}
        for package in lock.get("package", []):
            name = package["name"]
            if name.startswith("moenarch-text-"):
                seen.setdefault(name, []).append(package.get("source", "<path>"))
        for package in EXPECTED.values():
            if package not in seen:
                errors.append(f"Cargo.lock: {package} is not resolved")
        for name, sources in sorted(seen.items()):
            if sources != [expected_source]:
                errors.append(f"Cargo.lock: {name} must resolve once from {expected_source}, found {sources}")

    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
