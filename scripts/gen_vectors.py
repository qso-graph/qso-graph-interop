#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate QGI 0.1.0 vectors from Annexes B and I and the pinned released header.

No implementation imports, subprocesses, third-party generation dependencies or network access.
The byte packers transcribe wire primitives; expected common fields are explicit test oracles.
"""

import argparse
import base64
import copy
import hashlib
import json
from pathlib import Path
import struct
import sys

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
        status_parts = [pack('Q', 14074000), utf8('FT8'), utf8('W1AAA'), utf8('-09'), utf8('FT4'),
                        pack('BBBII', 1, 0, 1, 1201, 2302), utf8('K1AAA'), utf8('FN31'), utf8('FN42'),
                        pack('B', 0), utf8('sub-test'), pack('BBII', 1, 3, 50, 15), utf8('test-config'), utf8('test-tx')]
        status_fields = {'dial_frequency_hz': 14074000, 'mode': 'FT8', 'dx_call': 'W1AAA', 'report': '-09',
                         'tx_mode': 'FT4', 'tx_enabled': True, 'transmitting': False, 'decoding': True,
                         'rx_offset_hz': 1201, 'tx_offset_hz': 2302, 'station_call': 'K1AAA',
                         'station_grid': 'FN31', 'dx_grid': 'FN42', 'tx_watchdog': False, 'sub_mode': 'sub-test',
                         'fast_mode': True, 'frequency_tolerance_hz': 50, 'tr_period_seconds': 15,
                         'configuration_name': 'test-config', 'tx_message': 'test-tx'}
        s.positive('01-status' + suffix, schema, 1, b''.join(status_parts), 'radio_state', status_fields,
                   message={'dial_frequency_hz': '14074000', 'mode': 'FT8', 'dx_call': 'W1AAA', 'report': '-09',
                            'tx_mode': 'FT4', 'tx_enabled': True, 'transmitting': False, 'decoding': True,
                            'rx_df_hz': 1201, 'tx_df_hz': 2302, 'de_call': 'K1AAA', 'de_grid': 'FN31',
                            'dx_grid': 'FN42', 'tx_watchdog': False, 'sub_mode': 'sub-test', 'fast_mode': True,
                            'special_operation_mode': 3, 'frequency_tolerance_hz': 50, 'tr_period_seconds': 15,
                            'configuration_name': 'test-config', 'tx_message': 'test-tx'})
        alternate = copy.deepcopy(status_parts)
        alternate[5] = pack('BBBII', 0, 1, 1, 1201, 2302)
        alternate_fields = dict(status_fields, tx_enabled=False, transmitting=True, decoding=True)
        s.positive('status-distinct-booleans' + suffix, schema, 1, b''.join(alternate),
                   'radio_state', alternate_fields)
        decode = pack('BIidI', 1, 45296123, -17, 0.25, 1234) + utf8('~') + utf8('CQ W1AAA FN42')
        decode_fields = {'is_new': True, 'time_of_day': '12:34:56.123', 'snr_db': -17, 'dt_seconds': 0.25,
                         'audio_offset_hz': 1234, 'mode': '~', 'text': 'CQ W1AAA FN42'}
        s.positive('02-decode' + suffix, schema, 2, decode + b'\x00\x01', 'decode',
                   dict(decode_fields, low_confidence=False, off_air=True),
                   message={'new': True, 'time': 45296123, 'snr_db': -17, 'delta_time_seconds': 0.25,
                            'delta_frequency_hz': 1234, 'mode': '~', 'message': 'CQ W1AAA FN42',
                            'low_confidence': False, 'off_air': True})
        s.positive('03-clear' + suffix, schema, 3, b'\x02', 'control', {'operation': 'clear', 'window': 2},
                   direction='to_application', message={'window': 2})
        reply = pack('IidI', 45296123, -17, 0.25, 1234) + utf8('~') + utf8('CQ W1AAA FN42')
        reply_fields = {'operation': 'reply', 'time_of_day': '12:34:56.123', 'snr_db': -17, 'dt_seconds': 0.25,
                        'audio_offset_hz': 1234, 'mode': '~', 'text': 'CQ W1AAA FN42'}
        s.positive('04-reply' + suffix, schema, 4, reply + b'\x01\x04', 'control',
                   dict(reply_fields, low_confidence=True, modifiers=4),
                   message={'time': 45296123, 'snr_db': -17, 'delta_time_seconds': 0.25,
                            'delta_frequency_hz': 1234, 'mode': '~', 'message': 'CQ W1AAA FN42',
                            'low_confidence': True, 'modifiers': 4})
        # Date offset suffix differs in schema 1; common UTC oracle is explicit.
        dt, dt_view = date(schema, spec=2)
        on, on_view = date(schema, milliseconds=123)
        qso_core = dt + utf8('W1AAA') + utf8('FN42') + pack('Q', 14075500)
        qso_core += b''.join(utf8(v) for v in ['FT8', '-11', '-12', '5 W', 'synthetic', 'Test'])
        qso_tail = [on] + [utf8(v) for v in ['K1BBB', 'K1AAA', 'FN31', '001', '002', 'TR']]
        qso_fields = {'end': '1970-01-01T01:00:00.123Z' if schema == 1 else '1970-01-01T00:00:00.123Z',
                      'call': 'W1AAA', 'grid': 'FN42', 'tx_frequency_hz': 14075500, 'mode': 'FT8',
                      'report_sent': '-11', 'report_received': '-12', 'tx_power': '5 W',
                      'comments': 'synthetic', 'name': 'Test'}
        qso_extra = {'start': '1970-01-01T00:00:00.123Z', 'operator_call': 'K1BBB', 'station_call': 'K1AAA',
                     'station_grid': 'FN31', 'exchange_sent': '001', 'exchange_received': '002', 'propagation_mode': 'TR'}
        qso_view = {'time_off': dt_view, 'dx_call': 'W1AAA', 'dx_grid': 'FN42', 'frequency_hz': '14075500',
                    'mode': 'FT8', 'report_sent': '-11', 'report_received': '-12', 'tx_power': '5 W',
                    'comments': 'synthetic', 'name': 'Test', 'time_on': on_view, 'operator_call': 'K1BBB',
                    'my_call': 'K1AAA', 'my_grid': 'FN31', 'exchange_sent': '001', 'exchange_received': '002',
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
        wspr = pack('BIidQi', 0, 45296123, -27, -0.5, 14095600, -2) + utf8('W1AAA') + utf8('FN42') + pack('i', 37)
        wspr_fields = {'is_new': False, 'time_of_day': '12:34:56.123', 'snr_db': -27, 'dt_seconds': -0.5,
                       'frequency_hz': 14095600, 'drift_hz': -2, 'call': 'W1AAA', 'grid': 'FN42', 'power_dbm': 37}
        s.positive('10-wspr-decode' + suffix, schema, 10, wspr + b'\x01', 'decode', dict(wspr_fields, off_air=True),
                   message={'new': False, 'time': 45296123, 'snr_db': -27, 'delta_time_seconds': -0.5,
                            'frequency_hz': '14095600', 'drift_hz': -2, 'callsign': 'W1AAA', 'grid': 'FN42',
                            'power_dbm': 37, 'off_air': True})
        s.positive('11-location' + suffix, schema, 11, utf8('FN31aa'), 'control',
                   {'operation': 'wsjtx.location', 'location': 'FN31aa'}, message={'location': 'FN31aa'})
        adif = '<ADIF_VER:5>3.1.4<EOH><CALL:5>W1AAA<EOR>'
        s.positive('12-logged-adif' + suffix, schema, 12, utf8(adif), 'qso_logged', {'adif': adif}, message={'adif': adif})
        colors = pack('B5H', 1, 65535, 0x1111, 0x2222, 0x3333, 0x4444) + pack('B5H', 0, 1, 2, 3, 4, 5)
        s.positive('13-highlight-callsign' + suffix, schema, 13, utf8('W1AAA') + colors + b'\x01', 'control',
                   {'operation': 'wsjtx.highlight_callsign', 'call': 'W1AAA', 'highlight_last': True},
                   message={'callsign': 'W1AAA', 'background': {'spec': 1, 'channels': [65535, 0x1111, 0x2222, 0x3333, 0x4444]},
                            'foreground': {'spec': 0, 'channels': [1, 2, 3, 4, 5]}, 'highlight_last': True})
        s.positive('14-switch-configuration' + suffix, schema, 14, utf8('test-config'), 'control',
                   {'operation': 'wsjtx.switch_configuration', 'configuration_name': 'test-config'},
                   message={'configuration_name': 'test-config'})
        configure = utf8('FT8') + pack('I', 100) + utf8('sub-test') + pack('BII', 0, 15, 1800) + utf8('W1AAA') + utf8('FN42')
        cfg_fields = {'operation': 'wsjtx.configure', 'mode': 'FT8', 'frequency_tolerance_hz': 100, 'sub_mode': 'sub-test',
                      'fast_mode': False, 'tr_period_seconds': 15, 'rx_offset_hz': 1800, 'dx_call': 'W1AAA', 'dx_grid': 'FN42'}
        s.positive('15-configure' + suffix, schema, 15, configure + b'\x01', 'control', dict(cfg_fields, generate_messages=True),
                   message={'mode': 'FT8', 'frequency_tolerance_hz': 100, 'sub_mode': 'sub-test', 'fast_mode': False,
                            'tr_period_seconds': 15, 'rx_df_hz': 1800, 'dx_call': 'W1AAA', 'dx_grid': 'FN42', 'generate_messages': True})
        s.positive('16-annotation-info' + suffix, schema, 16, utf8('W1AAA') + pack('BI', 0, 12345), 'control',
                   {'operation': 'wsjtx.annotation_info', 'dx_call': 'W1AAA', 'sort_order_provided': False, 'sort_order': 12345},
                   message={'dx_call': 'W1AAA', 'sort_order_provided': False, 'sort_order': 12345})
        # Historical minima in every supported schema.
        s.positive('tail-heartbeat-empty' + suffix, schema, 0, b'', 'lifecycle', {'state': 'heartbeat'}, message={})
        s.positive('tail-status-minimal' + suffix, schema, 1, b''.join(status_parts[:6]), 'radio_state',
                   {k: status_fields[k] for k in ['dial_frequency_hz', 'mode', 'dx_call', 'report', 'tx_mode',
                                                'tx_enabled', 'transmitting', 'decoding', 'rx_offset_hz', 'tx_offset_hz']})
        s.positive('tail-decode-minimal' + suffix, schema, 2, decode, 'decode', decode_fields)
        s.positive('tail-reply-minimal' + suffix, schema, 4, reply, 'control', reply_fields)
        s.positive('tail-qso-minimal' + suffix, schema, 5, qso_core, 'qso_logged', qso_fields)
        s.positive('tail-wspr-minimal' + suffix, schema, 10, wspr, 'decode', wspr_fields)
        s.positive('tail-highlight-minimal' + suffix, schema, 13, utf8('W1AAA') + colors, 'control',
                   {'operation': 'wsjtx.highlight_callsign', 'call': 'W1AAA'})
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
            optional_status = [utf8('K1AAA'), utf8('FN31'), utf8('FN42'), b'\x00', utf8('sub-test'),
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
                       13: header(schema, 13) + utf8('W1AAA') + colors, 15: header(schema, 15) + configure}.get(typ, raw)
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
        raw += utf8('~') + utf8('CQ W1AAA FN42') + b'\x01\x04'
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
        'receiver_call': 'K1AAA', 'receiver_locator': 'FN31', 'sender_call': 'W1AAA',
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
    strict['$defs']['native_payload']['properties']['message'] = {'$ref': '#/$defs/wsjtx_message'}
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


def validate(suite, qgi, producer_strict=False):
    """Use a local schema only; importing jsonschema is optional, never install it."""
    check_status_sentinels(suite)
    check_command_text(suite)
    check_revision_outcomes(suite)
    try:
        import jsonschema
    except ImportError:
        if producer_strict:
            raise SystemExit('--producer-strict requires jsonschema; use uv run --with jsonschema')
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='check exact generated files without writing')
    parser.add_argument('--producer-strict', action='store_true',
                        help='validate authored outputs with QGI-SER-0009 producer profile')
    args = parser.parse_args()
    suite = build()
    qgi = qgi_vectors(suite)
    validation = validate(suite, qgi, args.producer_strict)
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
    print(validation)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
