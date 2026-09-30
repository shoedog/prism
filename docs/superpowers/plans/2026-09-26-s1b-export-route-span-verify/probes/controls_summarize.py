# Summarize P3 control outputs: per scenario, function inventory + every call-site row.
import json,glob
for f in sorted(glob.glob('C*.dump.jsonl')):
    sc=f.split('.dump')[0]
    fn=json.load(open(sc+'.functions.json'))
    fl=fn if isinstance(fn,list) else fn.get('functions',[])
    print('==',sc,'functions:',[(x.get('file'),x.get('name'),x.get('start_line'),x.get('end_line')) for x in fl])
    for l in open(f):
        r=json.loads(l)
        if r.get('record_kind')!='call_site': continue
        c=r['caller']
        tg=['%s:%s@%s-%s %s/%s'%(t['function_id']['file'],t['function_id']['name'],t['function_id']['start_line'],t['function_id']['end_line'],t['confidence'],t['kind']) for t in r.get('resolved_targets') or []]
        print('   %s:%s L%s %r drop=%s -> %s'%(c['file'],c['name'],r['source_span']['line'],r['callee_text'],r.get('drop'),tg))
