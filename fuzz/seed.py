#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Seed libFuzzer with every independent wire fixture."""
import json
from pathlib import Path
root=Path(__file__).resolve().parent.parent
corpus=root/'fuzz/corpus/wsjtx'
corpus.mkdir(parents=True,exist_ok=True)
for vector in json.loads((root/'crates/qso-graph-interop/tests/fixtures/wire.json').read_text())['vectors']:
    (corpus/vector['name']).write_bytes(bytes.fromhex(vector['wire_hex']))
