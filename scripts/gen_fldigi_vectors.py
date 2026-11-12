# SPDX-License-Identifier: GPL-3.0-or-later
"""Annex E synthetic vectors and independent policy oracles; standard library only."""

import base64
import copy
import hashlib
import json
import math
import re
from pathlib import Path

SPEC = '0.1.0'
RELEASE = '4.2.13'
RETRIEVED = '2026-11-12'
ARCHIVE_URL = 'https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz'
ARCHIVE_SHA256 = 'a1e8d990359ce9c0cce3ceb5116fd0cf72c95528969766898640ca6ca2dba8d4'
LIMIT = 71155
DECL = '<?xml version="1.0" encoding="utf-8"?>'
# Released table signatures are evidence, not a generic method writer API.
CATALOG = {'fldigi.list': 'A:n',
 'fldigi.name': 's:n',
 'fldigi.version_struct': 'S:n',
 'fldigi.version': 's:n',
 'fldigi.name_version': 's:n',
 'fldigi.config_dir': 's:n',
 'fldigi.terminate': 'n:i',
 'modem.get_mode': 's:n',
 'modem.get_submode': 's:n',
 'modem.get_name': 's:n',
 'modem.get_io_names': 'A:n',
 'modem.get_names': 'A:n',
 'modem.get_id': 'i:n',
 'modem.get_max_id': 'i:n',
 'modem.set_by_name': 's:s',
 'modem.set_by_id': 'i:i',
 'modem.set_carrier': 'i:i',
 'modem.inc_carrier': 'i:i',
 'modem.get_carrier': 'i:n',
 'modem.get_afc_search_range': 'i:n',
 'modem.set_afc_search_range': 'i:i',
 'modem.inc_afc_search_range': 'i:i',
 'modem.get_bandwidth': 'i:n',
 'modem.set_bandwidth': 'i:i',
 'modem.inc_bandwidth': 'i:i',
 'modem.get_quality': 'd:n',
 'modem.search_up': 'n:n',
 'modem.search_down': 'n:n',
 'modem.olivia.set_bandwidth': 'n:i',
 'modem.olivia.get_bandwidth': 'i:n',
 'modem.olivia.set_tones': 'n:i',
 'modem.olivia.get_tones': 'i:n',
 'main.get_status1': 's:n',
 'main.get_status2': 's:n',
 'main.get_sideband': 's:n',
 'main.set_sideband': 'n:s',
 'main.get_wf_sideband': 's:n',
 'main.set_wf_sideband': 'n:s',
 'main.get_frequency': 'd:n',
 'main.set_frequency': 'd:d',
 'main.inc_frequency': 'd:d',
 'main.get_afc': 'b:n',
 'main.set_afc': 'b:b',
 'main.toggle_afc': 'b:n',
 'main.get_squelch': 'b:n',
 'main.set_squelch': 'b:b',
 'main.toggle_squelch': 'b:n',
 'main.get_squelch_level': 'd:n',
 'main.set_squelch_level': 'd:d',
 'main.inc_squelch_level': 'd:d',
 'main.get_reverse': 'b:n',
 'main.set_reverse': 'b:b',
 'main.toggle_reverse': 'b:n',
 'main.get_lock': 'b:n',
 'main.set_lock': 'b:b',
 'main.toggle_lock': 'b:n',
 'main.get_txid': 'b:n',
 'main.set_txid': 'b:b',
 'main.toggle_txid': 'b:n',
 'main.get_rsid': 'b:n',
 'main.set_rsid': 'b:b',
 'main.toggle_rsid': 'b:n',
 'main.get_trx_status': 's:n',
 'main.tx': 'n:n',
 'main.tune': 'n:n',
 'main.rsid': 'n:n',
 'main.rx': 'n:n',
 'main.rx_tx': 'n:n',
 'main.rx_only': 'n:n',
 'main.abort': 'n:n',
 'main.get_trx_state': 's:n',
 'main.get_tx_timing': 'n:s',
 'main.get_char_rates': 's:n',
 'main.get_char_timing': 'n:i',
 'main.set_rig_name': 'n:s',
 'main.set_rig_frequency': 'd:d',
 'main.set_rig_modes': 'n:A',
 'main.set_rig_mode': 'n:s',
 'main.get_rig_modes': 'A:n',
 'main.get_rig_mode': 's:n',
 'main.set_rig_bandwidths': 'n:A',
 'main.set_rig_bandwidth': 'n:s',
 'main.get_rig_bandwidth': 's:n',
 'main.get_rig_bandwidths': 'n:A',
 'main.run_macro': 'n:i',
 'main.get_max_macro_id': 'i:n',
 'rig.set_name': 'n:s',
 'rig.get_name': 's:n',
 'rig.set_frequency': 'd:d',
 'rig.set_smeter': 'n:i',
 'rig.set_pwrmeter': 'n:i',
 'rig.set_modes': 'n:A',
 'rig.set_mode': 'n:s',
 'rig.get_modes': 'A:n',
 'rig.get_mode': 's:n',
 'rig.set_bandwidths': 'n:A',
 'rig.set_bandwidth': 'n:s',
 'rig.get_frequency': 'd:n',
 'rig.get_bandwidth': 's:n',
 'rig.get_bandwidths': 'A:n',
 'rig.get_notch': 's:n',
 'rig.set_notch': 'n:i',
 'rig.enable_qsy': 'n:i',
 'log.get_frequency': 's:n',
 'log.get_time_on': 's:n',
 'log.get_time_off': 's:n',
 'log.get_date_on': 's:n',
 'log.get_date_off': 's:n',
 'log.get_call': 's:n',
 'log.get_name': 's:n',
 'log.get_rst_in': 's:n',
 'log.get_rst_out': 's:n',
 'log.set_rst_in': 'n:s',
 'log.set_rst_out': 'n:s',
 'log.get_serial_number': 's:n',
 'log.set_serial_number': 'n:s',
 'log.set_contest_counter': 'n:s',
 'log.get_serial_number_sent': 's:n',
 'log.get_exchange': 's:n',
 'log.set_exchange': 'n:s',
 'log.get_state': 's:n',
 'log.get_province': 's:n',
 'log.get_country': 's:n',
 'log.get_qth': 's:n',
 'log.get_band': 's:n',
 'log.get_sideband': 's:n',
 'log.get_notes': 's:n',
 'log.get_locator': 's:n',
 'log.get_az': 's:n',
 'log.clear': 'n:n',
 'log.set_call': 'n:s',
 'log.set_name': 'n:s',
 'log.set_qth': 'n:s',
 'log.set_locator': 'n:s',
 'logbook.last_record': 's:n',
 'logbook.all_records': 's:n',
 'main.flmsg_online': 'n:n',
 'main.flmsg_available': 'n:n',
 'main.flmsg_transfer': 'n:n',
 'main.flmsg_squelch': 'b:n',
 'flmsg.online': 'n:n',
 'flmsg.available': 'n:n',
 'flmsg.transfer': 'n:n',
 'flmsg.squelch': 'b:n',
 'flmsg.get_data': '6:n',
 'io.in_use': 's:n',
 'io.enable_kiss': 'n:n',
 'io.enable_arq': 'n:n',
 'text.get_rx_length': 'i:n',
 'text.get_rx': '6:ii',
 'text.clear_rx': 'n:n',
 'text.add_tx': 'n:s',
 'text.add_tx_queu': 'n:s',
 'text.add_tx_bytes': 'n:6',
 'text.clear_tx': 'n:n',
 'rxtx.get_data': '6:n',
 'rx.get_data': '6:n',
 'tx.get_data': '6:n',
 'spot.get_auto': 'b:n',
 'spot.set_auto': 'b:b',
 'spot.toggle_auto': 'b:n',
 'spot.pskrep.get_count': 'i:n',
 'wefax.state_string': 's:n',
 'wefax.skip_apt': 's:n',
 'wefax.skip_phasing': 's:n',
 'wefax.set_tx_abort_flag': 's:n',
 'wefax.end_reception': 's:n',
 'wefax.start_manual_reception': 's:n',
 'wefax.set_adif_log': 's:b',
 'wefax.set_max_lines': 's:i',
 'wefax.get_received_file': 's:i',
 'wefax.send_file': 's:si',
 'navtex.get_message': 's:i',
 'navtex.send_message': 's:s'}
MAPPED = {'fldigi.version': ('', 'result', 'text'),
 'rig.get_frequency': ('', 'radio_state', 'dial_frequency_hz'),
 'main.get_frequency': ('', 'radio_state', 'dial_frequency_hz'),
 'rig.set_frequency': ('frequency_hz', 'result', 'frequency_hz'),
 'main.set_frequency': ('frequency_hz', 'result', 'frequency_hz'),
 'modem.get_carrier': ('', 'radio_state', 'rx_offset_hz'),
 'modem.set_carrier': ('audio_offset_hz', 'result', ''),
 'modem.get_name': ('', 'radio_state', 'mode'),
 'modem.get_mode': ('', 'result', 'text'),
 'modem.get_submode': ('', 'radio_state', 'sub_mode'),
 'modem.set_by_name': ('mode', 'result', 'text'),
 'rig.get_mode': ('', 'result', 'text'),
 'rig.set_mode': ('mode', 'result', ''),
 'main.get_trx_state': ('', 'radio_state', 'state'),
 'main.get_trx_status': ('', 'radio_state', 'state'),
 'main.tx': ('', 'result', ''),
 'main.rx': ('', 'result', ''),
 'main.abort': ('', 'result', ''),
 'main.tune': ('', 'result', ''),
 'rx.get_data': ('', 'decode', 'bytes'),
 'text.get_rx': ('rx_start rx_length', 'decode', 'bytes'),
 'text.get_rx_length': ('', 'result', ''),
 'text.add_tx': ('text', 'result', ''),
 'text.clear_rx': ('', 'result', ''),
 'text.clear_tx': ('', 'result', ''),
 'log.get_frequency': ('', 'result', 'text'),
 'log.get_time_on': ('', 'result', 'text'),
 'log.get_time_off': ('', 'result', 'text'),
 'log.get_date_on': ('', 'result', 'text'),
 'log.get_date_off': ('', 'result', 'text'),
 'log.get_call': ('', 'radio_state', 'dx_call'),
 'log.get_name': ('', 'result', 'text'),
 'log.get_rst_in': ('', 'result', 'text'),
 'log.get_rst_out': ('', 'result', 'text'),
 'log.get_serial_number': ('', 'result', 'text'),
 'log.get_serial_number_sent': ('', 'result', 'text'),
 'log.get_exchange': ('', 'result', 'text'),
 'log.get_state': ('', 'result', 'text'),
 'log.get_province': ('', 'result', 'text'),
 'log.get_country': ('', 'result', 'text'),
 'log.get_qth': ('', 'result', 'text'),
 'log.get_band': ('', 'result', 'text'),
 'log.get_sideband': ('', 'result', 'text'),
 'log.get_notes': ('', 'result', 'text'),
 'log.get_locator': ('', 'radio_state', 'dx_grid'),
 'log.get_az': ('', 'result', 'text'),
 'log.set_call': ('dx_call', 'result', ''),
 'log.set_locator': ('dx_grid', 'result', ''),
 'logbook.last_record': ('', 'result', 'text'),
 'logbook.all_records': ('', 'result', 'text')}


def operation(method):
    return {'rig.set_frequency': 'set_frequency', 'main.set_frequency': 'set_frequency',
            'text.add_tx': 'send_text', 'main.abort': 'halt_tx'}.get(
        method, 'fldigi.' + (method[7:] if method.startswith('fldigi.') else method.replace('.', '_')))


def text_value(s, encoding):
    if not any(0xd800 <= ord(c) <= 0xdfff for c in s):
        return s
    if encoding.startswith('utf-16'):
        raw = s.encode(encoding, 'surrogatepass')
        return {'utf16_units': [int.from_bytes(raw[i:i+2], 'little' if encoding.endswith('le') else 'big')
                                for i in range(0, len(raw), 2)]}
    return {'utf8_bytes': list(s.encode('utf-8', 'surrogateescape'))}


def xmlrpc(raw, substrate):
    """Independent grammar/type oracle; retains a small tree, not an implementation import."""
    root, enc = substrate(raw, body_limit=LIMIT, numeric_catalog=False)
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
            container(data, 'data', 'value', 256)
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
    if params and params['name'] == 'fault' and not extended:
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
            result = [(text_value(m['children'][0]['text'], enc), semantic(m['children'][1]))
                      for m in node['children']]
        elif typ == 'array':
            result = [semantic(v) for v in node['children'][0]['children']]
        else:
            result = text_value(node['text'], enc)
        node['semantic'] = ('int' if typ == 'i4' else typ, result, node['text'])
    fault = None
    if params and params['name'] == 'fault' and not extended:
        t, members, _ = semantic(params['children'][0])
        if t != 'struct':
            raise ValueError('malformed')
        codes = [v for n, v in members if n == 'faultCode']
        texts = [v for n, v in members if n == 'faultString']
        if len(codes) != 1 or len(texts) != 1 or codes[0][0] != 'int' or texts[0][0] != 'string':
            raise ValueError('malformed')
        fault = codes[0][2], texts[0][1]
    return root, enc, extended, params, fault


def semantic(node):
    return node.get('semantic', node['children'][0].get('semantic') if node['children'] else None)


def name_check(method):
    if not isinstance(method, str) or not re.fullmatch(r'[A-Za-z0-9_.:/]+', method):
        raise ValueError('invalid_value')
    if len(method.encode()) > 128:
        raise ValueError('oversized')


def observe(inp, substrate):
    raw = base64.b64decode(inp['data'])
    root, enc, extension, params, fault = xmlrpc(raw, substrate)
    method = root['children'][0]['text'] if root['name'] == 'methodCall' else None
    if method is not None:
        name_check(method)
    assoc = inp.get('request')
    if assoc is not None:
        if not isinstance(assoc, dict) or set(assoc) - {'method', 'arguments'} or 'method' not in assoc:
            raise ValueError('invalid_value')
        name_check(assoc['method'])
        if 'arguments' in assoc and (not isinstance(assoc['arguments'], list) or len(assoc['arguments']) > 32):
            raise ValueError('invalid_value')
        if method is not None and method != assoc['method']:
            raise ValueError('invalid_value')
        if root['name'] != 'methodCall':
            method = assoc['method']
    kind, fields = 'native', {}
    version = None
    if fault is not None:
        kind, fields = 'result', {'status': 'error', 'native_code': fault[0], 'text': fault[1]}
        if method and re.fullmatch(r'(?:[a-z][a-z0-9.-]*\.)?[a-z][a-z0-9_]*', operation(method)):
            fields['operation'] = operation(method)
    elif not extension and method in MAPPED:
        reqkeys, response_kind, key = MAPPED[method]
        signature = CATALOG[method]
        arguments = [] if params is None else [semantic(p['children'][0]) for p in params['children']]
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
                kind, fields = 'control', {'operation': operation(method)}
                for name, (_, val, _) in zip(reqkeys.split(), arguments):
                    if name == 'frequency_hz':
                        if val.is_integer() and abs(val) <= 9007199254740991:
                            fields[name] = int(val)
                    else:
                        fields[name] = val
            else:
                kind = response_kind
                typ, val, _ = arguments[0]
                fields = {'status': 'ok', 'operation': operation(method)} if kind == 'result' else {}
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
    parts = root['name'].split('|')
    literal = parts[2] + ':' + parts[1] if len(parts) == 3 else parts[-1]
    view = {'root': literal}
    if method is not None:
        view['method'] = method
    event = {'spec': SPEC, 'family': 'fldigi', 'mapping': 1, 'kind': kind,
             'context': copy.deepcopy(inp['context']), 'fields': fields,
             'native': {'format': 'fldigi_xmlrpc', 'data': inp['data']}}
    if version is not None:
        event['native_version'] = version
    return event, {**view, 'xml': text_value(raw.decode(enc, 'surrogatepass' if enc.startswith('utf-16') else 'surrogateescape').lstrip('\ufeff'), enc)}


def write_command(event, options, substrate):
    """Independent construction checker; fixtures below supply their own XML strings."""
    if event['mapping'] != 1:
        raise ValueError('unsupported_version')
    if any(k in event for k in ('native', 'native_version', 'wire_id')) or options or event['kind'] != 'control':
        raise ValueError('invalid_value')
    fields = event['fields']
    op = fields.get('operation')
    by_operation = {operation(n): n for n in MAPPED}
    by_operation.update(set_frequency='rig.set_frequency', send_text='text.add_tx', halt_tx='main.abort')
    if op not in by_operation:
        raise ValueError('invalid_value')
    method = by_operation[op]
    keys = MAPPED[method][0].split()
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
            if not isinstance(val, str) or any(not (ord(c) in (9, 10, 13) or 32 <= ord(c) <= 0xd7ff or
                   0xe000 <= ord(c) <= 0xfffd or 0x10000 <= ord(c) <= 0x10ffff) for c in val):
                raise ValueError('invalid_value')
            escaped = ''.join({'&': '&amp;', '<': '&lt;', '>': '&gt;', '\r': '&#13;'}.get(c, c) for c in val)
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
        pieces = [DECL, '<methodCall><methodName>', name, '</methodName><params>']
        for typ, content in params:
            pieces += ['<param><value><', typ, '>', content, '</', typ, '></value></param>']
        pieces.append('</params></methodCall>')
        body = ''.join(pieces).encode()
        xmlrpc(body, substrate)
        result.append(body)
    return result


def call(method, *values):
    return (DECL + '<methodCall><methodName>' + method + '</methodName><params>' +
            ''.join('<param><value>' + v + '</value></param>' for v in values) + '</params></methodCall>').encode()


def response(value):
    return ('<methodResponse><params><param><value>' + value + '</value></param></params></methodResponse>').encode()


def context(request=False):
    return {'direction': 'to_application' if request else 'from_application',
            'exchange': 'request' if request else 'response', 'correlation_id': 'synthetic-17'}


def vectors(Suite, b64):
    suite = Suite('fldigi')
    def positive(name, raw, kind, fields, method=None, args=None, *, is_request=False, version=None, ctx=None):
        inp = {'data': b64(raw), 'context': ctx or context(is_request)}
        if not is_request and method is not None:
            inp['request'] = {'method': method}
            if args is not None:
                inp['request']['arguments'] = args
        root = 'methodCall' if is_request else 'methodResponse'
        view = {'root': root}
        if method is not None:
            view['method'] = method
        event = {'spec': SPEC, 'family': 'fldigi', 'mapping': 1, 'kind': kind, 'context': inp['context'],
                 'fields': fields, 'native': {'format': 'fldigi_xmlrpc', 'data': b64(raw)}}
        if version is not None:
            event['native_version'] = version
        try:
            xml = raw.decode('utf-8')
        except UnicodeDecodeError:
            xml = {'utf8_bytes': list(raw)}
        suite.add(name, {'class': 'positive', 'input': inp, 'expected': {'event': event},
                         'expected_message': {**view, 'xml': xml}, 'reencode': {'data': b64(raw)}})
        return suite.vectors[name]
    def negative(name, raw, error, **metadata):
        suite.add(name, {'class': 'negative', 'input': {'data': b64(raw), 'context': context(), **metadata},
                         'expected': {'error': {'category': error}}})
    # Literal representative values and explicit expected shared fields.
    requests = {'frequency_hz': '<double>14070000.0</double>', 'audio_offset_hz': '<int>1501</int>',
                'mode': '<string>BPSK31</string>', 'text': '<string>CQ Q1AAA</string>',
                'dx_call': '<string>Q2BBB</string>', 'dx_grid': '<string>FN31</string>',
                'rx_start': '<int>7</int>', 'rx_length': '<int>4</int>'}
    argument_values = {'frequency_hz': 14070000, 'audio_offset_hz': 1501, 'mode': 'BPSK31',
                       'text': 'CQ Q1AAA', 'dx_call': 'Q2BBB', 'dx_grid': 'FN31', 'rx_start': 7, 'rx_length': 4}
    return_values = {'dial_frequency_hz': ('<double>14071000.0</double>', 14071000),
                     'frequency_hz': ('<double>14071000.0</double>', 14071000),
                     'rx_offset_hz': ('<int>1502</int>', 1502), 'mode': ('<string>BPSK31</string>', 'BPSK31'),
                     'sub_mode': ('<string>PSK31</string>', 'PSK31'), 'dx_call': ('<string>Q3CCC</string>', 'Q3CCC'),
                     'dx_grid': ('<string>FN42</string>', 'FN42'), 'text': ('<string>synthetic field 005</string>', 'synthetic field 005')}
    for method, (argkeys, kind, key) in MAPPED.items():
        stem = method.replace('.', '-').replace('_', '-')
        fields = {'operation': operation(method), **{k: argument_values[k] for k in argkeys.split()}}
        positive(stem + '-request', call(method, *(requests[k] for k in argkeys.split())), 'control', fields,
                 method, is_request=True)
        fields = {'status': 'ok', 'operation': operation(method)} if kind == 'result' else {}
        if key in return_values:
            value, val = return_values[key]
            fields[key] = val
        elif key == 'state':
            value = '<string>TX</string>' if method.endswith('_state') else '<string>tune</string>'
            fields['transmitting'] = True
        elif key == 'bytes':
            value = '<base64>UTFBQQ==</base64>'
            fields['text'] = 'Q1AA'
        else:
            value = '<int>12</int>' if CATALOG[method][0] == 'i' else '<nil></nil>'
        version = None
        if method == 'fldigi.version':
            value, version = '<string>4.2.13</string>', '4.2.13'
            fields.pop('text', None)
        positive(stem + '-response', response(value), kind, fields, method,
                 [7, 4] if method == 'text.get_rx' else None, version=version)
        # Each mapped family operation also has a construction oracle. Shared aliases
        # intentionally encode one canonical method; wire observations retain aliases.
        if method != 'main.set_frequency':
            command_fields = {'operation': operation(method), **{k: argument_values[k] for k in argkeys.split()}}
            if method == 'text.add_tx':
                command_fields['transmit'] = False
            evt = {'spec': SPEC, 'family': 'fldigi', 'mapping': 1, 'kind': 'control',
                   'context': context(True), 'fields': command_fields}
            suite.add(stem + '-command', {'class': 'command', 'input': {'event': evt, 'options': {}},
                'expected': {'requests': [b64(call(method, *(requests[k] for k in argkeys.split())))]}})
    samples = {'n': '<nil/>', 's': '<string>synthetic</string>', 'i': '<int>7</int>',
               'b': '<boolean>1</boolean>', 'd': '<double>1.25</double>', '6': '<base64>UTFB</base64>',
               'A': '<array><data></data></array>', 'S': '<struct></struct>'}
    # Body-derived alternatives override advertised signatures. Other samples are
    # synthetic acceptance cases, not claims of captures from the bundled backend.
    native_returns = {
        'rig.get_notch': '<int>1500</int>', 'rig.set_notch': '<int>1500</int>',
        'rig.enable_qsy': '<int>1</int>',
        'main.flmsg_available': '<int>7</int>', 'flmsg.available': '<int>7</int>',
        'main.flmsg_transfer': '<string>Q1AAA</string>', 'flmsg.transfer': '<string>Q1AAA</string>',
        'main.get_tx_timing': '<string>8000:8000:1.0</string>',
        'main.get_char_timing': '<string>8000:8000:0</string>',
    }
    for method, signature in CATALOG.items():
        if method in MAPPED:
            continue
        suffix = signature.split(':')[1]
        arguments = [samples[s] for s in suffix if s != 'n']
        if method in ('main.get_tx_timing', 'main.get_char_timing'):
            arguments = ['<base64>NjU=</base64>']
        raw = call(method, *arguments)
        positive(method.replace('.', '-').replace('_', '-') + '-native-request', raw, 'native', {}, method, is_request=True)
        positive(method.replace('.', '-').replace('_', '-') + '-native-response',
                 response(native_returns.get(method, samples[signature[0]])), 'native', {}, method)
    release_vectors(positive, negative)
    for name, value in {'i4': '<i4>-2147483648</i4>', 'int-max': '<int>2147483647</int>',
            'bool-zero': '<boolean>0</boolean>', 'double-exponent': '<double>-1.25e+2</double>',
            'double-negative-zero': '<double>-0.0</double>', 'datetime': '<dateTime.iso8601>20240229T12:34:56</dateTime.iso8601>',
            'nil': '<nil/>', 'base64-space': '<base64> U T F B \n</base64>',
            'array': '<array><data><value><int>1</int></value><value><string>Q1AAA</string></value></data></array>',
            'struct-duplicate': '<struct><member><name>x</name><value>one</value></member><member><name>x</name><value>two</value></member></struct>',
            'empty-untyped': '', 'untyped-space': '  Q1AAA  ', 'future-type': '<future><int>invalid opaque</int></future>'}.items():
        positive('value-' + name, response(value), 'native', {}, 'future.method')
    for name, value, val in [('untyped', ' Q1AAA ', ' Q1AAA '), ('empty', '<string/>', ''),
            ('entities', '<string>&lt;Q1AAA&gt;&amp;&#13;\r\n<![CDATA[x]]></string>', '<Q1AAA>&\r\nx'),
            ('unicode', '<string> Q1AAA café\t\n</string>', ' Q1AAA café\t\n'),
            ('invalid-utf8', '<string>Q1AAA\udcff</string>', {'utf8_bytes': [81,49,65,65,65,255]})]:
        raw = response(value.encode('utf-8', 'surrogateescape').decode('latin1')).decode().encode('latin1') if name == 'invalid-utf8' else response(value)
        positive('text-' + name, raw, 'result', {'status': 'ok', 'operation': 'fldigi.log_get_name', 'text': val}, 'log.get_name')
    for name, value, val in [('empty', b'', ''), ('nul', b'Q1\x00AAA', 'Q1\x00AAA'),
            ('split-first', b'Q1\xc3', {'utf8_bytes': [81,49,195]}), ('split-second', b'\xa9 AAA', {'utf8_bytes': [169,32,65,65,65]}),
            ('unicode', 'Q1AAA é'.encode(), 'Q1AAA é')]:
        positive('rx-' + name, response('<base64>' + b64(value) + '</base64>'), 'decode', {'text': val}, 'rx.get_data')
    for name, method, value, fields in [
            ('state-rx', 'main.get_trx_state', '<string>RX</string>', {'transmitting': False}),
            ('state-other', 'main.get_trx_state', '<string>OTHER</string>', {}),
            ('state-whitespace', 'main.get_trx_state', '<string> TX </string>', {}),
            ('status-rx', 'main.get_trx_status', '<string>rx</string>', {'transmitting': False}),
            ('status-tx', 'main.get_trx_status', '<string>tx</string>', {'transmitting': True}),
            ('fractional-hz', 'rig.get_frequency', '<double>14070000.5</double>', {}),
            ('out-of-common-hz', 'rig.get_frequency', '<double>9007199254740992</double>', {})]:
        positive(name, response(value), 'radio_state', fields, method)
    positive('response-no-request', response('<double>14070000.0</double>'), 'native', {})
    positive('response-wrong-method-type', response('<double>14070000.0</double>'), 'native', {}, 'modem.get_name')
    positive('response-undetectable-wrong-method', response('<string>Q1AAA</string>'), 'radio_state',
             {'dx_grid': 'Q1AAA'}, 'log.get_locator')
    positive('range-no-arguments', response('<base64>UTFB</base64>'), 'native', {}, 'text.get_rx')
    positive('range-negative-one', response('<base64>UTFB</base64>'), 'decode', {'text': 'Q1A'}, 'text.get_rx', [4, -1])
    positive('range-length-disagrees', response('<base64>UTFB</base64>'), 'decode', {'text': 'Q1A'}, 'text.get_rx', [4, 99])
    positive('mapped-wrong-arity', call('rig.get_frequency', '<int>1</int>'), 'native', {}, 'rig.get_frequency', is_request=True)
    positive('unknown-method-request', call('future.method', '<string>Q1AAA</string>'), 'native', {}, 'future.method', is_request=True)
    positive('unknown-context', call('main.tx'), 'control', {'operation': 'fldigi.main_tx'}, 'main.tx', is_request=True,
             ctx={'direction': 'unknown', 'exchange': 'unknown', 'correlation_id': 'unchanged'})
    positive('new-program-version', response('<string>99.0.0</string>'), 'result',
             {'status': 'ok', 'operation': 'fldigi.version'}, 'fldigi.version', version='99.0.0')
    positive('version-empty', response('<string/>'), 'result',
             {'status': 'ok', 'operation': 'fldigi.version', 'text': ''}, 'fldigi.version')
    positive('version-invalid-text', response('<string>!</string>').replace(b'!', b'\xff'), 'result',
             {'status': 'ok', 'operation': 'fldigi.version', 'text': {'utf8_bytes': [255]}}, 'fldigi.version')
    fault = b'<methodResponse><fault><value><struct><member><name>faultCode</name><value><int> +005 </int></value></member><member><name>faultString</name><value><string>Q1AAA rejected &amp; retained</string></value></member></struct></value></fault></methodResponse>'
    for name, method in [('fault-associated','main.abort'), ('fault-unassociated',None), ('fault-unknown-method','future.method')]:
        f = {'status': 'error', 'native_code': ' +005 ', 'text': 'Q1AAA rejected & retained'}
        if method:
            f['operation'] = operation(method)
        positive(name, fault, 'result', f, method)
    positive('fault-http-error', fault, 'result', {'status': 'error', 'native_code': ' +005 ', 'text': 'Q1AAA rejected & retained'})['input'].update(http_status=503, content_type='text/plain')
    # Independent lexical/encoding boundaries.
    v = positive('utf16-invalid-text', b'\xff\xfe' + response('<string>Q1AAA!</string>').decode().replace('!', '\ud800').encode('utf-16-le', 'surrogatepass'),
                 'result', {'status': 'ok', 'operation': 'fldigi.log_get_name', 'text': {'utf16_units': [81,49,65,65,65,55296]}}, 'log.get_name')
    v['expected_message']['xml'] = {'utf16_units': list('<methodResponse><params><param><value><string>Q1AAA'.encode()) + [55296] + list('</string></value></param></params></methodResponse>'.encode())}
    positive('mapped-http-metadata', response('<string>BPSK31</string>'), 'radio_state',
             {'mode': 'BPSK31'}, 'modem.get_name')['input'].update(http_status=500, content_type='application/octet-stream')
    # Additional positives/negatives are appended by boundary_vectors.
    boundary_vectors(suite, positive, negative, b64)
    negative('request-association-mismatch', call('main.tx'), 'invalid_value', request={'method': 'main.rx'})
    negative('association-extra-member', response('<nil/>'), 'invalid_value', request={'method': 'main.tx', 'extra': True})
    negative('method-name-invalid', call('bad method'), 'invalid_value')
    negative('method-name-too-long', call('a' * 129), 'oversized')
    negative('fault-missing-code', fault.replace(b'faultCode', b'otherCode'), 'malformed')
    negative('fault-duplicate-code', fault.replace(b'</struct>', b'<member><name>faultCode</name><value><int>1</int></value></member></struct>'), 'malformed')
    negative('fault-wrong-code-type', fault.replace(b'<int> +005 </int>', b'<string>5</string>'), 'malformed')
    negative('precedence-fault-shape-number', fault.replace(b'faultString', b'otherString').replace(b' +005 ', b'x'), 'malformed')
    # Commands with more than one body have exact independent component oracles.
    base_command = {'spec': SPEC, 'family': 'fldigi', 'mapping': 1, 'kind': 'control', 'context': context(True)}
    for name, fields, bodies in [
        ('text-and-transmit', {'operation': 'send_text', 'text': 'CQ Q1AAA', 'transmit': True}, [call('text.add_tx', '<string>CQ Q1AAA</string>'), call('main.tx')]),
        ('empty-text-and-transmit', {'operation': 'send_text', 'text': '', 'transmit': True}, [call('text.add_tx', '<string></string>'), call('main.tx')]),
        ('frequency-and-offset', {'operation': 'set_frequency', 'frequency_hz': 14070000, 'rx_offset_hz': 1500}, [call('rig.set_frequency', '<double>14070000.0</double>'), call('modem.set_carrier', '<int>1500</int>')]),
        ('escaping', {'operation': 'send_text', 'text': 'Q1AAA &<>\r\n\t"', 'transmit': False}, [call('text.add_tx', '<string>Q1AAA &amp;&lt;&gt;&#13;\n\t"</string>')]),
        ('frequency-min', {'operation': 'set_frequency', 'frequency_hz': 1}, [call('rig.set_frequency', '<double>1.0</double>')]),
        ('frequency-max', {'operation': 'set_frequency', 'frequency_hz': 2147483647}, [call('rig.set_frequency', '<double>2147483647.0</double>')]),
        ('range-zero', {'operation': 'fldigi.text_get_rx', 'rx_start': 0, 'rx_length': 0}, [call('text.get_rx', '<int>0</int>', '<int>0</int>')])]:
        suite.add('command-' + name, {'class': 'command', 'input': {'event': {**base_command, 'fields': fields}, 'options': {}},
                                    'expected': {'requests': [b64(b) for b in bodies]}})
    good = {'operation': 'send_text', 'text': 'Q1AAA', 'transmit': False}
    bads = [('missing-text', {'operation': 'send_text', 'transmit': False}),
            ('missing-transmit', {'operation': 'send_text', 'text': 'Q1AAA'}),
            ('extra-argument', {**good, 'window': 1}), ('null', {**good, 'text': None}),
            ('raw', {**good, 'text': {'utf8_bytes': [255]}}), ('surrogate', {**good, 'text': '\ud800'}),
            ('nul', {**good, 'text': '\x00'}), ('bad-bool', {**good, 'transmit': 1}),
            ('halt-auto-only', {'operation': 'halt_tx', 'auto_tx_only': True}),
            ('frequency-zero', {'operation': 'set_frequency', 'frequency_hz': 0}),
            ('frequency-float', {'operation': 'set_frequency', 'frequency_hz': 1.5}),
            ('frequency-range', {'operation': 'set_frequency', 'frequency_hz': 2147483648}),
            ('offset-zero', {'operation': 'set_frequency', 'frequency_hz': 1, 'rx_offset_hz': 0}),
            ('read-negative-one', {'operation': 'fldigi.text_get_rx', 'rx_start': 0, 'rx_length': -1}),
            ('unknown-operation', {'operation': 'fldigi.unknown'}),
            ('text-oversized', {**good, 'text': 'x' * LIMIT})]
    for name, fields in bads:
        suite.add('command-bad-' + name, {'class': 'command_negative', 'input': {'event': {**base_command, 'fields': fields}, 'options': {}},
                   'expected': {'error': {'category': 'oversized' if name == 'text-oversized' else 'invalid_value'}}})
    for name, extra, options, error in [('mapping', {'mapping': 2}, {}, 'unsupported_version'),
            ('wire-id', {'wire_id': 'target'}, {}, 'invalid_value'), ('version', {'native_version': '4.2.13'}, {}, 'invalid_value'),
            ('native', {'native': {'format': 'fldigi_xmlrpc', 'data': ''}}, {}, 'invalid_value'),
            ('options', {}, {'peer_version': '4.2.13'}, 'invalid_value')]:
        suite.add('command-bad-' + name, {'class': 'command_negative', 'input': {'event': {**base_command, 'fields': good, **extra}, 'options': options},
                                        'expected': {'error': {'category': error}}})
    compatibility_vectors(suite)
    return suite


def boundary_vectors(suite, positive, negative, b64):
    for name, value, error in [
        ('boolean-word', '<boolean>true</boolean>', 'invalid_value'),
        ('boolean-empty', '<boolean/>', 'invalid_value'),
        ('integer-suffix', '<int>12x</int>', 'invalid_value'),
        ('integer-range', '<int>2147483648</int>', 'invalid_value'),
        ('integer-empty', '<int/>', 'invalid_value'),
        ('numeric-token', '<int>' + '1' * 129 + '</int>', 'oversized'),
        ('double-nan', '<double>NaN</double>', 'invalid_value'),
        ('double-overflow', '<double>1e999</double>', 'invalid_value'),
        ('bad-date', '<dateTime.iso8601>20230229T12:34:56</dateTime.iso8601>', 'invalid_value'),
        ('date-no-zone', '<dateTime.iso8601>20240229T12:34:56Z</dateTime.iso8601>', 'invalid_value'),
        ('base64-pad-bits', '<base64>QR==</base64>', 'invalid_value'),
        ('base64-no-padding', '<base64>UQ</base64>', 'invalid_value'),
        ('nil-text', '<nil>value</nil>', 'invalid_value'),
        ('nested-scalar', '<string><x/></string>', 'malformed'),
        ('two-types', '<int>1</int><string>x</string>', 'malformed'),
        ('value-mixed', 'x<string>y</string>', 'malformed'),
        ('scalar-attribute', '<int a="x">1</int>', 'malformed'),
        ('qualified-scalar', '<s:string xmlns:s="urn:synthetic">x</s:string>', 'malformed'),
        ('array-extra', '<array><data/><data/></array>', 'malformed'),
        ('struct-wrong-order', '<struct><member><value>x</value><name>x</name></member></struct>', 'malformed')]:
        negative('bad-' + name, response(value), error)
    ordinary = response('<string>Q1AAA</string>')
    for name, raw, error in [
        ('empty', b'', 'truncated'), ('unfinished', ordinary[:-4], 'truncated'),
        ('conflicting-close', b'<methodResponse></methodCall>', 'malformed'),
        ('two-roots', ordinary + ordinary, 'malformed'),
        ('outside-text', ordinary + b'x', 'malformed'),
        ('doctype', b'<!DOCTYPE methodResponse>' + ordinary, 'malformed'),
        ('external-entity', b'<!DOCTYPE methodResponse [<!ENTITY x SYSTEM "https://invalid.example/entity">]>' + ordinary, 'malformed'),
        ('parameter-entity', b'<!DOCTYPE methodResponse [<!ENTITY % x SYSTEM "https://invalid.example/entity">%x;]>' + ordinary, 'malformed'),
        ('entity-expansion', b'<!DOCTYPE methodResponse [<!ENTITY a "x"><!ENTITY b "&a;&a;">]>' + ordinary, 'malformed'),
        ('unknown-entity', ordinary.replace(b'Q1AAA', b'&unknown;'), 'malformed'),
        ('forbidden-reference', ordinary.replace(b'Q1AAA', b'&#0;'), 'malformed'),
        ('forbidden-nul', ordinary.replace(b'Q1AAA', b'\x00'), 'malformed'),
        ('invalid-markup', ordinary.replace(b'string', b'str\xffing'), 'malformed'),
        ('invalid-attribute', b'<future x="\xff"/>', 'malformed'),
        ('duplicate-attribute', b'<future x="1" x="2"/>', 'malformed'),
        ('unbound-prefix', b'<f:future/>', 'malformed'),
        ('declaration-order', b'<?xml encoding="utf-8" version="1.0"?>' + ordinary, 'malformed'),
        ('declaration-unfinished', b'<?xml version="1.0"', 'malformed'),
        ('xml-version', b'<?xml version="1.1"?>' + ordinary, 'unsupported_version'),
        ('encoding-unsupported', b'<?xml version="1.0" encoding="iso-8859-1"?>' + ordinary, 'unsupported_version'),
        ('encoding-conflict', b'<?xml version="1.0" encoding="utf-16"?>' + ordinary, 'malformed'),
        ('odd-utf16', b'\xff\xfe<\x00x', 'truncated'),
        ('body-overflow', b'x' * (LIMIT + 1), 'oversized'),
        ('depth-overflow', b'<x>' * 17 + b'</x>' * 17, 'oversized'),
        ('elements-overflow', b'<x>' + b'<y/>' * 1024 + b'</x>', 'oversized'),
        ('attributes-overflow', ('<x ' + ' '.join(f'a{i}=""' for i in range(17)) + '/>').encode(), 'oversized'),
        ('name-overflow', ('<' + 'x' * 129 + '/>').encode(), 'oversized'),
        ('params-overflow', call('future.method', *([''] * 33)), 'oversized'),
        ('array-overflow', response('<array><data>' + '<value/>' * 257 + '</data></array>'), 'oversized'),
        ('members-overflow', response('<struct>' + '<member><name>x</name><value/></member>' * 257 + '</struct>'), 'oversized'),
        ('response-empty-params', b'<methodResponse><params/></methodResponse>', 'malformed'),
        ('params-and-fault', b'<methodResponse><params/><fault/></methodResponse>', 'malformed'),
        ('call-missing-name', b'<methodCall><params/></methodCall>', 'malformed'),
        ('call-duplicate-name', b'<methodCall><methodName>x</methodName><methodName>y</methodName></methodCall>', 'malformed'),
        ('container-mixed', ordinary.replace(b'<params>', b'<params>x'), 'malformed'),
        # Simultaneous failures establish the text's stage ordering.
        ('precedence-size-doctype', b'<!DOCTYPE x>' + b'x' * LIMIT, 'oversized'),
        ('precedence-version-doctype', b'<?xml version="1.1"?><!DOCTYPE x><x/>', 'unsupported_version'),
        ('precedence-declaration-version', b'<?xml encoding="utf-8" version="1.1"?><x/>', 'malformed'),
        ('precedence-doctype-depth', b'<x>' * 17 + b'<!DOCTYPE x>', 'malformed'),
        ('precedence-resource-malformed', b'<x>' * 17 + b'</y>', 'oversized'),
        ('precedence-grammar-number', response('<int>' + '1' * 129 + '</int>').replace(b'</params>', b'<extra/></params>'), 'malformed'),
        ('precedence-number-conversion', call('future.method', '<int>x</int>', '<int>' + '1' * 129 + '</int>'), 'oversized')]:
        negative(name, raw, error)
    # Known XML types count even for native-only methods.
    negative('document-attributes-overflow', ('<x>' + ''.join('<y ' + ' '.join(f'a{j}=""' for j in range(16)) + '/>' for _ in range(17)) + '</x>').encode(), 'oversized')
    negative('unclosed-entity', b'<methodResponse><params><param><value>&amp', 'truncated')
    negative('invalid-comment', b'<future><!-- \xff --></future>', 'malformed')
    negative('invalid-pi', b'<future><?test \xff?></future>', 'malformed')
    negative('native-numeric-limit' , call('spot.set_auto', '<int>' + '1' * 129 + '</int>'), 'oversized')
    positive('scalar-whitespace', response('<double> \t14070000.0\n</double>'), 'radio_state',
             {'dial_frequency_hz': 14070000}, 'rig.get_frequency')
    positive('nil-whitespace', response('<nil> \t\n</nil>'), 'result',
             {'status': 'ok', 'operation': 'fldigi.main_tx'}, 'main.tx')
    positive('empty-base64', response('<base64/>'), 'decode', {'text': ''}, 'rx.get_data')
    positive('numeric-token-boundary' , response('<int>' + '0' * 127 + '1</int>'), 'native', {}, 'future.method')
    positive('params-boundary', call('future.method', *([''] * 32)), 'native', {}, 'future.method', is_request=True)
    positive('array-boundary', response('<array><data>' + '<value/>' * 256 + '</data></array>'), 'native', {}, 'future.method')
    positive('members-boundary', response('<struct>' + '<member><name>x</name><value/></member>' * 256 + '</struct>'), 'native', {}, 'future.method')
    positive('comment-cdata-declaration-text', response('<string><!-- <!DOCTYPE x> --><![CDATA[<!DOCTYPE x>]]></string>'),
             'result', {'status': 'ok', 'operation': 'fldigi.log_get_name', 'text': '<!DOCTYPE x>'}, 'log.get_name')
    for name, raw, root in [('depth-boundary', b'<x>' * 16 + b'</x>' * 16, 'x'),
            ('elements-boundary', b'<x>' + b'<y/>' * 1023 + b'</x>', 'x'),
            ('attributes-boundary', ('<x ' + ' '.join(f'a{i}=""' for i in range(16)) + '/>').encode(), 'x'),
            ('name-boundary', ('<' + 'x' * 128 + '/>').encode(), 'x' * 128),
            ('unknown-root', b'<future>Q1AAA</future>', 'future'),
            ('namespaced-root', b'<f:methodResponse xmlns:f="urn:synthetic"/>', 'f:methodResponse')]:
        v = positive(name, raw, 'native', {})
        v['expected_message']['root'] = root
    overhead = len(response('<string></string>'))
    positive('maximum-body-with-view', response('<string>' + '"' * (LIMIT - overhead) + '</string>'), 'result',
             {'status': 'ok', 'operation': 'fldigi.log_get_name', 'text': '"' * (LIMIT - overhead)}, 'log.get_name')
    # UTF-16 source bytes are smaller than decoded UTF-8; expansion is bounded separately.
    raw = b'\xff\xfe' + ('<x>' + '\u0800' * 23719 + '</x>').encode('utf-16-le')
    negative('decoded-text-overflow', raw, 'oversized')
    for name, encoding, bom, decl in [('utf8-bom', 'utf-8', b'\xef\xbb\xbf', ''),
            ('utf16le', 'utf-16-le', b'\xff\xfe', '<?xml version="1.0" encoding="utf-16"?>'),
            ('utf16be', 'utf-16-be', b'\xfe\xff', '<?xml version="1.0" encoding="utf-16"?>'),
            ('utf16-signature', 'utf-16-be', b'', '<?xml version="1.0" encoding="utf-16be"?>')]:
        document = decl + ordinary.decode()
        v = positive(name, bom + document.encode(encoding), 'result',
                     {'status': 'ok', 'operation': 'fldigi.log_get_name', 'text': 'Q1AAA'}, 'log.get_name')
        v['expected_message']['xml'] = document


def compatibility_vectors(suite):
    old = copy.deepcopy(suite.vectors['modem-get-name-response']['expected']['event'])
    for name, changes, kind, fields, verification in [
        ('new-minor', {'spec': '0.2.0', 'fields': {'mode': 'BPSK31', 'future': 1}}, 'radio_state', {'mode': 'BPSK31'}, 'verified'),
        ('new-mapping', {'mapping': 2, 'fields': {'mode': 'BPSK31', 'future': 1}}, 'radio_state', {'mode': 'BPSK31'}, 'verified'),
        ('new-kind', {'spec': '0.2.0', 'kind': 'future_kind', 'fields': {'future': 1}}, 'native', {}, 'unavailable'),
        ('old-writer', {}, 'radio_state', {'mode': 'BPSK31'}, 'verified')]:
        event = {**copy.deepcopy(old), **changes}
        suite.add('compatibility-' + name, {'class': 'compatibility',
            'input': {'event': event, 'reader_spec': '0.2.0' if name == 'old-writer' else SPEC,
                      'writer_spec': event['spec'], 'request': {'method': 'modem.get_name'}},
            'expected': {'read': {'kind': kind, 'fields': fields, 'context': event['context'], 'native': event['native']},
                         'verification': {'status': verification}}})
    suite.add('compatibility-no-request-verification', {'class': 'compatibility',
        'input': {'event': {**copy.deepcopy(old), 'mapping': 2}, 'reader_spec': SPEC, 'writer_spec': SPEC},
        'expected': {'read': {k: old[k] for k in ('kind', 'fields', 'context', 'native')},
                     'verification': {'status': 'unavailable'}}})
    command = copy.deepcopy(suite.vectors['modem-get-name-command']['input']['event'])
    command['mapping'] = 2
    suite.add('compatibility-preserved-command', {'class': 'compatibility',
        'input': {'event': command, 'reader_spec': SPEC, 'writer_spec': SPEC},
        'expected': {'read': {'kind': 'control', 'fields': command['fields'], 'context': command['context']},
                     'preserved_command': command, 'verification': {'status': 'unavailable'}}})
    suite.add('compatibility-major', {'class': 'compatibility',
        'input': {'event': {**old, 'spec': '1.0.0'}, 'reader_spec': SPEC, 'writer_spec': '1.0.0'},
        'expected': {'error': {'category': 'unsupported_version'}}})


def release_vectors(positive, negative):
    # Distinct source-derived label changes, not mode x operation combinations.
    for name, method, text, kind, fields in [
            ('modem-name-domexm', 'modem.get_name', 'DOMEXM', 'radio_state', {'mode': 'DOMEXM'}),
            ('modem-name-thorm', 'modem.get_name', 'THORM', 'radio_state', {'mode': 'THORM'}),
            ('modem-name-scampfsk', 'modem.get_name', 'SCAMPFSK', 'radio_state', {'mode': 'SCAMPFSK'}),
            ('modem-mode-scampfsk', 'modem.get_mode', 'MFSK', 'result',
             {'status': 'ok', 'operation': 'fldigi.modem_get_mode', 'text': 'MFSK'}),
            ('modem-submode-scampfsk', 'modem.get_submode', 'SCAMPFSK', 'radio_state', {'sub_mode': 'SCAMPFSK'}),
            ('modem-name-thor25', 'modem.get_name', 'THOR25', 'radio_state', {'mode': 'THOR25'}),
            ('modem-mode-thor25', 'modem.get_mode', 'THOR', 'result',
             {'status': 'ok', 'operation': 'fldigi.modem_get_mode', 'text': 'THOR'}),
            ('modem-submode-thor25-empty', 'modem.get_submode', '', 'radio_state', {'sub_mode': ''}),
            ('modem-submode-slowhell', 'modem.get_submode', 'SLOWHELL', 'radio_state', {'sub_mode': 'SLOWHELL'}),
            ('modem-submode-hellx5', 'modem.get_submode', 'HELLX5', 'radio_state', {'sub_mode': 'HELLX5'}),
            ('modem-submode-hellx9', 'modem.get_submode', 'HELLX9', 'radio_state', {'sub_mode': 'HELLX9'}),
            ('modem-old-label-preserved', 'modem.get_name', 'DOMEX Micro', 'radio_state', {'mode': 'DOMEX Micro'}),
            ('modem-name-not-found', 'modem.set_by_name', 'No such modem', 'result',
             {'status': 'ok', 'operation': 'fldigi.modem_set_by_name', 'text': 'No such modem'})]:
        positive(name, response('<string>' + text + '</string>'), kind, fields, method)
    positive('modem-set-domexm', call('modem.set_by_name', '<string>DOMEXM</string>'),
             'control', {'operation': 'fldigi.modem_set_by_name', 'mode': 'DOMEXM'},
             'modem.set_by_name', is_request=True)
    for method in ('modem.get_afc_search_range', 'modem.set_afc_search_range', 'modem.inc_afc_search_range'):
        positive(method.replace('.', '-').replace('_', '-') + '-unsupported-response',
                 response('<string>Operation not supported by modem</string>'), 'native', {}, method)
    positive('corrected-queue-name-unknown', call('text.add_tx_queue', '<string>Q1AAA</string>'),
             'native', {}, 'text.add_tx_queue', is_request=True)
    positive('bandwidth-zero-response', response('<int>0</int>'), 'native', {}, 'modem.get_bandwidth')
    positive('frequency-setter-nil-response', response('<nil/>'), 'native', {}, 'rig.set_frequency')
    positive('append-double-response', response('<double>14070000.0</double>'), 'native', {}, 'text.add_tx')
    # Literal S12 response framing + empty S3 serialization, excluding HTTP headers.
    negative('bundled-invalid-result-empty-param',
             b'<?xml version="1.0"?>\r\n<methodResponse><params><param>\r\n\t\r\n</param></params></methodResponse>\r\n',
             'malformed', request={'method': 'text.add_tx'})


def validate(suite, substrate, root, producer_strict, producer_factory):
    document = (root / 'spec/annex-e-fldigi.md').read_text()
    # Every defined ID is unique, and all normative paragraphs are addressable.
    ids = re.findall(r'^\*\*`(QGI-FLD-[0-9]{4})` — ', document, re.M)
    assert len(ids) == len(set(ids)) == 69
    assert set(ids) == {f'QGI-FLD-{i:04d}' for i in range(2, 71)}
    rows = re.findall(r'^\| `([a-z][a-z0-9_.]+)` \| `([^`]+)` \|', document, re.M)
    assert dict(rows) == CATALOG, 'catalog prose and pinned vector signatures disagree'
    anchors = ['paired `<params></params>`', 'followed by exactly `.0`', 'quotes, tabs and LF remain literal',
               '71,155 bytes', '32, 256 and 256', 'first failure', 'aggregate result',
               'double for rig.set_frequency, nil for', 'without synthesizing nil',
               'without translating renamed']
    def contract(text):
        return all(a in text for a in anchors)
    assert contract(document)
    for anchor in anchors:
        assert not contract(document.replace(anchor, 'removed')), 'planted prose omission escaped'
    assert len(CATALOG) == 174 and len(MAPPED) == 50
    assert 'pinned to released `4.2.13`' in (root / 'spec/QGI-SPEC-001.md').read_text()
    # Check the maximum integer admitted by the published bound, independently.
    bound = lambda n: 4 * ((n + 2) // 3) + 12*n + 1536 + 32768 + 65536
    assert bound(LIMIT) == 1048576 and bound(LIMIT + 1) > 1048576
    positives = commands = 0
    for name, vector in suite.vectors.items():
        cls, inp, expected = vector['class'], vector['input'], vector['expected']
        try:
            if cls in ('positive', 'negative'):
                event, view = observe(inp, substrate)
                assert cls == 'positive', name + ': negative accepted'
                assert event == expected['event'], (name, event, expected['event'])
                assert view == vector['expected_message'], (name, view, vector['expected_message'])
                assert inp['data'] == vector['reencode']['data'] == event['native']['data']
                with_view = copy.deepcopy(event)
                with_view['native']['message'] = view
                assert len(json.dumps(with_view, ensure_ascii=False, separators=(',', ':')).encode()) <= 1048576
                positives += 1
            elif cls == 'compatibility':
                event = inp['event']
                if event['spec'].split('.')[0] != SPEC.split('.')[0]:
                    assert expected['error']['category'] == 'unsupported_version'
                    continue
                known = event['kind'] in ('radio_state', 'control')
                fields = {k: v for k, v in event['fields'].items() if k in ('mode', 'operation')} if known else {}
                recognized = {'kind': event['kind'] if known else 'native', 'fields': fields,
                              'context': event['context']}
                if 'native' in event:
                    recognized['native'] = event['native']
                assert recognized == expected['read'], name
                if 'preserved_command' in expected:
                    assert event == expected['preserved_command']
                elif expected['verification']['status'] == 'verified':
                    native, _ = observe({'data': event['native']['data'], 'context': event['context'],
                                         'request': inp['request']}, substrate)
                    assert native['fields'] == fields and native['kind'] == recognized['kind']
            elif cls in ('command', 'command_negative'):
                bodies = write_command(inp['event'], inp['options'], substrate)
                assert cls == 'command', name + ': negative command accepted'
                assert [base64.b64encode(b).decode() for b in bodies] == expected['requests'], name
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
        actual, _ = observe(vector['input'], substrate)
        mutant = copy.deepcopy(vector['expected']['event'])
        mutant['fields'][field] = wrong
        assert actual != mutant, name + ': planted mapping failure accepted'
    def command_matches(vector):
        actual = write_command(vector['input']['event'], vector['input']['options'], substrate)
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
            ('command-escaping', b'&#13;', b'\r'),
            ('command-range-zero', b'<int>0</int>', b'<i4>0</i4>')]:
        mutant = copy.deepcopy(suite.vectors[name])
        raw = base64.b64decode(mutant['expected']['requests'][0])
        assert before in raw
        mutant['expected']['requests'][0] = base64.b64encode(raw.replace(before, after)).decode()
        assert not command_matches(mutant), 'numeric/text/type canonical mutant escaped'
    # A sequence's first acknowledgment must satisfy its actual mapped type.
    # Malformed bundled replies, faults, lost association and type swaps stop it.
    def can_advance(method, inp):
        if inp.get('request', {}).get('method') != method:
            return False
        try:
            event, _ = observe(inp, substrate)
        except ValueError:
            return False
        return (event['kind'] == 'result' and event['fields'].get('status') == 'ok'
                and event['fields'].get('operation') == operation(method))
    for method, accepted, stopped in [
            ('rig.set_frequency', 'rig-set-frequency-response', 'frequency-setter-nil-response'),
            ('text.add_tx', 'text-add-tx-response', 'append-double-response')]:
        assert can_advance(method, suite.vectors[accepted]['input'])
        assert not can_advance(method, suite.vectors[stopped]['input'])
        lost = copy.deepcopy(suite.vectors[accepted]['input'])
        lost.pop('request')
        assert not can_advance(method, lost)
    assert not can_advance('text.add_tx', suite.vectors['bundled-invalid-result-empty-param']['input'])
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
    return f'fldigi: {positives} independent XML/type/mapping/view/reencode checks, {commands} exact command sequences; strict schema and planted failures passed'


def manifest(suite, dump, root):
    files = {name + '.json': dump(value) for name, value in suite.vectors.items()}
    # All evidence hashes are transcribed from E.1; no network at generation time.
    document = (root / 'spec/annex-e-fldigi.md').read_text()
    sources = re.findall(r'^\| S([0-9]+) \| \[([^\]]+)\]\(([^)]+)\) \| `([0-9a-f]{64})` \|', document, re.M)
    assert len(sources) == 16
    source_hashes = {f'fldigi-{RELEASE}/{member}' if url == ARCHIVE_URL else url: digest
                     for _, member, url, digest in sources}
    assert len(source_hashes) == 16, 'archive member hashes must not collapse onto one URL'
    assert f'SHA-256: `{ARCHIVE_SHA256}`' in document and ARCHIVE_URL in document
    files['manifest.json'] = dump({'$comment': 'SPDX-License-Identifier: GPL-3.0-or-later',
        'spec': SPEC, 'family': 'fldigi', 'mapping': 1, 'retrieved': RETRIEVED,
        'release': RELEASE,
        'source_archive': {'url': ARCHIVE_URL, 'sha256': ARCHIVE_SHA256, 'root': f'fldigi-{RELEASE}/'},
        'release_listing': {'url': 'https://sourceforge.net/projects/fldigi/files/fldigi/',
                            'source_modified_utc': '2026-08-03 14:16:04'},
        'source_sha256': source_hashes, 'profile_sha256': hashlib.sha256(document.encode()).hexdigest(),
        'hash_scope': 'SHA-256 of each complete vector file, including its final newline',
        'vectors': [{'file': name, 'class': suite.vectors[name[:-5]]['class'], 'spec': SPEC,
                     'family': 'fldigi', 'mapping': 1, 'sha256': hashlib.sha256(data).hexdigest()}
                    for name, data in sorted(files.items())]})
    return {'fldigi/' + name: data for name, data in files.items()}
