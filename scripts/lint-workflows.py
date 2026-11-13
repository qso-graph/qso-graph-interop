#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Run pinned actionlint without depending on optional shellcheck or pyflakes versions."""
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
cache = ROOT / "target/workflow-lint"
env = dict(os.environ, GOPATH=str(cache / "go"), GOCACHE=str(cache / "build"))
subprocess.run([
    "go", "run", "github.com/rhysd/actionlint/cmd/actionlint@v1.7.11",
    "-shellcheck=", "-pyflakes=",
    *map(str, sorted((ROOT / ".github/workflows").glob("*.yml"))),
], cwd=ROOT, env=env, check=True)
