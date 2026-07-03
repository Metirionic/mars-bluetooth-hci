#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Assert per-crate README dependency snippets and docs/ prose version literals
match the crate manifest versions.

For each crate in the root Cargo.toml's `[workspace].members`, read the package
`version` from the crate's Cargo.toml and check:
  * the `version = "<v>"` from the README's `<crate> = { version = "...", ... }`
    dependency snippet, normalizing both to MAJOR.MINOR; and
  * every docs/ prose version literal that names the crate — in the forms
    `` `<crate>` (vX.Y.Z) `` or `` <crate>@X.Y.Z `` — compared against the
    manifest on the full version (MAJOR.MINOR.PATCH core).
Exit non-zero on any mismatch, or if a README snippet or manifest version cannot
be found or parsed.

Invoked by the `version-check` job in .github/workflows/ci.yml and runnable
locally: `python3 .github/scripts/check_readme_versions.py`.

Scope: each workspace member's sub-README (MAJOR.MINOR) and the prose version
literals in docs/**/*.md (full version). The root README uses crates.io shields
badges and is intentionally out of scope, as are the cog-generated
*/CHANGELOG.md files (see CONTRIBUTING.md §5).
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


def version_core(version: str, where: str) -> str:
    """Return the full dotted core (MAJOR.MINOR.PATCH) with -pre/+build stripped,
    validating every dot-separated part is numeric. Used for the docs/ prose
    exact-match check (full version), unlike ``major_minor``'s MAJOR.MINOR
    reduction used for the minor-only README snippets. Raises CheckError on a
    non-numeric core."""
    core = version.split("-", 1)[0].split("+", 1)[0]
    parts = core.split(".")
    if not parts or not all(p.isdigit() for p in parts):
        raise CheckError(f"unparseable version {version!r} in {where}")
    return core


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


def load_docs() -> list[tuple[str, str]]:
    """Return ``(repo-relative path, text)`` for every ``docs/**/*.md`` file,
    sorted for stable output. Returns an empty list if ``docs/`` is absent, in
    which case the prose pass is a no-op."""
    docs_root = ROOT / "docs"
    if not docs_root.is_dir():
        return []
    out = []
    for path in sorted(docs_root.rglob("*.md")):
        rel = path.relative_to(ROOT).as_posix()
        out.append((rel, path.read_text(encoding="utf-8")))
    return out


def check_docs_prose(crate_name: str, manifest_version: str, docs: list[tuple[str, str]]) -> bool:
    """Check every docs/ prose version literal naming ``crate_name`` against the
    manifest version (full MAJOR.MINOR.PATCH core match). Returns True iff every
    literal matches; prints OK / MISMATCH / ERROR per literal. A crate with no
    prose literal is not an error (returns True) — not every crate must appear in
    docs prose. Per-literal isolation keeps one malformed literal from aborting
    the rest.

    Two prose forms are recognised, each anchored on the exact crate name so
    unrelated version mentions (e.g. external crates like ``postcard v1.1.3``)
    are not flagged:

      * `` `<crate>` (vX.Y.Z) ``  — backticked name + parenthesized v + full SemVer
      * `` <crate>@X.Y.Z ``       — name + @ + full SemVer (e.g. ``GIT_TAG <crate>@0.8.0``)

    The ``(?<![\\w-])`` lookbehind prevents matching the crate name as a
    substring of a longer token before ``@``.
    """
    ok = True
    manifest_core = version_core(manifest_version, f"{crate_name} Cargo.toml")
    paren_v = re.compile(r"`" + re.escape(crate_name) + r"`\s*\(v(?P<v>\d+\.\d+\.\d+)\)")
    at_ver = re.compile(r"(?<![\w-])" + re.escape(crate_name) + r"@(?P<v>\d+\.\d+\.\d+)")
    for rel, text in docs:
        for lineno, line in enumerate(text.splitlines(), 1):
            for pat, fmt in ((paren_v, "(vX.Y.Z)"), (at_ver, "@X.Y.Z")):
                for m in pat.finditer(line):
                    pv = m.group("v")
                    try:
                        pv_core = version_core(pv, f"{rel}:{lineno}")
                    except CheckError as exc:
                        ok = False
                        print(f"check_readme_versions: ERROR: {exc}", file=sys.stderr)
                        continue
                    if pv_core != manifest_core:
                        ok = False
                        print(
                            f"check_readme_versions: MISMATCH `{crate_name}` docs prose "
                            f"{rel}:{lineno} ({fmt} {pv!r}) != Cargo.toml "
                            f"{manifest_version!r}. Update the docs/ prose literal "
                            f"to match the manifest version.",
                            file=sys.stderr,
                        )
                    else:
                        print(
                            f"check_readme_versions: OK `{crate_name}` docs prose "
                            f"{rel}:{lineno} ({fmt} {pv!r} ~ manifest "
                            f"{manifest_version!r})"
                        )
    return ok


def main() -> int:
    ok = True
    docs = load_docs()
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
        # docs/ prose pass: full-version (MAJOR.MINOR.PATCH core) match.
        if not check_docs_prose(crate_name, mv, docs):
            ok = False
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())