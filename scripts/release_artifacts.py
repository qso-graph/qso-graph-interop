#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Stage unsigned files and assemble CLI/C bundles after the shared signing jobs.

Only stage reads target/. Archive reads only the downloaded (possibly signed) files.
"""
import shutil
import sys
import tarfile
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLATFORMS = {"macos-arm64", "linux-x86_64", "linux-arm64", "windows-x86_64"}


def version():
    return tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]


def names(platform):
    if platform not in PLATFORMS:
        raise SystemExit(f"Unknown platform: {platform}")
    if platform == "windows-x86_64":
        return "qg-interop.exe", ["qso_graph_interop_ffi.dll", "qso_graph_interop_ffi.dll.lib", "qso_graph_interop_ffi.lib"]
    shared = "dylib" if platform == "macos-arm64" else "so"
    return "qg-interop", [f"libqso_graph_interop_ffi.{shared}", "libqso_graph_interop_ffi.a"]


def expected(platform):
    exe, libraries = names(platform)
    return {"version.txt", f"cli/{exe}", "cli/README.md", "cli/LICENSE", "ffi/README.md", "ffi/LICENSE",
            "ffi/include/qso_graph_interop.h", *(f"ffi/lib/{lib}" for lib in libraries)}


def stage(platform, target, ffi_target, output):
    exe, libraries = names(platform)
    output = Path(output)
    if output.exists():
        raise SystemExit(f"Stage directory already exists: {output}")
    (output / "cli").mkdir(parents=True)
    (output / "ffi/lib").mkdir(parents=True)
    (output / "version.txt").write_text(version() + "\n")
    shutil.copy2(ROOT / "target" / target / "release" / exe, output / "cli" / exe)
    for lib in libraries:
        shutil.copy2(ROOT / "target" / ffi_target / "release" / lib, output / "ffi/lib" / lib)
    shutil.copytree(ROOT / "crates/qso-graph-interop-ffi/include", output / "ffi/include")
    for bundle, crate in [("cli", "qso-graph-interop-cli"), ("ffi", "qso-graph-interop-ffi")]:
        shutil.copy2(ROOT / "crates" / crate / "README.md", output / bundle / "README.md")
        shutil.copy2(ROOT / "LICENSE", output / bundle / "LICENSE")


def archive(platform, source, output):
    exe, libraries = names(platform)
    source, output = Path(source), Path(output)
    actual = {str(p.relative_to(source).as_posix()) for p in source.rglob("*") if p.is_file()}
    if actual != expected(platform) or any(p.is_symlink() for p in source.rglob("*")):
        raise SystemExit(f"Wrong bundle contents: missing {expected(platform) - actual}; extra {actual - expected(platform)}")
    if (source / "version.txt").read_text() != version() + "\n":
        raise SystemExit("Artifact version differs from the checked-out source")
    # Actions artifacts preserve bytes, but not Unix execute permissions.
    (source / "cli" / exe).chmod(0o755)
    for lib in libraries:
        (source / "ffi/lib" / lib).chmod(0o755 if lib.endswith((".so", ".dylib", ".dll")) else 0o644)
    output.mkdir(parents=True, exist_ok=True)
    for folder, prefix in [("cli", "qg-interop"), ("ffi", "qso-graph-interop-ffi")]:
        base = f"{prefix}-{version()}-{platform}"
        files = sorted(p for p in (source / folder).rglob("*") if p.is_file())
        if platform == "windows-x86_64":
            path = output / f"{base}.zip"
            with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as bundle:
                for file in files:
                    bundle.write(file, Path(base) / file.relative_to(source / folder))
        else:
            path = output / f"{base}.tar.gz"
            with tarfile.open(path, "w:gz") as bundle:
                for file in files:
                    bundle.add(file, arcname=str(Path(base) / file.relative_to(source / folder)), recursive=False)
        print(path)


def main(argv):
    if len(argv) == 5 and argv[0] == "stage":
        stage(*argv[1:])
    elif len(argv) == 4 and argv[0] == "archive":
        archive(*argv[1:])
    else:
        raise SystemExit("release_artifacts.py stage PLATFORM TARGET FFI_TARGET OUTPUT | archive PLATFORM SOURCE OUTPUT")


if __name__ == "__main__":
    main(sys.argv[1:])
