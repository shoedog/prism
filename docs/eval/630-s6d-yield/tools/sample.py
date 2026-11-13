import sys,json,random,re,collections
sys.path.insert(0,'.')
from join import *
from c1class import load_defs, classify
corp=['leveldb','re2','snappy','protobuf-x']
CAND={'C1':'c1','C2':'c2','C3':'c3','C4':'c4','C5':'c5'}
root=S.replace('out/','corpora/')
evc={c:load_events(c) for c in corp}; dfc={c:load_defs(c) for c in corp}
def src(c,f):
    return open(f'{root}{c}/{f}',errors='replace').read().split('\n')
out=[]; txt=[]
for cand,cfg in CAND.items():
    pool=[]
    for c in corp:
        for e in load_extra(c,cfg): pool.append((c,json.dumps(e,sort_keys=True)))
    pool.sort()
    rnd=random.Random(6302)
    pick=pool if len(pool)<=15 else rnd.sample(pool,15)
    txt.append(f'=== {cand} pool={len(pool)} picked={len(pick)}')
    for idx,(c,es) in enumerate(pick):
        e=json.loads(es)
        o,evs=edge_events(evc[c],e)
        L=src(c,e['from']['file']); f=e['from']['line']; t=e['to']['line']
        rec=dict(candidate=cand,idx=idx,corpus=c,file=e['from']['file'],def_line=f,use_line=t,var=e['from']['path']['base'],fields=e['from']['path']['fields'],confidence=e['confidence'],doubt=e['doubt'],
                 def_text=L[f-1].strip(),use_text=L[t-1].strip(),owner=list(o) if o else None,
                 events=sorted({(x[2],x[3],x[4]) for x in evs}))
        if cand=='C1':
            cl=[]
            for x in evs:
                if x[2]=='call_arg':
                    m=re.search(r';argpos=(\d+)$',x[4]); cl.append((x[3],)+classify(dfc[c],x[4].split(';argpos=')[0],int(m.group(1))))
            rec['c1_callee_class']=sorted(set(cl))
        out.append(rec)
        txt.append(f"--- [{cand}#{idx}] {c}:{rec['file']} var={rec['var']} def@{f} use@{t} {rec['confidence']}/{rec['doubt']}")
        span=range(f, min(t,f+14)+1) if t>=f else range(t,f+1)
        for ln in span: txt.append(f'   {ln:5d}| {L[ln-1][:140]}')
        if t-f>14: txt.append(f'   ...| (use at {t})'); txt.append(f'   {t:5d}| {L[t-1][:140]}')
        for ev in rec['events'][:8]: txt.append('   EV '+str(ev)[:230])
        if cand=='C1': txt.append('   C1CLASS '+str(rec['c1_callee_class'])[:300])
json.dump(out,open(S+'samples_raw.json','w'),indent=1)
open(S+'dossier.txt','w').write('\n'.join(txt))
print(len(out))
