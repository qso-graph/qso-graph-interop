#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate supplemental synthetic decode sequences not covered by specification vectors.

All layout, schema, command and rejection fixtures are superseded by spec/vectors/wsjtx.
This independent struct-based generator retains the 120-message variation corpus.
"""
import argparse
import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parent.parent
DEST = ROOT / 'crates/qso-graph-interop/tests/fixtures/wire.json'


def text(value):
    data = value.encode('utf-8')
    return struct.pack('>I', len(data)) + data


def vectors():
    result = []
    for index, line in enumerate((DEST.parent / 'ALL-sample.TXT').read_text().splitlines()):
        stamp, _, _, _, snr, delta, offset, message = line.split(maxsplit=7)
        clock = stamp.split('_')[1]
        ms = (int(clock[:2]) * 3600 + int(clock[2:4]) * 60 + int(clock[4:])) * 1000
        wire = (struct.pack('>III', 0xadbccbda, 3, 2) + text('synthetic-instance')
                + struct.pack('>BIidI', 1, ms, int(snr), float(delta), int(offset))
                + text('~') + text(message) + bytes([0, 0]))
        fields = dict(is_new=True, time_of_day=f'{clock[:2]}:{clock[2:4]}:{clock[4:]}.000',
                      snr_db=int(snr), dt_seconds=float(delta), audio_offset_hz=int(offset),
                      mode='~', text=message, low_confidence=False, off_air=False,
                      radio=dict(instance='synthetic-instance'))
        result.append(dict(name=f'sample_{index:03}', wire_hex=wire.hex(), fields=fields))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = json.dumps(dict(spdx='GPL-3.0-or-later', vectors=vectors()), indent=2) + '\n'
    if args.check:
        if DEST.read_text() != data:
            raise SystemExit('wire.json differs; run scripts/generate-fixtures.py')
        print(f'{len(vectors())} supplemental synthetic fixtures: pass')
    else:
        DEST.write_text(data)


if __name__ == '__main__':
    main()
