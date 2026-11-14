import json, collections, re, sys, os
# Inputs (regenerated, see RUN.txt) are read from $PRISM_PILOT_INPUTS, default the current directory.
# Gold-file lists are private: $PRISM_GOLDFILES points at the goldfiles.json that goldfiles.py wrote.
def inp(name):
    return os.path.join(os.environ.get('PRISM_PILOT_INPUTS', '.'), name)
def gold_path():
    v = os.environ.get('PRISM_GOLDFILES')
    if not v:
        sys.exit('PRISM_GOLDFILES is not set: point it at the goldfiles.json written by goldfiles.py (private store)')
    if not os.path.isfile(v):
        sys.exit('PRISM_GOLDFILES=%s does not exist' % v)
    return v
TEST_COMP={'test','tests','__tests__','testdata'}
def is_test(p):
    comps=[c.lower() for c in p.split('/') if c]
    b=comps[-1] if comps else p
    bo=p.split('/')[-1]
    if any(c in TEST_COMP for c in comps): return True
    if b.startswith('test_') and b.endswith('.py'): return True
    for s in ('_test.py','_test.go','_test.rs'):
        if b.endswith(s): return True
    if '.test.' in b or '.spec.' in b: return True
    if bo.endswith('Test.java') or bo.endswith('Tests.java'): return True
    return False

def pagerank(nodes, wedges, pers, d=0.85, iters=100):
    n=len(nodes); idx={f:i for i,f in enumerate(nodes)}
    out=collections.defaultdict(list); outw=[0.0]*n
    for (a,b),w in wedges.items():
        out[idx[a]].append((idx[b],w)); outw[idx[a]]+=w
    tot=sum(pers.values()); p=[pers[f]/tot for f in nodes]
    r=p[:]
    for _ in range(iters):
        nr=[(1-d)*p[i] for i in range(n)]
        dang=sum(r[i] for i in range(n) if outw[i]==0)
        for i in range(n):
            nr[i]+=d*dang*p[i]
        for i in range(n):
            if outw[i]==0: continue
            for j,w in out[i]:
                nr[j]+=d*r[i]*w/outw[i]
        r=nr
    return {f:r[idx[f]] for f in nodes}

def load(corpus):
    rm=json.load(open(inp(f'rm-{corpus}.json')))
    nodes=[n['location']['file'] for n in rm['graph']['nodes']]
    mdeps={(nodes[e['from']],nodes[e['to']]) for e in rm['graph']['edges']}
    calls=collections.Counter(); fcallers=collections.Counter(); fn_edges=collections.Counter()
    for l in open(inp(f'cs-{corpus}.jsonl')):
        d=json.loads(l)
        if d['record_kind']!='call_site': continue
        cf=d['caller']['file']
        for t in d['resolved_targets']:
            fid=t['function_id']; w=1.0 if t['confidence']=='exact' else 0.6
            key=(fid['file'],fid['name'],fid['start_line'])
            ck=(cf,d['caller']['name'],d['caller']['start_line'])
            fn_edges[(ck,key)]+=w
            if fid['file']!=cf:
                calls[(cf,fid['file'])]+=w
                fcallers[key]+=1
    return nodes,mdeps,calls,fcallers,fn_edges

def rankings(corpus):
    nodes,mdeps,calls,fcallers,fn_edges=load(corpus)
    R={}
    R['alpha']=list(nodes)
    deg=collections.Counter(); indeg=collections.Counter()
    for a,b in mdeps: deg[a]+=1; deg[b]+=1; indeg[b]+=1
    R['degree']=sorted(nodes,key=lambda f:(-deg[f],f))
    R['indeg']=sorted(nodes,key=lambda f:(-indeg[f],f))
    pers={f:(0.1 if is_test(f) else 1.0) for f in nodes}
    pr=pagerank(nodes,{e:1.0 for e in mdeps},{f:1.0 for f in nodes})
    R['pr_mdep']=sorted(nodes,key=lambda f:(-pr[f],f))
    pr=pagerank(nodes,{e:1.0 for e in mdeps},pers)
    R['pr_mdep_test']=sorted(nodes,key=lambda f:(-pr[f],f))
    w=collections.Counter({e:1.0 for e in mdeps})
    for e,c in calls.items():
        if e[0] in pers and e[1] in pers: w[e]+=c
    pr=pagerank(nodes,w,pers)
    R['pr_calls_test']=sorted(nodes,key=lambda f:(-pr[f],f))
    # degree with test prior
    R['degree_test']=sorted(nodes,key=lambda f:(-(deg[f]*(0.1 if is_test(f) else 1)),f))
    # symbol-level pagerank aggregated to file
    fns=set()
    for (a,b) in fn_edges: fns.add(a); fns.add(b)
    fns=sorted(fns); fset=set(nodes)
    fpers={k:(0.1 if is_test(k[0]) else 1.0) for k in fns}
    if fns:
        spr=pagerank(fns,dict(fn_edges),fpers)
        agg=collections.Counter()
        for k,v in spr.items():
            if k[0] in fset: agg[k[0]]+=v
        R['pr_symbol_test']=sorted(nodes,key=lambda f:(-agg[f],f))
    return R,nodes,fcallers

gold=json.load(open(gold_path()))
Ks=[12,25,50,100]
res=collections.defaultdict(lambda: collections.defaultdict(list))
per=[]
for corpus in ['httpx','ripgrep','cobra','excalidraw']:
    R,nodes,_=rankings(corpus)
    ns=set(nodes)
    for qid,g in gold.items():
        if g['corpus']!=corpus: continue
        gf=[f for f in g['files'] if f in ns]
        if not gf: print('no gold files in index',qid,g['files'][:5]); continue
        row=[qid,corpus,g['format'],len(gf),len(nodes)]
        for name,order in R.items():
            for K in Ks:
                top=set(order[:K]); hit=len(top & set(gf))/len(gf)
                res[name][K].append(hit)
            row.append((name,[round(len(set(order[:K])&set(gf))/len(gf),2) for K in Ks]))
        per.append(row)
print('macro mean gold-file hit rate, n=%d questions'%len(per))
print('%-16s'%'ranking'+''.join('%8s'%('@%d'%K) for K in Ks))
for name in res:
    print('%-16s'%name+''.join('%8.3f'%(sum(res[name][K])/len(res[name][K])) for K in Ks))
if os.environ.get('PRISM_OUT_DIR'):
    json.dump(per,open(os.path.join(os.environ['PRISM_OUT_DIR'],'per_question.json'),'w'),indent=0)
for r in per: print(r[0],r[1],r[2],'gold',r[3],'N',r[4],[ (n,v) for n,v in r[5:] if n in ('alpha','degree','pr_calls_test','pr_symbol_test')])
