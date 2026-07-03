#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Assert per-crate README dependency snippets match the crate manifest versions.

For each crate in the root Cargo.toml's `[workspace].members`, read the package
`version` from the crate's Cargo.toml and the `version = "<v>"` from the
README's `<crate> = { version = "...", ... }` dependency snippet, normalize both
to MAJOR.MINOR, and exit non-zero on mismatch or if either version cannot be
found.

Invoked by the `version-check` job in .github/workflows/ci.yml and runnable
locally: `python3 .github/scripts/check_readme_versions.py`.

Scope: each workspace member's sub-README. The root README uses crates.io
shields badges and is intentionally out of scope (see CONTRIBUTING.md §5). Prose
version literals in docs/ are also out of scope.
"""

from __future__ import annotations

import re
import sys

if sys.version_info < (3, 11):
    sys.exit(
        "check_readme_versions: requires Python 3.11+ (stdlib tomllib). "
        f"Found {sys.version.split()[0]}."
    )

import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class CheckError(Exception):
    """Per-crate error: report it but keep checking the remaining crates."""


def workspace_members() -> list[str]:
    """Return `[workspace].members` from the root Cargo.toml."""
    root_cargo = ROOT / "Cargo.toml"
    if not root_cargo.is_file():
        raise CheckError(f"workspace manifest not found: {root_cargo}")
    with root_cargo.open("rb") as fh:
        data = tomllib.load(fh)
    ws = data.get("workspace")
    members = ws.get("members") if isinstance(ws, dict) else None
    if not isinstance(members, list) or not members:
        raise CheckError(f"[workspace].members missing or empty in {root_cargo}")
    return members


def manifest_version(crate_dir: str) -> tuple[str, str]:
    """Return ``(package_name, version)`` from ``<crate_dir>/Cargo.toml``.

    Validates that both ``name`` and ``version`` are present strings, so a
    workspace-inherited (``version.workspace = true``) or otherwise non-string
    version raises a clean ``CheckError`` instead of a later ``AttributeError``
    deep in ``major_minor``.
    """
    cargo = ROOT / crate_dir / "Cargo.toml"
    if not cargo.is_file():
        raise CheckError(f"manifest not found: {cargo}")
    with cargo.open("rb") as fh:
        data = tomllib.load(fh)
    pkg = data.get("package")
    if not isinstance(pkg, dict):
        raise CheckError(f"[package] table missing in {cargo}")
    name = pkg.get("name")
    version = pkg.get("version")
    if not isinstance(name, str):
        raise CheckError(f"[package].name missing or non-string in {cargo}")
    if not isinstance(version, str):
        raise CheckError(f"[package].version missing or non-string in {cargo}")
    return name, version


def major_minor(version: str, where: str) -> str:
    """Reduce a version string to MAJOR.MINOR. Strips -pre/+build, tolerates
    minor-only requirements. Raises CheckError on non-numeric cores."""
    core = version.split("-", 1)[0].split("+", 1)[0]
    parts = core.split(".")
    if len(parts) < 2 or not all(p.isdigit() for p in parts):
        raise CheckError(f"unparseable version {version!r} in {where}")
    return f"{parts[0]}.{parts[1]}"


def readme_snippet_version(crate_name: str, readme_rel: str) -> str:
    readme = ROOT / readme_rel
    if not readme.is_file():
        raise CheckError(f"README not found: {readme}")
    text = readme.read_text(encoding="utf-8")
    line_pat = re.compile(
        r"^[ \t]*" + re.escape(crate_name) + r"[ \t]*=[ \t]*\{(?P<inner>[^}]*)\}",
        re.MULTILINE,
    )
    line_m = line_pat.search(text)
    if not line_m:
        raise CheckError(f"no `{crate_name} = {{ ... }}` dependency snippet found in {readme}")
    ver_m = re.search(r"\bversion[ \t]*=[ \t]*\"([^\"]+)\"", line_m.group("inner"))
    if not ver_m:
        raise CheckError(f"`{crate_name} = {{ ... }}` snippet in {readme} has no `version = \"...\"` key")
    return ver_m.group(1)


def main() -> int:
    ok = True
    for crate_dir in workspace_members():
        readme_rel = f"{crate_dir}/README.md"
        try:
            crate_name, mv = manifest_version(crate_dir)
            rv = readme_snippet_version(crate_name, readme_rel)
            mm_mv = major_minor(mv, f"{crate_dir}/Cargo.toml")
            mm_rv = major_minor(rv, f"{readme_rel} ({crate_name})")
        except CheckError as exc:
            ok = False
            print(f"check_readme_versions: ERROR: {exc}", file=sys.stderr)
            continue
        if mm_mv != mm_rv:
            ok = False
            print(
                f"check_readme_versions: MISMATCH `{crate_name}`: "
                f"{readme_rel} snippet {rv!r} (MAJOR.MINOR {mm_rv}) != "
                f"{crate_dir}/Cargo.toml {mv!r} (MAJOR.MINOR {mm_mv}). "
                f"Update the README snippet to match the manifest version.",
                file=sys.stderr,
            )
        else:
            print(
                f"check_readme_versions: OK `{crate_name}` "
                f"(README {rv!r} ~ manifest {mv!r})"
            )
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())