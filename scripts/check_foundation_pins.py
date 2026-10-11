#!/usr/bin/env python3
"""Fail unless moenarch-corpus-core is one exact public git revision of moenarch-foundation (#8).

corpus-core is not published, so it is the only Foundation crate consumed from git: the workspace
`corpus-core` dependency must be a git dependency on the public moenarch-foundation repository with
one full commit `rev` (no version/branch/tag/path), Cargo.lock must resolve
`moenarch-corpus-core` exactly once from that source, and no other package may resolve from the
Foundation git repository (other Foundation crates stay on their registry releases, so a second
copy of e.g. media-core cannot slip in).
"""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FOUNDATION_GIT = "https://github.com/moritzbrantner/moenarch-foundation"
KEY = "corpus-core"
PACKAGE = "moenarch-corpus-core"
FULL_REV = re.compile(r"^[0-9a-f]{40}$")


def main() -> int:
    errors: list[str] = []
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())
    dependencies = manifest.get("workspace", {}).get("dependencies", {})

    declared = [
        key
        for key, spec in dependencies.items()
        if (spec.get("package", key) if isinstance(spec, dict) else key) == PACKAGE
    ]
    if declared != [KEY]:
        errors.append(f"expected exactly one workspace dependency {KEY} -> {PACKAGE}, found {declared}")

    rev = None
    spec = dependencies.get(KEY)
    if isinstance(spec, dict):
        if spec.get("package") != PACKAGE:
            errors.append(f"{KEY}: package must be {PACKAGE}")
        if spec.get("git") != FOUNDATION_GIT:
            errors.append(f"{KEY}: git must be {FOUNDATION_GIT}")
        rev = spec.get("rev")
        if not isinstance(rev, str) or not FULL_REV.fullmatch(rev):
            errors.append(f"{KEY}: rev must be a full 40-hex commit")
            rev = None
        extra = set(spec) - {"package", "git", "rev", "features", "default-features", "optional"}
        if extra:
            errors.append(f"{KEY}: unexpected keys {sorted(extra)}")
    else:
        errors.append(f"{KEY}: missing or not a table")

    if rev is not None:
        expected_source = f"git+{FOUNDATION_GIT}?rev={rev}#{rev}"
        lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
        corpus_sources = []
        for package in lock.get("package", []):
            source = package.get("source", "<path>")
            if package["name"] == PACKAGE:
                corpus_sources.append(source)
            elif source.startswith(f"git+{FOUNDATION_GIT}"):
                errors.append(f"Cargo.lock: only {PACKAGE} may resolve from Foundation git, found {package['name']}")
        if corpus_sources != [expected_source]:
            errors.append(f"Cargo.lock: {PACKAGE} must resolve once from {expected_source}, found {corpus_sources}")

    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
