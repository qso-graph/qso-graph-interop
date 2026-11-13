#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Keep packaged licenses and embedded CLI inputs identical to their repository originals."""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COPIES = {
    "crates/qso-graph-interop/LICENSE": "LICENSE",
    "crates/qso-graph-interop-cli/LICENSE": "LICENSE",
    "crates/qso-graph-interop-cli/data/ALL-sample.TXT": "crates/qso-graph-interop/tests/fixtures/ALL-sample.TXT",
    "crates/qso-graph-interop-cli/data/annex-e-fldigi.md": "spec/annex-e-fldigi.md",
}


def main(argv):
    if argv not in ([], ["--check"]):
        sys.exit(__doc__)
    for dest, source in COPIES.items():
        original, copy = ROOT / source, ROOT / dest
        if argv:
            if not copy.is_file() or copy.read_bytes() != original.read_bytes():
                sys.exit(f"{dest} differs from {source}; run uv run --no-project python scripts/package-files.py")
        else:
            copy.parent.mkdir(parents=True, exist_ok=True)
            copy.write_bytes(original.read_bytes())


if __name__ == "__main__":
    main(sys.argv[1:])
