#!/usr/bin/env python3
"""Bump the project version and roll Keep a Changelog Unreleased notes.

Used by the Create release workflow. Detects a Cargo crate (Cargo.toml) and/or
an npm package (package.json) at the repo root. Does not write zola.toml;
the docs CLI injects CARGO_PKG_VERSION at build time.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tomllib
from datetime import date
from pathlib import Path

UNRELEASED = "## [Unreleased]"
PACKAGE_HEAD = re.compile(r"(?m)^\[package\]\s*$")
NEXT_TABLE = re.compile(r"(?m)^\[")
PACKAGE_VERSION = re.compile(r'(?m)^version = "([^"]+)"')


def die(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def bump_semver(version: str, bump: str) -> str:
    core = version.split("+", 1)[0].split("-", 1)[0]
    parts = core.split(".")
    if len(parts) != 3 or not all(p.isdigit() for p in parts):
        die(f"not a stable X.Y.Z version: {version}")
    major, minor, patch = (int(p) for p in parts)
    if bump == "major":
        return f"{major + 1}.0.0"
    if bump == "minor":
        return f"{major}.{minor + 1}.0"
    if bump == "patch":
        return f"{major}.{minor}.{patch + 1}"
    die(f"unknown bump: {bump}")


def cargo_package_version(text: str) -> str:
    data = tomllib.loads(text)
    package = data.get("package")
    if not isinstance(package, dict):
        die("Cargo.toml has no [package] table")
    version = package.get("version")
    if not isinstance(version, str):
        die("Cargo.toml [package] has no version string (workspace inheritance is not supported)")
    return version


def set_cargo_package_version(text: str, new: str) -> str:
    match = PACKAGE_HEAD.search(text)
    if not match:
        die("Cargo.toml has no [package] table")
    start = match.end()
    nxt = NEXT_TABLE.search(text, start)
    end = nxt.start() if nxt else len(text)
    section = text[start:end]
    new_section, n = PACKAGE_VERSION.subn(f'version = "{new}"', section, count=1)
    if n != 1:
        die("Cargo.toml [package] has no version = \"...\" line")
    return text[:start] + new_section + text[end:]


def npm_version(path: Path) -> str:
    data = json.loads(path.read_text())
    version = data.get("version")
    if not isinstance(version, str):
        die(f"{path} has no version string")
    return version


def current_version(root: Path) -> tuple[str, list[str]]:
    kinds: list[str] = []
    versions: list[str] = []
    cargo = root / "Cargo.toml"
    npm = root / "package.json"
    if cargo.is_file():
        versions.append(cargo_package_version(cargo.read_text()))
        kinds.append("cargo")
    if npm.is_file():
        versions.append(npm_version(npm))
        kinds.append("npm")
    if not kinds:
        die("no Cargo.toml or package.json at repo root")
    if len(set(versions)) != 1:
        die(f"version mismatch between package files: {', '.join(versions)}")
    return versions[0], kinds


def roll_changelog(text: str, new_version: str, today: str) -> str:
    if UNRELEASED not in text:
        die("CHANGELOG.md has no ## [Unreleased] heading")
    heading = f"## [{new_version}] - {today}"
    if f"## [{new_version}]" in text:
        die(f"CHANGELOG.md already has {new_version}")
    pre, rest = text.split(UNRELEASED, 1)
    nxt = re.search(r"\n## \[", rest)
    if nxt:
        notes = rest[: nxt.start()]
        after = rest[nxt.start() :]
    else:
        notes = rest
        after = ""
    notes = notes.strip("\n")
    out = f"{pre}{UNRELEASED}\n\n{heading}\n"
    if notes.strip():
        out += f"\n{notes.strip()}\n"
    if after and not after.startswith("\n"):
        out += "\n"
    out += after
    return out


def write_github_output(**values: str) -> None:
    dest = os.environ.get("GITHUB_OUTPUT")
    if not dest:
        for key, value in values.items():
            print(f"{key}={value}")
        return
    with open(dest, "a", encoding="utf-8") as handle:
        for key, value in values.items():
            handle.write(f"{key}={value}\n")


def run(cmd: list[str], cwd: Path) -> None:
    print("+", " ".join(cmd), flush=True)
    subprocess.run(cmd, cwd=cwd, check=True)


def cut(root: Path, bump: str, today: str) -> tuple[str, list[str]]:
    old, kinds = current_version(root)
    new = bump_semver(old, bump)
    changelog = root / "CHANGELOG.md"
    if not changelog.is_file():
        die("CHANGELOG.md is missing")
    changelog.write_text(
        roll_changelog(changelog.read_text(), new, today), encoding="utf-8"
    )
    if "cargo" in kinds:
        cargo = root / "Cargo.toml"
        cargo.write_text(set_cargo_package_version(cargo.read_text(), new), encoding="utf-8")
        name = tomllib.loads(cargo.read_bytes())["package"]["name"]
        run(["cargo", "update", "-p", name], cwd=root)
    if "npm" in kinds:
        run(
            ["npm", "version", new, "--no-git-tag-version", "--allow-same-version"],
            cwd=root,
        )
    print(f"bumped {old} -> {new} ({', '.join(kinds)})")
    return new, kinds


def self_test() -> None:
    assert bump_semver("0.2.6", "patch") == "0.2.7"
    assert bump_semver("0.2.6", "minor") == "0.3.0"
    assert bump_semver("1.2.3", "major") == "2.0.0"
    sample = """# Changelog\n\n## [Unreleased]\n\n### Added\n\n- New workflow\n\n## [0.2.6] - 2026-09-10\n"""
    rolled = roll_changelog(sample, "0.2.7", "2026-09-10")
    assert "## [Unreleased]\n\n## [0.2.7] - 2026-09-10\n\n### Added\n\n- New workflow\n" in rolled
    cargo = '[package]\nname = "docs"\nversion = "0.2.6"\n\n[dependencies]\nanyhow = "1"\n'
    assert 'version = "0.2.7"' in set_cargo_package_version(cargo, "0.2.7")
    assert 'anyhow = "1"' in set_cargo_package_version(cargo, "0.2.7")
    print("self-test ok")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bump", choices=("patch", "minor", "major"))
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--date", default=date.today().isoformat())
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    bump = args.bump or os.environ.get("BUMP")
    if bump not in {"patch", "minor", "major"}:
        die("pass --bump patch|minor|major or set BUMP")
    version, kinds = cut(args.root.resolve(), bump, args.date)
    write_github_output(version=version, tag=f"v{version}", kinds=",".join(kinds))


if __name__ == "__main__":
    main()
