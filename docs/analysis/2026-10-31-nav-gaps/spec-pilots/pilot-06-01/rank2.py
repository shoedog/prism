import json, collections, re, os, sys
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)),'rank.py')).read().split("gold=json.load")[0])
Q={}
_qp=os.path.join(os.environ.get('PRISM_TRIAL_DIR') or sys.exit('PRISM_TRIAL_DIR is not set (private store: questions.jsonl)'),'questions.jsonl')
if not os.path.isfile(_qp): sys.exit('questions.jsonl not found at '+_qp)
for l in open(_qp):
    q=json.loads(l); Q[q['id']]=q
pathre=re.compile(r'[A-Za-z0-9_./-]+\.(?:py|rs|go|jsx|js|tsx|ts)')
def focus(q, nodes, fns):
    text=q['text']
    ticks=re.findall(r'`([^`]+)`',text)
    mentioned_files=set()
    for m in pathre.findall(text):
        for f in nodes:
            if f==m or f.endswith('/'+m): mentioned_files.add(f)
    idents=set()
    for t in ticks:
        for tok in re.findall(r'[A-Za-z_][A-Za-z0-9_]*',t):
            if len(tok)>=3: idents.add(tok)
    byname=collections.defaultdict(set)
    for r in fns:
        if r['name']: byname[r['name']].add(r['file'])
    stems=collections.defaultdict(set)
    for f in nodes:
        stems[f.split('/')[-1].split('.')[0].lower()].add(f)
    ident_files=set()
    for i in idents:
        ident_files |= byname.get(i,set())
        ident_files |= stems.get(i.lower(),set())
    return mentioned_files, ident_files

gold=json.load(open(gold_path()))
Ks=[12,25,50]
agg=collections.defaultdict(lambda: collections.defaultdict(list))
agg2=collections.defaultdict(lambda: collections.defaultdict(list))
rows=[]
for corpus in ['httpx','ripgrep','cobra','excalidraw']:
    R,nodes,_=rankings(corpus)
    nodes_,mdeps,calls,fcallers,fn_edges=load(corpus)
    fns=json.load(open(inp(f'fn-{corpus}.json')))
    ns=set(nodes)
    for qid,g in gold.items():
        if g['corpus']!=corpus: continue
        gf=set(f for f in g['files'] if f in ns)
        if not gf: continue
        mf,idf=focus(Q[qid],nodes,fns)
        pers={}
        for f in nodes:
            base=0.1 if is_test(f) else 1.0
            if f in mf: base*=50
            elif f in idf: base*=10
            pers[f]=base
        w=collections.Counter({e:1.0 for e in mdeps})
        for e,c in calls.items():
            if e[0] in ns and e[1] in ns: w[e]+=c
        pr=pagerank(nodes,w,pers)
        orders=dict(R)
        orders['ppr_focus']=sorted(nodes,key=lambda f:(-pr[f],f))
        # undirected variant: add reverse edges at half weight so callers of focus files rank too
        w2=collections.Counter(w)
        for (a,b),c in w.items(): w2[(b,a)]+=0.5*c
        pr2=pagerank(nodes,w2,pers)
        orders['ppr_focus_bidir']=sorted(nodes,key=lambda f:(-pr2[f],f))
        rest=gf-mf
        row=[qid,corpus,len(gf),len(rest),len(mf),len(idf)]
        for name,order in orders.items():
            for K in Ks:
                top=set(order[:K])
                agg[name][K].append(len(top&gf)/len(gf))
                if rest: agg2[name][K].append(len(top&rest)/len(rest))
        row.append({n:[round(len(set(orders[n][:K])&gf)/len(gf),2) for K in Ks] for n in ('alpha','degree_test','ppr_focus','ppr_focus_bidir')})
        rows.append(row)
print('ALL gold files (n=%d)'%len(rows))
for n in agg: print('%-18s'%n+''.join('%8.3f'%(sum(agg[n][K])/len(agg[n][K])) for K in Ks))
print('gold files NOT named in the question (n=%d)'%len(agg2['alpha'][12]))
for n in agg2: print('%-18s'%n+''.join('%8.3f'%(sum(agg2[n][K])/len(agg2[n][K])) for K in Ks))
print('excalidraw only, all gold')
for n in agg:
    vals={K:[v for r,v in zip(rows,agg[n][K]) if r[1]=='excalidraw'] for K in Ks}
    print('%-18s'%n+''.join('%8.3f'%(sum(vals[K])/len(vals[K])) for K in Ks))
for r in rows: print(r)
