#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Run pinned actionlint without depending on optional shellcheck or pyflakes versions."""
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
cache = ROOT / "target/workflow-lint"
env = dict(os.environ, GOPATH=str(cache / "go"), GOCACHE=str(cache / "build"))
# go run module@version checks module deprecation even when its source is cached.
# In an explicitly offline run, compile that same pinned cached module directly.
offline = os.environ.get("GOPROXY") == "off"
module = cache / "go/pkg/mod/github.com/rhysd/actionlint@v1.7.11"
if offline and not module.is_dir():
    raise SystemExit("offline workflow lint requires cached actionlint v1.7.11")
runner = ["-mod=readonly", "./cmd/actionlint"] if offline else [
    "github.com/rhysd/actionlint/cmd/actionlint@v1.7.11"]
subprocess.run([
    "go", "run", *runner,
    "-shellcheck=", "-pyflakes=",
    *map(str, sorted((ROOT / ".github/workflows").glob("*.yml"))),
], cwd=module if offline else ROOT, env=env, check=True)
