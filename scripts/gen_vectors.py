#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate QGI 0.1.0 vectors from Annexes B, D, E and I and the pinned released header.

No implementation imports, subprocesses, third-party generation dependencies or network access.
The byte packers transcribe wire primitives; expected common fields are explicit test oracles.
"""

import argparse
import base64
import copy
import hashlib
import json
import math
import re
from pathlib import Path
import struct
import sys

import gen_fldigi_vectors as fldigi

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / 'spec/vectors/wsjtx'
SPEC = '0.1.0'
PIN = 'fd73b7f92fc0f66ea8f12a147b59a6072408c24656735d59df772dc55a5eeb62'


def b64(value):
    return base64.b64encode(value).decode('ascii')


def dump(value):
    return (json.dumps(value, ensure_ascii=True, indent=2, allow_nan=False) + '\n').encode()


def pack(fmt, *values):
    return struct.pack('>' + fmt, *values)


def utf8(value):
    if value is None:
        return pack('I', 0xffffffff)
    data = value.encode('utf-8') if isinstance(value, str) else value
    return pack('I', len(data)) + data


def qstring(value):
    if value is None:
        return pack('I', 0xffffffff)
    data = value.encode('utf-16-be') if isinstance(value, str) else pack('H' * len(value), *value)
    return pack('I', len(data)) + data


def text(value):
    if isinstance(value, bytes):
        try:
            return value.decode('utf-8')
        except UnicodeDecodeError:
            return {'utf8_bytes': list(value)}
    return value


def header(schema, typ, identity='test-instance'):
    return pack('III', 0xadbccbda, schema, typ) + utf8(identity)


def context(direction='from_application'):
    return {'direction': direction, 'exchange': 'unknown'}


def date(schema, spec=1, milliseconds=3600123, day=2440588, offset=3600, zone=None):
    """Raw date/time layout, Annex B.2; caller supplies the UTC oracle separately."""
    data = pack('qIB', day, milliseconds, spec)
    view = {'julian_day': str(day), 'milliseconds': milliseconds, 'time_spec': spec}
    if schema >= 2 and spec == 2:
        data += pack('i', offset)
        view['offset_seconds'] = offset
    elif schema >= 2 and spec == 3:
        if zone is None:
            zone = {'key': 'Etc/UTC'}
        key = zone['key']
        data += qstring(key)
        view['zone'] = copy.deepcopy(zone)
        if isinstance(key, list):
            view['zone']['key'] = {'utf16_units': key}
        if key == 'OffsetFromUtc':
            data += qstring(zone['id']) + pack('i', zone['offset_seconds'])
            data += qstring(zone['name']) + qstring(zone['abbreviation'])
            data += pack('i', zone['country']) + qstring(zone['comment'])
    return data, view


class Suite:
    def __init__(self, family='wsjtx'):
        self.family = family
        self.vectors = {}

    def add(self, name, value):
        assert name not in self.vectors, name
        self.vectors[name] = {'$comment': 'SPDX-License-Identifier: GPL-3.0-or-later', 'spec': SPEC, 'family': self.family, 'mapping': 1, **value}

    def positive(self, name, schema, typ, body, kind, fields, *, direction=None,
                 identity='test-instance', message=None, extensions=b'', canonical=None):
        if direction is None:
            direction = 'to_application' if typ in (4, 7, 8, 9, 11, 13, 14, 15, 16) else 'from_application'
        raw = header(schema, typ, identity) + body + extensions
        event = {'spec': SPEC, 'family': 'wsjtx', 'mapping': 1,
                 'native_version': str(schema), 'kind': kind, 'wire_id': text(identity),
                 'context': context(direction), 'fields': fields,
                 'native': {'format': 'wsjtx_datagram', 'data': b64(raw)}}
        if kind in ('decode', 'radio_state', 'qso_logged', 'qso_changed', 'qso_deleted', 'spot'):
            fields['radio'] = {'instance': text(identity)}
        if typ == 13:
            fields.update(background_color={'spec': 1, 'channels': [65535, 0x1111, 0x2222, 0x3333, 0x4444]},
                          foreground_color={'spec': 0, 'channels': [1, 2, 3, 4, 5]})
        value = {'class': 'positive', 'input': {'data': b64(raw), 'context': context(direction)},
                 'expected': {'event': event}}
        if message is not None:
            value['expected_message'] = {'schema': schema, 'type': typ, 'id': text(identity),
                                         'body': message, 'extensions': b64(extensions)}
        if typ in (0, 3, 4, 6, 7, 8, 9, 11, 13, 14, 15, 16):
            encoded = raw if canonical is None else header(schema, typ, identity) + canonical + extensions
            value['reencode'] = {'data': b64(encoded)}
        self.add(name, value)
        return event

    def negative(self, name, raw, category):
        self.add(name, {'class': 'negative', 'input': {'data': b64(raw), 'context': context()},
                        'expected': {'error': {'category': category}}})

    def skip(self, name, raw, reason, version, typ=None):
        skip = {'reason': reason, 'native_version': str(version)}
        if typ is not None:
            skip['type'] = typ
        self.add(name, {'class': 'skip', 'input': {'data': b64(raw), 'context': context()},
                        'expected': {'skip': skip}})


def build():
    s = Suite()
    # Independent sentinels distinguish every same-type field, especially Rx DF and Tx DF.
    for schema in (1, 2, 3):
        suffix = f'-s{schema}'
        s.positive('00-heartbeat' + suffix, schema, 0, pack('I', 3) + utf8('3.0.2') + utf8('test-r1'),
                   'lifecycle', {'state': 'heartbeat', 'max_schema': 3, 'version': '3.0.2', 'revision': 'test-r1'},
                   message={'max_schema': 3, 'version': '3.0.2', 'revision': 'test-r1'})
        status_parts = [pack('Q', 14074000), utf8('FT8'), utf8('Q1AAA'), utf8('-09'), utf8('FT4'),
                        pack('BBBII', 1, 0, 1, 1201, 2302), utf8('Q2AAA'), utf8('FN31'), utf8('FN42'),
                        pack('B', 0), utf8('sub-test'), pack('BBII', 1, 3, 50, 15), utf8('test-config'), utf8('test-tx')]
        status_fields = {'dial_frequency_hz': 14074000, 'mode': 'FT8', 'dx_call': 'Q1AAA', 'report': '-09',
                         'tx_mode': 'FT4', 'tx_enabled': True, 'transmitting': False, 'decoding': True,
                         'rx_offset_hz': 1201, 'tx_offset_hz': 2302, 'station_call': 'Q2AAA',
                         'station_grid': 'FN31', 'dx_grid': 'FN42', 'tx_watchdog': False, 'sub_mode': 'sub-test',
                         'fast_mode': True, 'frequency_tolerance_hz': 50, 'tr_period_seconds': 15,
                         'configuration_name': 'test-config', 'tx_message': 'test-tx'}
        s.positive('01-status' + suffix, schema, 1, b''.join(status_parts), 'radio_state', status_fields,
                   message={'dial_frequency_hz': '14074000', 'mode': 'FT8', 'dx_call': 'Q1AAA', 'report': '-09',
                            'tx_mode': 'FT4', 'tx_enabled': True, 'transmitting': False, 'decoding': True,
                            'rx_df_hz': 1201, 'tx_df_hz': 2302, 'de_call': 'Q2AAA', 'de_grid': 'FN31',
                            'dx_grid': 'FN42', 'tx_watchdog': False, 'sub_mode': 'sub-test', 'fast_mode': True,
                            'special_operation_mode': 3, 'frequency_tolerance_hz': 50, 'tr_period_seconds': 15,
                            'configuration_name': 'test-config', 'tx_message': 'test-tx'})
        alternate = copy.deepcopy(status_parts)
        alternate[5] = pack('BBBII', 0, 1, 1, 1201, 2302)
        alternate_fields = dict(status_fields, tx_enabled=False, transmitting=True, decoding=True)
        s.positive('status-distinct-booleans' + suffix, schema, 1, b''.join(alternate),
                   'radio_state', alternate_fields)
        decode = pack('BIidI', 1, 45296123, -17, 0.25, 1234) + utf8('~') + utf8('CQ Q1AAA FN42')
        decode_fields = {'is_new': True, 'time_of_day': '12:34:56.123', 'snr_db': -17, 'dt_seconds': 0.25,
                         'audio_offset_hz': 1234, 'mode': '~', 'text': 'CQ Q1AAA FN42'}
        s.positive('02-decode' + suffix, schema, 2, decode + b'\x00\x01', 'decode',
                   dict(decode_fields, low_confidence=False, off_air=True),
                   message={'new': True, 'time': 45296123, 'snr_db': -17, 'delta_time_seconds': 0.25,
                            'delta_frequency_hz': 1234, 'mode': '~', 'message': 'CQ Q1AAA FN42',
                            'low_confidence': False, 'off_air': True})
        s.positive('03-clear' + suffix, schema, 3, b'\x02', 'control', {'operation': 'clear', 'window': 2},
                   direction='to_application', message={'window': 2})
        reply = pack('IidI', 45296123, -17, 0.25, 1234) + utf8('~') + utf8('CQ Q1AAA FN42')
        reply_fields = {'operation': 'reply', 'time_of_day': '12:34:56.123', 'snr_db': -17, 'dt_seconds': 0.25,
                        'audio_offset_hz': 1234, 'mode': '~', 'text': 'CQ Q1AAA FN42'}
        s.positive('04-reply' + suffix, schema, 4, reply + b'\x01\x04', 'control',
                   dict(reply_fields, low_confidence=True, modifiers=4),
                   message={'time': 45296123, 'snr_db': -17, 'delta_time_seconds': 0.25,
                            'delta_frequency_hz': 1234, 'mode': '~', 'message': 'CQ Q1AAA FN42',
                            'low_confidence': True, 'modifiers': 4})
        # Date offset suffix differs in schema 1; common UTC oracle is explicit.
        dt, dt_view = date(schema, spec=2)
        on, on_view = date(schema, milliseconds=123)
        qso_core = dt + utf8('Q1AAA') + utf8('FN42') + pack('Q', 14075500)
        qso_core += b''.join(utf8(v) for v in ['FT8', '-11', '-12', '5 W', 'synthetic', 'Test'])
        qso_tail = [on] + [utf8(v) for v in ['Q2BBB', 'Q2AAA', 'FN31', '001', '002', 'TR']]
        qso_fields = {'end': '1970-01-01T01:00:00.123Z' if schema == 1 else '1970-01-01T00:00:00.123Z',
                      'call': 'Q1AAA', 'grid': 'FN42', 'tx_frequency_hz': 14075500, 'mode': 'FT8',
                      'report_sent': '-11', 'report_received': '-12', 'tx_power': '5 W',
                      'comments': 'synthetic', 'name': 'Test'}
        qso_extra = {'start': '1970-01-01T00:00:00.123Z', 'operator_call': 'Q2BBB', 'station_call': 'Q2AAA',
                     'station_grid': 'FN31', 'exchange_sent': '001', 'exchange_received': '002', 'propagation_mode': 'TR'}
        qso_view = {'time_off': dt_view, 'dx_call': 'Q1AAA', 'dx_grid': 'FN42', 'frequency_hz': '14075500',
                    'mode': 'FT8', 'report_sent': '-11', 'report_received': '-12', 'tx_power': '5 W',
                    'comments': 'synthetic', 'name': 'Test', 'time_on': on_view, 'operator_call': 'Q2BBB',
                    'my_call': 'Q2AAA', 'my_grid': 'FN31', 'exchange_sent': '001', 'exchange_received': '002',
                    'propagation_mode': 'TR'}
        s.positive('05-qso-logged' + suffix, schema, 5, qso_core + b''.join(qso_tail), 'qso_logged',
                   qso_fields | qso_extra, message=qso_view)
        s.positive('06-close' + suffix, schema, 6, b'', 'lifecycle', {'state': 'closed'}, message={})
        s.positive('07-replay' + suffix, schema, 7, b'', 'control', {'operation': 'replay'}, message={})
        s.positive('08-halt-tx' + suffix, schema, 8, b'\x01', 'control',
                   {'operation': 'halt_tx', 'auto_tx_only': True}, message={'auto_tx_only': True})
        s.positive('09-free-text' + suffix, schema, 9, utf8('test message') + b'\x00', 'control',
                   {'operation': 'send_text', 'text': 'test message', 'transmit': False},
                   message={'text': 'test message', 'send': False})
        wspr = pack('BIidQi', 0, 45296123, -27, -0.5, 14095600, -2) + utf8('Q1AAA') + utf8('FN42') + pack('i', 37)
        wspr_fields = {'is_new': False, 'time_of_day': '12:34:56.123', 'snr_db': -27, 'dt_seconds': -0.5,
                       'frequency_hz': 14095600, 'drift_hz': -2, 'call': 'Q1AAA', 'grid': 'FN42', 'power_dbm': 37}
        s.positive('10-wspr-decode' + suffix, schema, 10, wspr + b'\x01', 'decode', dict(wspr_fields, off_air=True),
                   message={'new': False, 'time': 45296123, 'snr_db': -27, 'delta_time_seconds': -0.5,
                            'frequency_hz': '14095600', 'drift_hz': -2, 'callsign': 'Q1AAA', 'grid': 'FN42',
                            'power_dbm': 37, 'off_air': True})
        s.positive('11-location' + suffix, schema, 11, utf8('FN31aa'), 'control',
                   {'operation': 'wsjtx.location', 'location': 'FN31aa'}, message={'location': 'FN31aa'})
        adif = '<ADIF_VER:5>3.1.4<EOH><CALL:5>Q1AAA<EOR>'
        s.positive('12-logged-adif' + suffix, schema, 12, utf8(adif), 'qso_logged', {'adif': adif}, message={'adif': adif})
        colors = pack('B5H', 1, 65535, 0x1111, 0x2222, 0x3333, 0x4444) + pack('B5H', 0, 1, 2, 3, 4, 5)
        s.positive('13-highlight-callsign' + suffix, schema, 13, utf8('Q1AAA') + colors + b'\x01', 'control',
                   {'operation': 'wsjtx.highlight_callsign', 'call': 'Q1AAA', 'highlight_last': True},
                   message={'callsign': 'Q1AAA', 'background': {'spec': 1, 'channels': [65535, 0x1111, 0x2222, 0x3333, 0x4444]},
                            'foreground': {'spec': 0, 'channels': [1, 2, 3, 4, 5]}, 'highlight_last': True})
        s.positive('14-switch-configuration' + suffix, schema, 14, utf8('test-config'), 'control',
                   {'operation': 'wsjtx.switch_configuration', 'configuration_name': 'test-config'},
                   message={'configuration_name': 'test-config'})
        configure = utf8('FT8') + pack('I', 100) + utf8('sub-test') + pack('BII', 0, 15, 1800) + utf8('Q1AAA') + utf8('FN42')
        cfg_fields = {'operation': 'wsjtx.configure', 'mode': 'FT8', 'frequency_tolerance_hz': 100, 'sub_mode': 'sub-test',
                      'fast_mode': False, 'tr_period_seconds': 15, 'rx_offset_hz': 1800, 'dx_call': 'Q1AAA', 'dx_grid': 'FN42'}
        s.positive('15-configure' + suffix, schema, 15, configure + b'\x01', 'control', dict(cfg_fields, generate_messages=True),
                   message={'mode': 'FT8', 'frequency_tolerance_hz': 100, 'sub_mode': 'sub-test', 'fast_mode': False,
                            'tr_period_seconds': 15, 'rx_df_hz': 1800, 'dx_call': 'Q1AAA', 'dx_grid': 'FN42', 'generate_messages': True})
        s.positive('16-annotation-info' + suffix, schema, 16, utf8('Q1AAA') + pack('BI', 0, 12345), 'control',
                   {'operation': 'wsjtx.annotation_info', 'dx_call': 'Q1AAA', 'sort_order_provided': False, 'sort_order': 12345},
                   message={'dx_call': 'Q1AAA', 'sort_order_provided': False, 'sort_order': 12345})
        # Historical minima in every supported schema.
        s.positive('tail-heartbeat-empty' + suffix, schema, 0, b'', 'lifecycle', {'state': 'heartbeat'}, message={})
        s.positive('tail-status-minimal' + suffix, schema, 1, b''.join(status_parts[:6]), 'radio_state',
                   {k: status_fields[k] for k in ['dial_frequency_hz', 'mode', 'dx_call', 'report', 'tx_mode',
                                                'tx_enabled', 'transmitting', 'decoding', 'rx_offset_hz', 'tx_offset_hz']})
        s.positive('tail-decode-minimal' + suffix, schema, 2, decode, 'decode', decode_fields)
        s.positive('tail-reply-minimal' + suffix, schema, 4, reply, 'control', reply_fields)
        s.positive('tail-qso-minimal' + suffix, schema, 5, qso_core, 'qso_logged', qso_fields)
        s.positive('tail-wspr-minimal' + suffix, schema, 10, wspr, 'decode', wspr_fields)
        s.positive('tail-highlight-minimal' + suffix, schema, 13, utf8('Q1AAA') + colors, 'control',
                   {'operation': 'wsjtx.highlight_callsign', 'call': 'Q1AAA'})
        s.positive('tail-configure-minimal' + suffix, schema, 15, configure, 'control', cfg_fields)
        # Invalid and unresolvable native times are successful messages with absent common times.
        for label, spec, ms, day, zone in [
            ('local', 0, 3600123, 2440588, None),
            ('null', 1, 0xffffffff, -9223372036854775808, None),
            ('bad-clock', 1, 86400000, 2440588, None),
            ('bad-calendar', 1, 0, 9223372036854775807, None),
            ('named-zone', 3, 3600123, 2440588, {'key': 'Etc/UTC'}),
            ('invalid-utf16-zone', 3, 3600123, 2440588, {'key': [65, 0xd800, 233]}),
            ('custom-zone', 3, 3600123, 2440588, {'key': 'OffsetFromUtc', 'id': 'test-zone', 'offset_seconds': 3600,
                                                       'name': 'Test zone', 'abbreviation': 'TST', 'country': 0, 'comment': ''}),
        ]:
            dt_raw, view = date(schema, spec=spec, milliseconds=ms, day=day, zone=zone)
            fields = dict(qso_fields)
            fields.pop('end')
            if label in ('local', 'named-zone', 'invalid-utf16-zone', 'custom-zone') and schema == 1:
                fields['end'] = '1970-01-01T01:00:00.123Z'
            elif label == 'custom-zone':
                fields['end'] = '1970-01-01T00:00:00.123Z'
            body = dt_raw + qso_core[len(dt):]
            native_view = {k: v for k, v in qso_view.items() if k not in ['time_on', 'operator_call', 'my_call', 'my_grid',
                                                                                       'exchange_sent', 'exchange_received', 'propagation_mode']}
            native_view['time_off'] = view
            s.positive('time-' + label + suffix, schema, 5, body, 'qso_logged', fields, message=native_view)
        # Every intermediate historical boundary is distinct from explicit null/empty.
        if schema == 3:
            for count, data, fields in [
                (1, pack('I', 3), {'state': 'heartbeat', 'max_schema': 3}),
                (2, pack('I', 3) + utf8(None), {'state': 'heartbeat', 'max_schema': 3, 'version': None}),
            ]:
                s.positive(f'tail-heartbeat-prefix-{count}', 3, 0, data, 'lifecycle', fields)
            optional_status = [utf8('Q2AAA'), utf8('FN31'), utf8('FN42'), b'\x00', utf8('sub-test'),
                               b'\x01', b'\x03', pack('I', 50), pack('I', 15), utf8('test-config'), utf8('test-tx')]
            optional_names = ['station_call', 'station_grid', 'dx_grid', 'tx_watchdog', 'sub_mode', 'fast_mode',
                              None, 'frequency_tolerance_hz', 'tr_period_seconds', 'configuration_name', 'tx_message']
            required_names = ['dial_frequency_hz', 'mode', 'dx_call', 'report', 'tx_mode', 'tx_enabled',
                              'transmitting', 'decoding', 'rx_offset_hz', 'tx_offset_hz']
            for count in range(1, len(optional_status)):
                names = required_names + [n for n in optional_names[:count] if n is not None]
                s.positive(f'tail-status-prefix-{count}', 3, 1,
                           b''.join(status_parts[:6] + optional_status[:count]), 'radio_state',
                           {n: status_fields[n] for n in names})
            s.positive('tail-decode-prefix-1', 3, 2, decode + b'\x00', 'decode', dict(decode_fields, low_confidence=False))
            s.positive('tail-reply-prefix-1', 3, 4, reply + b'\x01', 'control', dict(reply_fields, low_confidence=True))
            for count in range(1, len(qso_tail)):
                fields = qso_fields | dict(list(qso_extra.items())[:count])
                s.positive(f'tail-qso-prefix-{count}', 3, 5, qso_core + b''.join(qso_tail[:count]), 'qso_logged', fields)
            for key in (None, ''):
                raw_dt, view = date(3, spec=3, zone={'key': key})
                fields = dict(qso_fields)
                fields.pop('end')
                v = {k: val for k, val in qso_view.items() if k in ['dx_call', 'dx_grid', 'frequency_hz', 'mode',
                     'report_sent', 'report_received', 'tx_power', 'comments', 'name']}
                v['time_off'] = view
                s.positive('zone-null' if key is None else 'zone-empty', 3, 5, raw_dt + qso_core[len(dt):],
                           'qso_logged', fields, message=v)
        # A partial required body must fail for every type, including an incomplete ID on empty bodies.
        for typ in range(17):
            base_name = next(n for n in s.vectors if n.startswith(f'{typ:02}-') and n.endswith(suffix))
            raw = base64.b64decode(s.vectors[base_name]['input']['data'])
            # Known optional tails are not required; truncate each chosen mandatory prefix instead.
            minimal = {0: header(schema, 0), 1: header(schema, 1) + b''.join(status_parts[:6]),
                       2: header(schema, 2) + decode, 3: header(schema, 3), 4: header(schema, 4) + reply,
                       5: header(schema, 5) + qso_core, 10: header(schema, 10) + wspr,
                       13: header(schema, 13) + utf8('Q1AAA') + colors, 15: header(schema, 15) + configure}.get(typ, raw)
            s.negative(f'truncated-type-{typ:02}' + suffix, minimal[:-1], 'truncated')
    # Clear/Close ambiguity cannot be resolved by the presence of a Window.
    for direction in ['from_application', 'to_application', 'unknown']:
        for tail in (b'', b'\x01'):
            kind = {'from_application': 'notification', 'to_application': 'control', 'unknown': 'ambiguous'}[direction]
            fields = {'operation': 'clear'}
            if tail:
                fields['window'] = 1
            s.positive(f'direction-clear-{direction}-window-{len(tail)}', 3, 3, tail, kind, fields, direction=direction)
        fields = {'state': 'closed'} if direction == 'from_application' else {'operation': 'close'}
        s.positive('direction-close-' + direction, 3, 6, b'', 'lifecycle' if direction == 'from_application' else
                   'control' if direction == 'to_application' else 'ambiguous', fields, direction=direction)
        s.positive('direction-heartbeat-' + direction, 3, 0, b'', 'lifecycle', {'state': 'heartbeat'}, direction=direction)
    for label, value in [('null', None), ('empty', ''), ('invalid-utf8', b'\xff\xc0\xaf'),
                         ('unicode', 'é\u202e\n')]:
        s.positive('text-' + label, 3, 9, utf8(value) + b'\x00', 'control',
                   {'operation': 'send_text', 'text': text(value), 'transmit': False}, message={'text': text(value), 'send': False})
        s.positive('id-' + label, 3, 6, b'', 'lifecycle', {'state': 'closed'}, identity=value, message={})
    s.positive('id-maximum', 3, 6, b'', 'lifecycle', {'state': 'closed'}, identity='a' * 256)
    for byte in (2, 127, 255):
        s.positive('boolean-noncanonical-' + str(byte), 3, 8, bytes([byte]), 'control',
                   {'operation': 'halt_tx', 'auto_tx_only': True}, message={'auto_tx_only': True}, canonical=b'\x01')
    s.positive('extensions', 3, 6, b'', 'lifecycle', {'state': 'closed'}, extensions=b'\x00\xfe\xff', message={})
    s.positive('datagram-maximum', 3, 6, b'', 'lifecycle', {'state': 'closed'},
               extensions=b'\xa5' * (65507 - len(header(3, 6))))
    for label, bits, value in [('nan-payload', 0x7ff8000000001234, 'ieee754:7ff8000000001234'),
                               ('positive-infinity', 0x7ff0000000000000, 'ieee754:7ff0000000000000'),
                               ('negative-infinity', 0xfff0000000000000, 'ieee754:fff0000000000000'),
                               ('negative-zero', 0x8000000000000000, -0.0),
                               ('rounding', 0x3f99867e711c7600, struct.unpack('>d', pack('Q', 0x3f99867e711c7600))[0])]:
        body = pack('BIiQI', 1, 0, -1, bits, 100) + utf8('~') + utf8('test')
        s.positive('float-' + label, 3, 2, body, 'decode', {'is_new': True, 'time_of_day': '00:00:00.000',
                   'snr_db': -1, 'dt_seconds': value, 'audio_offset_hz': 100, 'mode': '~', 'text': 'test'})
    for number in (9007199254740991, 9007199254740992, 18446744073709551615):
        body = pack('BIidQi', 0, 0xffffffff, -27, 0.0, number, -2) + utf8(None) + utf8('') + pack('i', 37)
        fields = {'is_new': False, 'snr_db': -27, 'dt_seconds': 0.0, 'drift_hz': -2, 'call': None, 'grid': '', 'power_dbm': 37}
        if number <= 9007199254740991:
            fields['frequency_hz'] = number
        s.positive('integer-' + str(number), 3, 10, body, 'decode', fields)
    # Append sentinels to complete mandatory Status plus all optional fields.
    body = pack('Q', 1) + b''.join(utf8(v) for v in ['FT8', '', '', 'FT8']) + pack('BBBII', 0, 0, 0, 1, 2)
    body += utf8(None) + utf8('') + utf8('') + b'\x00' + utf8('') + b'\x00\x00'
    body += pack('II', 0xffffffff, 0xffffffff) + utf8('') + utf8('')
    s.positive('status-not-applicable', 3, 1, body, 'radio_state', {'dial_frequency_hz': 1, 'mode': 'FT8', 'dx_call': '',
               'report': '', 'tx_mode': 'FT8', 'tx_enabled': False, 'transmitting': False, 'decoding': False,
               'rx_offset_hz': 1, 'tx_offset_hz': 2, 'station_call': None, 'station_grid': '', 'dx_grid': '',
               'tx_watchdog': False, 'sub_mode': '', 'fast_mode': False, 'configuration_name': '', 'tx_message': ''})
    body = utf8('') + pack('I', 0xffffffff) + utf8('') + pack('BII', 0, 0xffffffff, 0xffffffff) + utf8('') + utf8('')
    s.positive('configure-no-change', 3, 15, body, 'control', {'operation': 'wsjtx.configure', 'mode': '',
               'frequency_tolerance_hz': 0xffffffff, 'sub_mode': '', 'fast_mode': False,
               'tr_period_seconds': 0xffffffff, 'rx_offset_hz': 0xffffffff, 'dx_call': '', 'dx_grid': ''})
    s.positive('free-text-empty-transmit', 3, 9, utf8('') + b'\x01', 'control',
               {'operation': 'send_text', 'text': '', 'transmit': True},
               message={'text': '', 'send': True})
    decode_source = s.vectors['02-decode-s3']
    raw = base64.b64decode(decode_source['input']['data'])
    body = raw[len(header(3, 2)):]
    for label, identity in [('null', None), ('empty', ''), ('raw', b'\xff'), ('unicode', 'receiver-\u03b1')]:
        fields = copy.deepcopy(decode_source['expected']['event']['fields'])
        s.positive('radio-instance-' + label, 3, 2, body, 'decode', fields, identity=identity)
    # Maximum native size with both complete views and maximally escaped text/ID.
    source = s.vectors['01-status-s3']
    original = base64.b64decode(source['input']['data'])
    prefix = original[len(header(3, 1)):-len(utf8('test-tx'))]
    identity = '\x01' * 256
    maximal_text = '\x01' * (65507 - len(header(3, 1, identity)) - len(prefix) - 4)
    fields = dict(copy.deepcopy(source['expected']['event']['fields']), tx_message=maximal_text)
    message = dict(copy.deepcopy(source['expected_message']['body']), tx_message=maximal_text)
    s.positive('output-budget-maximum-text', 3, 1, prefix + utf8(maximal_text),
               'radio_state', fields, identity=identity, message=message)
    # Error precedence and planted failures for every category.
    s.negative('error-empty', b'', 'truncated')
    s.negative('error-short-schema', pack('I', 0xadbccbda) + b'\x00', 'truncated')
    s.negative('error-magic', pack('II', 0, 0), 'malformed')
    s.negative('error-schema-zero', pack('II', 0xadbccbda, 0), 'unsupported_version')
    s.negative('error-size-before-magic', b'\x00' * 65508, 'oversized')
    s.negative('error-size-before-future', pack('II', 0xadbccbda, 4) + b'\x00' * 65500, 'oversized')
    s.negative('error-id-bound-before-remaining', pack('III', 0xadbccbda, 3, 6) + pack('I', 257), 'oversized')
    s.negative('error-text-bound', header(3, 9) + pack('I', 65508), 'oversized')
    s.negative('error-partial-tail', header(3, 0) + b'\x00', 'truncated')
    s.negative('error-invalid-time-spec', header(3, 5) + pack('qIB', 2440588, 0, 4), 'invalid_value')
    s.negative('error-odd-utf16', header(3, 5) + pack('qIBI', 2440588, 0, 3, 1) + b'A', 'malformed')
    s.negative('error-short-utf16', header(3, 5) + pack('qIBI', 2440588, 0, 3, 2) + b'A', 'truncated')
    s.skip('skip-future-prefix', pack('II', 0xadbccbda, 4), 'future_version', 4)
    s.skip('skip-future-garbage', pack('II', 0xadbccbda, 0xffffffff) + b'bad type and ID', 'future_version', 0xffffffff)
    for schema in (1, 2, 3):
        s.skip('skip-unknown-type-s' + str(schema), pack('III', 0xadbccbda, schema, 99), 'unknown_type', schema, 99)
    for typ, base in [(4, '04-reply-s3'), (15, '15-configure-s3')]:
        vector = copy.deepcopy(s.vectors[base])
        supplied = {'direction': 'unknown', 'exchange': 'request', 'source_id': 'test-controller',
                    'target_id': 'test-station', 'correlation_id': 'test-exchange-1'}
        vector['input']['context'] = supplied
        vector['expected']['event']['context'] = copy.deepcopy(supplied)
        s.add('context-target-type-' + str(typ), vector)
    compatibility(s)
    commands(s)
    return s


def compatibility(s):
    old = copy.deepcopy(s.vectors['06-close-s3']['expected']['event'])

    def add(name, event, reader='0.1.0', expected=None, error=None):
        value = {'class': 'compatibility', 'writer_spec': event['spec'], 'reader_spec': reader, 'input': {'event': event}}
        if error:
            value['expected'] = {'error': {'category': error}}
        else:
            value['expected'] = {'read': expected or {k: event[k] for k in ['kind', 'fields', 'context', 'native']}}
        s.add('compat-' + name, value)

    add('new-reader-old-writer', old, reader='0.2.0')
    new = copy.deepcopy(old)
    new['spec'] = '0.2.0'
    new['new_member'] = {'test': True}
    new['fields']['x-org.example.extra'] = 'test'
    read = {k: copy.deepcopy(old[k]) for k in ['kind', 'fields', 'context', 'native']}
    add('old-reader-new-members', new, expected=read)
    new = copy.deepcopy(old)
    new['spec'] = '0.2.0'
    new['mapping'] = 2
    new['native']['data'] = b64(header(3, 17))
    new['kind'] = 'future_kind'
    new['fields'] = {'future_field': True}
    read_native = dict(read, kind='native', fields={})
    add('old-reader-new-kind', new, expected=dict(read_native, native=new['native']))
    new = copy.deepcopy(old)
    new['spec'] = '0.2.0'
    new['family'] = 'futurefamily'
    new['kind'] = 'native'
    new['fields'] = {'future_field': True}  # Unknown-family fields are not interpreted.
    add('old-reader-new-family', new, expected=read_native)
    new = copy.deepcopy(old)
    new['spec'] = '0.2.0'
    new['native']['data'] = b64(header(3, 0))
    new['fields'] = {'state': 'heartbeat'}
    new['context'] = {'direction': 'future_direction', 'exchange': 'future_exchange'}
    add('old-reader-new-context', new, expected={'kind': 'lifecycle', 'fields': {'state': 'heartbeat'},
        'native': new['native'], 'context': {'direction': 'unknown', 'exchange': 'unknown'}})
    new = copy.deepcopy(old)
    new['spec'] = '0.2.0'
    new['mapping'] = 2
    new['native']['data'] = b64(header(3, 18))
    new['kind'] = 'control'
    new['fields'] = {'operation': 'future_operation'}
    add('old-reader-new-operation', new)
    s.vectors['compat-old-reader-new-operation']['expected']['verification'] = 'unavailable'
    for name, native_type, kind, fields in [
        ('known-kind-new-revision', 17, 'radio_state', {'dial_frequency_hz': 14074000}),
        ('known-operation-new-revision', 8, 'control', {'operation': 'halt_tx', 'auto_tx_only': True}),
    ]:
        new = copy.deepcopy(old)
        new.update(mapping=2, kind=kind, fields=fields)
        new['native']['data'] = b64(header(3, native_type) + (b'\x01' if native_type == 8 else b''))
        add(name, new)
        s.vectors['compat-' + name]['expected']['verification'] = 'verified' if native_type == 8 else 'unavailable'
    new = copy.deepcopy(s.vectors['compat-known-operation-new-revision']['input']['event'])
    new['native_version'] = '4'
    # A familiar type number does not license interpreting an unsupported schema's payload.
    # The zero byte deliberately disagrees with auto_tx_only under the schema 3 layout.
    new['native']['data'] = b64(header(4, 8) + b'\x00')
    add('known-operation-future-native-version', new, expected={
        'kind': 'native', 'fields': {}, 'context': copy.deepcopy(new['context']),
        'native': copy.deepcopy(new['native'])})
    s.vectors['compat-known-operation-future-native-version']['expected']['verification'] = 'unavailable'
    new = copy.deepcopy(s.vectors['13-highlight-callsign-s3']['expected']['event'])
    new['mapping'] = 2
    new['fields']['background_color']['future_component'] = 7
    read = {k: copy.deepcopy(new[k]) for k in ('kind', 'fields', 'context', 'native')}
    read['fields']['background_color'].pop('future_component')
    add('nested-addition', new, expected=read)
    new = copy.deepcopy(old)
    new['spec'] = '1.0.0'
    add('different-major', new, error='unsupported_version')
    for name, event_json, category in [
        ('duplicate-envelope', '{"spec":"0.1.0","spec":"0.2.0"}', 'malformed'),
        ('duplicate-nested', '{"fields":{"text":"a","text":"b"}}', 'malformed'),
        ('record-oversized', json.dumps(old)[:-1] + ',"padding":"' + 'a' * 1048576 + '"}', 'oversized'),
    ]:
        s.add('json-' + name, {'class': 'negative', 'input': {'event_json': event_json},
                              'expected': {'error': {'category': category}}})
    # Schema failures are event-reading inputs, not native decoder failures.
    for name, mutate in [
        ('missing-mapping', lambda e: e.pop('mapping')),
        ('missing-state', lambda e: e['fields'].pop('state')),
        ('missing-native-data', lambda e: e['native'].pop('data')),
        ('native-data-not-string', lambda e: e['native'].update(data=123)),
        ('native-format-not-string', lambda e: e['native'].update(format=123)),
        ('native-message-not-object', lambda e: e['native'].update(message=[])),
        ('base64-noncanonical', lambda e: e['native'].update(data='AB==')),
        ('base64-newline', lambda e: e['native'].update(data=e['native']['data'] + '\n')),
        ('time-newline', lambda e: e.update(kind='decode', fields={'time_of_day': '00:00:00\n'})),
        ('nonfinite-token-finite', lambda e: e.update(kind='decode', fields={'dt_seconds': 'ieee754:0000000000000000'})),
        ('integer-too-large', lambda e: e['fields'].update(max_schema=9007199254740992)),
        ('null-time', lambda e: e.update(kind='decode', fields={'time_of_day': None})),
        ('text-wrapper-bounds', lambda e: e.update(kind='decode', fields={'text': {'utf8_bytes': [256]}})),
    ]:
        invalid = copy.deepcopy(old)
        mutate(invalid)
        s.add('event-' + name, {'class': 'negative', 'input': {'event': invalid},
                              'expected': {'error': {'category': 'invalid_value'}}})
    inconsistent = copy.deepcopy(old)
    inconsistent['fields']['state'] = 'heartbeat'
    s.add('event-native-disagreement', {'class': 'negative', 'input': {'event': inconsistent},
                                       'expected': {'error': {'category': 'invalid_value'}}})



# Command inputs contain only the public field model, never native bytes/views.
# Expected bytes are the separately specified B.4 fixture layouts above, not an encoder result.
SEND_TYPES = {
    'wsjtx.heartbeat': (0, []), 'clear': (3, []),
    'reply': (4, ['time_of_day', 'snr_db', 'dt_seconds', 'audio_offset_hz', 'mode', 'text']),
    'close': (6, []), 'replay': (7, []), 'halt_tx': (8, ['auto_tx_only']),
    'send_text': (9, ['text', 'transmit']), 'wsjtx.location': (11, ['location']),
    'wsjtx.highlight_callsign': (13, ['call', 'background_color', 'foreground_color']),
    'wsjtx.switch_configuration': (14, ['configuration_name']),
    'wsjtx.configure': (15, ['mode', 'frequency_tolerance_hz', 'sub_mode', 'fast_mode',
                             'tr_period_seconds', 'rx_offset_hz', 'dx_call', 'dx_grid']),
    'wsjtx.annotation_info': (16, ['dx_call', 'sort_order_provided', 'sort_order']),
}


def commands(s):
    baselines = {}
    for schema in (1, 2, 3):
        for operation, (typ, required) in SEND_TYPES.items():
            source = next(v for name, v in s.vectors.items()
                          if name.startswith(f'{typ:02d}-') and name.endswith(f'-s{schema}'))
            event = copy.deepcopy(source['expected']['event'])
            event.pop('native')
            event.pop('native_version')
            event['kind'] = 'control'
            event['context'] = context('to_application')
            event['fields'].pop('state', None)
            event['fields']['operation'] = operation
            name = f'command-{typ:02d}-s{schema}'
            s.add(name, {'class': 'command', 'input': {'event': event, 'options': {'schema': schema}},
                         'expected': {'data': source['reencode']['data']}})
            if schema == 3:
                baselines[operation] = copy.deepcopy(event)
    # Default schema, optional tail boundaries, null/empty/raw text and boolean canonicalization.
    sources = [name for name in s.vectors if name.startswith(('tail-heartbeat-empty-', 'tail-reply-minimal-',
                'tail-highlight-minimal-', 'tail-configure-minimal-', 'direction-clear-to_application-',
                'text-', 'id-', 'configure-no-change', 'free-text-empty-transmit'))]
    # Explicitly include all noncanonical-boolean sending vectors when present.
    sources += [name for name, v in s.vectors.items() if v.get('reencode') and
                v['input'].get('data') != v['reencode']['data']]
    for name in dict.fromkeys(sources):
        source = s.vectors[name]
        event = copy.deepcopy(source['expected']['event'])
        if 'reencode' not in source:
            continue
        schema = int(event.pop('native_version'))
        event.pop('native')
        if event['kind'] == 'lifecycle':
            state = event['fields'].pop('state')
            event['fields']['operation'] = 'close' if state == 'closed' else 'wsjtx.heartbeat'
        event['kind'] = 'control'
        event['context'] = context('to_application')
        s.add('command-' + name, {'class': 'command', 'input': {'event': event, 'options': {'schema': schema}},
                                  'expected': {'data': source['reencode']['data']}})
    for label, bits in [('nan', 0x7ff8000000001234), ('negative-zero', 0x8000000000000000)]:
        event = copy.deepcopy(baselines['reply'])
        event['fields']['dt_seconds'] = 'ieee754:7ff8000000001234' if label == 'nan' else -0.0
        raw = header(3, 4) + pack('IiQ', 45296123, -17, bits) + pack('I', 1234)
        raw += utf8('~') + utf8('CQ Q1AAA FN42') + b'\x01\x04'
        s.add('command-reply-' + label, {'class': 'command', 'input': {'event': event},
                                        'expected': {'data': b64(raw)}})
    default = copy.deepcopy(s.vectors['command-09-s3'])
    default['input'].pop('options')
    s.add('command-default-schema', default)

    def negative(name, event, options=None, category='invalid_value'):
        inp = {'event': event}
        if options is not None:
            inp['options'] = options
        s.add('command-error-' + name, {'class': 'command_negative', 'input': inp,
                                       'expected': {'error': {'category': category}}})

    for operation, (_, required) in SEND_TYPES.items():
        label = operation.replace('.', '-')
        # No-argument commands still require an explicit target ID and operation.
        for key in required:
            event = copy.deepcopy(baselines[operation])
            event['fields'].pop(key)
            negative(label + '-missing-' + key, event)
        event = copy.deepcopy(baselines[operation])
        event.pop('wire_id')
        negative(label + '-missing-wire-id', event)
    for range_index, (operation, key, bad) in enumerate([
        ('wsjtx.heartbeat', 'max_schema', -1), ('wsjtx.heartbeat', 'max_schema', 2**32),
        ('clear', 'window', -1), ('clear', 'window', 3), ('clear', 'window', 256),
        ('reply', 'snr_db', -2**31 - 1), ('reply', 'snr_db', 2**31),
        ('reply', 'audio_offset_hz', -1), ('reply', 'audio_offset_hz', 2**32),
        ('reply', 'modifiers', -1), ('reply', 'modifiers', 256),
        ('reply', 'time_of_day', '24:00:00'), ('reply', 'time_of_day', '12:00:00.0001'),
        ('halt_tx', 'auto_tx_only', 1), ('send_text', 'transmit', 1),
        ('wsjtx.configure', 'frequency_tolerance_hz', 2**32),
        ('wsjtx.configure', 'rx_offset_hz', -1), ('wsjtx.configure', 'tr_period_seconds', 2**32),
        ('wsjtx.annotation_info', 'sort_order', 2**32),
        ('wsjtx.highlight_callsign', 'background_color', {'spec': 256, 'channels': [0]*5}),
        ('wsjtx.highlight_callsign', 'foreground_color', {'spec': 0, 'channels': [0, 0, 0, 0, 65536]}),
    ], 1):
        event = copy.deepcopy(baselines[operation])
        event['fields'][key] = bad
        if key == 'modifiers':
            event['fields']['low_confidence'] = False
        negative(operation.replace('.', '-') + '-' + key + '-' + str(range_index), event)
    for op, hole in [('reply', 'low_confidence'), ('wsjtx.heartbeat', 'version')]:
        event = copy.deepcopy(baselines[op])
        event['fields'].pop(hole)
        negative(op.replace('.', '-') + '-tail-hole', event)
    event = copy.deepcopy(baselines['close'])
    event['fields'].pop('operation')
    negative('missing-operation', event)
    event = copy.deepcopy(baselines['close'])
    event['fields']['operation'] = 'wsjtx.not_registered'
    negative('unknown-operation', event)
    for label, bad in [('utf16-for-utf8', {'utf16_units': [0xd800]}),
                       ('valid-text-raw-wrapper', {'utf8_bytes': [65]})]:
        event = copy.deepcopy(baselines['send_text'])
        event['fields']['text'] = bad
        negative(label, event)
    event = copy.deepcopy(baselines['send_text'])
    event['mapping'] = 2
    negative('known-operation-new-revision', event, category='unsupported_version')
    for schema in (0, 4, '3', True):
        negative('schema-' + str(schema), baselines['close'], {'schema': schema}, 'unsupported_version')
    old_command = copy.deepcopy(baselines['send_text'])
    s.add('compat-new-reader-old-command', {'class': 'compatibility', 'writer_spec': SPEC,
        'reader_spec': '0.2.0', 'input': {'event': old_command},
        'expected': {'read': {k: copy.deepcopy(old_command[k]) for k in ('kind', 'fields', 'context')}}})
    new_command = copy.deepcopy(old_command)
    new_command.update(spec='0.2.0', family='futurefamily')
    s.add('compat-old-reader-new-command-family', {'class': 'compatibility', 'writer_spec': '0.2.0',
        'reader_spec': SPEC, 'input': {'event': new_command},
        'expected': {'read': {'kind': 'native', 'fields': {}, 'context': new_command['context']},
                     'preserved_command': copy.deepcopy(new_command)}})
    new_command = copy.deepcopy(old_command)
    new_command.update(spec='0.2.0', mapping=2)
    new_command['fields'] = {'operation': 'wsjtx.future_operation'}
    s.add('compat-old-reader-new-command-operation', {'class': 'compatibility', 'writer_spec': '0.2.0',
        'reader_spec': SPEC, 'input': {'event': new_command},
        'expected': {'read': {k: copy.deepcopy(new_command[k]) for k in ('kind', 'fields', 'context')},
                     'preserved_command': copy.deepcopy(new_command)}})



def qgi_vectors(wsjtx):
    """Explicit carriage oracles, independent of any Rust implementation."""
    s = Suite('qgi')
    observed = copy.deepcopy(next(v['expected']['event'] for v in wsjtx.vectors.values()
        if v['class'] == 'positive' and v['expected']['event']['kind'] == 'decode'
        and v['expected']['event']['native_version'] == '3'))
    # Deliberately spoofed serialized identities must not replace observer context.
    observed['context'] = {'direction': 'to_application', 'exchange': 'request',
                           'source_id': 'untrusted-serialized-source'}
    command = {'spec': SPEC, 'family': 'qgi', 'mapping': 1, 'kind': 'control',
               'context': context('to_application'),
               'fields': {'operation': 'send_text', 'text': 'synthetic\nmessage', 'transmit': False}}

    def wire(event):
        return json.dumps(event, ensure_ascii=True, separators=(',', ':'), allow_nan=False).encode()

    def positive(name, carried, transport='udp', delimiter=b'', observer=None):
        raw = wire(carried)
        observer = observer or context('unknown')
        event = copy.deepcopy(carried)
        event.setdefault('via', []).append({'family': 'qgi', 'context': copy.deepcopy(observer)})
        s.add(name, {'class': 'positive',
            'input': {'transport': transport, 'data': b64(raw + delimiter), 'context': observer},
            'expected': {'event': event},
            'reencode': {'event': copy.deepcopy(event), 'delimiter': delimiter.decode('ascii')}})

    positive('carried-decode', observed)
    positive('carried-command', command)
    wsjtx_command = copy.deepcopy(wsjtx.vectors['command-09-s3']['input']['event'])
    positive('carried-wsjtx-command', wsjtx_command)
    positive('stream-decode-lf', observed, 'tcp', b'\n')
    positive('stream-command-crlf', command, 'tcp', b'\r\n')
    newer = copy.deepcopy(observed)
    newer.update(spec='0.2.0', kind='future_kind')
    newer['fields'] = {'future_field': 17}
    positive('newer-minor-unknown-kind', newer)
    s.vectors['newer-minor-unknown-kind']['expected']['read'] = {'kind': 'native', 'fields': {}}
    newer_mapping = copy.deepcopy(wsjtx.vectors['compat-nested-addition']['input']['event'])
    positive('newer-mapping-same-spec', newer_mapping)
    s.vectors['newer-mapping-same-spec']['expected']['read'] = copy.deepcopy(
        wsjtx.vectors['compat-nested-addition']['expected']['read'])
    positive('two-hops', s.vectors['carried-decode']['expected']['event'],
             observer=dict(context('from_application'), source_id='second-observer'))
    eight = copy.deepcopy(observed)
    eight['via'] = [{'family': 'qgi', 'context': dict(context('unknown'), source_id=f'hop-{i}')}
                    for i in range(1, 8)]
    positive('eight-hops', eight)
    max_context = dict(context('unknown'), source_id='')
    max_context['source_id'] = 'a' * (4096 - len(wire(max_context)))
    positive('context-budget-boundary', observed, observer=max_context)
    unicode_event = copy.deepcopy(command)
    unicode_event['fields']['text'] = '\U0001f4fb'
    positive('valid-unicode-surrogate-pair', unicode_event)

    # Six envelope members, two context members, three fields and one text-wrapper
    # member, plus six for the newly created via member, array entry and hop/context.
    elements = copy.deepcopy(command)
    elements['fields']['text'] = {'utf8_bytes': [255] * (65536 - 18)}
    positive('output-element-boundary', elements, 'tcp', b'\n')

    # Exactly one MiB after appending the hop, using synthetic text padding. String length and JSON element bounds remain independent.
    boundary = copy.deepcopy(command)
    boundary['fields']['text'] = ''
    trial = copy.deepcopy(boundary)
    trial['via'] = [{'family': 'qgi', 'context': context('unknown')}]
    boundary['fields']['text'] = 'a' * (1048576 - len(wire(trial)))
    positive('output-budget-boundary', boundary, 'tcp', b'\n')


    def negative(name, raw, category, transport='udp'):
        s.add(name, {'class': 'negative',
            'input': {'transport': transport, 'data': b64(raw), 'context': context('unknown')},
            'expected': {'error': {'category': category}}})

    negative('oversized-datagram', b' ' * 65508, 'oversized')
    negative('invalid-json', b'{', 'malformed')
    negative('invalid-utf8', b'{"bad":"\xff"}', 'malformed')
    negative('duplicate-members', wire(command)[:-1] + b',"kind":"control"}', 'malformed')
    duplicate = copy.deepcopy(command)
    duplicate['fields']['x-org.example.nested'] = {'a': 1}
    negative('duplicate-nested-members', wire(duplicate).replace(b'"a":1', b'"a":1,"a":2'), 'malformed')
    other_major = copy.deepcopy(command)
    other_major['spec'] = '1.0.0'
    negative('different-major', wire(other_major), 'unsupported_version')
    invalid = copy.deepcopy(command)
    invalid.pop('fields')
    negative('schema-missing-fields', wire(invalid), 'invalid_value')
    too_much_context = dict(max_context, source_id=max_context['source_id'] + 'a')
    negative('context-budget-overflow', wire(observed), 'oversized')
    s.vectors['context-budget-overflow']['input']['context'] = too_much_context
    negative('nine-hops', wire(s.vectors['eight-hops']['expected']['event']), 'invalid_value')
    too_large = copy.deepcopy(boundary)
    too_large['fields']['text'] += 'a'
    negative('output-budget-overflow', wire(too_large) + b'\n', 'oversized', 'tcp')
    too_many_elements = copy.deepcopy(elements)
    too_many_elements['fields']['text']['utf8_bytes'].append(255)
    negative('output-element-overflow', wire(too_many_elements) + b'\n', 'oversized', 'tcp')
    # Exactly 65,536 input elements would become 65,542 after the hop is appended.
    too_many_elements['fields']['text']['utf8_bytes'].extend([255] * 5)
    negative('input-element-limit-no-hop-room', wire(too_many_elements) + b'\n', 'oversized', 'tcp')
    negative('invalid-unicode-surrogate', wire(command).replace(b'synthetic', b'\\ud800synthetic'), 'malformed')
    negative('invalid-unicode-key', b'{"\\udfff":0}', 'malformed')
    negative('not-an-object', b'[]', 'malformed')
    negative('trailing-object', wire(command) + b'{}', 'malformed')
    negative('oversized-stream', b' ' * 1048577 + b'\n', 'oversized', 'tcp')
    negative('unfinished-stream-record', wire(command), 'truncated', 'tcp')
    negative('empty-stream-record', b'\n', 'malformed', 'tcp')
    negative('json-depth-bound', b'{"nested":' + b'[' * 32 + b'0' + b']' * 32 + b'}', 'oversized')
    negative('json-element-bound', b'{"nested":[' + b'0,' * 65536 + b'0]}\n', 'oversized', 'tcp')
    negative('numeric-token-bound', b'{"number":' + b'1' * 129 + b'}', 'oversized')
    return s


def core_shapes():
    """Structural schema cases only, not invented native-family decode fixtures."""
    cases = []
    observation = {'spec': SPEC, 'family': 'x-org.example.shape', 'mapping': 1,
                   'kind': 'native', 'context': context(), 'fields': {},
                   'native': {'format': 'example_datagram', 'data': ''}}

    def add(name, event, valid=True):
        cases.append({'name': name, 'event': copy.deepcopy(event), 'valid': valid})

    add('observed-native', observation)
    command = {k: copy.deepcopy(v) for k, v in observation.items() if k != 'native'}
    command.update(kind='control', fields={'operation': 'x-org.example.shape.send_packet'})
    add('extension-command', command)
    for kind in ('decode', 'notification', 'result', 'native', 'ambiguous', 'lifecycle'):
        event = copy.deepcopy(command)
        event['kind'] = kind
        event['fields'].update(status='ok', state='heartbeat')
        add('no-native-' + kind, event, False)
    event = copy.deepcopy(command)
    event['fields'] = {}
    add('command-missing-operation', event, False)
    event = copy.deepcopy(command)
    event['native_version'] = '3'
    add('command-declares-native-version', event, False)
    event = copy.deepcopy(observation)
    event.update(kind='control', fields={'operation': 'halt_tx', 'auto_tx_only': True})
    add('observed-control', event)
    event['fields'] = {}
    add('observed-control-missing-operation', event, False)
    event = copy.deepcopy(observation)
    event['native']['family_extra'] = 1
    add('unknown-native-member', event)
    event['spec'] = '0.2.0'
    add('future-minor-native-member', event)
    for status in ('ok', 'error'):
        event = copy.deepcopy(observation)
        event.update(kind='result', fields={'status': status, 'native_code': '-1' if status == 'error' else '0'})
        add('result-' + status, event)
    event['fields']['status'] = 'maybe'
    add('invalid-result-status', event, False)
    event['fields'] = {}
    add('missing-result-status', event, False)
    publish = copy.deepcopy(observation)
    publish.update(family='pskreporter-mqtt', kind='spot', fields={
        'receiver_call': 'Q2AAA', 'receiver_locator': 'FN31', 'sender_call': 'Q1AAA',
        'sender_locator': 'FN42', 'frequency_hz': 14074000, 'mode': 'FT8', 'snr_db': -17,
        'time': '2026-11-12T12:00:00Z', 'reporting_software': 'synthetic decoder'})
    publish['native'] = {'format': 'mqtt_publish', 'data': '', 'topic': 'example/spot', 'retained': False}
    add('publish-shape', publish)
    event = copy.deepcopy(publish)
    event['native']['retained'] = True
    add('retained-publish-shape', event)
    event['native']['data'] = ''
    add('empty-payload-shape', event)
    for key in ('topic', 'retained'):
        event = copy.deepcopy(publish)
        event['native'].pop(key)
        add('publish-missing-' + key, event, False)
    event = copy.deepcopy(publish)
    event['native']['retained'] = 'true'
    add('publish-retained-not-boolean', event, False)
    event = copy.deepcopy(publish)
    event['native']['topic'] = 3
    add('publish-topic-not-text', event, False)
    event = copy.deepcopy(publish)
    event['native']['template'] = ''
    add('publish-with-template', event, False)
    event = copy.deepcopy(observation)
    event['native']['retained'] = False
    add('datagram-with-retained', event, False)
    record = copy.deepcopy(observation)
    record['family'] = 'pskreporter-ipfix'
    record['native'] = {'format': 'ipfix_record', 'data': '', 'template': '', 'message_data': '',
        'set_id': 256, 'set_index': 0, 'record_index': 0,
        'header': {'version': 10, 'length': 16, 'export_time': 0, 'sequence_number': 0, 'observation_domain_id': 7}}
    add('templated-record-shape', record)
    event = copy.deepcopy(record)
    event['native']['dependencies'] = [{k: copy.deepcopy(record['native'][k])
                                         for k in ('data', 'template', 'header', 'set_id')}]
    add('templated-dependency-shape', event)
    for key in ('template', 'message_data', 'header', 'set_id', 'set_index', 'record_index'):
        event = copy.deepcopy(record)
        event['native'].pop(key)
        add('record-missing-' + key, event, False)
    event = copy.deepcopy(record)
    event['native']['header']['sequence_number'] = 2**32
    add('record-header-overflow', event, False)
    event = copy.deepcopy(record)
    event['native']['set_index'] = -1
    add('negative-set-index', event, False)
    event = copy.deepcopy(record)
    event['native']['dependencies'] = [{'data': ''}]
    add('dependency-missing-template', event, False)
    event = copy.deepcopy(record)
    event['native']['topic'] = 'wrong/class'
    add('record-with-topic', event, False)
    carried = copy.deepcopy(observation)
    carried['via'] = [{'family': 'qgi', 'context': context('unknown')}]
    add('qgi-one-hop', carried)
    carried['via'] *= 8
    add('qgi-eight-hops', carried)
    carried['via'].append(copy.deepcopy(carried['via'][0]))
    add('qgi-nine-hops', carried, False)
    for label, base, change in [
        ('nested-header-addition', record, lambda e: e['native']['header'].update(future=1)),
        ('nested-dependency-addition', record, lambda e: e['native'].update(dependencies=[
            dict({k: copy.deepcopy(record['native'][k]) for k in ('data', 'template', 'header', 'set_id')}, future=1)])),
    ]:
        event = copy.deepcopy(base)
        change(event)
        event['mapping'] = 2
        add(label, event)
    for kind in ('decode', 'radio_state', 'qso_logged', 'qso_changed', 'qso_deleted', 'spot'):
        event = copy.deepcopy(observation)
        event.update(family='n1mm', kind=kind, fields={'radio': {'instance': 'synthetic-station', 'number': 2}})
        add('radio-' + kind, event)
    event = copy.deepcopy(event)
    event['fields']['radio']['number'] = '2'
    add('radio-number-not-integer', event, False)
    event.update(kind='radio_state', fields={'radio': {'number': 2}, 'focus_radio': 1,
        'active_radio': 2, 'transmitting': True, 'running': False, 'stereo': True})
    add('so2r-provisional-shape', event)
    event = copy.deepcopy(observation)
    event.update(family='qgi', kind='decode', fields={'snr_db': 'invalid'})
    add('qgi-common-field-type', event, False)
    return {'$comment': 'SPDX-License-Identifier: GPL-3.0-or-later. Schema shapes only; empty byte strings are structural placeholders, not valid MQTT/IPFIX decode fixtures.',
            'spec': SPEC, 'class': 'schema_shape', 'cases': cases}


def validate_qgi(suite, validator):
    """Check explicit oracles with local schema and planted carriage parser failures.

    This test-only parser checks Annex I's JSON bounds, not any carried family's native codec.
    Expected events are authored above, never generated by this parser.
    """
    def bounded_number(token):
        if len(token) > 128:
            raise ValueError('oversized')
        return float(token) if any(c in token for c in '.eE') else int(token)

    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError('malformed')
            result[key] = value
        return result

    def bad_constant(_):
        raise ValueError('malformed')

    def read_json(raw, limit):
        # Apply the same parser limits to received JSON and the resulting event.
        try:
            event = json.loads(raw.decode('utf-8'), object_pairs_hook=pairs,
                               parse_int=bounded_number, parse_float=bounded_number,
                               parse_constant=bad_constant)
        except (json.JSONDecodeError, UnicodeDecodeError) as error:
            raise ValueError('malformed') from error
        if not isinstance(event, dict):
            raise ValueError('malformed')
        count = 0
        pending = [(event, 1)]
        while pending:
            value, depth = pending.pop()
            if isinstance(value, (dict, list)):
                if depth > 32:
                    raise ValueError('oversized')
                count += len(value)
                if count > 65536:
                    raise ValueError('oversized')
                if isinstance(value, dict):
                    pending.extend((key, depth + 1) for key in value)
                    value = value.values()
                pending.extend((item, depth + 1) for item in value)
            elif isinstance(value, str):
                if any(0xd800 <= ord(char) <= 0xdfff for char in value):
                    raise ValueError('malformed')
                if len(value.encode('utf-8')) > limit:
                    raise ValueError('oversized')
        return event

    def parse(inp):
        raw = base64.b64decode(inp['data'], validate=True)
        limit = 65507 if inp['transport'] == 'udp' else 1048576
        if inp['transport'] == 'tcp':
            terminated = raw.endswith(b'\n')
            raw = raw[:-2] if raw.endswith(b'\r\n') else raw[:-1] if terminated else raw
        if len(raw) > limit:
            raise ValueError('oversized')
        if inp['transport'] == 'tcp' and not terminated:
            raise ValueError('truncated')
        event = read_json(raw, limit)
        version = event.get('spec')
        if isinstance(version, str) and version.split('.')[0].isdigit() and version.split('.')[0] != '0':
            raise ValueError('unsupported_version')
        if not validator.is_valid(event):
            raise ValueError('invalid_value')
        if len(event.get('via', [])) == 8:
            raise ValueError('invalid_value')
        contexts = [inp['context'], event['context']] + [hop['context'] for hop in event.get('via', [])]
        if any(len(json.dumps(c, ensure_ascii=True, separators=(',', ':')).encode()) > 4096 for c in contexts):
            raise ValueError('oversized')
        output = copy.deepcopy(event)
        output.setdefault('via', []).append({'family': 'qgi', 'context': inp['context']})
        output_raw = json.dumps(output, ensure_ascii=True, separators=(',', ':'), allow_nan=False).encode()
        if len(output_raw) > 1048576:
            raise ValueError('oversized')
        read_json(output_raw, limit)
        return event, output

    count = 0
    for name, vector in suite.vectors.items():
        if vector['class'] == 'negative':
            try:
                parse(vector['input'])
            except ValueError as error:
                assert str(error) == vector['expected']['error']['category'], (name, error)
            else:
                raise AssertionError(name + ' planted failure was accepted')
            continue
        carried, actual = parse(vector['input'])
        observed = vector['expected']['event']
        validator.validate(observed)
        assert actual == observed, name
        assert observed['context'] == carried['context'], name
        assert observed.get('native') == carried.get('native'), name
        assert observed['via'][-1] == {'family': 'qgi', 'context': vector['input']['context']}, name
        assert vector['reencode']['event'] == observed, name
        if name == 'output-budget-boundary':
            assert len(json.dumps(observed, ensure_ascii=True, separators=(',', ':')).encode()) == 1048576
        if name == 'output-element-boundary':
            assert len(observed['fields']['text']['utf8_bytes']) + 18 == 65536
        count += 1
    return count


def producer_validator(schema, jsonschema):
    """QGI-SER-0009's producer_strict flag: close defined object member sets."""
    strict = copy.deepcopy(schema)
    strict['additionalProperties'] = False
    strict['properties']['spec'] = {'const': SPEC}
    strict['properties']['mapping'] = {'const': 1}
    strict['properties']['family'] = {'enum': ['wsjtx', 'n1mm', 'fldigi', 'js8call',
        'pskreporter-mqtt', 'pskreporter-ipfix', 'qgi']}
    strict['properties']['kind'] = {'enum': [k for k in strict['$defs']
        if k in ('decode', 'radio_state', 'qso_logged', 'qso_changed', 'qso_deleted', 'spot',
                 'score', 'control', 'result', 'notification', 'lifecycle', 'ambiguous', 'native')]}
    uint = lambda bits: {'type': 'integer', 'minimum': 0, 'maximum': 2**bits - 1}
    sint = lambda bits: {'type': 'integer', 'minimum': -2**(bits - 1), 'maximum': 2**(bits - 1) - 1}
    decimal = {'type': 'string', 'pattern': r'^(?:0|-?[1-9][0-9]*)(?![\s\S])'}
    strict['$defs']['wsjtx_zone'] = {'type': 'object', 'required': ['key'], 'properties': {
        **{key: {'$ref': '#/$defs/text'} for key in ('key', 'id', 'name', 'abbreviation', 'comment')},
        'offset_seconds': sint(32), 'country': sint(32)}}
    strict['$defs']['wsjtx_datetime'] = {'type': 'object',
        'required': ['julian_day', 'milliseconds', 'time_spec'], 'properties': {
            'julian_day': decimal, 'milliseconds': uint(32), 'time_spec': uint(8),
            'offset_seconds': sint(32), 'zone': {'$ref': '#/$defs/wsjtx_zone'}}}
    types = {'u8': uint(8), 'u32': uint(32), 'u64': decimal, 'i32': sint(32), 'i64': decimal,
        'bool': {'type': 'boolean'}, 'f64': {'$ref': '#/$defs/binary64'},
        'utf8': {'$ref': '#/$defs/text'}, 'QString': {'$ref': '#/$defs/text'},
        'QTime': uint(32), 'QDateTime': {'$ref': '#/$defs/wsjtx_datetime'},
        'QColor': {'$ref': '#/$defs/qcolor'}}
    layouts = (ROOT / 'spec/annex-b-wsjtx.md').read_text().split('## B.4 Message layouts')[1].split('## B.5')[0]
    message = {'type': 'object', 'required': ['schema', 'type', 'id', 'body', 'extensions'],
        'properties': {'schema': {'enum': [1, 2, 3]}, 'type': {'enum': list(range(17))},
            'id': {'$ref': '#/$defs/text'}, 'body': {'type': 'object'},
            'extensions': {'$ref': '#/$defs/base64'}}, 'allOf': []}
    for section in layouts.split('### ')[1:]:
        typ = int(section.split(':')[0])
        properties, required = {}, []
        for line in section.splitlines():
            if line.startswith('| `'):
                columns = line.split('|')
                key, wire_type = columns[1].strip(' `'), columns[2].strip(' `')
                properties[key] = types[wire_type.rstrip('?')]
                if not wire_type.endswith('?'):
                    required.append(key)
        body_name = f'wsjtx_body_{typ}'
        strict['$defs'][body_name] = {'type': 'object', 'properties': properties, 'required': required}
        message['allOf'].append({'if': {'properties': {'type': {'const': typ}}},
            'then': {'properties': {'body': {'$ref': '#/$defs/' + body_name}}}})
    strict['$defs']['wsjtx_message'] = message
    strict['$defs']['n1mm_message'] = {'type': 'object', 'required': ['root', 'xml'],
        'properties': {'root': {'type': 'string'}, 'xml': {'$ref': '#/$defs/text'}}}
    strict['$defs']['fldigi_message'] = {'type': 'object', 'required': ['root', 'xml'],
        'properties': {'root': {'type': 'string'}, 'method': {'type': 'string'},
                       'xml': {'$ref': '#/$defs/text'}}}
    for family, profile in [('n1mm', 'n1mm_message'), ('fldigi', 'fldigi_message'),
                            ('wsjtx', 'wsjtx_message')]:
        strict['allOf'].append({'if': {'properties': {'family': {'const': family}}},
            'then': {'properties': {'native': {'properties': {'message': {'$ref': '#/$defs/' + profile}}}}}})
    # Full definitions are closed; conditional fragments and the discriminator's
    # body placeholder remain open so their referenced type can define members.
    pending = list(strict['$defs'].values())
    while pending:
        definition = pending.pop()
        if isinstance(definition, dict):
            if definition.get('type') == 'object' and 'properties' in definition:
                definition['additionalProperties'] = False
            pending.extend(definition.values())
        elif isinstance(definition, list):
            pending.extend(definition)
    strict['$defs']['context']['properties']['direction'] = {
        'enum': ['from_application', 'to_application', 'unknown']}
    strict['$defs']['context']['properties']['exchange'] = {
        'enum': ['notification', 'request', 'response', 'unknown']}
    jsonschema.Draft202012Validator.check_schema(strict)
    return jsonschema.Draft202012Validator(strict, format_checker=jsonschema.FormatChecker())


def check_status_sentinels(suite):
    """Independent fixed-prefix reader plants each pair swap; no Rust implementation."""
    names = ('tx_enabled', 'transmitting', 'decoding')
    distinguished = set()
    for vector in suite.vectors.values():
        if vector['class'] != 'positive' or vector['expected']['event']['kind'] != 'radio_state':
            continue
        raw = base64.b64decode(vector['input']['data'])
        offset = 16 + struct.unpack_from('>I', raw, 12)[0] + 8
        for _ in range(4):
            length = struct.unpack_from('>I', raw, offset)[0]
            offset += 4 + (0 if length == 0xffffffff else length)
        actual = dict(zip(names, (bool(b) for b in raw[offset:offset + 3])))
        expected = vector['expected']['event']['fields']
        assert all(actual[key] == expected[key] for key in names)
        for i in range(3):
            for j in range(i + 1, 3):
                swapped = dict(actual)
                swapped[names[i]], swapped[names[j]] = actual[names[j]], actual[names[i]]
                if actual[names[i]] != actual[names[j]]:
                    assert any(swapped[key] != expected[key] for key in names)
                    distinguished.add((i, j))
    assert len(distinguished) == 3, 'every Status boolean pair needs a planted failing swap'


def check_command_text(suite):
    """Encoding wrappers must match UTF-8 and be necessary, even if schema-valid."""
    def valid(value):
        if value is None or isinstance(value, str):
            return True
        if not isinstance(value, dict) or set(value) != {'utf8_bytes'}:
            return False
        try:
            bytes(value['utf8_bytes']).decode('utf-8')
        except UnicodeDecodeError:
            return True
        return False
    for name in ('utf16-for-utf8', 'valid-text-raw-wrapper'):
        assert not valid(suite.vectors['command-error-' + name]['input']['event']['fields']['text'])
    assert valid({'utf8_bytes': [255]}) and valid('A')


def check_revision_outcomes(suite):
    """Native verification requires both a supported schema and a known type."""
    for name in ('old-reader-new-operation', 'known-kind-new-revision', 'known-operation-new-revision',
                 'known-operation-future-native-version'):
        vector = suite.vectors['compat-' + name]
        event = vector['input']['event']
        assert event['mapping'] == 2
        raw = base64.b64decode(event['native']['data'])
        schema = struct.unpack_from('>I', raw, 4)[0]
        assert event['native_version'] == str(schema), name
        verification = 'unavailable'
        if schema in (1, 2, 3):
            typ = struct.unpack_from('>I', raw, 8)[0]
            if typ in range(17):
                verification = 'verified'
            if typ == 8:
                assert event['fields'] == {'operation': 'halt_tx', 'auto_tx_only': bool(raw[-1])}, name
        else:
            # Do not read a type or payload using an unsupported schema's layout.
            assert vector['expected']['read']['kind'] == 'native', name
            assert vector['expected']['read']['fields'] == {}, name
        assert vector['expected']['verification'] == verification, name
        assert 'verification' not in event
    vector = suite.vectors['command-error-known-operation-new-revision']
    assert vector['input']['event']['fields']['operation'] == 'send_text'
    assert vector['input']['event']['mapping'] == 2
    assert vector['expected']['error']['category'] == 'unsupported_version'


def require_date_time(checker):
    """A FormatChecker silently accepts all timestamps without its optional extra."""
    if 'date-time' not in checker.checkers:
        raise SystemExit("date-time format checker unavailable; run uv run --no-project "
                         "--with 'jsonschema[format-nongpl]==4.26.0' python "
                         "scripts/gen_vectors.py --check --producer-strict")


def schema_preflight(producer_strict):
    try:
        import jsonschema
    except ImportError:
        if producer_strict:
            raise SystemExit("--producer-strict requires jsonschema; use uv run --no-project "
                             "--with 'jsonschema[format-nongpl]==4.26.0' python "
                             "scripts/gen_vectors.py --check --producer-strict")
        return
    # Plant the missing optional dependency before doing any vector/schema work.
    try:
        require_date_time(jsonschema.FormatChecker(formats=[]))
    except SystemExit as error:
        assert 'date-time' in str(error) and 'jsonschema[format-nongpl]==4.26.0' in str(error)
    else:
        raise AssertionError('missing date-time checker was silently accepted')
    require_date_time(jsonschema.FormatChecker())


def validate(suite, qgi, producer_strict=False):
    """Use a local schema only; importing jsonschema is optional, never install it."""
    check_status_sentinels(suite)
    check_command_text(suite)
    check_revision_outcomes(suite)
    try:
        import jsonschema
    except ImportError:
        if producer_strict:
            raise SystemExit("--producer-strict requires jsonschema[format-nongpl]==4.26.0")
        return 'jsonschema unavailable; schema validation skipped'
    schema = json.loads((ROOT / 'spec/schema/qgi-event-0.1.0.schema.json').read_text())
    jsonschema.Draft202012Validator.check_schema(schema)
    validator = jsonschema.Draft202012Validator(schema, format_checker=jsonschema.FormatChecker())
    producer = producer_validator(schema, jsonschema) if producer_strict else validator
    count = 0
    for name, vector in suite.vectors.items():
        if vector['class'] == 'positive':
            event = vector['expected']['event']
            validator.validate(event)
            producer.validate(event)
            if event['kind'] in ('decode', 'radio_state', 'qso_logged'):
                assert event['fields']['radio'] == {'instance': event['wire_id']}, name
                assert 'source_id' not in event['context'], name
            assert len(json.dumps(event, ensure_ascii=True, separators=(',', ':')).encode()) <= 1048576
            # Validate the optional structured-view envelope too, when its oracle is supplied.
            if 'expected_message' in vector:
                with_view = copy.deepcopy(event)
                with_view['native']['message'] = vector['expected_message']
                validator.validate(with_view)
                producer.validate(with_view)
                assert len(json.dumps(with_view, ensure_ascii=True, separators=(',', ':')).encode()) <= 1048576
            count += 1
        elif vector['class'] == 'command':
            validator.validate(vector['input']['event'])
            producer.validate(vector['input']['event'])
            assert 'native' not in vector['input']['event']
            assert 'native_version' not in vector['input']['event']
        elif vector['class'] == 'compatibility' and vector['writer_spec'].startswith('0.'):
            validator.validate(vector['input']['event'])
        elif name.startswith('event-') and name != 'event-native-disagreement':
            assert not validator.is_valid(vector['input']['event']), name
    shapes = core_shapes()['cases']
    for case in shapes:
        assert validator.is_valid(case['event']) == case['valid'], case['name']
    qgi_count = validate_qgi(qgi, validator)
    hop_profile = producer if producer_strict else producer_validator(schema, jsonschema)
    hop_producer = hop_profile.evolve(schema={'$ref': '#/$defs/hop', '$defs': hop_profile.schema['$defs']})

    def validate_forwarded(event):
        newer_spec = tuple(map(int, event['spec'].split('.'))) > tuple(map(int, SPEC.split('.')))
        newer_mapping = event['mapping'] > 1  # This runner supports family mapping revision 1.
        (validator if newer_spec or newer_mapping else producer).validate(event)
        hop_producer.validate(event['via'][-1])

    for vector in qgi.vectors.values():
        if vector['class'] == 'positive':
            validate_forwarded(vector['expected']['event'])
    if producer_strict:
        for name in ('newer-minor-unknown-kind', 'newer-mapping-same-spec'):
            event = copy.deepcopy(qgi.vectors[name]['expected']['event'])
            event['via'][-1]['context']['future'] = 1
            assert validator.is_valid(event) and not hop_producer.is_valid(event['via'][-1]), name
            try:
                validate_forwarded(event)
            except jsonschema.ValidationError:
                pass
            else:
                raise AssertionError(name + ' accepted an unknown member in the appended hop')
        # Plant nested additions where the reader must accept and producer must reject.
        event = copy.deepcopy(qgi.vectors['carried-decode']['expected']['event'])
        event['via'][0]['future'] = 1
        assert validator.is_valid(event) and not producer.is_valid(event)
        for base_name, mutate in [
            ('13-highlight-callsign-s3', lambda e: e['fields']['background_color'].update(future=1)),
            ('01-status-s3', lambda e: e['fields']['radio'].update(future=1)),
            ('radio-instance-raw', lambda e: e['fields']['radio']['instance'].update(future=1)),
        ]:
            event = copy.deepcopy(suite.vectors[base_name]['expected']['event'])
            mutate(event)
            assert validator.is_valid(event) and not producer.is_valid(event)
        for case in shapes:
            if case['name'].startswith('nested-'):
                assert validator.is_valid(case['event']) and not producer.is_valid(case['event'])
        event = copy.deepcopy(suite.vectors['00-heartbeat-s3']['expected']['event'])
        event['native']['message'] = copy.deepcopy(suite.vectors['00-heartbeat-s3']['expected_message'])
        for mutate in (lambda e: e['native']['message'].update(future=1),
                       lambda e: e['native']['message']['body'].update(future=1)):
            planted = copy.deepcopy(event)
            mutate(planted)
            assert validator.is_valid(planted) and not producer.is_valid(planted)
        event = copy.deepcopy(suite.vectors['00-heartbeat-s3']['expected']['event'])
        for mutate in (lambda e: e.update(future=1), lambda e: e['context'].update(future=1),
                       lambda e: e['native'].update(future=1), lambda e: e['fields'].update(future=1)):
            planted = copy.deepcopy(event)
            mutate(planted)
            assert validator.is_valid(planted) and not producer.is_valid(planted)

    return f"schema checked {len(shapes)} planted core shapes; validated {count} WSJT-X and {qgi_count} qgi positive observations, all positive commands and same-MAJOR compatibility inputs; qgi negative parsing/bounds cases checked; producer_strict={producer_strict}; all three Status boolean swap mutations rejected"


# Annex D fixture catalogs are independent of every implementation. Each row has
# native sample text and a separately authored common-field oracle, not a decoder.
N1MM_SOURCES = {
    'https://n1mmwp.hamdocs.com/appendices/external-udp-broadcasts/':
        '4bd29b8de620b867d8573d2f0dbff78b7af1f50d64f40804d809b4c281bafab9',
    'https://n1mmwp.hamdocs.com/mmfiles/udp_external_udp_broadcasts-xml/':
        '784c869b8a7d56ca57e6bb8d6c0308e36ebf6caed44ed28fd6b4c8619c6a5d51',
    'https://n1mmwp.hamdocs.com/mmfile/get/file/UDP_External_UDP_Broadcasts.xml':
        '58e0d335c325d1fde94a285b581fdaa4d75965b1aaa564e40969dcbf7eb8400a',
    'https://n1mmwp.hamdocs.com/getting-started/installing-and-upgrading-n1mm-logger/':
        '17243f496261d1b53bb96640a9d9f4beedf2c1f8724790c2619bf092f4186506',
    'https://n1mmwp.hamdocs.com/faq/':
        'af14392fdb130b033351417515fb8362863e8ec1392185d0072ed8fcbfe67e21',
    'https://n1mmwp.hamdocs.com/n1mm-manual/windows/':
        'c3cf58fb16dcdfb24074317680298fab374069c29e93e8904eeb93a60d7f3ee4',
}
XML_DECL = '<?xml version="1.0" encoding="utf-8"?>'


def n1mm_vectors():
    s = Suite('n1mm')

    def row(name, value, key=None, oracle=None):
        return name, value, {} if key is None else {key: value if oracle is None else oracle}

    contact = [row('app', 'synthetic-logger'), row('contestname', 'SYNTHETIC', 'contest_name'),
        row('contestnr', '23'), row('timestamp', '2026-11-12 12:34:56', 'start', '2026-11-12T12:34:56Z'),
        row('mycall', 'Q2AAA', 'station_call'), row('band', '3,5', 'band'),
        row('rxfreq', '352519', 'dial_frequency_hz', 3525190),
        row('txfreq', '352621', 'tx_frequency_hz', 3526210),
        row('operator', 'Q2BBB', 'operator_call'), row('mode', 'CW', 'mode'), row('call', 'Q1AAA', 'call'),
        row('countryprefix', 'Q'), row('wpxprefix', 'Q1'), row('stationprefix', 'Q2AAA'),
        row('continent', 'NA'), row('snt', '599', 'report_sent'), row('sntnr', '005', 'serial_sent'),
        row('rcv', '579', 'report_received'), row('rcvnr', '009', 'serial_received'),
        row('gridsquare', 'FN42', 'grid'), row('exchangel', 'synthetic exchange', 'exchange_received'),
        row('section', 'SYN', 'section'), row('comment', 'synthetic comment', 'comments'), row('qth', 'Synthetic City'),
        row('name', 'Synthetic Operator', 'name'), row('power', '100'), row('misctext', 'synthetic'),
        row('zone', '5'), row('prec', 'A', 'precedence'), row('ck', '07', 'check'),
        row('ismultiplierl', '1'), row('ismultiplier2', '0'), row('ismultiplier3', '0'),
        row('points', '2'), row('radionr', '2', 'radio', {'number': 2}), row('run1run2', '1'),
        row('RoverLocation', 'synthetic-rover'), row('RadioInterfaced', '1'), row('NetworkedCompNr', '3'),
        row('IsOriginal', 'False'), row('NetBiosName', 'SYNTHETIC-ORIG'), row('IsRunQSO', '0'),
        row('StationName', 'SYNTHETIC-PC', 'radio', {'instance': 'SYNTHETIC-PC'}),
        row('ID', '0123456789abcdef0123456789abcdef', 'record_id'), row('IsClaimedQso', '1'),
        row('oldtimestamp', '2026-11-12 12:30:00', 'previous_start', '2026-11-12T12:30:00Z'),
        row('oldcall', 'Q1BBB'), row('SentExchange', '# SYN')]
    historical = [row('exchange1', 'historical exchange', 'exchange_received'), row('ismultiplier1', '1'),
                  row('Run1Run2', '2'), row('ContactType', 'synthetic')]
    radio = [row('app', 'synthetic-logger'), row('StationName', 'SYNTHETIC-PC', 'radio', {'instance': 'SYNTHETIC-PC'}),
        row('RadioNr', '2', 'radio', {'number': 2}), row('Freq', '1407123', 'dial_frequency_hz', 14071230),
        row('TXFreq', '1407234'), row('Mode', 'USB', 'mode'), row('mycall', 'Q2AAA', 'station_call'),
        row('OpCall', 'Q2BBB'), row('IsRunning', 'True', 'running', True), row('FocusEntry', '12345'),
        row('EntryWindowHwnd', '23456'), row('Antenna', '8'), row('Rotors', 'synthetic-rotor'),
        row('FocusRadioNr', '1', 'focus_radio', 1), row('IsStereo', 'True', 'stereo', True), row('IsSplit', 'False'),
        row('ActiveRadioNr', '2', 'active_radio', 2), row('IsTransmitting', 'False', 'transmitting', False),
        row('FunctionKeyCaption', 'synthetic-CQ'), row('RadioName', 'synthetic-radio'), row('AuxAntSelected', '-1'),
        row('AuxAntSelectedName', ''), row('IsConnected', '')]
    spot = [row('app', 'synthetic-logger'), row('StationName', 'Q2AAA', 'radio', {'instance': 'Q2AAA'}),
        row('dxcall', 'Q1AAA', 'call'), row('frequency', '7061.2', 'frequency_hz', 7061200), row('spottercall', 'Q2BBB', 'spotter_call'),
        row('timestamp', '2026-11-12 12:34:56', 'time', '2026-11-12T12:34:56Z'),
        row('action', 'add'), row('mode', 'CW', 'mode'),
        row('comment', 'synthetic spot ', 'text'), row('status', 'single mult'), row('statuslist', 'single mult')]
    app = [row('app', 'synthetic-logger'), row('dbname', 'synthetic.s3db'), row('contestnr', '23'),
        row('contestname', 'SYNTHETIC'), row('StationName', 'SYNTHETIC-PC'), row('mycall', 'Q2AAA')]
    deletion = [r for r in contact if r[0] in ('app', 'timestamp', 'mycall', 'band', 'call', 'contestnr', 'StationName', 'ID')]
    score = [row('contest', 'SYNTHETIC', 'contest_name'), row('call', 'Q2AAA', 'station_call'),
        row('ops', 'Q2BBB'), row('class', ''), row('club', 'synthetic-club'), row('qth', ''),
        row('breakdown', ''), row('score', '4321', 'total_score', 4321),
        row('timestamp', '2026-11-12 12:34:56', 'time', '2026-11-12T12:34:56Z')]
    qsy = [row('app', 'synthetic-client', 'app'), row('radionr', '2', 'radio', {'number': 2}),
           row('frequency', '14053.284', 'frequency_hz', 14053284), row('mousebutton', 'Right')]
    spectrum = [row('app', 'synthetic-client', 'app'), row('Name', 'synthetic-spectrum', 'spectrum_name'),
        row('LowScopeFrequency', '14000', 'low_scope_frequency', 14000000),
        row('HighScopeFrequency', '14055', 'high_scope_frequency', 14055000), row('ScalingFactor', '0.3125', 'scaling_factor'),
        row('DataCount', '4'), row('SpectrumData', '0,1,160,65535')]
    catalogs = {'AppInfo': (app, 'native'), 'contactinfo': (contact + historical, 'qso_logged'),
        'contactreplace': (contact + historical, 'qso_changed'), 'contactdelete': (deletion, 'qso_deleted'),
        'lookupinfo': (contact + historical, 'native'), 'lookup': (contact + historical, 'native'),
        'RadioInfo': (radio, 'radio_state'), 'spot': (spot, 'spot'), 'dynamicresults': (score, 'score'),
        'radio_setfrequency': (qsy, 'control'), 'Spectrum': (spectrum, 'control')}

    def xml(root, rows):
        # Only synthetic ASCII sample content; dedicated escaping cases are below.
        return ('<' + root + '>' + ''.join('<' + n + '>' + v + '</' + n + '>' for n, v, _ in rows)
                + '</' + root + '>').encode()

    def fields(root, rows):
        out = {}
        for name, _, oracle in rows:
            if root == 'AppInfo' or root in ('lookupinfo', 'lookup') and name not in ('timestamp', 'oldtimestamp'):
                continue
            if name == 'exchangel' and any(n == 'exchange1' for n, _, _ in rows):
                continue
            for key, value in oracle.items():
                if key == 'radio':
                    out.setdefault(key, {}).update(value)
                else:
                    out[key] = value
        if root in ('radio_setfrequency', 'Spectrum'):
            out['operation'] = 'set_frequency' if root == 'radio_setfrequency' else 'n1mm.spectrum'
        return out

    def positive(name, raw, kind, expected, direction='from_application', view=False, supplied=None):
        ctx = supplied or context(direction)
        event = {'spec': SPEC, 'family': 'n1mm', 'mapping': 1, 'kind': kind,
                 'context': copy.deepcopy(ctx), 'fields': copy.deepcopy(expected),
                 'native': {'format': 'n1mm_xml', 'data': b64(raw)}}
        value = {'class': 'positive', 'input': {'data': b64(raw), 'context': copy.deepcopy(ctx)},
                 'expected': {'event': event}, 'reencode': {'data': b64(raw)}}
        if view:
            import re
            content = raw.decode('utf-8-sig')
            value['expected_message'] = {'root': re.search(r'<([A-Za-z_][\w.-]*)[\s>]', content)[1], 'xml': content}
        s.add(name, value)
        return event

    for root, (rows, kind) in catalogs.items():
        direction = 'to_application' if root in ('radio_setfrequency', 'Spectrum') else 'from_application'
        positive(root + '-full', XML_DECL.encode() + xml(root, rows), kind, fields(root, rows), direction, view=True)
        positive(root + '-absent', ('<' + root + '/>').encode(), kind, fields(root, []), direction)
        # Each documented leaf has separate presence and empty oracles; absence is
        # the minimal case above, without a required-field assumption from examples.
        for name, value, oracle in rows:
            positive(root + '-only-' + name, xml(root, [(name, value, oracle)]), kind,
                     fields(root, [(name, value, oracle)]), direction)
            empty_oracle = {k: ({'instance': ''} if k == 'radio' and 'instance' in v else '')
                            for k, v in oracle.items()
                            if k not in ('start', 'previous_start', 'time')
                            and (isinstance(v, str) or k == 'radio' and 'instance' in v)}
            positive(root + '-empty-' + name, xml(root, [(name, '', {})]), kind,
                     fields(root, [(name, '', empty_oracle)]), direction)
    # Score nested children and every optional category/selector attribute.
    score_raw = (b'<dynamicresults><contest>SYNTHETIC</contest><call>Q2AAA</call><ops>Q2BBB</ops>'
        b'<class power="HIGH" assisted="ASSISTED" transmitter="UNLIMITED" ops="MULTI-OP" bands="ALL" mode="MIXED" overlay="N/A"/>'
        b'<club>synthetic-club</club><qth><dxcccountry>K</dxcccountry><cqzone>5</cqzone><iaruzone>8</iaruzone>'
        b'<arrlsection>SYN</arrlsection><stprvoth>SYN</stprvoth><grid6>FN42AA</grid6></qth>'
        b'<breakdown><qso band="20" mode="CW">11</qso><point band="20" mode="CW">22</point>'
        b'<qso band="total" mode="ALL">33</qso><point band="total" mode="ALL">44</point></breakdown>'
        b'<score>55</score><timestamp>2026-11-12 12:34:56</timestamp></dynamicresults>')
    positive('score-nested', score_raw, 'score', {'contest_name': 'SYNTHETIC', 'station_call': 'Q2AAA',
        'qso_count': 33, 'points': 44, 'total_score': 55, 'time': '2026-11-12T12:34:56Z'}, view=True)
    for attr in ('power', 'assisted', 'transmitter', 'ops', 'bands', 'mode', 'overlay'):
        positive('score-class-attribute-' + attr, f'<dynamicresults><class {attr}="synthetic"/></dynamicresults>'.encode(), 'score', {})
        positive('score-class-empty-attribute-' + attr, f'<dynamicresults><class {attr}=""/></dynamicresults>'.encode(), 'score', {})
    for tag in ('dxcccountry', 'cqzone', 'iaruzone', 'arrlsection', 'stprvoth', 'grid6'):
        for label, val in (('present', 'synthetic'), ('empty', '')):
            positive('score-qth-' + tag + '-' + label,
                f'<dynamicresults><qth><{tag}>{val}</{tag}></qth></dynamicresults>'.encode(), 'score', {})
    for tag, key, count in (('qso', 'qso_count', 33), ('point', 'points', 44)):
        for label, attrs, val, out in [
            ('total', 'band="total" mode="ALL"', str(count), {key: count}),
            ('empty', 'band="total" mode="ALL"', '', {}),
            ('no-band', 'mode="ALL"', str(count), {}), ('no-mode', 'band="total"', str(count), {}),
            ('no-attributes', '', str(count), {}), ('empty-band', 'band="" mode="ALL"', str(count), {}),
            ('empty-mode', 'band="total" mode=""', str(count), {}),
            ('other-band', 'band="20" mode="ALL"', str(count), {}),
            ('unknown-attribute', 'band="total" mode="ALL" extra="synthetic"', str(count), {key: count}),
        ]:
            raw = f'<dynamicresults><breakdown><{tag} {attrs}>{val}</{tag}></breakdown></dynamicresults>'.encode()
            positive('score-' + tag + '-' + label, raw, 'score', out)
    for raw, name in [
        (b'<dynamicresults><breakdown><qso band="total" mode="ALL">1</qso><qso band="total" mode="ALL">2</qso></breakdown></dynamicresults>', 'duplicate-totals'),
        (b'<dynamicresults><breakdown/><breakdown><qso band="total" mode="ALL">1</qso></breakdown></dynamicresults>', 'duplicate-breakdown'),
        (b'<dynamicresults><qth grid6="FN42AA"/></dynamicresults>', 'qth-attribute-not-child'),
    ]:
        positive('score-' + name, raw, 'score', {})
    for spelling, value in [('True', True), ('true', True), ('1', True), ('False', False), ('false', False), ('0', False)]:
        for tag, key in [('IsRunning', 'running'), ('IsStereo', 'stereo'), ('IsTransmitting', 'transmitting')]:
            positive('bool-' + tag + '-' + spelling, f'<RadioInfo><{tag}>{spelling}</{tag}></RadioInfo>'.encode(),
                     'radio_state', {key: value})
    for spelling in ('yes', 'no', 'TRUE', 'FALSE', '2', ''):
        positive('bool-unmapped-' + (spelling or 'empty'), f'<RadioInfo><IsRunning>{spelling}</IsRunning></RadioInfo>'.encode(), 'radio_state', {})
    positive('bool-whitespace', b'<RadioInfo><IsRunning> \ttrue\r\n</IsRunning></RadioInfo>', 'radio_state', {'running': True})
    for nr, focus, active in [(1, 2, 1), (2, 1, 2), (1, 1, 2), (2, 2, 1), (0, 3, 4)]:
        positive(f'so2r-{nr}-{focus}-{active}', f'<RadioInfo><RadioNr>{nr}</RadioNr><FocusRadioNr>{focus}</FocusRadioNr><ActiveRadioNr>{active}</ActiveRadioNr></RadioInfo>'.encode(),
            'radio_state', {'radio': {'number': nr}, 'focus_radio': focus, 'active_radio': active})
    for root, freq, factor, kind, key in [('RadioInfo', 'Freq', 10, 'radio_state', 'dial_frequency_hz'),
            ('contactinfo', 'rxfreq', 10, 'qso_logged', 'dial_frequency_hz'),
            ('contactinfo', 'txfreq', 10, 'qso_logged', 'tx_frequency_hz')]:
        for label, val, out in [('signed', ' +001234 ', {key: 12340}), ('zero', '0', {key: 0}),
            ('negative', '-1', {key: -10}), ('maximum', '900719925474099', {key: 9007199254740990}),
            ('overflow', '900719925474100', {}), ('decimal', '14.5', {}), ('invalid', 'synthetic', {})]:
            positive(f'number-{root}-{freq}-{label}', f'<{root}><{freq}>{val}</{freq}></{root}>'.encode(), kind, out)
    # QGI-N1M-0036: independently authored kHz text / integer-Hz oracles.
    for root, tag, kind, key in [('spot', 'frequency', 'spot', 'frequency_hz'),
            ('radio_setfrequency', 'frequency', 'control', 'frequency_hz'),
            ('Spectrum', 'LowScopeFrequency', 'control', 'low_scope_frequency'),
            ('Spectrum', 'HighScopeFrequency', 'control', 'high_scope_frequency')]:
        for label, val, hz in [('whole-khz', '7000', 7000000),
            ('fractional', '14053.284', 14053284), ('630m', '475.0', 475000),
            ('2200m', '137.5', 137500), ('zero', '0', 0), ('one-hz', '0.001', 1),
            ('minimum', '-9007199254740.991', -9007199254740991),
            ('maximum', '9007199254740.991', 9007199254740991),
            ('underflow', '-9007199254740.992', None), ('overflow', '9007199254740.992', None),
            ('sub-hz', '7061.2001', None), ('sub-hz-zero', '0.0001', None),
            ('trailing-zeros', '7061.2000', 7061200), ('signed', ' \t+007061.200\r\n', 7061200),
            ('negative', '-0.001', -1), ('exponent', '7e3', None),
            ('leading-point', '.001', None), ('trailing-point', '7000.', None),
            ('invalid', 'synthetic', None), ('empty', '', None)]:
            expected = {} if hz is None else {key: hz}
            if kind == 'control':
                expected['operation'] = 'set_frequency' if root == 'radio_setfrequency' else 'n1mm.spectrum'
            positive(f'khz-{root}-{tag}-{label}', f'<{root}><{tag}>{val}</{tag}></{root}>'.encode(),
                     kind, expected, 'to_application' if kind == 'control' else 'from_application', view=True)
        for label, content in [('duplicate', f'<{tag}>7000</{tag}><{tag}>7001</{tag}>'),
                ('nested', f'<{tag}><part>7000</part></{tag}>')]:
            expected = {} if kind == 'spot' else {'operation': 'set_frequency' if root == 'radio_setfrequency' else 'n1mm.spectrum'}
            positive(f'khz-{root}-{tag}-{label}', f'<{root}>{content}</{root}>'.encode(), kind, expected)
        s.add(f'khz-{root}-{tag}-token-overflow', {'class': 'negative',
            'input': {'data': b64(f'<{root}><{tag}>{"1" * 129}</{tag}></{root}>'.encode()),
                      'context': context('to_application' if kind == 'control' else 'from_application')},
            'expected': {'error': {'category': 'oversized'}}})
    for label, raw, out in [
        ('entities', b'<RadioInfo><StationName>A&amp;B&lt;C&gt;&quot;&apos;&#65;&#x42;</StationName></RadioInfo>', {'radio': {'instance': 'A&B<C>"\'AB'}}),
        ('cdata', b'<RadioInfo><StationName>a<![CDATA[<b>]]>c</StationName></RadioInfo>', {'radio': {'instance': 'a<b>c'}}),
        ('line-endings', b'<RadioInfo><StationName> a\r\nb\rc </StationName></RadioInfo>', {'radio': {'instance': ' a\nb\nc '}}),
        ('self-closing', b'<RadioInfo><StationName/></RadioInfo>', {'radio': {'instance': ''}}),
        ('unknown', b'<RadioInfo future="yes"><Future><Freq>999</Freq></Future><Freq>1407123</Freq></RadioInfo>', {'dial_frequency_hz': 14071230}),
        ('duplicate-leaf', b'<RadioInfo><Freq>1</Freq><Freq>2</Freq></RadioInfo>', {}),
        ('nested-leaf', b'<RadioInfo><Freq><part>1</part></Freq></RadioInfo>', {}),
        ('attribute-not-leaf', b'<RadioInfo Freq="1407123"/>', {}),
        ('comments-pi', b'<?synthetic test?><RadioInfo><!-- <!DOCTYPE innocent> --><StationName>synthetic</StationName></RadioInfo>', {'radio': {'instance': 'synthetic'}}),
        ('invalid-utf8', b'<RadioInfo><StationName>A\xffB</StationName></RadioInfo>', {'radio': {'instance': {'utf8_bytes': [65, 255, 66]}}}),
    ]:
        positive('xml-' + label, raw, 'radio_state', out)
    positive('aliases-prefer-digit', b'<contactinfo><exchangel>old</exchangel><exchange1>new</exchange1></contactinfo>', 'qso_logged', {'exchange_received': 'new'})
    positive('aliases-prefer-empty', b'<contactinfo><exchangel>old</exchangel><exchange1/></contactinfo>', 'qso_logged', {'exchange_received': ''})
    positive('aliases-no-fallback-on-duplicate', b'<contactinfo><exchangel>old</exchangel><exchange1>a</exchange1><exchange1>b</exchange1></contactinfo>', 'qso_logged', {})
    positive('spot-delete', b'<spot><action>delete</action><dxcall>Q1AAA</dxcall></spot>', 'spot', {'call': 'Q1AAA'})
    # Timestamp oracles are literal pairs, independent of the calendar checker.
    # Bad timestamp scalars still produce positive messages with native evidence.
    timestamp_cases = [
        ('documented', '2020-01-17 16:43:38', '2020-01-17T16:43:38Z'),
        ('leap-day', '2024-02-29 00:00:00', '2024-02-29T00:00:00Z'),
        ('leap-century', '2000-02-29 23:59:59', '2000-02-29T23:59:59Z'),
        ('year-end', '2020-12-31 23:59:59', '2020-12-31T23:59:59Z'),
        ('minimum-year', '0001-01-01 00:00:00', '0001-01-01T00:00:00Z'),
        ('maximum-year', '9999-12-31 23:59:59', '9999-12-31T23:59:59Z'),
        ('bad-leap-day', '2023-02-29 12:00:00', None),
        ('bad-century', '1900-02-29 12:00:00', None),
        ('bad-month', '2020-13-01 12:00:00', None),
        ('bad-day', '2020-04-31 12:00:00', None),
        ('zero-year', '0000-01-01 00:00:00', None),
        ('bad-hour', '2020-01-17 24:00:00', None),
        ('bad-minute', '2020-01-17 16:60:00', None),
        ('bad-second', '2020-01-17 16:43:60', None),
        ('fractional', '2020-01-17 16:43:38.125', None),
        ('zoned', '2020-01-17 16:43:38Z', None),
        ('offset', '2020-01-17 16:43:38+00:00', None),
        ('iso-separator', '2020-01-17T16:43:38', None),
        ('short-component', '2020-1-17 16:43:38', None),
        ('surrounding-space', ' 2020-01-17 16:43:38 ', None),
        ('trailing-newline', '2020-01-17 16:43:38\n', None),
        ('unicode-digits', '２０２０-01-17 16:43:38', None),
        ('text', 'synthetic-clock', None),
    ]
    for root, tag, key in [
        ('contactinfo', 'timestamp', 'start'), ('contactinfo', 'oldtimestamp', 'previous_start'),
        ('contactreplace', 'timestamp', 'start'), ('contactreplace', 'oldtimestamp', 'previous_start'),
        ('contactdelete', 'timestamp', 'start'), ('lookupinfo', 'timestamp', 'start'),
        ('lookupinfo', 'oldtimestamp', 'previous_start'), ('lookup', 'timestamp', 'start'),
        ('lookup', 'oldtimestamp', 'previous_start'), ('spot', 'timestamp', 'time'),
        ('dynamicresults', 'timestamp', 'time')]:
        kind = catalogs[root][1]
        for label, text, instant in timestamp_cases:
            raw = f'<{root}><{tag}>{text}</{tag}></{root}>'.encode()
            positive(f'time-{root}-{tag}-{label}', raw, kind, {} if instant is None else {key: instant})
        for label, content in [('duplicate', '<{t}>2020-01-17 16:43:38</{t}>' * 2),
                ('nested', '<{t}><part>2020-01-17 16:43:38</part></{t}>')]:
            positive(f'time-{root}-{tag}-{label}',
                ('<' + root + '>' + content.format(t=tag) + '</' + root + '>').encode(), kind, {})
        positive(f'time-{root}-{tag}-invalid-utf8',
            f'<{root}><{tag}>2020-01-17 16:43:'.encode() + b'\xff' + f'</{tag}></{root}>'.encode(), kind, {})
    positive('time-replace-bad-current',
        b'<contactreplace><timestamp>synthetic</timestamp><oldtimestamp>2020-01-17 16:43:38</oldtimestamp></contactreplace>',
        'qso_changed', {'previous_start': '2020-01-17T16:43:38Z'})
    positive('time-replace-bad-prior',
        b'<contactreplace><timestamp>2020-01-17 16:43:38</timestamp><oldtimestamp>synthetic</oldtimestamp></contactreplace>',
        'qso_changed', {'start': '2020-01-17T16:43:38Z'})
    positive('time-xml-text',
        b'<contactinfo><timestamp>2020-01-17&#32;16:<![CDATA[43:38]]></timestamp></contactinfo>',
        'qso_logged', {'start': '2020-01-17T16:43:38Z'})
    for encoding in ('utf-8-sig', 'utf-16-le', 'utf-16-be'):
        declaration = '<?xml version="1.0" encoding="UTF-16"?>' if encoding.startswith('utf-16') else XML_DECL
        raw = (declaration + '<RadioInfo><StationName>synthetic-α</StationName></RadioInfo>').encode(encoding)
        if encoding.startswith('utf-16'):
            raw = (b'\xff\xfe' if encoding.endswith('le') else b'\xfe\xff') + raw
        positive('encoding-' + encoding, raw, 'radio_state', {'radio': {'instance': 'synthetic-α'}})
    for encoding in ('utf-8-sig', 'utf-16-le', 'utf-16-be'):
        v = s.vectors['encoding-' + encoding]
        v['expected_message'] = {'root': 'RadioInfo',
            'xml': base64.b64decode(v['input']['data'])[3 if encoding == 'utf-8-sig' else 2:].decode('utf-8' if encoding == 'utf-8-sig' else encoding)}
    positive('encoding-utf16-signature', '<?xml version="1.0" encoding="UTF-16BE"?><RadioInfo/>'.encode('utf-16-be'), 'radio_state', {})
    raw = b'\xff\xfe' + '<RadioInfo><StationName>A\ud800B</StationName></RadioInfo>'.encode('utf-16-le', 'surrogatepass')
    positive('invalid-utf16-leaf', raw, 'radio_state', {'radio': {'instance': {'utf16_units': [65, 0xd800, 66]}}})
    positive('caller-context', b'<RadioInfo/>', 'radio_state', {}, supplied={
        'direction': 'unknown', 'exchange': 'request', 'source_id': 'synthetic-observer',
        'target_id': 'synthetic-target', 'correlation_id': 'synthetic-request'})
    positive('depth-boundary', b'<RadioInfo>' + b'<x>' * 15 + b'</x>' * 15 + b'</RadioInfo>', 'radio_state', {})
    positive('element-boundary', b'<RadioInfo>' + b'<x/>' * 1023 + b'</RadioInfo>', 'radio_state', {})
    positive('attributes-per-element-boundary', ('<RadioInfo ' + ' '.join(f'a{i}=""' for i in range(16)) + '/>').encode(), 'radio_state', {})
    positive('attributes-total-boundary', ('<RadioInfo>' + ''.join('<x ' + ' '.join(f'a{i}=""' for i in range(16)) + '/>' for _ in range(16)) + '</RadioInfo>').encode(), 'radio_state', {})
    positive('name-boundary', ('<RadioInfo><' + 'x' * 128 + '/></RadioInfo>').encode(), 'radio_state', {})
    positive('numeric-token-boundary', b'<RadioInfo><Freq>' + b'9' * 128 + b'</Freq></RadioInfo>', 'radio_state', {})
    pad = 'a' * (65507 - len(b'<RadioInfo><StationName></StationName></RadioInfo>'))
    positive('datagram-output-boundary', ('<RadioInfo><StationName>' + pad + '</StationName></RadioInfo>').encode(),
             'radio_state', {'radio': {'instance': pad}}, view=True)
    for name, raw, category in [
        ('doctype', b'<!DOCTYPE RadioInfo><RadioInfo/>', 'malformed'),
        ('entity-declaration', b'<!ENTITY x "synthetic"><RadioInfo/>', 'malformed'),
        ('entity-expansion', b'<!DOCTYPE RadioInfo [<!ENTITY a "aaaaaaaa"><!ENTITY b "&a;&a;&a;&a;">]><RadioInfo>&b;</RadioInfo>', 'malformed'),
        ('external-entity', b'<!DOCTYPE RadioInfo [<!ENTITY x SYSTEM "https://example.invalid/synthetic">]><RadioInfo>&x;</RadioInfo>', 'malformed'),
        ('external-file', b'<!DOCTYPE RadioInfo SYSTEM "file:///synthetic"><RadioInfo/>', 'malformed'),
        ('parameter-entity', b'<!DOCTYPE RadioInfo [<!ENTITY % x SYSTEM "https://example.invalid/synthetic">%x;]><RadioInfo/>', 'malformed'),
        ('depth-overflow', b'<RadioInfo>' + b'<x>' * 16 + b'</x>' * 16 + b'</RadioInfo>', 'oversized'),
        ('elements-overflow', b'<RadioInfo>' + b'<x/>' * 1024 + b'</RadioInfo>', 'oversized'),
        ('attributes-overflow', ('<RadioInfo ' + ' '.join(f'a{i}=""' for i in range(17)) + '/>').encode(), 'oversized'),
        ('total-attributes-overflow', ('<RadioInfo>' + ''.join('<x ' + ' '.join(f'a{i}=""' for i in range(16)) + '/>' for _ in range(17)) + '</RadioInfo>').encode(), 'oversized'),
        ('name-overflow', ('<RadioInfo><' + 'x' * 129 + '/></RadioInfo>').encode(), 'oversized'),
        ('numeric-token-overflow', b'<RadioInfo><Freq>' + b'9' * 129 + b'</Freq></RadioInfo>', 'oversized'),
        ('datagram-overflow', b' ' * 65508, 'oversized'), ('size-before-attack', b'<!DOCTYPE x>' + b' ' * 65500, 'oversized'),
        ('empty', b'', 'truncated'), ('unfinished', b'<RadioInfo><Freq>1', 'truncated'),
        ('unfinished-tag', b'<RadioInfo', 'truncated'), ('unfinished-entity', b'<RadioInfo>&am', 'truncated'),
        ('mismatched', b'<RadioInfo><x></y></RadioInfo>', 'malformed'), ('multiple-roots', b'<RadioInfo/><AppInfo/>', 'malformed'),
        ('invalid-nul', b'<RadioInfo><StationName>\x00</StationName></RadioInfo>', 'malformed'),
        ('undeclared-entity', b'<RadioInfo>&synthetic;</RadioInfo>', 'malformed'),
        ('duplicate-attribute', b'<RadioInfo x="a" x="b"/>', 'malformed'),
        ('unbound-prefix', b'<RadioInfo><x:y/></RadioInfo>', 'malformed'),
        ('invalid-markup-utf8', b'<Radio\xffInfo/>', 'malformed'),
        ('odd-utf16', b'\xff\xfe<\x00R', 'truncated'),
        ('encoding-conflict', b'<?xml version="1.0" encoding="utf-16"?><RadioInfo/>', 'malformed'),
        ('unsupported-encoding', b'<?xml version="1.0" encoding="iso-8859-1"?><RadioInfo/>', 'unsupported_version'),
        ('unsupported-xml-version', b'<?xml version="1.1"?><RadioInfo/>', 'unsupported_version'),
        ('source-declaration-typo', b'<?xml version="1 .0"?><AppInfo/>', 'malformed'),
        ('source-run-tag-typo', b'<contactinfo><run1run2>1<run1run2></contactinfo>', 'malformed'),
    ]:
        s.negative('error-' + name, raw, category)
    for name, raw, typ in [('future', b'<FutureRadio><Freq>1</Freq></FutureRadio>', 'FutureRadio'),
        ('case', b'<radioinfo/>', 'radioinfo'), ('namespace', b'<n:RadioInfo xmlns:n="urn:synthetic"/>', 'n:RadioInfo')]:
        s.add('skip-' + name, {'class': 'skip', 'input': {'data': b64(raw), 'context': context()},
            'expected': {'skip': {'reason': 'unknown_type', 'type': typ}}})
    # A known/unknown root never bypasses complete isolation and bounds checks.
    s.negative('error-unknown-root-depth', b'<FutureRadio>' + b'<x>' * 16 + b'</x>' * 16 + b'</FutureRadio>', 'oversized')
    s.negative('error-unknown-root-doctype', b'<!DOCTYPE FutureRadio><FutureRadio/>', 'malformed')
    s.negative('error-namespace-attribute-overflow', ('<RadioInfo ' + ' '.join(f'xmlns:n{i}="urn:synthetic:{i}"' for i in range(17)) + '/>').encode(), 'oversized')
    s.negative('error-utf16-doctype', ('<?xml version="1.0" encoding="utf-16"?><!DOCTYPE RadioInfo><RadioInfo/>').encode('utf-16'), 'malformed')
    s.negative('error-invalid-attribute-text', b'<RadioInfo label="\xff"/>', 'malformed')
    s.add('skip-default-namespace', {'class': 'skip',
        'input': {'data': b64(b'<RadioInfo xmlns="urn:synthetic"/>'), 'context': context()},
        'expected': {'skip': {'reason': 'unknown_type', 'type': 'RadioInfo'}}})
    # Namespace-bound children do not match an unqualified mapped leaf.
    positive('xml-namespaced-child', b'<RadioInfo xmlns:n="urn:synthetic"><n:StationName>synthetic</n:StationName></RadioInfo>', 'radio_state', {})
    positive('invalid-utf8-view', b'<RadioInfo><StationName>A\xffB</StationName></RadioInfo>',
        'radio_state', {'radio': {'instance': {'utf8_bytes': [65, 255, 66]}}})
    s.vectors['invalid-utf8-view']['expected_message'] = {'root': 'RadioInfo',
        'xml': {'utf8_bytes': list(b'<RadioInfo><StationName>A\xffB</StationName></RadioInfo>')}}
    positive('xml-attribute-normalization', b'<dynamicresults><breakdown><qso band="total" mode="A&#76;L">17</qso></breakdown></dynamicresults>', 'score', {'qso_count': 17})
    s.negative('error-future-xml-declaration', b'<?xml version="1.9"?><RadioInfo/>', 'unsupported_version')
    n1mm_review_vectors(s, positive)
    n1mm_commands(s)
    n1mm_compatibility(s)
    return s


def n1mm_review_vectors(s, positive):
    """Independently authored review reproductions, not another catalog matrix."""
    positive('review-unknown-numeric-path', b'<AppInfo><Freq>' + b'9' * 129 + b'</Freq></AppInfo>', 'native', {})
    positive('review-long-namespace-uri', ('<RadioInfo xmlns:n="urn:' + 'u' * 200 + '"><n:x/></RadioInfo>').encode(), 'radio_state', {})
    positive('review-invalid-utf8-budget', b'<RadioInfo><StationName>' + b'\xff' * 22000 + b'</StationName></RadioInfo>',
             'radio_state', {'radio': {'instance': {'utf8_bytes': [255] * 22000}}})
    positive('review-invalid-utf8-cdata', b'<RadioInfo><StationName><![CDATA[A\xffB]]></StationName></RadioInfo>',
             'radio_state', {'radio': {'instance': {'utf8_bytes': [65, 255, 66]}}})
    positive('review-invalid-text-interactions',
             b'<RadioInfo><StationName>A&amp;\xff<![CDATA[<\xfe\r\n]]>Z\r&#13;</StationName></RadioInfo>',
             'radio_state', {'radio': {'instance': {'utf8_bytes': [65, 38, 255, 60, 254, 10, 90, 10, 13]}}})
    positive('review-invalid-utf16-interactions',
             ('<RadioInfo><StationName>A&amp;\ud800<![CDATA[<\udc00\r\n]]>Z\r&#13;</StationName></RadioInfo>').encode('utf-16', 'surrogatepass'),
             'radio_state', {'radio': {'instance': {'utf16_units': [65, 38, 55296, 60, 56320, 10, 90, 10, 13]}}})
    positive('review-invalid-mixed-content', b'<RadioInfo>\xff<Mode>CW</Mode>\xfe<x/>\xfd</RadioInfo>', 'radio_state', {'mode': 'CW'})
    positive('review-invalid-nested-text', b'<RadioInfo><StationName>\xff<x/>\xfe</StationName></RadioInfo>', 'radio_state', {})
    positive('review-default-namespace-child', b'<RadioInfo><Mode xmlns="urn:synthetic">CW</Mode><StationName>SYNTHETIC</StationName></RadioInfo>',
             'radio_state', {'radio': {'instance': 'SYNTHETIC'}})
    positive('review-default-namespace-reset', b'<RadioInfo><x xmlns="urn:synthetic"><Mode xmlns="">CW</Mode></x><Mode>USB</Mode></RadioInfo>',
             'radio_state', {'mode': 'USB'})
    positive('review-declaration-whitespace', b'<?xml version = \'1.0\' encoding\t=\n"UTF-8" standalone = "yes"?><RadioInfo/>', 'radio_state', {})
    # U+0800 expands from two UTF-16 bytes to three UTF-8 bytes. Payloads stay
    # below the datagram bound while processed text reaches 65,507/65,508.
    positive('review-utf16-decoded-text-boundary',
             ('<RadioInfo><StationName>' + '\u0800' * 21835 + 'ab</StationName></RadioInfo>').encode('utf-16'),
             'radio_state', {'radio': {'instance': '\u0800' * 21835 + 'ab'}})
    positive('review-utf16-name-boundary', ('<RadioInfo><' + 'n' * 64 + '/></RadioInfo>').encode('utf-16'), 'radio_state', {})
    positive('review-native-only-numeric', b'<Spectrum><DataCount>' + b'9' * 129 + b'</DataCount><ScalingFactor>' + b'1' * 129 + b'</ScalingFactor></Spectrum>',
             'control', {'operation': 'n1mm.spectrum', 'scaling_factor': '1' * 129})
    positive('review-duplicate-numeric-before-limit', b'<RadioInfo><Freq>' + b'9' * 129 + b'</Freq><Freq>1</Freq></RadioInfo>', 'radio_state', {})
    positive('review-nested-numeric-before-limit', b'<RadioInfo><Freq>' + b'9' * 129 + b'<x/></Freq></RadioInfo>', 'radio_state', {})
    positive('review-invalid-numeric-before-limit', b'<RadioInfo><Freq>' + b'9' * 129 + b'\xff</Freq></RadioInfo>', 'radio_state', {})
    positive('review-score-nonselected-numeric', b'<dynamicresults><breakdown><qso band="20" mode="CW">' + b'9' * 129 + b'</qso></breakdown></dynamicresults>', 'score', {})
    positive('review-score-duplicate-before-limit', b'<dynamicresults><breakdown><qso band="total" mode="ALL">' + b'9' * 129 + b'</qso><qso band="total" mode="ALL">1</qso></breakdown></dynamicresults>', 'score', {})
    positive('review-score-container-before-limit', b'<dynamicresults><breakdown><point band="total" mode="ALL">' + b'9' * 129 + b'</point></breakdown><breakdown/></dynamicresults>', 'score', {})
    # Numeric references must never acquire invalid-unit provenance. These three
    # collision cases independently exercise leaf preservation, attributes and
    # the decoded-text budget (one invalid UTF-16 unit counts as two bytes).
    positive('review-invalid-unit-reference-leaf', b'<RadioInfo><Mode>\xff&#xF0000;</Mode></RadioInfo>',
             'radio_state', {'mode': {'utf8_bytes': [255, 243, 176, 128, 128]}})
    positive('review-invalid-unit-reference-attribute', b'<RadioInfo a="&#xF0000;"><Mode>\xff</Mode></RadioInfo>',
             'radio_state', {'mode': {'utf8_bytes': [255]}})
    positive('review-invalid-unit-reference-budget-boundary',
             ('<RadioInfo><Mode>\ud800' + '\u0800' * 21833 + 'ab&#xF0000;</Mode></RadioInfo>').encode('utf-16', 'surrogatepass'),
             'radio_state', {'mode': {'utf16_units': [55296] + [2048] * 21833 + [97, 98, 56192, 56320]}})
    # Expat can hold trailing ']' across input chunks, including inside CDATA.
    # Both a literal and a referenced U+FFFD remain valid beside an invalid unit.
    positive('review-invalid-unit-chunk-boundary', b'<RadioInfo><Mode>]\xff<![CDATA[]]\xfe]]>\xef\xbf\xbd&#xFFFD;</Mode></RadioInfo>',
             'radio_state', {'mode': {'utf8_bytes': [93, 255, 93, 93, 254, 239, 191, 189, 239, 191, 189]}})
    positive('review-pi-target-boundary', ('<RadioInfo><?' + 'n' * 128 + '?></RadioInfo>').encode(),
             'radio_state', {})
    for name, raw, category in [
        ('invalid-unit-reference-budget-overflow', ('<RadioInfo><Mode>\ud800' + '\u0800' * 21834 + '&#xF0000;</Mode></RadioInfo>').encode('utf-16', 'surrogatepass'), 'oversized'),
        ('invalid-unit-unfinished-attribute', b'<RadioInfo a="\xff', 'truncated'),
        ('nul-before-eof', b'<RadioInfo>\x00', 'malformed'),
        ('pi-target-overflow', ('<RadioInfo><?' + 'n' * 129 + '?></RadioInfo>').encode(), 'oversized'),
        ('declaration-eq-encoding', b'<?xml version="1.0" encoding = "UTF-16"?><RadioInfo/>', 'malformed'),
        ('declaration-eq-version', b'<?xml version = "1.1"?><RadioInfo/>', 'unsupported_version'),
        ('namespace-lexical-name', ('<RadioInfo xmlns:' + 'n' * 129 + '="urn:synthetic"/>').encode(), 'oversized'),
        ('utf16-decoded-text-overflow', ('<RadioInfo><StationName>' + '\u0800' * 21835 + 'abc</StationName></RadioInfo>').encode('utf-16'), 'oversized'),
        ('utf16-name-overflow', ('<RadioInfo><' + 'n' * 65 + '/></RadioInfo>').encode('utf-16'), 'oversized'),
        ('invalid-comment', b'<RadioInfo><!--\xff--><Mode>CW</Mode></RadioInfo>', 'malformed'),
        ('invalid-pi', b'<RadioInfo><?synthetic \xff?></RadioInfo>', 'malformed'),
        ('invalid-outside-root', b'<RadioInfo/>\xff', 'malformed'),
        ('declaration-syntax-before-version', b'<?xml version="1.1" standalone="maybe"?><!DOCTYPE RadioInfo><RadioInfo/>', 'malformed'),
        ('version-before-isolation', b'<?xml version = "1.1"?><!DOCTYPE RadioInfo><RadioInfo/>', 'unsupported_version'),
        ('version-before-encoding', b'<?xml version="1.1" encoding="UTF-16"?><!DOCTYPE RadioInfo><RadioInfo/>', 'unsupported_version'),
        ('encoding-support-before-isolation', b'<?xml version="1.0" encoding = "latin1"?><!DOCTYPE RadioInfo><RadioInfo/>', 'unsupported_version'),
        ('encoding-conflict-before-isolation', b'<?xml version="1.0" encoding = "UTF-16"?><!DOCTYPE RadioInfo><RadioInfo/>', 'malformed'),
        ('isolation-before-resource', b'<RadioInfo>' + b'<x/>' * 1024 + b'</RadioInfo><!DOCTYPE x>', 'malformed'),
        ('resource-before-later-malformed', b'<RadioInfo>' + b'<x/>' * 1024 + b'</wrong>', 'oversized'),
        ('resource-before-later-truncated', b'<RadioInfo>' + b'<x/>' * 1024 + b'<', 'oversized'),
        ('construct-before-resource', ('<RadioInfo ' + ' '.join(f'a{i}=""' for i in range(17)) + ' a0=""/>').encode(), 'malformed'),
        ('structure-before-numeric', b'<RadioInfo><Freq>' + b'9' * 129 + b'</Freq></wrong>', 'malformed'),
        ('score-selected-numeric', b'<dynamicresults><breakdown><qso band="total" mode="ALL">' + b'9' * 129 + b'</qso></breakdown></dynamicresults>', 'oversized'),
    ]:
        s.negative('review-error-' + name, raw, category)


def n1mm_commands(s):
    def event(fields):
        return {'spec': SPEC, 'family': 'n1mm', 'mapping': 1, 'kind': 'control',
                'context': context('to_application'), 'fields': fields}

    qsy = {'operation': 'set_frequency', 'app': 'synthetic-client', 'radio': {'number': 2}, 'frequency_hz': 14053284}
    spectrum = {'operation': 'n1mm.spectrum', 'app': 'synthetic-client', 'spectrum_name': 'synthetic-spectrum',
        'low_scope_frequency': 14000000, 'high_scope_frequency': 14055000, 'scaling_factor': '0.3125',
        'spectrum_data': [0, 1, 160, 65535]}
    for nr, hz, native in [(1, 14053284, '14053.284'), (2, 14053284, '14053.284'),
        (1, 7000000, '7000'), (2, 1, '0.001'), (1, 0, '0'),
        (2, 9007199254740991, '9007199254740.991'), (1, 475000, '475'), (2, 137500, '137.5'),
        (1, 7061200, '7061.2'), (2, 7061020, '7061.02')]:
        command = event(dict(qsy, radio={'number': nr}, frequency_hz=hz))
        nr_text = '1' if nr == 1 else '2'
        raw = (XML_DECL + f'<radio_setfrequency><app>synthetic-client</app><radionr>{nr_text}</radionr><frequency>{native}</frequency><mousebutton>{"Left" if nr == 1 else "Right"}</mousebutton></radio_setfrequency>').encode()
        s.add(f'command-qsy-{nr}-{hz}', {'class': 'command', 'input': {'event': command},
              'expected': {'data': b64(raw)}})
    for label, name, escaped in [('normal', 'synthetic-spectrum', 'synthetic-spectrum'),
        ('xml-text', 'A&B<C>\r"\t\n', 'A&amp;B&lt;C&gt;&#13;"\t\n'), ('empty', '', '')]:
        command = event(dict(spectrum, spectrum_name=name))
        raw = (XML_DECL + '<Spectrum><app>synthetic-client</app><Name>' + escaped + '</Name><LowScopeFrequency>14000</LowScopeFrequency><HighScopeFrequency>14055</HighScopeFrequency><ScalingFactor>0.3125</ScalingFactor><DataCount>4</DataCount><SpectrumData>0,1,160,65535</SpectrumData></Spectrum>').encode()
        s.add('command-spectrum-' + label, {'class': 'command', 'input': {'event': command}, 'expected': {'data': b64(raw)}})
    for label, low, high, low_text, high_text in [
        ('minimum', 0, 1, '0', '0.001'),
        ('maximum', 9007199254740990, 9007199254740991, '9007199254740.99', '9007199254740.991'),
        ('fractional', 14053284, 14053285, '14053.284', '14053.285'),
        ('630m', 472000, 475000, '472', '475'), ('2200m', 135700, 137500, '135.7', '137.5')]:
        command = event(dict(spectrum, low_scope_frequency=low, high_scope_frequency=high))
        raw = (XML_DECL + f'<Spectrum><app>synthetic-client</app><Name>synthetic-spectrum</Name><LowScopeFrequency>{low_text}</LowScopeFrequency><HighScopeFrequency>{high_text}</HighScopeFrequency><ScalingFactor>0.3125</ScalingFactor><DataCount>4</DataCount><SpectrumData>0,1,160,65535</SpectrumData></Spectrum>').encode()
        s.add('command-spectrum-' + label, {'class': 'command', 'input': {'event': command}, 'expected': {'data': b64(raw)}})

    for label, levels, count_text, data_text, scale in [
        ('one', [0], '1', '0', '0.3125'),
        ('4096', [0, 65535, 160, 1] * 1024, '4096', '0,65535,160,1,' * 1023 + '0,65535,160,1', '0.3125'),
        ('scale-lexical', [0, 1, 160, 65535], '4', '0,1,160,65535', '000.5000'),
    ]:
        command = event(dict(spectrum, spectrum_data=levels, scaling_factor=scale))
        raw = (XML_DECL + '<Spectrum><app>synthetic-client</app><Name>synthetic-spectrum</Name>'
               '<LowScopeFrequency>14000</LowScopeFrequency><HighScopeFrequency>14055</HighScopeFrequency>'
               '<ScalingFactor>' + scale + '</ScalingFactor><DataCount>' + count_text + '</DataCount>'
               '<SpectrumData>' + data_text + '</SpectrumData></Spectrum>').encode()
        s.add('command-spectrum-' + label, {'class': 'command', 'input': {'event': command},
              'expected': {'data': b64(raw)}})

    def negative(label, fields, options=None, category='invalid_value', mutate=None):
        cmd = event(copy.deepcopy(fields))
        if mutate:
            mutate(cmd)
        inp = {'event': cmd}
        if options is not None:
            inp['options'] = options
        s.add('command-error-' + label, {'class': 'command_negative', 'input': inp,
              'expected': {'error': {'category': category}}})
    for label, baseline, opts in [('qsy', qsy, {}), ('spectrum', spectrum, {})]:
        for key in baseline:
            missing = copy.deepcopy(baseline)
            missing.pop(key)
            negative(label + '-missing-' + key, missing, opts)
    for label, changes in [('negative-frequency', {'frequency_hz': -1}), ('frequency-overflow', {'frequency_hz': 9007199254740992}),
        ('boolean-frequency', {'frequency_hz': True}), ('fractional-frequency', {'frequency_hz': 1.5}),
        ('radio-zero', {'radio': {'number': 0}}), ('radio-three', {'radio': {'number': 3}}),
        ('radio-bool', {'radio': {'number': True}}), ('radio-string', {'radio': {'number': '2'}}),
        ('radio-instance', {'radio': {'number': 2, 'instance': 'synthetic'}}), ('offset', {'rx_offset_hz': 100}),
        ('null-app', {'app': None}), ('raw-app', {'app': {'utf8_bytes': [255]}}), ('nul-app', {'app': '\x00'}),
        ('surrogate-app', {'app': '\ud800'}), ('unknown-operation', {'operation': 'n1mm.unknown'})]:
        negative(label, dict(qsy, **changes))
    for label, opts in [('schema-option', {'schema': 1}), ('frequency-unit-option', {'frequency_unit': 'khz'})]:
        negative(label, qsy, opts)
    for label, changes in [('empty-levels', {'spectrum_data': []}), ('negative-level', {'spectrum_data': [-1]}),
        ('level-overflow', {'spectrum_data': [65536]}), ('level-bool', {'spectrum_data': [True]}),
        ('too-many-levels', {'spectrum_data': [1] * 4097}), ('zero-scale', {'scaling_factor': '0'}),
        ('reversed-endpoints', {'high_scope_frequency': 1}), ('negative-endpoint', {'low_scope_frequency': -1}),
        ('scale-sign', {'scaling_factor': '+1'}), ('scale-bare-point', {'scaling_factor': '1.'}),
        ('scale-unicode-digit', {'scaling_factor': '\u0661'}), ('exponent', {'scaling_factor': '1e3'}), ('decimal-whitespace', {'scaling_factor': ' 1'}),
        ('endpoint-text', {'low_scope_frequency': '14000'})]:
        negative(label, dict(spectrum, **changes))
    for key in ('low_scope_frequency', 'high_scope_frequency'):
        for label, value in [('underflow', -1), ('overflow', 9007199254740992),
                ('boolean', True), ('fractional', 1.5)]:
            negative(key + '-' + label, dict(spectrum, **{key: value}))
    negative('equal-endpoints', dict(spectrum, high_scope_frequency=14000000))
    negative('numeric-token-bound', dict(spectrum, scaling_factor='1' * 129), category='oversized')
    negative('datagram-bound', dict(spectrum, spectrum_name='a' * 65507), category='oversized')
    negative('revision', qsy, category='unsupported_version', mutate=lambda e: e.update(mapping=2))
    negative('native-version', qsy, mutate=lambda e: e.update(native_version='1'))
    negative('wire-id', qsy, mutate=lambda e: e.update(wire_id='synthetic'))


def n1mm_compatibility(s):
    base = s.vectors['RadioInfo-full']['expected']['event']
    for label, writer, reader, mutate, kind, verification in [
        ('new-reader-old-writer', SPEC, '0.2.0', lambda e: None, 'radio_state', 'verified'),
        ('newer-program-label', SPEC, SPEC, lambda e: e['native'].update(data=b64(base64.b64decode(e['native']['data']).replace(b'synthetic-logger', b'synthetic-newer-program'))), 'radio_state', 'verified'),
        ('old-reader-new-member', '0.2.0', SPEC, lambda e: e.update(future_member='synthetic'), 'radio_state', 'verified'),
        ('known-root-new-revision', SPEC, SPEC, lambda e: e.update(mapping=2), 'radio_state', 'verified'),
        ('unknown-root-known-kind', SPEC, SPEC, lambda e: (e.update(mapping=2), e['native'].update(data=b64(b'<FutureRadio/>'))), 'native', 'unavailable'),
        ('unknown-kind', '0.2.0', SPEC, lambda e: (e.update(mapping=2, kind='future_kind'), e['native'].update(data=b64(b'<FutureRadio/>'))), 'native', 'unavailable'),
    ]:
        event = copy.deepcopy(base)
        event['spec'] = writer
        mutate(event)
        read = {k: copy.deepcopy(event[k]) for k in ('kind', 'fields', 'context', 'native')}
        if label == 'unknown-kind':
            read.update(kind='native', fields={})
        s.add('compat-' + label, {'class': 'compatibility', 'writer_spec': writer, 'reader_spec': reader,
            'input': {'event': event}, 'expected': {'read': read, 'verification': verification}})
    cmd = copy.deepcopy(s.vectors['command-qsy-2-14053284']['input']['event'])
    for label, event, reader in [('new-reader-old-command', cmd, '0.2.0'),
            ('old-reader-new-command', dict(cmd, spec='0.2.0', mapping=2), SPEC)]:
        s.add('compat-' + label, {'class': 'compatibility', 'writer_spec': event['spec'], 'reader_spec': reader,
            'input': {'event': event}, 'expected': {'read': {k: copy.deepcopy(event[k]) for k in ('kind', 'fields', 'context')},
                                                  'preserved_command': copy.deepcopy(event)}})
    event = copy.deepcopy(base)
    event['spec'] = '1.0.0'
    s.add('compat-major', {'class': 'compatibility', 'writer_spec': '1.0.0', 'reader_spec': SPEC,
        'input': {'event': event}, 'expected': {'error': {'category': 'unsupported_version'}}})
    event = copy.deepcopy(base)
    event['fields']['focus_radio'] = 2
    s.add('event-native-disagreement', {'class': 'negative', 'input': {'event': event},
        'expected': {'error': {'category': 'invalid_value'}}})


def check_n1mm_xml(raw, *, body_limit=65507, element_limit=1024, numeric_catalog=True):
    """Independent Annex D XML acceptance oracle; no codec imports or resolution.

    Expat validates XML grammar/namespaces. Invalid units travel in separate
    input chunks with byte-position provenance, never in a character lookup.
    The returned small tree is solely for independent text/mapping checks.
    """
    import re
    from xml.parsers import expat
    if len(raw) > body_limit:
        raise ValueError('oversized')
    if not raw:
        raise ValueError('truncated')
    enc, bom = 'utf-8', 0
    if raw.startswith(b'\xff\xfe'):
        enc, bom = 'utf-16-le', 2
    elif raw.startswith(b'\xfe\xff'):
        enc, bom = 'utf-16-be', 2
    elif raw.startswith(b'\xef\xbb\xbf'):
        bom = 3
    elif raw.startswith(b'<\x00?\x00'):
        enc = 'utf-16-le'
    elif raw.startswith(b'\x00<\x00?'):
        enc = 'utf-16-be'
    if enc.startswith('utf-16') and (len(raw) - bom) % 2:
        raise ValueError('truncated')
    content = raw[bom:].decode(enc, 'surrogatepass' if enc.startswith('utf-16') else 'surrogateescape')
    # Parse the *whole* declaration with an XML parser before interpreting it.
    # This handles XML Eq/S/quote grammar and pseudo-attribute order, unlike a
    # prefix regex. A probe root avoids accepting a declaration truncated at EOF.
    masked = content
    if content.startswith('<?xml') and (len(content) == 5 or content[5] in ' \t\r\n?'):
        boundary = content.find('?>')
        if boundary < 0:
            raise ValueError('malformed')
        declaration = content[:boundary + 2]
        decl_parser = expat.ParserCreate()
        declared = []
        decl_parser.XmlDeclHandler = lambda version, label, standalone: declared.append((version, label))
        try:
            decl_parser.Parse(declaration + '<probe/>', True)
        except (expat.ExpatError, UnicodeError, LookupError) as error:
            raise ValueError('malformed') from error
        if not declared:
            raise ValueError('malformed')
        version, label = declared[0]
        if version != '1.0':
            raise ValueError('unsupported_version')
        if label:
            label = label.lower()
            if label not in ('utf-8', 'utf-16', 'utf-16le', 'utf-16be'):
                raise ValueError('unsupported_version')
            compatible = {'utf-8': ('utf-8',), 'utf-16-le': ('utf-16', 'utf-16le'),
                          'utf-16-be': ('utf-16', 'utf-16be')}
            if label not in compatible[enc]:
                raise ValueError('malformed')
        # The structure parser receives Unicode, independently of source encoding.
        masked = XML_DECL + content[boundary + 2:]
    # Comments, CDATA and PI contents cannot introduce declarations. Tokenizing
    # those constructs also prevents a quote or '>' in them from confusing this
    # isolation scan. It performs no expansion and never invokes a resolver.
    isolation_tokens = re.finditer(r'<!--[\s\S]*?(?:-->|\Z)|<!\[CDATA\[[\s\S]*?(?:\]\]>|\Z)|'
                                  r'<\?[\s\S]*?(?:\?>|\Z)|<![^>]*(?:>|\Z)', masked)
    for token in isolation_tokens:
        if re.match(r'<!\s*(?:DOCTYPE|ENTITY)\b', token[0]):
            raise ValueError('malformed')
    def valid(value):
        if invalid_markup or any(0xd800 <= ord(c) <= 0xdfff for c in value):
            raise ValueError('malformed')

    def lexical(name):
        parts = name.split('|')
        return parts[2] + ':' + parts[1] if len(parts) == 3 else parts[-1]

    def name_length(name):
        return len(name.encode(enc))

    parser = expat.ParserCreate(namespace_separator='|')
    parser.buffer_text = False
    parser.namespace_prefixes = True
    parser.ordered_attributes = True
    depth = count = attrs = expanded = 0
    stack, pending_namespaces = [], []
    root = None
    invalid = None
    invalid_seen = False
    invalid_markup = False

    def add_text(value):
        nonlocal expanded
        for c in value:
            expanded += (2 if enc.startswith('utf-16') else 1) if 0xd800 <= ord(c) <= 0xdfff else len(c.encode('utf-8'))
            if expanded > body_limit:
                raise ValueError('oversized')

    def start(name, attributes):
        nonlocal depth, count, attrs, root
        valid(name)
        for item in attributes:
            valid(item)
        for prefix, uri in pending_namespaces:
            valid(prefix)
            valid(uri)
        depth += 1
        count += 1
        per_element = len(attributes) // 2 + len(pending_namespaces)
        attrs += per_element
        if depth > 16 or count > element_limit or per_element > 16 or attrs > 256:
            raise ValueError('oversized')
        names = [lexical(name)] + [lexical(a) for a in attributes[::2]]
        names += ['xmlns' + (':' + prefix if prefix else '') for prefix, _ in pending_namespaces]
        if any(name_length(n) > 128 for n in names):
            raise ValueError('oversized')
        for value in attributes[1::2] + [uri for _, uri in pending_namespaces]:
            add_text(value)
        pending_namespaces.clear()
        node = {'name': name, 'text': '', 'children': [], 'attributes': dict(zip(attributes[::2], attributes[1::2]))}
        if stack:
            stack[-1]['children'].append(node)
        else:
            root = node
        stack.append(node)

    def end(name):
        nonlocal depth
        valid(name)
        stack.pop()
        depth -= 1

    def chars(value):
        nonlocal invalid_seen
        if invalid is not None:
            position, unit = invalid
            offset = position - parser.CurrentByteIndex
            data = value.encode('utf-8')
            if 0 <= offset < len(data):
                # A callback may include ']' held from the preceding chunk.
                # Only this source position belongs to the invalid unit; actual
                # U+FFFD text and entity expansions have different positions.
                assert data[offset:] == b'\xef\xbf\xbd'
                value = data[:offset].decode('utf-8') + unit
                invalid_seen = True
        if not stack:
            valid(value)
        add_text(value)
        if stack:
            stack[-1]['text'] += value

    def namespace(prefix, uri):
        pending_namespaces.append((prefix or '', uri or ''))

    def instruction(target, data):
        valid(target)
        valid(data)
        if name_length(target) > 128:
            raise ValueError('oversized')

    parser.StartElementHandler = start
    parser.EndElementHandler = end
    parser.CharacterDataHandler = chars
    parser.StartNamespaceDeclHandler = namespace
    parser.CommentHandler = valid
    parser.ProcessingInstructionHandler = instruction
    try:
        position = 0
        for index, part in enumerate(re.split(r'([\ud800-\udfff])', masked)):
            invalid = (position, part) if index % 2 else None
            invalid_seen = False
            data = ('\ufffd' if invalid is not None else part).encode('utf-8')
            parser.Parse(data, False)
            # Only character data may consume an invalid unit. In markup the
            # parser buffers it instead. Reject the completed construct in its
            # handler, leaving unfinished markup at EOF classified as truncated.
            if invalid is not None and not invalid_seen:
                invalid_markup = True
            position += len(data)
        invalid = None
        parser.Parse(b'', True)
    except expat.ExpatError as error:
        unfinished = {expat.errors.codes[k] for k in (expat.errors.XML_ERROR_NO_ELEMENTS,
            expat.errors.XML_ERROR_UNCLOSED_TOKEN, expat.errors.XML_ERROR_PARTIAL_CHAR,
            expat.errors.XML_ERROR_UNCLOSED_CDATA_SECTION)}
        if error.code in unfinished:
            raise ValueError('truncated') from error
        raise ValueError('malformed') from error

    def children(node, name):
        return [c for c in node['children'] if c['name'] == name]

    def eligible(nodes):
        return len(nodes) == 1 and not nodes[0]['children'] and not any(
            0xd800 <= ord(c) <= 0xdfff for c in nodes[0]['text'])

    # This literal mapping-path catalog is independent of fixture construction.
    paths = {'RadioInfo': ('Freq', 'RadioNr', 'FocusRadioNr', 'ActiveRadioNr'),
             'contactinfo': ('rxfreq', 'txfreq', 'radionr'),
             'contactreplace': ('rxfreq', 'txfreq', 'radionr'),
             'spot': ('frequency',), 'radio_setfrequency': ('radionr', 'frequency'),
             'Spectrum': ('LowScopeFrequency', 'HighScopeFrequency'), 'dynamicresults': ('score',)}
    numeric = []
    for name in paths.get(root['name'], ()) if numeric_catalog else ():
        nodes = children(root, name)
        if eligible(nodes):
            numeric.append(nodes[0]['text'])
    if numeric_catalog and root['name'] == 'dynamicresults':
        containers = children(root, 'breakdown')
        if len(containers) == 1:
            for name in ('qso', 'point'):
                selected = [c for c in children(containers[0], name)
                            if c['attributes'].get('band') == 'total' and c['attributes'].get('mode') == 'ALL']
                if eligible(selected):
                    numeric.append(selected[0]['text'])
    if any(len(value.strip(' \t\r\n')) > 128 for value in numeric):
        raise ValueError('oversized')
    return root, enc


def check_n1mm_commands(suite):
    """A small independent writer checker exercises command oracles and errors."""
    import re
    from decimal import Decimal

    def unsigned(value):
        # QGI-N1M-0043: construct ASCII digits explicitly, without Python's
        # integer string/format conventions determining the wire spelling.
        if value == 0:
            return '0'
        digits = ''
        while value:
            value, digit = divmod(value, 10)
            digits = '0123456789'[digit] + digits
        return digits

    def levels_text(levels):
        result = ''
        for index, value in enumerate(levels):
            if index:
                result += ','
            result += unsigned(value)
        return result

    def encode(inp):
        event, options = inp['event'], inp.get('options', {})
        if event['mapping'] != 1:
            raise ValueError('unsupported_version')
        if 'wire_id' in event or 'native_version' in event or 'native' in event:
            raise ValueError('invalid_value')
        f = event['fields']

        def string(key):
            val = f[key]
            if not isinstance(val, str) or any(not (ord(c) in (9, 10, 13) or 32 <= ord(c) <= 0xd7ff or 0xe000 <= ord(c) <= 0xfffd or 0x10000 <= ord(c) <= 0x10ffff) for c in val):
                raise ValueError('invalid_value')
            return val.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;').replace('\r', '&#13;')

        def decimal(key):
            val = f[key]
            if isinstance(val, str) and len(val) > 128:
                raise ValueError('oversized')
            if not isinstance(val, str) or not re.fullmatch(r'[0-9]+(?:\.[0-9]+)?', val):
                raise ValueError('invalid_value')
            return Decimal(val)

        def khz(key):
            hz = f[key]
            if type(hz) is not int or not 0 <= hz <= 9007199254740991:
                raise ValueError('invalid_value')
            q, r = divmod(hz, 1000)
            fraction = unsigned(r)
            while len(fraction) < 3:
                fraction = '0' + fraction
            return unsigned(q) + ('.' + fraction.rstrip('0') if r else '')

        try:
            if f['operation'] == 'set_frequency':
                if set(f) != {'operation', 'app', 'frequency_hz', 'radio'} or options:
                    raise ValueError('invalid_value')
                hz, radio = f['frequency_hz'], f['radio']
                if type(hz) is not int or not 0 <= hz <= 9007199254740991 or not isinstance(radio, dict) or set(radio) != {'number'} or type(radio['number']) is not int or radio['number'] not in (1, 2):
                    raise ValueError('invalid_value')
                # Integer arithmetic, independent from the expected decimal strings.
                native = khz('frequency_hz')
                nr = radio['number']
                root = 'radio_setfrequency'
                children = [('app', string('app')), ('radionr', unsigned(nr)), ('frequency', native),
                            ('mousebutton', 'Left' if nr == 1 else 'Right')]
            elif f['operation'] == 'n1mm.spectrum':
                if set(f) != {'operation', 'app', 'spectrum_name', 'low_scope_frequency', 'high_scope_frequency', 'scaling_factor', 'spectrum_data'} or options:
                    raise ValueError('invalid_value')
                low, high = khz('low_scope_frequency'), khz('high_scope_frequency')
                if f['high_scope_frequency'] <= f['low_scope_frequency'] or decimal('scaling_factor') <= 0:
                    raise ValueError('invalid_value')
                levels = f['spectrum_data']
                if not isinstance(levels, list) or not 1 <= len(levels) <= 4096 or any(type(v) is not int or not 0 <= v <= 65535 for v in levels):
                    raise ValueError('invalid_value')
                root = 'Spectrum'
                children = [('app', string('app')), ('Name', string('spectrum_name')),
                    ('LowScopeFrequency', low), ('HighScopeFrequency', high),
                    ('ScalingFactor', f['scaling_factor']), ('DataCount', unsigned(len(levels))),
                    ('SpectrumData', levels_text(levels))]
            else:
                raise ValueError('invalid_value')
        except (KeyError, TypeError) as error:
            raise ValueError('invalid_value') from error
        raw = bytearray(XML_DECL.encode('ascii'))
        raw.extend(('<' + root + '>').encode('ascii'))
        for key, value in children:
            raw.extend(('<' + key + '>').encode('ascii'))
            raw.extend(value.encode('utf-8'))
            raw.extend(('</' + key + '>').encode('ascii'))
        raw.extend(('</' + root + '>').encode('ascii'))
        raw = bytes(raw)
        check_n1mm_xml(raw)
        return raw

    for name, vector in suite.vectors.items():
        if vector['class'] not in ('command', 'command_negative'):
            continue
        try:
            actual = encode(vector['input'])
        except ValueError as error:
            assert vector['class'] == 'command_negative', (name, error)
            assert str(error) == vector['expected']['error']['category'], (name, error)
        else:
            assert vector['class'] == 'command', name + ' planted command failure accepted'
            assert b64(actual) == vector['expected']['data'], name

    # Canonical byte faults: these XML documents are semantically equivalent
    # (or acceptable observations), but each must fail command byte comparison.
    for name, old, new in [
        ('command-spectrum-empty', b'<Name></Name>', b'<Name/>'),
        ('command-qsy-2-14053284', b'<radionr>2', b'<radionr>02'),
        ('command-spectrum-normal', b'<DataCount>4', b'<DataCount>04'),
        ('command-spectrum-normal', b'>0,1,160,65535<', b'>00,1,160,65535<'),
        ('command-spectrum-normal', b'>0,1,160,65535<', b'>1,0,160,65535<'),
        ('command-spectrum-normal', b'>0,1,160,65535<', b'>0, 1,160,65535<'),
        ('command-spectrum-normal', b'>0,1,160,65535<', b'>0,1,160,65535,<'),
        ('command-spectrum-normal', b'</app><Name>', b'</app>\n<Name>'),
        ('command-spectrum-xml-text', b'"\t\n</Name>', b'"\t</Name>'),
        ('command-spectrum-scale-lexical', b'>000.5000<', b'>0.5<'),
    ]:
        vector = suite.vectors[name]
        expected = base64.b64decode(vector['expected']['data'])
        changed = expected.replace(old, new, 1)
        assert changed != expected, name + ' mutation did not change bytes'
        assert encode(vector['input']) != changed, name + ' canonical byte fault accepted'


def n1mm_text_matches(raw, fields):
    """Check processed text independently, including reversible invalid units.

    This intentionally small mapping oracle exercises namespace/path eligibility,
    mixed content, CDATA, entities and line endings, not a full event decoder.
    """
    node, enc = check_n1mm_xml(raw)
    text_paths = {'RadioInfo': {'Mode': ('mode',), 'StationName': ('radio', 'instance')},
                  'Spectrum': {'app': ('app',), 'Name': ('spectrum_name',), 'ScalingFactor': ('scaling_factor',)}}
    for tag, path in text_paths.get(node['name'], {}).items():
        leaves = [c for c in node['children'] if c['name'] == tag]
        expected = None
        if len(leaves) == 1 and not leaves[0]['children']:
            value = leaves[0]['text']
            if any(0xd800 <= ord(c) <= 0xdfff for c in value):
                if enc == 'utf-8':
                    expected = {'utf8_bytes': list(value.encode(enc, 'surrogateescape'))}
                else:
                    data = value.encode(enc, 'surrogatepass')
                    order = 'little' if enc == 'utf-16-le' else 'big'
                    expected = {'utf16_units': [int.from_bytes(data[i:i + 2], order) for i in range(0, len(data), 2)]}
            else:
                expected = value
        actual = fields
        for key in path:
            actual = actual.get(key) if isinstance(actual, dict) else None
        if actual != expected:
            return False
    return True


def check_n1mm_requirement_ids(document):
    """Keep the review's formerly unnumbered normative rules addressable."""
    import re
    definitions = re.findall(r'^\*\*`(QGI-N1M-005[1-4])` — ', document, re.M)
    assert set(definitions) == {'QGI-N1M-0051', 'QGI-N1M-0052', 'QGI-N1M-0053', 'QGI-N1M-0054'}, 'missing Annex D review requirement ID'
    assert len(definitions) == 4, 'duplicate Annex D review requirement ID'


def check_n1mm_command_contract(document, readme):
    """Check that exact-byte oracles have governing prose, and no contrary guide.

    These few lexical anchors intentionally guard the review's prose omissions;
    the byte writer and hand-authored byte oracles are checked independently.
    """
    rows = {line.split('|')[1].strip(): line for line in document.splitlines()
            if line.startswith('| `QGI-N1M-')}
    integers = rows['`QGI-N1M-0043`']
    elements = rows['`QGI-N1M-0044`']
    assert all(rule in integers for rule in ('unsigned ASCII decimal', 'without leading zeros',
        'array order', 'exactly one comma', 'no whitespace or trailing separator')), 'incomplete canonical integer/level rules'
    assert all(rule in elements for rule in ('paired start/end tags', '<Name></Name>', '<Name/>',
        'formatting newlines', 'LF supplied within text')), 'incomplete canonical element/LF rules'
    assert 'N1MM QSY requires an explicit frequency unit' not in readme, 'contradictory N1MM unit option instruction'
    assert 'N1MM takes integer Hz arguments, emits kHz, and accepts no encoding options' in readme, 'missing N1MM command unit/option instruction'


def validate_n1mm(suite, producer_strict):
    document = (ROOT / 'spec/annex-d-n1mm.md').read_text()
    readme = (ROOT / 'spec/vectors/README.md').read_text()
    check_n1mm_requirement_ids(document)
    check_n1mm_command_contract(document, readme)
    for before, after in [('unsigned ASCII decimal', 'decimal'),
                          ('paired start/end tags', 'tags'),
                          ('LF supplied within text', 'Text')]:
        try:
            check_n1mm_command_contract(document.replace(before, after), readme)
        except AssertionError:
            pass
        else:
            raise AssertionError('planted canonical prose omission accepted: ' + before)
    try:
        check_n1mm_command_contract(document, readme.replace(
            'N1MM takes integer Hz arguments, emits kHz, and accepts no encoding options',
            'N1MM QSY requires an explicit frequency unit'))
    except AssertionError:
        pass
    else:
        raise AssertionError('planted contradictory unit option instruction accepted')
    for identity in ('QGI-N1M-0051', 'QGI-N1M-0052', 'QGI-N1M-0053', 'QGI-N1M-0054'):
        try:
            check_n1mm_requirement_ids(document.replace('**`' + identity + '` — ', '**Unnumbered — '))
        except AssertionError:
            pass
        else:
            raise AssertionError(identity + ' planted unnumbered rule accepted')
    check_n1mm_commands(suite)
    for name, vector in suite.vectors.items():
        if vector['class'] not in ('positive', 'negative', 'skip') or 'data' not in vector['input']:
            continue
        raw = base64.b64decode(vector['input']['data'], validate=True)
        try:
            check_n1mm_xml(raw)
        except ValueError as error:
            assert vector['class'] == 'negative', (name, error)
            assert str(error) == vector['expected']['error']['category'], (name, error)
        else:
            assert vector['class'] != 'negative', name + ' planted XML failure accepted'
        if vector['class'] == 'positive':
            assert vector['expected']['event']['native']['data'] == vector['input']['data'] == vector['reencode']['data'], name
            assert n1mm_text_matches(raw, vector['expected']['event']['fields']), name + ' processed text disagreement'
    # Plant precisely the losses that a replacement decoder or DOM-only reader
    # can hide: replacement bytes, retained entities/CRLF, CDATA delimiters,
    # ignored invalid mixed content and a default-namespaced common leaf.
    for name, changed in [
        ('review-invalid-unit-reference-leaf', {'mode': {'utf8_bytes': [255, 255]}}),
        ('review-invalid-utf8-budget', {'radio': {'instance': '\ufffd' * 22000}}),
        ('review-invalid-utf8-cdata', {'radio': {'instance': {'utf8_bytes': list(b'<![CDATA[A\xffB]]>')}}}),
        ('review-invalid-text-interactions', {'radio': {'instance': {'utf8_bytes': list(b'A&amp;\xff<\xfe\r\nZ\r\r')}}}),
        ('review-invalid-utf16-interactions', {'radio': {'instance': {'utf16_units': [65, 38, 65533, 60, 65533, 10, 90, 10, 13]}}}),
        ('review-invalid-mixed-content', {}),
        ('review-default-namespace-child', {'mode': 'CW', 'radio': {'instance': 'SYNTHETIC'}}),
    ]:
        raw = base64.b64decode(suite.vectors[name]['input']['data'])
        assert not n1mm_text_matches(raw, changed), name + ' planted text failure accepted'
    # Exact rational arithmetic checks the separately authored frequency oracles.
    import re
    from fractions import Fraction
    from datetime import datetime
    import xml.etree.ElementTree as ET
    conversions = {'spot': [('frequency', 'frequency_hz')],
        'radio_setfrequency': [('frequency', 'frequency_hz')],
        'Spectrum': [('LowScopeFrequency', 'low_scope_frequency'), ('HighScopeFrequency', 'high_scope_frequency')]}

    def frequencies_match(root, fields):
        for tag, key in conversions.get(root.tag, []):
            leaves = root.findall(tag)
            hz = None
            if len(leaves) == 1 and not len(leaves[0]):
                text = ''.join(leaves[0].itertext()).strip(' \t\r\n')
                if re.fullmatch(r'[+-]?[0-9]+(?:\.[0-9]+)?', text):
                    exact = Fraction(text) * 1000
                    if exact.denominator == 1 and abs(exact) <= 9007199254740991:
                        hz = int(exact)
            if not (key not in fields if hz is None else fields.get(key) == hz):
                return False
        return True

    timestamps = {
        'contactinfo': [('timestamp', 'start'), ('oldtimestamp', 'previous_start')],
        'contactreplace': [('timestamp', 'start'), ('oldtimestamp', 'previous_start')],
        'contactdelete': [('timestamp', 'start')],
        'lookupinfo': [('timestamp', 'start'), ('oldtimestamp', 'previous_start')],
        'lookup': [('timestamp', 'start'), ('oldtimestamp', 'previous_start')],
        'spot': [('timestamp', 'time')], 'dynamicresults': [('timestamp', 'time')]}

    def timestamps_match(root, fields):
        for tag, key in timestamps.get(root.tag, []):
            leaves = root.findall(tag)
            instant = None
            if len(leaves) == 1 and not len(leaves[0]):
                text = leaves[0].text or ''
                if re.fullmatch(r'[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}', text):
                    parts = [int(text[a:b]) for a, b in [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)]]
                    try:
                        datetime(*parts)  # Calendar validity only; no receiver clock or local zone.
                    except ValueError:
                        pass
                    else:
                        instant = text[:10] + 'T' + text[11:] + 'Z'
            if not (key not in fields if instant is None else fields.get(key) == instant):
                return False
        return not (root.tag in timestamps and ('end' in fields or 'time_of_day' in fields))

    frequency_checks = timestamp_checks = 0
    for name, vector in suite.vectors.items():
        if vector['class'] != 'positive':
            continue
        try:
            root = ET.fromstring(base64.b64decode(vector['input']['data']))
        except ET.ParseError:
            continue  # Invalid-code-unit oracles are checked separately.
        assert frequencies_match(root, vector['expected']['event']['fields']), name
        assert timestamps_match(root, vector['expected']['event']['fields']), name
        frequency_checks += len(conversions.get(root.tag, []))
        timestamp_checks += len(timestamps.get(root.tag, []))
    # Plant unit, truncation, rounding and boundary errors against the same checker.
    for name, changed in [('khz-spot-frequency-630m', 475000000),
            ('khz-spot-frequency-2200m', 137), ('khz-spot-frequency-sub-hz', 7061200),
            ('khz-spot-frequency-overflow', 9007199254740991)]:
        vector = suite.vectors[name]
        fields = dict(vector['expected']['event']['fields'], frequency_hz=changed)
        root = ET.fromstring(base64.b64decode(vector['input']['data']))
        assert not frequencies_match(root, fields), name + ' planted frequency failure accepted'
    assert frequency_checks > 0
    # Plant omission, clock correction, bad-scalar repair, field swaps and
    # inferred endpoint failures, without consulting any implementation or clock.
    for name, changed in [
        ('time-contactinfo-timestamp-documented', {}),
        ('time-contactinfo-timestamp-documented', {'start': '2020-01-17T15:43:38Z'}),
        ('time-contactinfo-timestamp-bad-leap-day', {'start': '2023-03-01T12:00:00Z'}),
        ('time-spot-timestamp-fractional', {'time': '2020-01-17T16:43:38.125Z'}),
        ('time-dynamicresults-timestamp-zoned', {'time': '2020-01-17T16:43:38Z'}),
        ('time-contactreplace-oldtimestamp-documented', {'start': '2020-01-17T16:43:38Z'}),
        ('time-contactinfo-timestamp-documented', {'start': '2020-01-17T16:43:38Z', 'end': '2020-01-17T16:43:38Z'}),
    ]:
        root = ET.fromstring(base64.b64decode(suite.vectors[name]['input']['data']))
        assert not timestamps_match(root, changed), name + ' planted timestamp failure accepted'
    assert timestamp_checks > 0
    # Read native SO2R sentinels independently of the fixture row/oracle catalog.
    vector = suite.vectors['RadioInfo-full']
    root = ET.fromstring(base64.b64decode(vector['input']['data']))
    actual = {'focus_radio': int(root.findtext('FocusRadioNr')),
        'active_radio': int(root.findtext('ActiveRadioNr')),
        'running': root.findtext('IsRunning') in ('True', 'true', '1'),
        'stereo': root.findtext('IsStereo') in ('True', 'true', '1'),
        'transmitting': root.findtext('IsTransmitting') in ('True', 'true', '1')}
    expected = vector['expected']['event']['fields']
    assert all(actual[k] == expected[k] for k in actual)
    assert expected['radio']['number'] == int(root.findtext('RadioNr'))
    for a, b in [('focus_radio', 'active_radio'), ('running', 'transmitting'), ('stereo', 'transmitting')]:
        planted = dict(expected)
        planted[a], planted[b] = expected[b], expected[a]
        assert any(actual[k] != planted[k] for k in actual), 'undetected N1MM swap'
    disagreement = suite.vectors['event-native-disagreement']['input']['event']
    assert any(actual[k] != disagreement['fields'][k] for k in actual)
    for name, vector in suite.vectors.items():
        if name.startswith('compat-') and 'verification' in vector['expected']:
            known = ET.fromstring(base64.b64decode(vector['input']['event']['native']['data'])).tag == 'RadioInfo'
            assert vector['expected']['verification'] == ('verified' if known else 'unavailable'), name
    try:
        import jsonschema
    except ImportError:
        if producer_strict:
            raise SystemExit('--producer-strict requires jsonschema')
        return 'N1MM XML bounds/command checks passed; schema validation skipped'
    schema = json.loads((ROOT / 'spec/schema/qgi-event-0.1.0.schema.json').read_text())
    reader = jsonschema.Draft202012Validator(schema, format_checker=jsonschema.FormatChecker())
    producer = producer_validator(schema, jsonschema) if producer_strict else reader
    for name, vector in suite.vectors.items():
        if vector['class'] == 'positive':
            event = vector['expected']['event']
            reader.validate(event)
            producer.validate(event)
            assert 'native_version' not in event and 'wire_id' not in event, name
            if 'expected_message' in vector:
                event = copy.deepcopy(event)
                event['native']['message'] = vector['expected_message']
                producer.validate(event)
            assert len(json.dumps(event, ensure_ascii=False, separators=(',', ':')).encode()) <= 1048576, name
        elif vector['class'] == 'command':
            reader.validate(vector['input']['event'])
            producer.validate(vector['input']['event'])
        elif vector['class'] == 'compatibility' and vector['writer_spec'].startswith('0.'):
            reader.validate(vector['input']['event'])
    if producer_strict:
        for name, mutation in [('RadioInfo-full', lambda e: e['fields']['radio'].update(future=1)),
            ('radio_setfrequency-full', lambda e: e['fields']['radio'].update(future=1)),
            ('Spectrum-full', lambda e: e['fields'].update(future=1))]:
            event = copy.deepcopy(suite.vectors[name]['expected']['event'])
            mutation(event)
            assert reader.is_valid(event) and not producer.is_valid(event), name
        event = copy.deepcopy(suite.vectors['RadioInfo-full']['expected']['event'])
        event['native']['message'] = dict(suite.vectors['RadioInfo-full']['expected_message'], future=1)
        assert reader.is_valid(event) and not producer.is_valid(event)
        for name, key in [('contactreplace-full', 'previous_start'), ('lookupinfo-full', 'start')]:
            event = copy.deepcopy(suite.vectors[name]['expected']['event'])
            event['fields'][key] = 123
            assert not reader.is_valid(event), name + ' accepted a non-instant'
            event = copy.deepcopy(suite.vectors[name]['expected']['event'])
            event['family'] = 'wsjtx'
            assert reader.is_valid(event) and not producer.is_valid(event), name + ' leaked N1MM profile members'
        event = copy.deepcopy(suite.vectors['dynamicresults-full']['expected']['event'])
        event['fields']['time'] = '2023-02-29T12:00:00Z'
        assert not reader.is_valid(event), 'score accepted an invalid common calendar date'
    return 'N1MM requirement IDs, XML isolation/precedence/lexical bounds and processed-text wrappers, exact kHz/Hz and UTC/calendar timestamp oracles with planted failures, canonical command bytes/errors, radio swap sentinels, positive/command schemas and same-MAJOR compatibility shapes checked'



# Annex E review oracle. Fixture builders remain in gen_fldigi_vectors; these
# literal expectations and policy checks never consult its MAPPED or operation().
# Columns: method | literal operation | request keys | response kind | response key
FLDIGI_ORACLE = """fldigi.version|fldigi.version||result|text
rig.get_frequency|fldigi.rig_get_frequency||radio_state|dial_frequency_hz
main.get_frequency|fldigi.main_get_frequency||radio_state|dial_frequency_hz
rig.set_frequency|set_frequency|frequency_hz|result|frequency_hz
main.set_frequency|set_frequency|frequency_hz|result|frequency_hz
modem.get_carrier|fldigi.modem_get_carrier||radio_state|rx_offset_hz
modem.set_carrier|fldigi.modem_set_carrier|audio_offset_hz|result|
modem.get_name|fldigi.modem_get_name||radio_state|mode
modem.get_mode|fldigi.modem_get_mode||result|text
modem.get_submode|fldigi.modem_get_submode||radio_state|sub_mode
modem.set_by_name|fldigi.modem_set_by_name|mode|result|text
rig.get_mode|fldigi.rig_get_mode||result|text
rig.set_mode|fldigi.rig_set_mode|mode|result|
main.get_trx_state|fldigi.main_get_trx_state||radio_state|state
main.get_trx_status|fldigi.main_get_trx_status||radio_state|state
main.tx|fldigi.main_tx||result|
main.rx|fldigi.main_rx||result|
main.abort|halt_tx||result|
main.tune|fldigi.main_tune||result|
rx.get_data|fldigi.rx_get_data||decode|bytes
text.get_rx|fldigi.text_get_rx|rx_start rx_length|decode|bytes
text.get_rx_length|fldigi.text_get_rx_length||result|
text.add_tx|send_text|text|result|
text.clear_rx|fldigi.text_clear_rx||result|
text.clear_tx|fldigi.text_clear_tx||result|
log.get_frequency|fldigi.log_get_frequency||result|text
log.get_time_on|fldigi.log_get_time_on||result|text
log.get_time_off|fldigi.log_get_time_off||result|text
log.get_date_on|fldigi.log_get_date_on||result|text
log.get_date_off|fldigi.log_get_date_off||result|text
log.get_call|fldigi.log_get_call||radio_state|dx_call
log.get_name|fldigi.log_get_name||result|text
log.get_rst_in|fldigi.log_get_rst_in||result|text
log.get_rst_out|fldigi.log_get_rst_out||result|text
log.get_serial_number|fldigi.log_get_serial_number||result|text
log.get_serial_number_sent|fldigi.log_get_serial_number_sent||result|text
log.get_exchange|fldigi.log_get_exchange||result|text
log.get_state|fldigi.log_get_state||result|text
log.get_province|fldigi.log_get_province||result|text
log.get_country|fldigi.log_get_country||result|text
log.get_qth|fldigi.log_get_qth||result|text
log.get_band|fldigi.log_get_band||result|text
log.get_sideband|fldigi.log_get_sideband||result|text
log.get_notes|fldigi.log_get_notes||result|text
log.get_locator|fldigi.log_get_locator||radio_state|dx_grid
log.get_az|fldigi.log_get_az||result|text
log.set_call|fldigi.log_set_call|dx_call|result|
log.set_locator|fldigi.log_set_locator|dx_grid|result|
logbook.last_record|fldigi.logbook_last_record||result|text
logbook.all_records|fldigi.logbook_all_records||result|text
"""
FLDIGI_EXPECTATIONS = {p[0]: tuple(p[1:]) for p in
                       (line.split('|') for line in FLDIGI_ORACLE.splitlines())}


def fldigi_operation(method):
    if method in FLDIGI_EXPECTATIONS:
        return FLDIGI_EXPECTATIONS[method][0]
    # Unknown-method faults still use the published token rule, without builders.
    return 'fldigi.' + (method.removeprefix('fldigi.') if method.startswith('fldigi.')
                        else method.replace('.', '_'))


def fldigi_context_budget(inp):
    contexts = [inp['context']] + [hop['context'] for hop in inp.get('via', [])]
    for ctx in contexts:
        if len(json.dumps(ctx, ensure_ascii=False, separators=(',', ':'), allow_nan=False).encode()) > 4096:
            raise ValueError('oversized')
    if len(inp.get('via', [])) > 8:
        raise ValueError('oversized')


def fldigi_xmlrpc(raw, substrate):
    """Independent grammar/type oracle; retains a small tree, not an implementation import."""
    root, enc = substrate(raw, body_limit=fldigi.LIMIT, element_limit=fldigi.ELEMENT_LIMIT, numeric_catalog=False)
    values, containers = [], []
    extended = False

    def shape(node, names=None, leaf=False):
        if node['attributes'] or '|' in node['name']:
            raise ValueError('malformed')
        if leaf:
            if node['children']:
                raise ValueError('malformed')
        elif node['text'].strip(' \t\r\n'):
            raise ValueError('malformed')
        if names is not None and [c['name'] for c in node['children']] != names:
            raise ValueError('malformed')

    def container(node, name, children, maximum):
        shape(node)
        if node['name'] != name or any(c['name'] != children for c in node['children']):
            raise ValueError('malformed')
        containers.append((node, maximum))

    def value(node):
        nonlocal extended
        if node['name'] != 'value':
            raise ValueError('malformed')
        shape(node, leaf=not node['children'])
        if not node['children']:
            values.append(('string', node))
            return
        if len(node['children']) != 1:
            raise ValueError('malformed')
        child = node['children'][0]
        name = child['name']
        if '|' in name and name.split('|')[1] in ('string', 'int', 'boolean', 'double', 'nil', 'array', 'struct', 'base64', 'i4', 'dateTime.iso8601'):
            raise ValueError('malformed')
        if name not in ('int', 'i4', 'boolean', 'string', 'double', 'dateTime.iso8601',
                         'base64', 'nil', 'struct', 'array'):
            extended = True
            return
        shape(child, leaf=name not in ('struct', 'array'))
        if name == 'array':
            shape(child, ['data'])
            data = child['children'][0]
            container(data, 'data', 'value', 1024)
            for item in data['children']:
                value(item)
        elif name == 'struct':
            container(child, 'struct', 'member', 256)
            for member in child['children']:
                shape(member, ['name', 'value'])
                shape(member['children'][0], leaf=True)
                value(member['children'][1])
        values.append((name, child))

    name = root['name']
    if name not in ('methodCall', 'methodResponse'):
        return root, enc, True, None, None
    shape(root)
    if name == 'methodCall':
        names = [c['name'] for c in root['children']]
        if names not in (['methodName'], ['methodName', 'params']):
            raise ValueError('malformed')
        shape(root['children'][0], leaf=True)
        params = root['children'][1] if len(names) == 2 else None
    else:
        if len(root['children']) != 1 or root['children'][0]['name'] not in ('params', 'fault'):
            raise ValueError('malformed')
        params = root['children'][0]
    if params:
        if params['name'] == 'fault':
            shape(params, ['value'])
            value(params['children'][0])
        else:
            container(params, 'params', 'param', 32)
            if name == 'methodResponse' and len(params['children']) != 1:
                raise ValueError('malformed')
            for param in params['children']:
                shape(param, ['value'])
                value(param['children'][0])
    if params and params['name'] == 'fault':
        fault_value = params['children'][0]
        if not fault_value['children'] or fault_value['children'][0]['name'] != 'struct':
            raise ValueError('malformed')
        members = fault_value['children'][0]['children']
        for required_name, types in [('faultCode', ('int', 'i4')), ('faultString', ('string',))]:
            selected = [m['children'][1] for m in members if m['children'][0]['text'] == required_name]
            if len(selected) != 1:
                raise ValueError('malformed')
            node = selected[0]
            actual_type = node['children'][0]['name'] if node['children'] else 'string'
            if actual_type not in types:
                raise ValueError('malformed')
    if any(len(n['children']) > maximum for n, maximum in containers):
        raise ValueError('oversized')
    if any(len(n['text'].strip(' \t\r\n')) > 128 for t, n in values
           if t in ('int', 'i4', 'boolean', 'double')):
        raise ValueError('oversized')
    # Grammar/resource passes finish before any scalar is converted.
    for typ, node in values:
        s = node['text'].strip(' \t\r\n')
        if typ in ('int', 'i4'):
            if not re.fullmatch(r'[+-]?[0-9]+', s) or not -2**31 <= int(s) < 2**31:
                raise ValueError('invalid_value')
            result = int(s)
        elif typ == 'boolean':
            if s not in ('0', '1'):
                raise ValueError('invalid_value')
            result = s == '1'
        elif typ == 'double':
            if not re.fullmatch(r'[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?', s):
                raise ValueError('invalid_value')
            result = float(s)
            if not math.isfinite(result):
                raise ValueError('invalid_value')
        elif typ == 'dateTime.iso8601':
            from datetime import datetime
            if not re.fullmatch(r'[0-9]{8}T[0-9]{2}:[0-9]{2}:[0-9]{2}', s):
                raise ValueError('invalid_value')
            try:
                datetime(int(s[:4]), int(s[4:6]), int(s[6:8]), int(s[9:11]), int(s[12:14]), int(s[15:17]))
            except ValueError as error:
                raise ValueError('invalid_value') from error
            result = s
        elif typ == 'base64':
            token = re.sub(r'[ \t\r\n]', '', s)
            try:
                result = base64.b64decode(token, validate=True)
            except (ValueError, UnicodeError) as error:
                raise ValueError('invalid_value') from error
            if base64.b64encode(result).decode() != token:
                raise ValueError('invalid_value')
        elif typ == 'nil':
            if s:
                raise ValueError('invalid_value')
            result = None
        elif typ == 'struct':
            result = [(fldigi.text_value(m['children'][0]['text'], enc), fldigi.semantic(m['children'][1]))
                      for m in node['children']]
        elif typ == 'array':
            result = [fldigi.semantic(v) for v in node['children'][0]['children']]
        else:
            result = fldigi.text_value(node['text'], enc)
        node['semantic'] = ('int' if typ == 'i4' else typ, result, node['text'])
    fault = None
    if params and params['name'] == 'fault':
        members = params['children'][0]['children'][0]['children']
        code = next(m['children'][1] for m in members if m['children'][0]['text'] == 'faultCode')
        text = next(m['children'][1] for m in members if m['children'][0]['text'] == 'faultString')
        fault = fldigi.semantic(code)[2], fldigi.semantic(text)[1]
    return root, enc, extended, params, fault


def fldigi_observe(inp, substrate, *, include_message=True):
    """Decode evidence; build and budget the XML view only when requested."""
    raw = base64.b64decode(inp['data'])
    root, enc, extension, params, fault = fldigi_xmlrpc(raw, substrate)
    method = root['children'][0]['text'] if root['name'] == 'methodCall' else None
    if method is not None:
        fldigi.name_check(method)
    assoc = inp.get('request')
    if 'request' in inp:
        if not isinstance(assoc, dict) or set(assoc) - {'method', 'arguments'} or 'method' not in assoc:
            raise ValueError('invalid_value')
        fldigi.name_check(assoc['method'])
        if 'arguments' in assoc and (not isinstance(assoc['arguments'], list) or len(assoc['arguments']) > 32):
            raise ValueError('invalid_value')
        if method is not None and method != assoc['method']:
            raise ValueError('invalid_value')
        if root['name'] != 'methodCall':
            method = assoc['method']
    fldigi_context_budget(inp)
    kind, fields = 'native', {}
    version = None
    if fault is not None:
        kind, fields = 'result', {'status': 'error', 'native_code': fault[0], 'text': fault[1]}
        if method and re.fullmatch(r'(?:[a-z][a-z0-9.-]*\.)?[a-z][a-z0-9_]*', fldigi_operation(method)):
            fields['operation'] = fldigi_operation(method)
    elif not extension and method in FLDIGI_EXPECTATIONS:
        _, reqkeys, response_kind, key = FLDIGI_EXPECTATIONS[method]
        signature = fldigi.CATALOG[method]
        arguments = [] if params is None else [fldigi.semantic(p['children'][0]) for p in params['children']]
        request = root['name'] == 'methodCall'
        required = signature.split(':')[1] if request else signature[0]
        symbols = {'int': 'i', 'boolean': 'b', 'double': 'd', 'string': 's', 'nil': 'n', 'base64': '6',
                   'array': 'A', 'struct': 'S', 'dateTime.iso8601': 't'}
        actual = ''.join(symbols.get(v[0], '?') if v else '?' for v in arguments)
        if request and required == 'n':
            required = ''
        matched = actual == required
        if not request and method == 'text.get_rx':
            args = assoc.get('arguments') if assoc else None
            matched = matched and isinstance(args, list) and len(args) == 2 and all(
                type(v) is int and -2**31 <= v < 2**31 for v in args) and args[0] >= 0 and args[1] >= -1
        if matched:
            if request:
                kind, fields = 'control', {'operation': fldigi_operation(method)}
                for name, (_, val, _) in zip(reqkeys.split(), arguments):
                    if name == 'frequency_hz':
                        if val.is_integer() and abs(val) <= 9007199254740991:
                            fields[name] = int(val)
                    else:
                        fields[name] = val
            else:
                kind = response_kind
                typ, val, _ = arguments[0]
                fields = {'status': 'ok', 'operation': fldigi_operation(method)} if kind == 'result' else {}
                if key == 'bytes':
                    try:
                        fields['text'] = val.decode('utf-8')
                    except UnicodeDecodeError:
                        fields['text'] = {'utf8_bytes': list(val)}
                elif key == 'state':
                    states = {'TX': True, 'RX': False} if method.endswith('_state') else {'tx': True, 'tune': True, 'rx': False}
                    if isinstance(val, str) and val in states:
                        fields['transmitting'] = states[val]
                elif key:
                    if key in ('frequency_hz', 'dial_frequency_hz'):
                        if val.is_integer() and abs(val) <= 9007199254740991:
                            fields[key] = int(val)
                    else:
                        fields[key] = val
                if method == 'fldigi.version' and isinstance(val, str) and val:
                    version = val
                    fields.pop('text', None)
    event = {'spec': SPEC, 'family': 'fldigi', 'mapping': 1, 'kind': kind,
             'context': copy.deepcopy(inp['context']), 'fields': fields,
             'native': {'format': 'fldigi_xmlrpc', 'data': inp['data']}}
    if 'via' in inp:
        event['via'] = copy.deepcopy(inp['via'])
    if version is not None:
        event['native_version'] = version
    view = None
    output = copy.deepcopy(event)
    if include_message:
        parts = root['name'].split('|')
        literal = parts[2] + ':' + parts[1] if len(parts) == 3 else parts[-1]
        view = {'root': literal}
        if method is not None:
            view['method'] = method
        view['xml'] = fldigi.text_value(raw.decode(enc, 'surrogatepass' if enc.startswith('utf-16') else 'surrogateescape').lstrip('\ufeff'), enc)
        output['native']['message'] = view
    if len(json.dumps(output, ensure_ascii=False, separators=(',', ':')).encode()) > 1048576:
        raise ValueError('oversized')
    return event, view


def fldigi_write_command(event, options, substrate):
    """Independent construction checker; fixtures below supply their own XML strings."""
    if event['mapping'] != 1:
        raise ValueError('unsupported_version')
    if any(k in event for k in ('native', 'native_version', 'wire_id')) or options or event['kind'] != 'control':
        raise ValueError('invalid_value')
    fields = event['fields']
    op = fields.get('operation')
    by_operation = {row[0]: method for method, row in FLDIGI_EXPECTATIONS.items()}
    by_operation.update(set_frequency='rig.set_frequency', send_text='text.add_tx', halt_tx='main.abort')
    if op not in by_operation:
        raise ValueError('invalid_value')
    method = by_operation[op]
    keys = FLDIGI_EXPECTATIONS[method][1].split()
    allowed = {'operation', *keys}
    if op == 'set_frequency':
        allowed.add('rx_offset_hz')
    if op == 'send_text':
        allowed.add('transmit')
        keys.append('transmit')
    if set(fields) - allowed or any(k not in fields for k in keys):
        raise ValueError('invalid_value')
    values = []
    for key in keys:
        val = fields[key]
        if key == 'transmit':
            if type(val) is not bool:
                raise ValueError('invalid_value')
        elif key in ('frequency_hz', 'audio_offset_hz', 'rx_start', 'rx_length'):
            if type(val) is not int or not (0 if key.startswith('rx_') else 1) <= val <= 2147483647:
                raise ValueError('invalid_value')
            values.append(('double' if key == 'frequency_hz' else 'int', str(val) + ('.0' if key == 'frequency_hz' else '')))
        else:
            if not isinstance(val, str) or any(not (ord(c) in (9, 10) or 32 <= ord(c) <= 0xd7ff or
                   0xe000 <= ord(c) <= 0xfffd or 0x10000 <= ord(c) <= 0x10ffff) for c in val):
                raise ValueError('invalid_value')
            escaped = ''.join({'&': '&amp;', '<': '&lt;', '>': '&gt;'}.get(c, c) for c in val)
            values.append(('string', escaped))
    calls = [(method, values)]
    if op == 'send_text' and fields['transmit']:
        calls.append(('main.tx', []))
    if op == 'set_frequency' and 'rx_offset_hz' in fields:
        offset = fields['rx_offset_hz']
        if type(offset) is not int or not 1 <= offset <= 2147483647:
            raise ValueError('invalid_value')
        calls.append(('modem.set_carrier', [('int', str(offset))]))
    result = []
    for name, params in calls:
        pieces = [fldigi.DECL, '<methodCall><methodName>', name, '</methodName><params>']
        for typ, content in params:
            pieces += ['<param><value><', typ, '>', content, '</', typ, '></value></param>']
        pieces.append('</params></methodCall>')
        body = ''.join(pieces).encode()
        fldigi_xmlrpc(body, substrate)
        result.append(body)
    return result



def fldigi_bundled_string(content):
    """S17 xmlDecode: one left-to-right pass; never recurse or decode numbers."""
    named = {'&lt;': '<', '&gt;': '>', '&amp;': '&', '&apos;': "'", '&quot;': '"'}
    return re.sub(r'&(?:lt|gt|amp|apos|quot);', lambda m: named[m[0]], content)


def fldigi_verify(inp, substrate):
    """Observed consistency/re-encoding, separate from original byte retrieval."""
    event = inp['event']
    if event['kind'] not in ('native', 'control', 'radio_state', 'result', 'decode'):
        return 'unavailable', None
    if 'native' not in event:
        return 'unavailable', None
    decoded_input = {'data': event['native']['data'], 'context': event['context']}
    for key in ('request', 'via'):
        if key in inp:
            decoded_input[key] = inp[key]
        elif key == 'via' and key in event:
            decoded_input[key] = event[key]
    actual, view = fldigi_observe(decoded_input, substrate, include_message='message' in event['native'])
    root, _, extension, _, fault = fldigi_xmlrpc(base64.b64decode(event['native']['data']), substrate)
    # Compare all known evidence-derived envelope members even if mapping needs
    # association. spec/mapping may be newer; context/via are caller-owned.
    for key in ('family', 'native_version', 'wire_id'):
        if (key in event) != (key in actual) or (key in event and event[key] != actual[key]):
            # Only a nonfault response needs a method to decide native_version.
            if (key == 'native_version' and root['name'] == 'methodResponse'
                    and fault is None and 'request' not in inp):
                continue
            raise ValueError('invalid_value')
    if event['native']['format'] != 'fldigi_xmlrpc':
        raise ValueError('invalid_value')
    if extension and fault is None:
        return 'unavailable', None
    if root['name'] == 'methodResponse' and fault is None:
        if 'request' not in inp:
            return 'unavailable', None
        if inp['request']['method'] == 'text.get_rx' and actual['kind'] == 'native':
            return 'unavailable', None
    # Known common members remain recognized even when absent from the expected
    # event. Unknown additive fields alone can be ignored on newer revisions.
    schema = json.loads((ROOT / 'spec/schema/qgi-event-0.1.0.schema.json').read_text())
    known_fields = set().union(*(set(v.get('properties', {})) for k, v in schema['$defs'].items()
                                if k in ('lifecycle', 'radio_state', 'result', 'control', 'decode', 'qso', 'log', 'spectrum', 'n1mm_control', 'fldigi_control')))
    fields = {k: v for k, v in event['fields'].items() if k in known_fields}
    if event['kind'] != actual['kind'] or fields != actual['fields']:
        raise ValueError('invalid_value')
    if 'message' in event['native']:
        recognized_view = {k: v for k, v in event['native']['message'].items() if k in ('root', 'method', 'xml')}
        if recognized_view != view:
            raise ValueError('invalid_value')
    return 'verified', actual['native']['data']


def fldigi_output_bound(n):
    return 4 * ((n + 2) // 3) + 12 * n + 1536 + 32768 + 65536


def fldigi_source_response(value):
    return ('<?xml version="1.0"?>\r\n<methodResponse><params><param>\r\n\t' +
            value + '\r\n</param></params></methodResponse>\r\n').encode()


def fldigi_limit_vectors(suite):
    def positive(name, raw, method, kind='native', fields=None, context=None):
        ctx = context if context is not None else fldigi.context()
        evt = {'spec': SPEC, 'family': 'fldigi', 'mapping': 1, 'kind': kind,
               'context': ctx, 'fields': fields or {},
               'native': {'format': 'fldigi_xmlrpc', 'data': b64(raw)}}
        vector = {'class': 'positive', 'input': {'data': b64(raw), 'context': ctx},
                  'expected': {'event': evt},
                  'expected_message': {'root': 'methodResponse', 'xml': fldigi.text_value(raw.decode('utf-8', 'surrogateescape'), 'utf-8')},
                  'reencode': {'data': b64(raw)}}
        if method is not None:
            vector['input']['request'] = {'method': method}
            vector['expected_message']['method'] = method
        suite.add(name, vector)
        return suite.vectors[name]
    for method, names, count, body_bytes in [
            ('modem.get_names', fldigi.SOURCE_MODE_NAMES, 169, 4013),
            ('modem.get_io_names', [fldigi.SOURCE_MODE_NAMES[i] for i in fldigi.SOURCE_IO_INDICES], 101, 2509)]:
        assert len(names) == count
        raw = fldigi_source_response('<value><array><data>' + ''.join(
            '<value>' + name + '</value>' for name in names) + '</data></array></value>')
        assert len(raw) == body_bytes
        positive(method.replace('.', '-').replace('_', '-') + '-full-source', raw, method)
    raw = fldigi_source_response('<value><struct><member><name>major</name><value><i4>4</i4></value></member>'
        '<member><name>minor</name><value><i4>2</i4></value></member>'
        '<member><name>patch</name><value>13</value></member></struct></value>')
    assert len(raw) == 299
    positive('fldigi-version-struct-full-source', raw, 'fldigi.version_struct')
    # S2 enables S12 introspection; its method map deduplicates setters.
    names = sorted(set(fldigi.SOURCE_REGISTRATIONS) | {'system.listMethods', 'system.methodHelp'}) + ['system.multicall']
    raw = fldigi_source_response('<value><array><data>' + ''.join(
        '<value>' + name + '</value>' for name in names) + '</data></array></value>')
    assert len(names) == 177 and len(raw) == 5664
    positive('system-list-methods-full-source', raw, 'system.listMethods')
    # A real multicall wraps each successful result in a one-value array.
    raw = fldigi_source_response('<value><array><data><value><array><data>'
        '<value>fldigi</value></data></array></value>'
        '<value><struct><member><name>faultCode</name><value><i4>-1</i4></value></member>'
        '<member><name>faultString</name><value>future.method: unknown method name</value></member>'
        '</struct></value></data></array></value>')
    positive('system-multicall-source-shape', raw, 'system.multicall')
    # Fixed 4,096-byte raw buffers: S6 RAW_BUFF_LEN, S18 LF every 72 characters.
    encoded = b64(b'x' * 4096)
    wrapped = '\n'.join(encoded[i:i + 72] for i in range(0, len(encoded), 72))
    raw = ('<?xml version="1.0"?>\r\n<methodResponse><params><param>\r\n\t'
           '<value><base64>' + wrapped + '</base64></value>\r\n</param></params></methodResponse>\r\n').encode()
    assert len(raw) == 5666
    for method in ('rx.get_data', 'tx.get_data', 'rxtx.get_data', 'flmsg.get_data'):
        positive(method.replace('.', '-') + '-full-buffer', raw, method,
                 'decode' if method == 'rx.get_data' else 'native',
                 {'text': 'x' * 4096} if method == 'rx.get_data' else {})
    # Exact processed-text boundary (markup leaves source body below its limit).
    for suffix, size in [('boundary', 262144), ('overflow', 262145)]:
        raw = b'\xff\xfe' + ('<x>' + '\u0800' * (size // 3) + 'x' * (size % 3) + '</x>').encode('utf-16-le')
        if suffix == 'boundary':
            v = positive('decoded-text-' + suffix, raw, None)
            v['expected_message'] = {'root': 'x', 'xml': raw[2:].decode('utf-16-le')}
        else:
            suite.vectors['decoded-text-overflow']['input']['data'] = b64(raw)
    # Invalid UTF-8 requires byte wrappers in both the common text and XML view.
    # Tune legal ASCII padding to land exactly on the actual compact JSON limit.
    size = 112000
    raw = (
        b'<methodResponse><params><param><value><string>' + b'\xff' * size +
        b'</string></value></param></params></methodResponse>')
    fields = {'status': 'ok', 'operation': 'fldigi.log_get_name', 'text': {'utf8_bytes': [255] * size}}
    v = positive('output-boundary', raw, 'log.get_name', 'result', fields)
    def output_size(vector):
        event = copy.deepcopy(vector['expected']['event'])
        event['native']['message'] = vector['expected_message']
        return len(json.dumps(event, ensure_ascii=False, separators=(',', ':')).encode())
    gap = 1048576 - output_size(v) - len(',"source_id":""')
    assert 0 <= gap <= 3000
    v['input']['context']['source_id'] = 'x' * gap
    assert output_size(v) == 1048576
    too_big = copy.deepcopy(v['input'])
    too_big['context']['source_id'] += 'x'
    suite.add('output-overflow', {'class': 'negative', 'input': too_big,
              'expected': {'error': {'category': 'oversized'}}})
    # A view is optional: this 600,143-byte event fits only when it is absent.
    raw = (b'<methodResponse><params><param><value><string>' + b'\xff' * 112400 +
           b'</string></value></param></params></methodResponse>')
    event = copy.deepcopy(v['expected']['event'])
    event['native']['data'] = b64(raw)
    event['fields']['text'] = {'utf8_bytes': [255] * 112400}
    event['context']['source_id'] = ''
    size = len(json.dumps(event, ensure_ascii=False, separators=(',', ':')).encode())
    event['context']['source_id'] = 'x' * (600143 - size)
    suite.add('compatibility-large-absent-view', {'class': 'compatibility',
        'input': {'event': event, 'request': {'method': 'log.get_name'},
                  'reader_spec': SPEC, 'writer_spec': SPEC},
        'expected': {'read': {k: event[k] for k in ('kind', 'fields', 'context', 'native')},
                     'verification': {'status': 'verified'}}})
    present = copy.deepcopy(event)
    present['native']['message'] = {'root': 'methodResponse', 'method': 'log.get_name',
                                   'xml': {'utf8_bytes': list(raw)}}
    suite.add('reencode-large-present-view', {'class': 'negative',
        'input': {'event': present, 'request': {'method': 'log.get_name'}},
        'expected': {'error': {'category': 'oversized'}}})


def fldigi_review_vectors():
    suite = fldigi.vectors(Suite, b64)
    # CR is portable-command invalid; preserve observation acceptance separately.
    v = suite.vectors['command-escaping']
    v['input']['event']['fields']['text'] = 'Q1AAA &<>\n\t" &amp; &#13;'
    v['expected']['requests'] = [b64(fldigi.call('text.add_tx',
        '<string>Q1AAA &amp;&lt;&gt;\n\t" &amp;amp; &amp;#13;</string>'))]
    for name, fields in [('cr', {'operation': 'send_text', 'text': 'Q1AAA &<>\r\n\t"', 'transmit': False}),
                         ('family-cr', {'operation': 'fldigi.rig_set_mode', 'mode': 'Q1\rAAA'})]:
        event = copy.deepcopy(v['input']['event'])
        event['fields'] = fields
        suite.add('command-bad-' + name, {'class': 'command_negative', 'input': {'event': event, 'options': {}},
                                        'expected': {'error': {'category': 'invalid_value'}}})
    def negative(name, raw, error, **extra):
        suite.add(name, {'class': 'negative', 'input': {'data': b64(raw), 'context': fldigi.context(), **extra},
                         'expected': {'error': {'category': error}}})
    fault = base64.b64decode(suite.vectors['fault-associated']['input']['data'])
    extra = b'<member><name>extra</name><value><future><int>opaque</int></future></value></member>'
    valid = fault.replace(b'</struct>', extra + b'</struct>')
    v = copy.deepcopy(suite.vectors['fault-associated'])
    v['input']['data'] = v['expected']['event']['native']['data'] = v['reencode']['data'] = b64(valid)
    v['expected_message']['xml'] = valid.decode()
    suite.add('fault-optional-future', v)
    negative('fault-optional-future-missing-code', valid.replace(b'faultCode', b'otherCode'), 'malformed')
    negative('fault-optional-future-required-type', valid.replace(b'<int> +005 </int>', b'<future/>'), 'malformed')
    negative('fault-optional-future-bad-scalar', valid.replace(b'<int> +005 </int>', b'<int>x</int>'), 'invalid_value')
    negative('fault-optional-future-shape-before-scalar', valid.replace(b'faultString', b'otherString').replace(b' +005 ', b'x'), 'malformed')
    ordinary = fldigi.response('<string>BPSK31</string>')
    negative('association-null', ordinary, 'invalid_value', request=None)
    # Exact compact byte boundaries include multibyte characters and escaping.
    for suffix, text in [('ascii', 'x'), ('unicode', 'é'), ('escaped', '"')]:
        ctx = fldigi.context()
        ctx['source_id'] = ''
        spare = 4096 - len(json.dumps(ctx, ensure_ascii=False, separators=(',', ':')).encode())
        cost = len(json.dumps(text, ensure_ascii=False).encode()) - 2
        ctx['source_id'] = text * (spare // cost) + 'x' * (spare % cost)
        v = copy.deepcopy(suite.vectors['modem-get-name-response'])
        v['input']['context'] = v['expected']['event']['context'] = ctx
        suite.add('context-boundary-' + suffix, v)
        too_big = {**ctx, 'source_id': ctx['source_id'] + 'x'}
        negative('context-overflow-' + suffix, ordinary, 'oversized', request={'method': 'modem.get_name'}, context=too_big)
    too_big = {**fldigi.context(), 'source_id': 'x' * 5000}
    for name, raw, error, request in [
            ('xml-before-context', b'<x></wrong>', 'malformed', {'method': 'modem.get_name'}),
            ('scalar-before-context', fldigi.response('<int>x</int>'), 'invalid_value', {'method': 'modem.get_name'}),
            ('association-before-context', ordinary, 'invalid_value', None),
            ('context-before-dispatch', ordinary, 'oversized', {'method': 'modem.get_name'})]:
        negative('precedence-' + name, raw, error, request=request, context=too_big)
    v = copy.deepcopy(suite.vectors['modem-get-name-response'])
    v['input']['via'] = v['expected']['event']['via'] = [{'family': 'qgi', 'context': fldigi.context()} for _ in range(8)]
    suite.add('context-eight-hops', v)
    hops = copy.deepcopy(v['input']['via'])
    hops[-1]['context'] = too_big
    negative('context-hop-overflow', ordinary, 'oversized', via=hops)
    negative('context-ninth-hop', ordinary, 'oversized', via=v['input']['via'] + [v['input']['via'][0]])
    # S2 Fldigi_list shape, all 176 registrations (two duplicates).
    # Synthetic help uses the largest released help length (99 bytes) in every row.
    entries = []
    registrations = [(name, fldigi.CATALOG[name]) for name in fldigi.SOURCE_REGISTRATIONS]
    for method, signature in registrations:
        entries.append('<value><struct>' + ''.join(
            '<member><name>' + name + '</name><value>' + text + '</value></member>'
            for name, text in sorted([('name', method), ('signature', signature), ('help', 'Synthetic method help. ' + 'x' * 76)])) + '</struct></value>')
    listing = fldigi_source_response('<value><array><data>' + ''.join(entries) + '</data></array></value>')
    positive = copy.deepcopy(suite.vectors['fldigi-list-native-response'])
    positive['input']['data'] = positive['expected']['event']['native']['data'] = positive['reencode']['data'] = b64(listing)
    positive['expected_message']['xml'] = listing.decode()
    suite.add('fldigi-list-full-source-shape', positive)
    fldigi_limit_vectors(suite)
    maximum = base64.b64decode(suite.vectors['maximum-body-with-view']['input']['data'])
    suite.vectors['body-overflow']['input']['data'] = b64(maximum + b' ')
    # Mutated observed events exercise actual rejection, not stored byte equality.
    base = suite.vectors['modem-get-name-response']['expected']['event']
    for name, mutation in [('native-version', {'native_version': '4.2.13'}),
                           ('wire-id', {'wire_id': 'Q1AAA'}), ('kind', {'kind': 'decode'}),
                           ('mapped-value', {'fields': {'mode': 'BROKEN'}}),
                           ('mapped-extra', {'fields': {'mode': 'BPSK31', 'station_call': 'Q1AAA'}})]:
        event = {**copy.deepcopy(base), **mutation}
        suite.add('reencode-bad-' + name, {'class': 'negative',
            'input': {'event': event, 'request': {'method': 'modem.get_name'}},
            'expected': {'error': {'category': 'invalid_value'}}})
    # Version query requires the member, so omission or alteration is inconsistent.
    for name, version in [('missing', None), ('altered', '99.0.0')]:
        event = copy.deepcopy(suite.vectors['fldigi-version-response']['expected']['event'])
        if version is None:
            event.pop('native_version')
        else:
            event['native_version'] = version
        suite.add('reencode-bad-version-' + name, {'class': 'negative',
            'input': {'event': event, 'request': {'method': 'fldigi.version'}},
            'expected': {'error': {'category': 'invalid_value'}}})
    event = copy.deepcopy(suite.vectors['fault-unassociated']['expected']['event'])
    event['native_version'] = '4.2.13'
    suite.add('reencode-bad-unassociated-fault-version', {'class': 'negative',
        'input': {'event': event}, 'expected': {'error': {'category': 'invalid_value'}}})
    for name, source, request, status in [
            ('opaque-type', 'value-future-type', {'method': 'future.method'}, 'unavailable'),
            ('opaque-root', 'unknown-root', None, 'unavailable'),
            ('fault-optional-future', 'fault-optional-future', {'method': 'main.abort'}, 'verified'),
            ('range-no-arguments', 'text-get-rx-response', {'method': 'text.get_rx'}, 'unavailable'),
            ('range-associated', 'text-get-rx-response', {'method': 'text.get_rx', 'arguments': [7, 4]}, 'verified'),
            ('version-no-request', 'fldigi-version-response', None, 'unavailable'),
            ('fault-no-request', 'fault-unassociated', None, 'verified')]:
        event = copy.deepcopy(suite.vectors[source]['expected']['event'])
        inp = {'event': event, 'reader_spec': SPEC, 'writer_spec': SPEC}
        if request is not None:
            inp['request'] = request
        suite.add('compatibility-' + name, {'class': 'compatibility', 'input': inp,
            'expected': {'read': {k: event[k] for k in ('kind', 'fields', 'context', 'native')},
                         'verification': {'status': status}}})
    return suite



def validate_fldigi(suite, substrate, root, producer_strict, producer_factory):
    document = (root / 'spec/annex-e-fldigi.md').read_text()
    # Every defined ID is unique, and all normative paragraphs are addressable.
    ids = re.findall(r'^\*\*`(QGI-FLD-[0-9]{4})` — ', document, re.M)
    assert len(ids) == len(set(ids)) == 69
    assert set(ids) == {f'QGI-FLD-{i:04d}' for i in range(2, 71)}
    rows = re.findall(r'^\| `([a-z][a-z0-9_.]+)` \| `([^`]+)` \|', document, re.M)
    assert dict(rows) == fldigi.CATALOG, 'catalog prose and pinned vector signatures disagree'
    anchors = ['paired `<params></params>`', 'followed by exactly `.0`', 'quotes, tabs and LF remain literal',
               '262,144 bytes', '32, 1,024 and 256', 'first failure', 'aggregate result',
               'double for rig.set_frequency, nil for', 'without synthesizing nil',
               'without translating renamed', '8,192 elements', '42,903', '3,595,096',
               '671,880', 'no finite source', '1,048,577']
    def contract(text):
        return all(a in text for a in anchors)
    assert contract(document)
    for anchor in anchors:
        assert not contract(document.replace(anchor, 'removed')), 'planted prose omission escaped'
    assert len(fldigi.CATALOG) == 174 and len(FLDIGI_EXPECTATIONS) == 50
    fldigi_review_contract(suite, document)
    assert 'pinned to released `4.2.13`' in (root / 'spec/QGI-SPEC-001.md').read_text()
    # Check the maximum integer admitted by the published bound, independently.
    bound = lambda n: 4 * ((n + 2) // 3) + 12*n + 1536 + 32768 + 65536
    assert bound(71155) == 1048576 and bound(71156) > 1048576
    assert bound(fldigi.LIMIT) == 3595096
    assert fldigi.LIMIT == 262144 and fldigi.ELEMENT_LIMIT == 8192
    assert fldigi.ARRAY_LIMIT == 1024 and fldigi.MEMBER_LIMIT == 256 and fldigi.PARAM_LIMIT == 32
    for observed, inherited, limit in [(1942, 1024, fldigi.ELEMENT_LIMIT),
                                      (177, 256, fldigi.ARRAY_LIMIT),
                                      (3, 256, fldigi.MEMBER_LIMIT),
                                      (2, 32, fldigi.PARAM_LIMIT),
                                      (42903, 71155, fldigi.LIMIT)]:
        assert limit == 1 << (max(4 * observed, inherited) - 1).bit_length()
    positives = commands = 0
    for name, vector in suite.vectors.items():
        cls, inp, expected = vector['class'], vector['input'], vector['expected']
        try:
            if cls == 'negative' and 'event' in inp:
                fldigi_verify(inp, substrate)
                raise AssertionError(name + ': inconsistent event accepted')
            elif cls in ('positive', 'negative'):
                event, view = fldigi_observe(inp, substrate)
                assert cls == 'positive', name + ': negative accepted'
                assert event == expected['event'], (name, event, expected['event'])
                assert view == vector['expected_message'], (name, view, vector['expected_message'])
                assert inp['data'] == vector['reencode']['data'] == event['native']['data']
                verify_input = {'event': event}
                if 'request' in inp:
                    verify_input['request'] = inp['request']
                status, data = fldigi_verify(verify_input, substrate)
                if status == 'verified':
                    assert data == vector['reencode']['data']
                with_view = copy.deepcopy(event)
                with_view['native']['message'] = view
                assert len(json.dumps(with_view, ensure_ascii=False, separators=(',', ':')).encode()) <= 1048576
                positives += 1
            elif cls == 'compatibility':
                event = inp['event']
                if event['spec'].split('.')[0] != SPEC.split('.')[0]:
                    assert expected['error']['category'] == 'unsupported_version'
                    continue
                known = event['kind'] in ('native', 'radio_state', 'control', 'result', 'decode')
                recognized = {'kind': event['kind'] if known else 'native',
                              'fields': {k: val for k, val in event['fields'].items() if k != 'future'} if known else {},
                              'context': event['context']}
                if 'native' in event:
                    recognized['native'] = event['native']
                assert recognized == expected['read'], name
                if 'preserved_command' in expected:
                    assert event == expected['preserved_command']
                status, _ = fldigi_verify(inp, substrate)
                assert status == expected['verification']['status'], (name, status)
            elif cls in ('command', 'command_negative'):
                bodies = fldigi_write_command(inp['event'], inp['options'], substrate)
                assert cls == 'command', name + ': negative command accepted'
                assert [base64.b64encode(b).decode() for b in bodies] == expected['requests'], name
                strings = [value for key, value in inp['event']['fields'].items()
                           if key in ('text', 'mode', 'dx_call', 'dx_grid')]
                returned = [fldigi_bundled_string(content) for body in bodies
                            for content in re.findall(r'<string>(.*?)</string>', body.decode(), re.S)]
                assert returned == strings, name + ': bundled handler argument changed'
                commands += 1
        except ValueError as error:
            assert cls in ('negative', 'command_negative'), (name, str(error))
            assert str(error) == expected['error']['category'], (name, str(error), expected)
    # Planted mapping failures distinguish RF/audio, state polarity and scalar/raw handling.
    for name, field, wrong in [('rig-get-frequency-response', 'dial_frequency_hz', 14071001),
            ('modem-get-carrier-response', 'rx_offset_hz', 1503),
            ('main-get-trx-state-response', 'transmitting', False),
            ('text-invalid-utf8', 'text', 'Q1AAA\ufffd'),
            ('modem-name-domexm', 'mode', 'DOMEX Micro'),
            ('modem-submode-slowhell', 'sub_mode', ''),
            ('text-entities', 'text', '<Q1AAA>&\n\nx'), ('rx-split-first', 'text', 'Q1\ufffd')]:
        vector = suite.vectors[name]
        actual, _ = fldigi_observe(vector['input'], substrate)
        mutant = copy.deepcopy(vector['expected']['event'])
        mutant['fields'][field] = wrong
        assert actual != mutant, name + ': planted mapping failure accepted'
    def command_matches(vector):
        actual = fldigi_write_command(vector['input']['event'], vector['input']['options'], substrate)
        return actual == [base64.b64decode(b) for b in vector['expected']['requests']]
    sequence = suite.vectors['command-text-and-transmit']
    mutant = copy.deepcopy(sequence)
    mutant['expected']['requests'].reverse()
    assert not command_matches(mutant), 'reversed request sequence accepted'
    mutant['expected']['requests'] = mutant['expected']['requests'][:1]
    assert not command_matches(mutant), 'missing request accepted'
    canonical = base64.b64decode(suite.vectors['command-empty-text-and-transmit']['expected']['requests'][0])
    for mutation in [canonical.replace(b'<string></string>', b'<string/>'), canonical + b'\n',
                     canonical.replace(b'<params>', b'<params>\n')]:
        mutant = copy.deepcopy(suite.vectors['command-empty-text-and-transmit'])
        mutant['expected']['requests'][0] = base64.b64encode(mutation).decode()
        assert not command_matches(mutant), 'canonical-byte mutant escaped'
    for name, before, after in [('command-frequency-min', b'1.0', b'01.0'),
            ('command-escaping', b'&amp;#13;', b'&#13;'),
            ('command-range-zero', b'<int>0</int>', b'<i4>0</i4>')]:
        mutant = copy.deepcopy(suite.vectors[name])
        raw = base64.b64decode(mutant['expected']['requests'][0])
        assert before in raw
        mutant['expected']['requests'][0] = base64.b64encode(raw.replace(before, after)).decode()
        assert not command_matches(mutant), 'numeric/text/type canonical mutant escaped'
    fldigi_review_plants(suite, substrate)
    # A sequence's first acknowledgment must satisfy its actual mapped type.
    # Normalized empty strings, malformed replies, faults and type swaps stop it.
    def can_advance(method, inp):
        if inp.get('request', {}).get('method') != method:
            return False
        try:
            event, _ = fldigi_observe(inp, substrate)
        except ValueError:
            return False
        return (event['kind'] == 'result' and event['fields'].get('status') == 'ok'
                and event['fields'].get('operation') == fldigi_operation(method))
    for method, accepted, stopped in [
            ('rig.set_frequency', 'rig-set-frequency-response', 'frequency-setter-nil-response'),
            ('text.add_tx', 'text-add-tx-response', 'append-double-response')]:
        assert can_advance(method, suite.vectors[accepted]['input'])
        assert not can_advance(method, suite.vectors[stopped]['input'])
        lost = copy.deepcopy(suite.vectors[accepted]['input'])
        lost.pop('request')
        assert not can_advance(method, lost)
    assert not can_advance('text.add_tx', suite.vectors['response-empty-param']['input'])
    assert not can_advance('text.add_tx', suite.vectors['bundled-invalid-result-empty-string']['input'])
    fault = copy.deepcopy(suite.vectors['fault-associated']['input'])
    fault['request']['method'] = 'text.add_tx'
    assert not can_advance('text.add_tx', fault)
    try:
        import jsonschema
    except ImportError:
        if producer_strict:
            raise SystemExit('--producer-strict requires jsonschema')
        return f'fldigi checked {positives} body/mapping/view oracles and {commands} command sequences; schema unavailable'
    schema = json.loads((root / 'spec/schema/qgi-event-0.1.0.schema.json').read_text())
    reader = jsonschema.Draft202012Validator(schema, format_checker=jsonschema.FormatChecker())
    producer = producer_factory(schema, jsonschema)
    for vector in suite.vectors.values():
        cls = vector['class']
        if cls == 'positive':
            event = vector['expected']['event']
            (producer if producer_strict else reader).validate(event)
            event = copy.deepcopy(event)
            event['native']['message'] = vector['expected_message']
            (producer if producer_strict else reader).validate(event)
        elif cls == 'command':
            (producer if producer_strict else reader).validate(vector['input']['event'])
        elif cls == 'compatibility' and vector['input']['writer_spec'].startswith('0.'):
            reader.validate(vector['input']['event'])
    # Unknown members remain reader-compatible and are recursively closed for authors.
    event = copy.deepcopy(suite.vectors['text-get-rx-request']['expected']['event'])
    event['native']['message'] = suite.vectors['text-get-rx-request']['expected_message']
    for path in [('fields',), ('native', 'message'), ('context',)]:
        mutant = copy.deepcopy(event)
        part = mutant
        for key in path:
            part = part[key]
        part['future'] = 1
        reader.validate(mutant)
        assert not producer.is_valid(mutant), 'unknown fldigi member accepted by strict producer'
    # fldigi-only arguments cannot leak to another family's strict control catalog.
    mutant = copy.deepcopy(event)
    mutant['family'] = 'wsjtx'
    mutant['native'].pop('message')
    assert not producer.is_valid(mutant)
    return f'fldigi: {positives} independent XML/type/mapping/view/reencode checks, {commands} exact command sequences; strict schema and review regressions/planted failures passed'

def fldigi_manifest(suite, dump, root):
    files = {name + '.json': dump(value) for name, value in suite.vectors.items()}
    # All evidence hashes are transcribed from E.1; no network at generation time.
    document = (root / 'spec/annex-e-fldigi.md').read_text()
    sources = re.findall(r'^\| S([0-9]+) \| \[([^\]]+)\]\(([^)]+)\) \| `([0-9a-f]{64})` \|', document, re.M)
    assert len(sources) == 18
    source_hashes = {f'fldigi-{fldigi.RELEASE}/{member}' if url == fldigi.ARCHIVE_URL else url: digest
                     for _, member, url, digest in sources}
    assert len(source_hashes) == 18, 'archive member hashes must not collapse onto one URL'
    assert f'SHA-256: `{fldigi.ARCHIVE_SHA256}`' in document and fldigi.ARCHIVE_URL in document
    files['manifest.json'] = dump({'$comment': 'SPDX-License-Identifier: GPL-3.0-or-later',
        'spec': SPEC, 'family': 'fldigi', 'mapping': 1, 'retrieved': fldigi.RETRIEVED,
        'release': fldigi.RELEASE,
        'source_archive': {'url': fldigi.ARCHIVE_URL, 'sha256': fldigi.ARCHIVE_SHA256, 'root': f'fldigi-{fldigi.RELEASE}/'},
        'release_listing': {'url': 'https://sourceforge.net/projects/fldigi/files/fldigi/',
                            'source_modified_utc': '2026-08-03 14:16:04'},
        'source_sha256': source_hashes, 'profile_sha256': hashlib.sha256(document.encode()).hexdigest(),
        'hash_scope': 'SHA-256 of each complete vector file, including its final newline',
        'vectors': [{'file': name, 'class': suite.vectors[name[:-5]]['class'], 'spec': SPEC,
                     'family': 'fldigi', 'mapping': 1, 'sha256': hashlib.sha256(data).hexdigest()}
                    for name, data in sorted(files.items())]})
    return {'fldigi/' + name: data for name, data in files.items()}

def fldigi_review_contract(suite, document):
    core = (ROOT / 'spec/QGI-SPEC-001.md').read_text()
    # Each repaired prose requirement has a planted omission, including the two
    # core clarifications. Wrapping changes do not alter these tests.
    flat = ' '.join(document.split())
    core_flat = ' '.join(core.split())
    clauses = [
        (flat, 'CR and XML-forbidden characters'),
        (flat, 'MUST NOT suppress required fault validation'),
        (core_flat, 'unless the family explicitly defines an opaque `native` fallback'),
        (flat, '4,096-byte compact UTF-8 JSON bound'),
        (flat, 'presence and value of every recognized evidence-derived envelope member'),
        (flat, 'An absent native.message MUST NOT be added or budgeted during re-encoding'),
        (flat, 'S12 executeMethod normalizes an invalid result to an empty string before S3 stringToXml serializes it as `<value></value>`'),
        (flat, '`send_text` MUST NOT advance to main.tx on this response'),
        (flat, 'its name with dots replaced by underscores'),
        (flat, 'MUST accept the complete released `fldigi.list` response'),
        (flat, 'supplied null is `invalid_value`, distinct from an absent association'),
        (core_flat, 'Annexes B, D, E and I define normative profiles and vectors'),
    ]
    for text, clause in clauses:
        assert clause in text, clause
        assert clause not in text.replace(clause, 'planted omission')
    implementation = 'Annex D has an implemented N1MM reader and command writer'
    assert implementation in core_flat
    assert implementation not in core_flat.replace(implementation, 'planted omission')
    assert 'Annexes C and G reserve scope' in core_flat
    assert 'Annexes C, E and G' not in core_flat
    # Check every mapping column, not only method/signature, against independent
    # literals. They do not use the builder's MAPPED table or operation function.
    rows = re.findall(r'^\| `([^`]+)` \| `([^`]+)` \| ([^|]+) \| ([^|]+) \| ([^|]+) \|$', document, re.M)
    mapped_rows = {m: (req.strip(), ret.strip()) for m, sig, req, ret, handler in rows}
    assert set(mapped_rows) == set(FLDIGI_EXPECTATIONS)
    for method, (op, keys, kind, key) in FLDIGI_EXPECTATIONS.items():
        request = keys or 'none'
        if op in ('set_frequency', 'send_text', 'halt_tx'):
            request += '; `' + op + '`'
        response_key = {'': 'no extra common member', 'state': 'transmitting under 0038',
                        'bytes': 'text under 0040'}.get(key, key)
        if method == 'fldigi.version':
            response_key = 'native_version under 0011'
        assert mapped_rows[method] == (request, '`' + kind + '` / ' + response_key), method
    assert 'src/xmlrpcpp/XmlRpcUtil.cpp' in document
    assert '14310e98508a7c767a0277c403264fe2db46b8440f8d55acd242afd57118f5de' in document
    listing = suite.vectors['fldigi-list-full-source-shape']
    raw = base64.b64decode(listing['input']['data'])
    assert len(raw) < fldigi.LIMIT
    # Source shape: methodResponse/params/param/value/array/data = six elements,
    # then value/struct plus three member/name/value groups = 11 per row.
    # S3 stringToXml uses untyped strings: 176 * 11 + 6 = 1942.
    import xml.etree.ElementTree as ET
    tree = ET.fromstring(raw)
    assert len(list(tree.iter())) == 1942
    assert len(tree.findall('.//array/data/value')) == 176
    registered = [member.find('value').text for member in tree.findall('.//member')
                  if member.find('name').text == 'name']
    assert hashlib.sha256('\n'.join(registered).encode()).hexdigest() == 'ce540263aa21fb50cc9e69e6688dc027680c9b5752bd3dc16182b4d21cf2484a'
    assert listing['class'] == 'positive' and listing['expected']['event']['kind'] == 'native'
    assert len(raw) == 53325
    assert all(len(m.find('value').text) == 99 for m in tree.findall('.//member')
               if m.find('name').text == 'help')
    assert 4 * ((len(raw) + 2) // 3) + 12 * len(raw) + 99840 <= 1048576
    assert fldigi_output_bound(42903) == 671880
    # Literal resource witnesses do not use the builder limit constants.
    for name, count in [('elements-boundary', 8192), ('elements-overflow', 8193)]:
        assert len(list(ET.fromstring(base64.b64decode(suite.vectors[name]['input']['data'])).iter())) == count
    for name, count in [('array-boundary', 1024), ('array-overflow', 1025)]:
        assert len(ET.fromstring(base64.b64decode(suite.vectors[name]['input']['data'])).findall('.//data/value')) == count
    for name, count in [('members-boundary', 256), ('members-overflow', 257)]:
        assert len(ET.fromstring(base64.b64decode(suite.vectors[name]['input']['data'])).findall('.//struct/member')) == count
    for name, count in [('params-boundary', 32), ('params-overflow', 33)]:
        assert len(ET.fromstring(base64.b64decode(suite.vectors[name]['input']['data'])).findall('.//params/param')) == count
    assert len(base64.b64decode(suite.vectors['maximum-body-with-view']['input']['data'])) == 262144
    assert len(base64.b64decode(suite.vectors['body-overflow']['input']['data'])) == 262145
    # Pin S12 normalization -> S3 stringToXml -> S12 response framing, rather
    # than deriving a response from TypeInvalid::toXml in isolation.
    for name in ('bundled-invalid-result-empty-string', 'bundled-no-op-bandwidth-empty-string'):
        vector = suite.vectors[name]
        assert base64.b64decode(vector['input']['data']) == fldigi_source_response('<value></value>')
        assert vector['class'] == 'positive'
        assert vector['expected']['event']['kind'] == 'native'
        assert vector['expected']['event']['fields'] == {}
    assert 'S12 wraps that in an empty param' not in document
    event = suite.vectors['compatibility-large-absent-view']['input']['event']
    assert 'message' not in event['native']
    assert len(json.dumps(event, ensure_ascii=False, separators=(',', ':')).encode()) == 600143
    present = suite.vectors['reencode-large-present-view']['input']['event']
    assert len(json.dumps(present, ensure_ascii=False, separators=(',', ':')).encode()) > 1048576


def fldigi_builder_oracle(suite, substrate):
    for method in FLDIGI_EXPECTATIONS:
        stem = method.replace('.', '-').replace('_', '-')
        for suffix in ('request', 'response'):
            vector = suite.vectors[stem + '-' + suffix]
            actual, _ = fldigi_observe(vector['input'], substrate)
            assert actual == vector['expected']['event'], (method, suffix, 'builder mapping disagrees')


def fldigi_review_plants(suite, substrate):
    # Alter the builder itself, regenerate, then check with a separate oracle.
    # A generated expected-output mutation alone would miss this class of bug.
    fldigi_builder_oracle(suite, substrate)
    original_operation = fldigi.operation
    try:
        fldigi.operation = lambda method: 'fldigi.broken' if method == 'modem.get_name' else original_operation(method)
        mutant = fldigi.vectors(Suite, b64)
        try:
            fldigi_builder_oracle(mutant, substrate)
        except AssertionError:
            pass
        else:
            raise AssertionError('regenerated operation mutation accepted')
    finally:
        fldigi.operation = original_operation
    for method, broken in [
            ('modem.get_name', ('', 'radio_state', 'sub_mode')),
            ('log.set_call', ('dx_grid', 'result', '')),
            ('modem.get_name', ('', 'result', 'mode'))]:
        original = fldigi.MAPPED[method]
        try:
            fldigi.MAPPED[method] = broken
            mutant = fldigi.vectors(Suite, b64)
            try:
                fldigi_builder_oracle(mutant, substrate)
            except AssertionError:
                pass
            else:
                raise AssertionError('regenerated field/kind mutation accepted')
        finally:
            fldigi.MAPPED[method] = original
    # Decoder source behavior is a different oracle from XML conformance.
    assert fldigi_bundled_string('&lt;&gt;&amp;&apos;&quot;') == '<>&\'"'
    assert fldigi_bundled_string('&amp;lt; &#13; &#xD;') == '&lt; &#13; &#xD;'
    escaping = suite.vectors['command-escaping']
    body = base64.b64decode(escaping['expected']['requests'][0]).decode()
    content = re.search(r'<string>(.*?)</string>', body, re.S)[1]
    assert fldigi_bundled_string(content) == escaping['input']['event']['fields']['text']
    # The previous canonical CR reference demonstrably changes the handler argument.
    cr = suite.vectors['command-bad-cr']['input']['event']['fields']['text']
    previous = 'Q1AAA &amp;&lt;&gt;&#13;\n\t"'
    assert fldigi_bundled_string(previous) != cr
    # Recognized evidence envelope cannot be smuggled through newer mappings.
    mutant = copy.deepcopy(suite.vectors['compatibility-new-mapping']['input'])
    mutant['event']['native_version'] = '4.2.13'
    try:
        fldigi_verify(mutant, substrate)
    except ValueError as error:
        assert str(error) == 'invalid_value'
    else:
        raise AssertionError('inconsistent newer-revision envelope verified')
    # Removing a real association loses method-dependent verification; it does
    # not cause an error or let a response invent a method from the event.
    associated = copy.deepcopy(suite.vectors['compatibility-range-associated']['input'])
    assert fldigi_verify(associated, substrate)[0] == 'verified'
    associated.pop('request')
    assert fldigi_verify(associated, substrate)[0] == 'unavailable'
    # An isolated opaque fallback keeps exact bytes and never verifies its type.
    for name in ('compatibility-opaque-type', 'compatibility-opaque-root'):
        assert fldigi_verify(suite.vectors[name]['input'], substrate) == ('unavailable', None)



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='check exact generated files without writing')
    parser.add_argument('--producer-strict', action='store_true',
                        help='validate authored outputs with QGI-SER-0009 producer profile')
    args = parser.parse_args()
    schema_preflight(args.producer_strict)
    suite = build()
    qgi = qgi_vectors(suite)
    validation = validate(suite, qgi, args.producer_strict)
    n1mm = n1mm_vectors()
    n1mm_validation = validate_n1mm(n1mm, args.producer_strict)
    fldigi_suite = fldigi_review_vectors()
    fldigi_validation = validate_fldigi(fldigi_suite, check_n1mm_xml, ROOT,
                                       args.producer_strict, producer_validator)
    outputs = {name + '.json': dump(vector) for name, vector in suite.vectors.items()}
    manifest = {'$comment': 'SPDX-License-Identifier: GPL-3.0-or-later', 'spec': SPEC, 'family': 'wsjtx', 'mapping': 1, 'source_sha256': PIN,
                'hash_scope': 'SHA-256 of each complete vector file, including its final newline',
                'vectors': [{'file': name, 'class': suite.vectors[name[:-5]]['class'], 'spec': SPEC,
                             'family': 'wsjtx', 'mapping': 1, 'sha256': hashlib.sha256(data).hexdigest()}
                            for name, data in sorted(outputs.items())]}
    outputs['manifest.json'] = dump(manifest)
    outputs = {'wsjtx/' + name: data for name, data in outputs.items()}
    shapes = dump(core_shapes())
    outputs['core/shape-cases.json'] = shapes
    outputs['core/manifest.json'] = dump({
        '$comment': 'SPDX-License-Identifier: GPL-3.0-or-later', 'spec': SPEC,
        'files': [{'file': 'shape-cases.json', 'sha256': hashlib.sha256(shapes).hexdigest()}]})
    qgi_outputs = {name + '.json': dump(vector) for name, vector in qgi.vectors.items()}
    qgi_outputs['manifest.json'] = dump({
        '$comment': 'SPDX-License-Identifier: GPL-3.0-or-later', 'spec': SPEC, 'family': 'qgi', 'mapping': 1,
        'source': '../../annex-i-qgi.md',
        'source_sha256': hashlib.sha256((ROOT / 'spec/annex-i-qgi.md').read_bytes()).hexdigest(),
        'hash_scope': 'SHA-256 of each complete vector file, including its final newline',
        'vectors': [{'file': name, 'class': qgi.vectors[name[:-5]]['class'], 'spec': SPEC,
                     'family': 'qgi', 'mapping': 1, 'sha256': hashlib.sha256(data).hexdigest()}
                    for name, data in sorted(qgi_outputs.items())]})
    outputs.update({'qgi/' + name: data for name, data in qgi_outputs.items()})
    n1mm_outputs = {name + '.json': dump(vector) for name, vector in n1mm.vectors.items()}
    n1mm_outputs['manifest.json'] = dump({
        '$comment': 'SPDX-License-Identifier: GPL-3.0-or-later', 'spec': SPEC, 'family': 'n1mm', 'mapping': 1,
        'retrieved': '2026-11-12', 'examples_updated': '2019-09-30', 'source_sha256': N1MM_SOURCES,
        'hash_scope': 'SHA-256 of each complete vector file, including its final newline',
        'vectors': [{'file': name, 'class': n1mm.vectors[name[:-5]]['class'], 'spec': SPEC,
                     'family': 'n1mm', 'mapping': 1, 'sha256': hashlib.sha256(data).hexdigest()}
                    for name, data in sorted(n1mm_outputs.items())]})
    outputs.update({'n1mm/' + name: data for name, data in n1mm_outputs.items()})
    outputs.update(fldigi_manifest(fldigi_suite, dump, ROOT))
    vector_root = DEST.parent
    if args.check:
        missing_or_different = [name for name, data in outputs.items()
                                if not (vector_root / name).is_file() or (vector_root / name).read_bytes() != data]
        extra = sorted(str(p.relative_to(vector_root)) for p in vector_root.rglob('*.json')
                       if str(p.relative_to(vector_root)) not in outputs)
        if missing_or_different or extra:
            print('Vector mismatch: ' + ', '.join(missing_or_different + extra), file=sys.stderr)
            return 1
    else:
        for name, data in outputs.items():
            target = vector_root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
    counts = {kind: sum(v['class'] == kind for v in suite.vectors.values())
              for kind in ['positive', 'command', 'command_negative', 'negative', 'skip', 'compatibility']}
    print(('Checked' if args.check else 'Wrote') + ' WSJT-X: ' + ', '.join(f'{n} {k}' for k, n in counts.items()))
    print('qgi: ' + ', '.join(f"{sum(v['class'] == kind for v in qgi.vectors.values())} {kind}"
                            for kind in ('positive', 'negative')))
    print('N1MM: ' + ', '.join(f"{sum(v['class'] == kind for v in n1mm.vectors.values())} {kind}"
        for kind in ('positive', 'command', 'command_negative', 'negative', 'skip', 'compatibility')))
    print('fldigi: ' + ', '.join(f"{sum(v['class'] == kind for v in fldigi_suite.vectors.values())} {kind}"
        for kind in ('positive', 'command', 'command_negative', 'negative', 'compatibility')))
    print(fldigi_validation)
    print(n1mm_validation)
    print(validation)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
