#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Seed libFuzzer with every independent wire fixture."""
import base64
import json
from pathlib import Path
root=Path(__file__).resolve().parent.parent
corpus=root/'fuzz/corpus/wsjtx'
corpus.mkdir(parents=True,exist_ok=True)
for vector in json.loads((root/'crates/qso-graph-interop/tests/fixtures/wire.json').read_text())['vectors']:
    (corpus/vector['name']).write_bytes(bytes.fromhex(vector['wire_hex']))

for family in ('wsjtx', 'qgi', 'n1mm', 'fldigi', 'pskreporter-ipfix', 'pskreporter-mqtt'):
    corpus = root / 'fuzz/corpus' / ('mqtt' if family == 'pskreporter-mqtt' else 'ipfix' if family == 'pskreporter-ipfix' else family if family in ('n1mm', 'fldigi') else 'wsjtx')
    corpus.mkdir(parents=True, exist_ok=True)
    folder = root / 'spec/vectors' / family
    manifest = json.loads((folder / 'manifest.json').read_text())
    for entry in manifest['vectors']:
        vector = json.loads((folder / entry['file']).read_text())
        supplied = vector['input']
        stem = family + '-' + entry['file'].removesuffix('.json')
        if family == 'fldigi':
            (corpus / (stem + '-input')).write_text(json.dumps(supplied, separators=(',', ':')))
        if family == 'pskreporter-mqtt' and 'data' in supplied:
            raw = base64.b64decode(supplied['data'])
            (corpus / (stem + '-topic')).write_bytes(supplied['mqtt']['topic'].encode() + b'\n' + raw)
            (corpus / (stem + '-capture')).write_text(json.dumps(dict(topic=supplied['mqtt']['topic'],payload_b64=supplied['data'],retain=True,qos=0)))
        if 'data' in supplied:
            (corpus / stem).write_bytes(base64.b64decode(supplied['data']))
        if 'event' in supplied:
            (corpus / (stem + '-event')).write_text(json.dumps(supplied['event'], separators=(',', ':')))
        if 'event_json' in supplied:
            (corpus / (stem + '-json')).write_text(supplied['event_json'])

# Local input parsing shares the same arbitrary-byte harness.
corpus = root / 'fuzz/corpus/wsjtx'
(corpus / "alltxt-sample").write_bytes((root / "crates/qso-graph-interop/tests/fixtures/ALL-sample.TXT").read_bytes())
(corpus / "alltxt-edges").write_bytes(b"header\n260407_120000    14.074 Rx FT8 -12 -0.2 1234 CQ Q1AAA AA00 ?\r\n260229_000000 14 Tx FT8 1 0 1 BAD\n")

# Reader contexts can contain future enum values; strict producer validation is separate.
for fixture in (root / 'crates/qso-graph-interop/tests/fixtures').glob('fldigi-*.json'):
    (root / 'fuzz/corpus/fldigi' / fixture.stem).write_bytes(fixture.read_bytes())
