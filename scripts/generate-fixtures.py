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


def beta6_vectors():
    """Separate source transcription, independent of spec/vector builders and Rust."""
    def packet(kind, payload):
        return struct.pack('>III', 0xadbccbda, 3, kind) + text('synthetic-instance') + payload
    inhibit = packet(17, b'\x01\x01' + text('Q3CCC') + struct.pack('>IIII', 7, 11, 13, 17))
    command = packet(18, text('key-controller') + struct.pack('>I', 30000) + text('Q3CCC'))
    status = packet(1, struct.pack('>Q', 14074000) + text('JTTY') + text('') + text('-10') + text('JTTY')
                    + struct.pack('>BBBII', 0, 0, 0, 1500, 1500)
                    + text('Q2AAA') + text('AA00') + text('') + b'\x00' + text('')
                    + struct.pack('>BBII', 0, 0, 2**32-1, 2**32-1) + text('Default') + text(''))
    radio = {'instance': 'synthetic-instance'}
    return dict(spdx='GPL-3.0-or-later', source_tag='v3.3.0-beta6',
        source_tarball_sha256='cbbbfd213de2dabfb0d2bac626a3b2ebb9e7b1836bd2771f90d12800771694be',
        vectors=[
            dict(name='inhibit-status', wire_hex=inhibit.hex(), kind='radio_state', mapping=2,
                 fields=dict(inhibit_supported=True, tx_inhibited=True, source_station='Q3CCC',
                             hold_rx=7, release_rx=11, expiries=13, invalid=17, radio=radio)),
            dict(name='tx-inhibit', wire_hex=command.hex(), kind='control', mapping=2,
                 fields=dict(operation='wsjtx.tx_inhibit', controller_id='key-controller', ttl_ms=30000, station='Q3CCC')),
            dict(name='jtty-status', wire_hex=status.hex(), kind='radio_state', mapping=1,
                 fields=dict(dial_frequency_hz=14074000, mode='JTTY', dx_call='', report='-10', tx_mode='JTTY',
                             tx_enabled=False, transmitting=False, decoding=False, rx_offset_hz=1500, tx_offset_hz=1500,
                             station_call='Q2AAA', station_grid='AA00', dx_grid='', tx_watchdog=False, sub_mode='',
                             fast_mode=False,
                             configuration_name='Default', tx_message='', radio=radio)),
        ])


def check_jtty19_capture_fixture():
    """Read copied datagrams independently of the Rust codec and spec generators."""
    import datetime
    fixture = json.loads((DEST.parent / 'wsjtx-jtty19.json').read_text())
    assert len(fixture['vectors']) == 22
    counts = {}
    for vector in fixture['vectors']:
        data = bytes.fromhex(vector['wire_hex'])
        magic, schema, typ, length = struct.unpack_from('>IIII', data)
        assert (magic, schema, typ) == (0xadbccbda, 2, 19)
        ident = data[16:16+length].decode('utf-8')
        offset = 16+length
        mid, jd, ms, ts, df, snr, length = struct.unpack_from('>qqIBIiI', data, offset)
        assert ts == 1 and ms < 86400000
        offset += 33
        text = data[offset:offset+length].decode('utf-8')
        complete, off_air = data[offset+length:]
        assert complete in (0, 1) and off_air in (0, 1)
        start = datetime.datetime.combine(datetime.date.fromordinal(jd-1721425), datetime.time())
        start += datetime.timedelta(milliseconds=ms)
        fields = dict(message_id=str(mid), start=start.isoformat(timespec='milliseconds')+'Z',
                      audio_offset_hz=df, snr_db=snr, text=text, complete=bool(complete),
                      off_air=bool(off_air), radio=dict(instance=ident))
        assert vector['fields'] == fields and vector['wire_id'] == ident
        counts[vector['capture']] = counts.get(vector['capture'], 0)+1
    assert sorted(counts.values()) == [11, 11]
    print('22 byte-exact JTTYText captured fixtures: pass')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    check_jtty19_capture_fixture()
    data = json.dumps(dict(spdx='GPL-3.0-or-later', vectors=vectors()), indent=2) + '\n'
    beta_path = DEST.parent / 'wsjtx-beta6.json'
    beta = json.dumps(beta6_vectors(), indent=2) + '\n'
    if args.check:
        if beta_path.read_text() != beta:
            raise SystemExit('wsjtx-beta6.json differs; run scripts/generate-fixtures.py')
        if DEST.read_text() != data:
            raise SystemExit('wire.json differs; run scripts/generate-fixtures.py')
        print(f'{len(vectors())} supplemental synthetic fixtures: pass')
    else:
        DEST.write_text(data)
        beta_path.write_text(beta)


if __name__ == '__main__':
    main()
