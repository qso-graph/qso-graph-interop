#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Independent struct-based Qt wire vectors; no Rust encoder is used.

The heartbeat, status, decode and header-only builders port the Python reference.
Other layouts follow the released headers pinned in docs/PROTOCOL.md.
Run with --check to prove the checked-in vectors are reproducible.
"""
import argparse
import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parent.parent
DEST = ROOT / 'crates/qso-graph-udp/tests/fixtures/wire.json'
SCHEMA = 3

def uint8(v): return struct.pack('>B', v)
def uint32(v): return struct.pack('>I', v)
def int32(v): return struct.pack('>i', v)
def uint64(v): return struct.pack('>Q', v)
def float64(v): return struct.pack('>d', v)
def boolean(v): return uint8(int(v))
def utf8(v):
    if v is None: return uint32(0xffffffff)
    b = bytes(v)
    return uint32(len(b)) + b

def text(s): return list(s.encode('utf8'))
def qstring(v):
    if v is None: return uint32(0xffffffff)
    return uint32(2 * len(v)) + b''.join(struct.pack('>H', i) for i in v)
def string(s): return list(struct.unpack('>' + 'H'*(len(s.encode('utf-16-be'))//2), s.encode('utf-16-be')))
def qdatetime(v):
    b = struct.pack('>qIB', v['julian_day'], v['time'], v['time_spec'])
    if SCHEMA >= 2 and v['time_spec'] == 2: b += int32(v['offset_seconds'])
    if SCHEMA >= 2 and v['time_spec'] == 3:
        z = v['time_zone']
        if 'Iana' in z: b += qstring(z['Iana'])
        else:
            z = z['Custom']
            b += qstring(string('OffsetFromUtc')) + qstring(z['id']) + int32(z['offset_seconds']) + qstring(z['name']) + qstring(z['abbreviation']) + int32(z['country']) + qstring(z['comment'])
    return b

def color(v): return uint8(v['spec']) + b''.join(struct.pack('>H', n) for n in v['channels'])
def dt(jd=2461353, ms=45300000, spec=1, offset=None, zone=None):
    return dict(julian_day=jd, time=ms, time_spec=spec, offset_seconds=offset, time_zone=zone)

# Required fields are always present. None for a tail means absent; None for
# a QByteArray means a transmitted null. Field order is explicitly independent.
layouts = {
'Heartbeat': ('u32? utf8? utf8?', 'max_schema version revision'),
'Status': ('u64 utf8 utf8 utf8 utf8 bool bool bool u32 u32 utf8? utf8? utf8? bool? utf8? bool? u8? u32? u32? utf8? utf8?', 'dial_frequency_hz mode dx_call report tx_mode tx_enabled transmitting decoding rx_df_hz tx_df_hz de_call de_grid dx_grid tx_watchdog sub_mode fast_mode special_operation_mode frequency_tolerance_hz tr_period_seconds configuration_name tx_message'),
'Decode': ('bool u32 i32 f64 u32 utf8 utf8 bool? bool?', 'new time snr_db delta_time_seconds delta_frequency_hz mode message low_confidence off_air'),
'Clear': ('u8?', 'window'),
'Reply': ('u32 i32 f64 u32 utf8 utf8 bool? u8?', 'time snr_db delta_time_seconds delta_frequency_hz mode message low_confidence modifiers'),
'QsoLogged': ('datetime utf8 utf8 u64 utf8 utf8 utf8 utf8 utf8 utf8 datetime? utf8? utf8? utf8? utf8? utf8? utf8?', 'time_off dx_call dx_grid frequency_hz mode report_sent report_received tx_power comments name time_on operator_call my_call my_grid exchange_sent exchange_received propagation_mode'),
'Close': ('',''), 'Replay': ('',''),
'HaltTx': ('bool','auto_tx_only'),
'FreeText': ('utf8 bool','text send'),
'WsprDecode': ('bool u32 i32 f64 u64 i32 utf8 utf8 i32 bool?', 'new time snr_db delta_time_seconds frequency_hz drift_hz callsign grid power_dbm off_air'),
'Location': ('utf8', 'location'),
'LoggedAdif': ('utf8', 'adif'),
'HighlightCallsign': ('utf8 color color bool?', 'callsign background foreground highlight_last'),
'SwitchConfiguration': ('utf8','configuration_name'),
'Configure': ('utf8 u32 utf8 bool u32 u32 utf8 utf8 bool?', 'mode frequency_tolerance_hz sub_mode fast_mode tr_period_seconds rx_df_hz dx_call dx_grid generate_messages'),
'AnnotationInfo': ('utf8 bool u32','dx_call sort_order_provided sort_order'),
'JtdxStatus': ('u64 utf8 utf8 utf8 utf8 bool bool bool i32 i32 utf8? utf8? utf8? bool? utf8? bool? bool?', 'dial_frequency_hz mode dx_call report tx_mode tx_enabled transmitting decoding rx_df_hz tx_df_hz de_call de_grid dx_grid tx_watchdog sub_mode fast_mode tx_first'),
'SetTxDeltaFreq': ('u32','tx_delta_frequency_hz'),
'TriggerCq': ('utf8 bool? bool','direction tx_period send'),
}
numbers = dict(zip(['Heartbeat','Status','Decode','Clear','Reply','QsoLogged','Close','Replay','HaltTx','FreeText','WsprDecode','Location','LoggedAdif','HighlightCallsign','SwitchConfiguration','Configure','AnnotationInfo'], range(17)))
numbers.update(JtdxStatus=1, SetTxDeltaFreq=50, TriggerCq=51)
writers = dict(u8=uint8, u32=uint32, i32=int32, u64=uint64, f64=float64, bool=boolean, utf8=utf8, datetime=qdatetime, color=color)

def vector(name, kind, fields=None, variant='Wsjtx', sender='WSJT-X', schema=3, extensions=b''):
    global SCHEMA
    SCHEMA = schema
    b = struct.pack('>III',0xadbccbda,schema,numbers[kind]) + utf8(text(sender))
    types, keys = layouts[kind]
    for t,k in zip(types.split(), keys.split()):
        v=fields[k]
        if t.endswith('?'):
            if v is None: continue
            t=t[:-1]
        b+=writers[t](v)
    b+=extensions
    expected = None if fields is None else dict(fields)
    for t,k in zip(types.split(),keys.split()):
        if t.rstrip('?')=='utf8' and (not t.endswith('?') or fields[k] is not None): expected[k]={'bytes':fields[k]}
    return dict(name=name, wire_hex=b.hex(), packet=dict(schema=schema,variant=variant,id={'bytes':text(sender)},body={kind:expected} if fields is not None else kind,extensions=list(extensions)))


def vectors():
    h = dict(max_schema=3,version=text('3.3.0'),revision=text('abc'))
    s = dict(dial_frequency_hz=14074000,mode=text('FT8'),dx_call=text('W1AW'),report=text('-07'),tx_mode=text('FT4'),tx_enabled=False,transmitting=False,decoding=True,rx_df_hz=1201,tx_df_hz=2302,de_call=text('N0CALL'),de_grid=text('DN13'),dx_grid=text('FN31'),tx_watchdog=False,sub_mode=text('A'),fast_mode=False,special_operation_mode=3,frequency_tolerance_hz=47,tr_period_seconds=30,configuration_name=text('Field'),tx_message=text('CQ TEST'))
    d = dict(new=True,time=(23*3600+39*60+15)*1000,snr_db=-12,delta_time_seconds=0.2,delta_frequency_hz=1234,mode=text('~'),message=text('CQ K7ABC DN13'),low_confidence=False,off_air=False)
    q = dict(time_off=dt(),dx_call=text('W1AW'),dx_grid=text('FN31'),frequency_hz=14074000,mode=text('FT8'),report_sent=text('-12'),report_received=text('-10'),tx_power=text('5'),comments=text('Test QSO'),name=text('Example'),time_on=dt(ms=45255000),operator_call=text('K7ABC'),my_call=text('N0CALL'),my_grid=text('DN13'),exchange_sent=text('1D CT'),exchange_received=text('2A CT'),propagation_mode=text('TR'))
    w = dict(new=True,time=45000000,snr_db=-24,delta_time_seconds=-0.3,frequency_hz=14097100,drift_hz=-1,callsign=text('N0CALL'),grid=text('FN31'),power_dbm=37,off_air=False)
    r = {k:v for k,v in d.items() if k not in ('new','off_air')}; r['modifiers']=4
    b = dict(Heartbeat=h,Status=s,Decode=d,Clear=dict(window=2),Reply=r,QsoLogged=q,Close=None,Replay=None,HaltTx=dict(auto_tx_only=True),FreeText=dict(text=text('CQ TEST'),send=True),WsprDecode=w,Location=dict(location=text('FN31pr')),LoggedAdif=dict(adif=text('<adif_ver:5>3.1.0<EOH><CALL:4>W1AW<EOR>')),HighlightCallsign=dict(callsign=text('W1AW'),background=dict(spec=1,channels=[65001,1101,2202,3303,4404]),foreground=dict(spec=1,channels=[64002,5505,6606,7707,8808]),highlight_last=True),SwitchConfiguration=dict(configuration_name=text('Field')),Configure=dict(mode=text('FT8'),frequency_tolerance_hz=73,sub_mode=text('B'),fast_mode=False,tr_period_seconds=30,rx_df_hz=2890,dx_call=text('W1AW'),dx_grid=text('FN31'),generate_messages=True),AnnotationInfo=dict(dx_call=text('W1AW'),sort_order_provided=True,sort_order=0xffffffff))
    out=[vector('wsjtx_'+k,k,f) for k,f in b.items()]
    for k in ['Heartbeat','Status','Decode','Clear','Reply','QsoLogged','Close','Replay','HaltTx','FreeText','WsprDecode','LoggedAdif']:
        f = None if b[k] is None else dict(b[k])
        kind = k
        if k=='Status':
            kind='JtdxStatus'
            f={key:f[key] for key in layouts[kind][1].split() if key!='tx_first'}
            f.update(rx_df_hz=-150,tx_df_hz=-300,tx_first=True)
        elif k=='Heartbeat':f['revision']=None
        elif k=='Clear':f['window']=None
        elif k=='Reply':f.update(low_confidence=None,modifiers=None)
        elif k=='QsoLogged':f.update(exchange_sent=None,exchange_received=None,propagation_mode=None)
        elif k=='WsprDecode':f['off_air']=None
        out.append(vector('jtdx_'+k,kind,f,'Jtdx','JTDX'))
    out.append(vector('jtdx_SetTxDeltaFreq','SetTxDeltaFreq',dict(tx_delta_frequency_hz=1800),'Jtdx','JTDX'))
    out.append(vector('jtdx_TriggerCq','TriggerCq',dict(direction=text('DX'),tx_period=True,send=True),'Jtdx','JTDX'))
    out.append(vector('jtdx_TriggerCq_source','TriggerCq',dict(direction=text('DX'),tx_period=None,send=True),'Jtdx','JTDX'))
    out.append(vector('jtdx_Reply_source','Reply',r,'Jtdx','JTDX'))
    out.append(vector('jtdx_WsprDecode_source','WsprDecode',w,'Jtdx','JTDX'))
    # Give every boolean position a unique signature across independent vectors.
    # All-false/all-true also exercise both values for every present boolean.
    boolean_bases = [v for v in out if isinstance(v['packet']['body'], dict)]
    for base in boolean_bases:
        kind, fields = next(iter(base['packet']['body'].items()))
        keys = [k for t,k in zip(*[v.split() for v in layouts[kind]])
                if t.rstrip('?') == 'bool' and fields[k] is not None]
        if not keys: continue
        # Reconstruct native builder input (vector wraps UTF-8 fields in JSON objects).
        native = {k: (v['bytes'] if isinstance(v,dict) and 'bytes' in v else v)
                  for k,v in fields.items()}
        patterns = [[bool((index+1) & (1 << bit)) for index in range(len(keys))]
                    for bit in range(len(keys).bit_length())]
        patterns += [[False]*len(keys), [True]*len(keys)]
        for index, pattern in enumerate(patterns):
            values = dict(native)
            values.update(zip(keys, pattern))
            out.append(vector(f"{base['name']}_bool_{index}", kind, values,
                              base['packet']['variant'], bytes(base['packet']['id']['bytes']).decode()))
    # Date/time vectors cover both working schemas and Qt 5.0's raw legacy layout.
    for schema in [1,2,3]:
        for spec in [0,1,2,3]:
            v=dict(q)
            v['time_off']=dt(spec=spec,offset=3600 if schema>=2 and spec==2 else None,zone={'Iana':string('America/New_York')} if schema>=2 and spec==3 else None)
            out.append(vector(f'datetime_s{schema}_spec{spec}','QsoLogged',v,schema=schema))
    v=dict(q); v['time_off']=dt(jd=-(1<<63),ms=0xffffffff,spec=0)
    out.append(vector('datetime_null','QsoLogged',v))
    v=dict(q);v['time_off']=dt(spec=3,zone={'Custom':dict(id=string('Test/Zone'),offset_seconds=-18000,name=string('Test'),abbreviation=string('TST'),country=225,comment=None)})
    out.append(vector('datetime_custom_zone','QsoLogged',v))
    out.append(vector('prototype_Clear','Clear',dict(window=None)))
    out.append(vector('prototype_Heartbeat_old','Heartbeat',dict(max_schema=None,version=None,revision=None),schema=2))
    v=dict(s)
    for key in layouts['Status'][1].split()[10:]:v[key]=None
    out.append(vector('prototype_Status_old','Status',v))
    out.append(vector('null_string','FreeText',dict(text=None,send=False)))
    out.append(vector('empty_string','FreeText',dict(text=[],send=False)))
    out.append(vector('utf8_unicode','FreeText',dict(text=text('CQ café 日本'),send=True)))
    out.append(vector('utf8_invalid','FreeText',dict(text=[255,254,0,192],send=False)))
    out.append(vector('future_tail','Decode',d,extensions=b'\x00\x01\x02'))
    # The entire received-decode sample, not a selected subset.
    for index,line in enumerate((DEST.parent/'ALL-sample.TXT').read_text().splitlines()):
        stamp,mhz,direction,mode,snr,delta,df,message=line.split(maxsplit=7)
        clock=stamp.split('_')[1]
        ms=(int(clock[:2])*3600+int(clock[2:4])*60+int(clock[4:])*1)*1000
        out.append(vector(f'sample_{index:03}','Decode',dict(new=True,time=ms,snr_db=int(snr),delta_time_seconds=float(delta),delta_frequency_hz=int(df),mode=text('~'),message=text(message),low_confidence=False,off_air=False)))
    return out


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    data=json.dumps(dict(spdx='GPL-3.0-or-later',vectors=vectors()),indent=2)+'\n'
    if args.check:
        if DEST.read_text()!=data: raise SystemExit('wire.json differs; run scripts/generate-fixtures.py')
        print(f'{len(vectors())} independent fixtures: pass')
    else: DEST.write_text(data)

if __name__=='__main__':main()
