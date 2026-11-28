import json, collections, os
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)),'rank.py')).read().split("gold=json.load")[0])
gold=json.load(open(gold_path()))
Ks=[12,25,50]; agg=collections.defaultdict(lambda: collections.defaultdict(list)); ex=collections.defaultdict(lambda: collections.defaultdict(list))
for corpus in ['httpx','ripgrep','cobra','excalidraw']:
    nodes,mdeps,calls,_,_=load(corpus); ns=set(nodes)
    pers={f:(0.1 if is_test(f) else 1.0) for f in nodes}
    w=collections.Counter({e:1.0 for e in mdeps})
    for e,c in calls.items():
        if e[0] in ns and e[1] in ns: w[e]+=c
    w2=collections.Counter(w)
    for (a,b),c in w.items(): w2[(b,a)]+=0.5*c
    orders={}
    pr=pagerank(nodes,w2,pers); orders['pr_bidir_test']=sorted(nodes,key=lambda f:(-pr[f],f))
    deg=collections.Counter()
    for (a,b) in mdeps: deg[a]+=1; deg[b]+=1
    orders['degree_test']=sorted(nodes,key=lambda f:(-(deg[f]*(0.1 if is_test(f) else 1)),f))
    wdeg=collections.Counter()
    for (a,b),c in w.items(): wdeg[a]+=c; wdeg[b]+=c
    orders['wdegree_test']=sorted(nodes,key=lambda f:(-(wdeg[f]*(0.1 if is_test(f) else 1)),f))
    for qid,g in gold.items():
        if g['corpus']!=corpus: continue
        gf=set(f for f in g['files'] if f in ns)
        if not gf: continue
        for n,o in orders.items():
            for K in Ks:
                v=len(set(o[:K])&gf)/len(gf); agg[n][K].append(v)
                if corpus=='excalidraw': ex[n][K].append(v)
for n in agg: print('%-16s'%n+''.join('%8.3f'%(sum(agg[n][K])/len(agg[n][K])) for K in Ks), ' excalidraw', ''.join('%8.3f'%(sum(ex[n][K])/len(ex[n][K])) for K in Ks))
