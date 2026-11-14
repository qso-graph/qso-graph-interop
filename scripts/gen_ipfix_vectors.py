# SPDX-License-Identifier: GPL-3.0-or-later
"""Independent Annex H wire packers and literal common-field oracles; no I/O or peers."""
import base64
import hashlib
import json
import struct


def pack(fmt, *values):
    return struct.pack('>' + fmt, *values)


def b64(raw):
    return base64.b64encode(raw).decode()


def variable(raw):
    if isinstance(raw, str):
        raw = raw.encode()
    return (bytes([len(raw)]) if len(raw) < 255 else b'\xff' + pack('H', len(raw))) + raw


def template(tid, fields, scope=None):
    return pack('HH', tid, len(fields)) + (b'' if scope is None else pack('H', scope)) + b''.join(
        pack('HH', field if pen == 0 else field | 0x8000, length) + (pack('I', pen) if pen else b'')
        for pen, field, length in fields)


def set_bytes(tid, data, padding=True):
    size = len(data) + 4
    pad = (-size % 4) if padding else 0
    return pack('HH', tid, size + pad) + data + bytes(pad)


def message(*sets, domain=0x51474901, sequence=0, time=1700000000):
    return pack('HHIII', 10, 16 + sum(map(len, sets)), time, sequence, domain) + b''.join(sets)


WS = [(30351,1,65535),(30351,5,5),(30351,6,1),(30351,10,65535),(30351,3,65535),(30351,11,1),(0,150,4)]
WR = [(30351,i,65535) for i in (2,4,8,9,13)]
FS = [(30351,1,65535),(0,150,4),(30351,5,4),(30351,10,65535),(30351,3,65535),(30351,11,1)]
FR = [(30351,2,65535),(30351,4,65535),(30351,12,8),(30351,8,65535),(30351,9,65535)]
CTX = {'direction':'unknown','exchange':'unknown'}
RX = {'receiver_call':'Q2AAA','receiver_locator':'AA00aa','reporting_software':'synthetic-1','antenna_information':'dipole'}
TX = {'sender_call':'Q1AAA','sender_locator':'BB11bb','frequency_hz':14074000,'mode':'FT8','information_source':1,'time':'2023-11-14T22:13:20Z'}


def profile(wsjtx=True, short=False, source=1):
    sid, rid = (0x50e3,0x50e2) if wsjtx else (0x022c,0x0218 if short else 0x0219)
    st = template(sid, WS if wsjtx else FS)
    rt = template(rid, WR if wsjtx else FR[:3] if short else FR, 1 if wsjtx else 0)
    rf = dict(RX)
    if wsjtx:
        rf['rig_information'] = 'synthetic rig'
    else:
        rf['persistent_identifier'] = 'QGI00001'
    if short:
        rf.pop('reporting_software')
        rf.pop('antenna_information')
    r = variable(rf['receiver_call']) + variable(rf['receiver_locator'])
    if not wsjtx:
        r += b'QGI00001'
    if not short:
        r += variable('synthetic-1') + variable('dipole')
    if wsjtx:
        r += variable('synthetic rig')
    tf = dict(TX, information_source=source)
    if wsjtx:
        tf['snr_db'] = -12
    t = variable('Q1AAA')
    if not wsjtx:
        t += pack('I',1700000000)
    t += (14074000).to_bytes(5 if wsjtx else 4,'big')
    if wsjtx:
        t += pack('b',-12)
    t += variable('FT8') + variable('BB11bb') + bytes([source])
    if wsjtx:
        t += pack('I',1700000000)
    raw = message(set_bytes(2,st),set_bytes(3,rt),set_bytes(rid,r),set_bytes(sid,t))
    header = dict(version=10,length=len(raw),export_time=1700000000,sequence_number=0,observation_domain_id=0x51474901)
    def evidence(data, templ, tid):
        return dict(data=b64(data),template=b64(templ),header=header,set_id=tid)
    def event(kind,fields,data,templ,tid,index):
        return dict(spec='0.2.0',family='pskreporter-ipfix',mapping=1,native_version='10',kind=kind,context=CTX,
                    fields=fields,native=dict(format='ipfix_record',message_data=b64(raw),set_index=index,record_index=0,**evidence(data,templ,tid)))
    receiver=event('receiver_information',rf,r,rt,rid,2)
    spot=event('spot',dict(tf,**rf),t,st,sid,3)
    spot['native']['dependencies']=[evidence(r,rt,rid)]
    return raw,[receiver,spot]


def outputs(root, dump, producer=None):
    vectors={}
    def add(name, cls, raw, expected):
        vectors[name]=dict({'$comment':'SPDX-License-Identifier: GPL-3.0-or-later'},spec='0.2.0',family='pskreporter-ipfix',mapping=1,
                           **{'class':cls,'input':{'data':b64(raw)},'expected':expected})
    for name, wsjtx, short, source in [('wsjtx',True,False,1),('fldigi',False,False,1),('fldigi-short',False,True,2),('qso',True,False,2),('manual-test',False,False,131)]:
        raw,events=profile(wsjtx,short,source)
        for event in events:
            if producer:
                producer.validate(event)
        add(name,'templated',raw,{'events':events,'skips':[]})
    add('template-only','templated',message(set_bytes(2,template(256,WS))),{'events':[],'skips':[]})
    unknown=message(set_bytes(999,b'opaque',False))
    add('unknown-template','skip',unknown,{'skip':{'reason':'unknown_template','native_version':'10','type':999}})
    raw,_=profile()
    # Literal common oracle for page-only IEs plus an unknown enterprise attribute.
    extra_template=template(257,[(30351,1,65535),(30351,7,1),(30351,14,2),(30351,15,2),(30351,16,1),(424242,99,65535)])
    extra_data=variable('Q7TEST')+pack('b',-33)+bytes.fromhex('aaff800080')+variable(bytes.fromhex('fffe'))
    extra_raw=message(set_bytes(2,extra_template),set_bytes(257,extra_data))
    extra_header=dict(version=10,length=len(extra_raw),export_time=1700000000,sequence_number=0,observation_domain_id=0x51474901)
    extra_event=dict(spec='0.2.0',family='pskreporter-ipfix',mapping=1,native_version='10',kind='spot',context=CTX,
        fields={'sender_call':'Q7TEST','imd_db':-33},native=dict(format='ipfix_record',data=b64(extra_data),template=b64(extra_template),header=extra_header,set_id=257,set_index=1,record_index=0,message_data=b64(extra_raw)))
    if producer:
        producer.validate(extra_event)
    add('page-elements-and-unknown-enterprise','templated',extra_raw,{'events':[extra_event],'skips':[]})
    add('udp-withdrawals-ignored','templated',message(set_bytes(2,pack('HH',2,0)),set_bytes(3,pack('HH',3,0))),{'events':[],'skips':[]})
    bad={
        'header-truncated':(raw[:15],'truncated'),
        'set-truncated':(message(b'\x00\x02\x00'),'truncated'),
        'set-overrun':(message(pack('HH',2,100)),'truncated'),
        'set-length-below-header':(message(pack('HH',2,3)),'malformed'),
        'message-length':(raw[:-1],'malformed'),
        'reserved-set':(message(set_bytes(4,b'')),'malformed'),
        'template-truncated':(message(set_bytes(2,pack('HHHH',256,1,0x8001,65535),False)),'truncated'),
        'scope-zero-unknown':(message(set_bytes(3,template(256,WS,0))),'malformed'),
        'scope-too-large':(message(set_bytes(3,template(256,WS,8))),'malformed'),
        'template-fields-overflow':(message(set_bytes(2,template(256,[(0,1,1)]*65))),'oversized'),
        'template-overflow':(message(set_bytes(2,b''.join(template(i,[(0,1,1)]) for i in range(256,321)))),'oversized'),
        'zero-record-length':(message(set_bytes(2,template(256,[(0,1,0)]))),'malformed'),
        'record-truncated':(message(set_bytes(2,template(256,[(30351,1,65535)])),set_bytes(256,b'\x09abc',False)),'truncated'),
        'variable-prefix-truncated':(message(set_bytes(2,template(256,[(30351,1,65535)])),set_bytes(256,b'\xff\x01',False)),'truncated'),
        'variable-length-overrun':(message(set_bytes(2,template(256,[(30351,1,65535)])),set_bytes(256,b'\xff\xff\xffabc',False)),'truncated'),
        'padding-not-shorter-than-record':(message(set_bytes(2,template(256,[(30351,1,65535)])),set_bytes(256,b'\xff\x01\x00',False)),'truncated'),
        'event-overflow':(message(set_bytes(2,template(256,[(0,1,1)])),set_bytes(256,b'x'*4097,False)),'oversized'),
        'message-overflow':(bytes(60001),'oversized'),
    }
    for name,(wire,category) in bad.items():
        add(name,'negative',wire,{'error':{'category':category}})
    result={f'pskreporter-ipfix/{name}.json':dump(value) for name,value in vectors.items()}
    result['pskreporter-ipfix/manifest.json']=dump(dict({'$comment':'SPDX-License-Identifier: GPL-3.0-or-later'},spec='0.2.0',family='pskreporter-ipfix',mapping=1,
        source='../../annex-h-pskreporter-ipfix.md',source_sha256=hashlib.sha256((root/'spec/annex-h-pskreporter-ipfix.md').read_bytes()).hexdigest(),retrieved='2026-11-14T17:28:27Z',
        vectors=[dict(file=name+'.json',spec='0.2.0',family='pskreporter-ipfix',mapping=1,**{'class':value['class'],'sha256':hashlib.sha256(result[f'pskreporter-ipfix/{name}.json']).hexdigest()}) for name,value in sorted(vectors.items())]))
    fixture={'$comment':'SPDX-License-Identifier: GPL-3.0-or-later','source':'Annex H source-derived independent program profiles','profiles':{name:{'wire_hex':profile(ws,short)[0].hex(),'events':profile(ws,short)[1]} for name,ws,short in [('wsjtx',True,False),('fldigi',False,False),('fldigi-short',False,True)]}}
    target=root/'crates/qso-graph-interop/tests/fixtures/pskreporter-ipfix.json'
    print(f"PSK Reporter IPFIX: {len(vectors)} independently packed vectors; both program profiles and schemas checked")
    return result, target, dump(fixture)
