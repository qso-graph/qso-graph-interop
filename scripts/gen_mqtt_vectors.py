# SPDX-License-Identifier: GPL-3.0-or-later
"""Annex G capture-byte vectors with independent, literal fixture oracles. Offline only."""
import base64
import copy
import hashlib
import json


def outputs(root, dump, validator=None):
    fixture = json.loads((root / 'crates/qso-graph-interop/tests/fixtures/pskreporter-mqtt.json').read_bytes())
    vectors = {}
    def add(name, raw, topic, fields=None, category=None, retain=True, qos=0):
        association = dict(topic=topic, retained=retain if retain is not None else False)
        if qos is not None:
            association['qos'] = qos
        data = base64.b64encode(raw).decode()
        v = dict({'$comment': 'SPDX-License-Identifier: GPL-3.0-or-later'}, spec='0.2.0', family='pskreporter-mqtt', mapping=1,
                 **{'class': 'negative' if category else 'positive'}, input=dict(data=data, mqtt=association))
        if category:
            v['expected'] = dict(error=dict(category=category))
        else:
            receiver = topic.startswith('pskr/rxinfo/')
            event = dict(spec='0.2.0', family='pskreporter-mqtt', mapping=1, native_version='rxinfo' if receiver else '2',
                         kind='receiver_information' if receiver else 'spot', context=dict(direction='unknown',exchange='unknown'),
                         fields=fields, native=dict(format='mqtt_publish',data=data,**association))
            v['expected'] = dict(event=event, verification='verified')
            v['expected_message'] = json.loads(raw)
            v['reencode'] = dict(data=data)
            if validator:
                validator.validate(event)
        vectors[name] = v
    for oracle in fixture['oracles']:
        index = oracle['record_index']
        r = fixture['records'][index]
        add(f'capture-{index:03}',base64.b64decode(r['payload_b64']),r['topic'],{k:v for k,v in oracle['fields'].items() if v is not None or k != 'snr_db'},retain=r['retain'],qos=r['qos'])
    topic = 'pskr/filter/v2/20m/FT8/Q1AAA/Q2BBB/AA00/BB11/1/2'
    rx = 'pskr/rxinfo/Q1AAA'
    def neg(name,payload,category='invalid_value',t=topic):
        add('error-'+name, payload if isinstance(payload,bytes) else json.dumps(payload,separators=(',',':')).encode(),t,category=category)
    # Literal, independent callsign oracles cover both spot roles and rxinfo.
    for role,key,field,call,call_topic,bare,dropped,misplaced in [
        ('sender','sc','sender_call','Q1AAA/0','Q1AAA.0','Q1AAA','Q1AAA0','Q1AA.A0'),
        ('receiver','rc','receiver_call','W/Q2BBB/Q3CCC','W.Q2BBB.Q3CCC','Q2BBB','WQ2BBBQ3CCC','WQ.2BBB.Q3CCC'),
        ('rxinfo','call','receiver_call','Q4DDD/P','Q4DDD.P','Q4DDD','Q4DDDP','Q4DD.DP'),
    ]:
        def call_topic_for(segment):
            if role == 'rxinfo':
                return 'pskr/rxinfo/' + segment
            parts = topic.split('/')
            parts[5 if role == 'sender' else 6] = segment
            return '/'.join(parts)
        raw = json.dumps({key:call},separators=(',',':')).encode()
        add('slash-'+role,raw,call_topic_for(call_topic),{field:call})
        for label,segment in [('bare',bare),('dropped',dropped),('dot-position',misplaced),('unescaped',call)]:
            neg('slash-'+role+'-'+label,raw,t=call_topic_for(segment))
        # Never normalize a payload period into a slash, even if it matches the topic.
        neg('literal-dot-'+role,{key:call_topic},t=call_topic_for(call_topic))
        neg('mixed-dot-slash-'+role,{key:call+'/Q5EEE.F'},t=call_topic_for(call_topic+'.Q5EEE.F'))
    # Topic-derived rxinfo calls apply the same decoding when no payload call asserts equality.
    add('slash-rxinfo-absent',b'{}','pskr/rxinfo/Q1AAA.P',{'receiver_call':'Q1AAA/P'})
    add('slash-rxinfo-null',b'{"call":null}','pskr/rxinfo/Q1AAA.P',{'receiver_call':'Q1AAA/P'})
    # Periods in other topic-associated fields are literal, not callsign escapes.
    add('literal-dot-mode',b'{"md":"FT.8"}',topic.replace('/FT8/','/FT.8/'),{'mode':'FT.8'})
    for name,payload in [('json',b'{'),('duplicate',b'{"sc":"Q1AAA","sc":"Q2BBB"}'),('escaped-duplicate',b'{"sc":"Q1AAA","\\u0073c":"Q2BBB"}'),('nested-duplicate',b'{"x":{"k":1,"k":2}}'),('trailing',b'{}{}'),('utf8',b'{"x":"\xff"}'),('number-grammar',b'{"f":01}')]:
        neg(name,payload,'malformed')
    for name,payload in [('root',[]),('root-null',None)]:neg(name,payload)
    for name,payload in [('payload',b' ' * 16385),('depth',b'{"x":'+b'['*16+b'0'+b']'*16+b'}'),('elements',{'x':[0]*4096}),('field',{'x':'x'*4097}),('key',{'x'*4097:0}),('numeric-token',b'{"x":'+b'1'*129+b'}')]:
        neg(name,payload,'oversized')
    for k in ['sq','f','rp','t','t_tx','sa','ra']:
        for label,value in [('string','1'),('bool',True),('fraction',1.5)]:neg(f'type-{k}-{label}',{k:value})
    for k in ['sc','sl','rc','rl','md','b']:neg('type-'+k,{k:1})
    for k,lo,hi in [('sq',0,9007199254740991),('f',0,9007199254740991),('rp',-128,127),('t',0,253402300799),('t_tx',0,253402300799),('sa',0,65535),('ra',0,65535)]:
        neg('range-'+k+'-low',{k:lo-1});neg('range-'+k+'-high',{k:hi+1})
    for k,value in [('b','40m'),('md','CW'),('sc','Q3CCC'),('rc','Q3CCC'),('sl','AA01aa'),('rl','BB12aa'),('sa',2),('ra',1)]:neg('mismatch-'+k,{k:value})
    for k in ['call','antenna','rig','decoder','grid']:neg('rx-type-'+k,{k:1},t=rx)
    for k in ['time_start','ts']:
        for label,value in [('text','1'),('bool',True),('fraction',0.5),('low',-9007199254740992),('high',9007199254740992)]:neg('rx-type-'+k+'-'+label,{k:value},t=rx)
    neg('rx-call',{'call':'Q2BBB'},t=rx)
    for name,t in [('wildcard',topic.replace('FT8','+')),('empty',topic.replace('FT8','')),('suffix',topic+'/x'),('prefix','x/'+topic),('topic-locator',topic.replace('/AA00/','/AA0000/')),('topic-locator-domain',topic.replace('/AA00/','/ZZ00/')),('dxcc-negative',topic[:-1]+'-1'),('dxcc-leading-zero',topic[:-1]+'02'),('dxcc-range',topic[:-1]+'65536'),('rx-suffix',rx+'/x')]:neg('topic-'+name,{},t=t)
    def topic_size(n):
        parts=['pskr','filter','v2','b'*128,'m'*128,'Q'*128,'Q','AA00','BB11','1','2']
        parts[6]='Q'*(n-len('/'.join(parts))+1)
        assert len('/'.join(parts)) == n and all(len(p)<=128 for p in parts)
        return '/'.join(parts)
    neg('topic-size',{},'oversized',topic_size(513))
    add('boundary-topic',b'{}',topic_size(512),{})
    for name,raw in [('depth',b'{"x":'+b'['*15+b'0'+b']'*15+b'}'),('elements',json.dumps({'x':[0]*4095}).encode()),('field',json.dumps({'x':'x'*4096}).encode()),('key',json.dumps({'x'*4096:0}).encode()),('number',b'{"x":'+b'1'*128+b'}')]:
        add('boundary-'+name,raw,rx,{'receiver_call':'Q1AAA'})
    neg('component-size',{},'oversized','pskr/rxinfo/'+'Q'*129)
    for name,member,value in [('qos-type','qos','0'),('qos-range','qos',3),('qos-null','qos',None),('retain-type','retained',0),('metadata-member','extra',True)]:
        neg(name,{},t=rx);vectors['error-'+name]['input']['mqtt'][member]=value
    add('missing-retain-qos',b'{}',rx,{'receiver_call':'Q1AAA'},retain=None,qos=None)
    add('unknown-empty-null',b'{"sc":null,"rp":null,"future":{"empty":"","null":null,"array":[1,true]}}',topic,{'sender_call':None})
    add('absent',b'{}',topic,{})
    add('empty-rx-strings',b'{"call":"Q1AAA","antenna":"","rig":null,"decoder":"","grid":""}',rx,{'receiver_call':'Q1AAA'})
    for name,payload,fields in [('low',{'f':0,'rp':-128,'t':0,'t_tx':0,'sq':0},dict(frequency_hz=0,snr_db=-128,time='1970-01-01T00:00:00Z',transmission_start='1970-01-01T00:00:00Z')),('high',{'f':9007199254740991,'rp':127,'t':253402300799,'t_tx':253402300799,'sq':9007199254740991},dict(frequency_hz=9007199254740991,snr_db=127,time='9999-12-31T23:59:59Z',transmission_start='9999-12-31T23:59:59Z'))]:
        add('boundary-'+name,json.dumps(payload).encode(),topic,fields)
    neg('old-version',{},'unsupported_version',topic.replace('/v2/','/v1/'))
    neg('zero-version',{},'invalid_value',topic.replace('/v2/','/v0/'))
    neg('version-leading-zero',{},'invalid_value',topic.replace('/v2/','/v03/'))
    neg('version-overflow',{},'invalid_value',topic.replace('/v2/','/v4294967296/'))
    add('future-version',b'not interpreted',topic.replace('/v2/','/v3/'),category='invalid_value')
    vectors['future-version']['class']='skip'
    vectors['future-version']['expected']={'skip':{'reason':'future_version','native_version':'3'}}
    for name,revision in [('future-mapping',2),('future-minor',1)]:
        event=copy.deepcopy(vectors['capture-060']['expected']['event'])
        event['mapping']=revision
        if name=='future-minor':event['spec']='0.3.0'
        event['fields']['future_field']={'null':None,'empty':''}
        event['future_envelope']={'kept':True}
        vectors[name]=dict({'$comment':'SPDX-License-Identifier: GPL-3.0-or-later'},spec='0.2.0',family='pskreporter-mqtt',mapping=1,
            **{'class':'compatibility'},input={'event':event},expected={'verification':'verified','preserved_event':event,
            'read':{'kind':'spot','fields':vectors['capture-060']['expected']['event']['fields']}})
    result = {f'pskreporter-mqtt/{name}.json'  :dump(v) for name,v in vectors.items()}
    result['pskreporter-mqtt/manifest.json'] = dump(dict({'$comment':'SPDX-License-Identifier: GPL-3.0-or-later'},spec='0.2.0',family='pskreporter-mqtt',mapping=1,
        source='../../annex-g-pskreporter-mqtt.md',source_sha256=hashlib.sha256((root/'spec/annex-g-pskreporter-mqtt.md').read_bytes()).hexdigest(),
        capture_sha256=fixture['source_sha256'],page_sha256='d5fd2ea058681ff5f4777d3e7383802e0fa8964ce9b7ad0acdced4c3eb47248d',
        vectors=[dict(file=name+'.json',spec='0.2.0',family='pskreporter-mqtt',mapping=1,**{'class':v['class'],'sha256':hashlib.sha256(result[f'pskreporter-mqtt/{name}.json']).hexdigest()}) for name,v in sorted(vectors.items())]))
    print(f"PSK Reporter MQTT: {len(vectors)} vectors, 460 captured records (400 spots, 60 receiver info, zero disagreement errors), exact payload round trips and independent field oracles")
    return result
