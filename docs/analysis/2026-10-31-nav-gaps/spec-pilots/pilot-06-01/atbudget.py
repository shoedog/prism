import json, collections, os
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)),'rank2.py')).read().split("gold=json.load(open(gold_path()))\nKs")[0])
gold=json.load(open(gold_path()))
K2={'httpx':60,'ripgrep':94,'cobra':36,'excalidraw':57}; K4={'httpx':60,'ripgrep':102,'cobra':36,'excalidraw':125}
res=collections.defaultdict(list); exr=collections.defaultdict(list)
for corpus in K2:
    nodes,mdeps,calls,_,_=load(corpus); ns=set(nodes); fns=json.load(open(inp(f'fn-{corpus}.json')))
    w=collections.Counter({e:1.0 for e in mdeps})
    for e,c in calls.items():
        if e[0] in ns and e[1] in ns: w[e]+=c
    w2=collections.Counter(w)
    for (a,b),c in w.items(): w2[(b,a)]+=0.5*c
    base={f:(0.1 if is_test(f) else 1.0) for f in nodes}
    pr=pagerank(nodes,w2,base); unf=sorted(nodes,key=lambda f:(-pr[f],f))
    for qid,g in gold.items():
        if g['corpus']!=corpus: continue
        gf=set(f for f in g['files'] if f in ns)
        if not gf: continue
        mf,idf=focus(Q[qid],nodes,fns)
        pers={f:base[f]*(50 if f in mf else (10 if f in idf else 1)) for f in nodes}
        prf=pagerank(nodes,w2,pers); foc=sorted(nodes,key=lambda f:(-prf[f],f))
        foc=[f for f in nodes if f in mf]+[f for f in foc if f not in mf]
        row={'today_first50':len(set(nodes[:50])&gf)/len(gf),
             'path_2k':len(set(nodes[:K2[corpus]])&gf)/len(gf),'path_4k':len(set(nodes[:K4[corpus]])&gf)/len(gf),
             'ranked_2k':len(set(unf[:K2[corpus]])&gf)/len(gf),
             'focused_2k':len(set(foc[:K2[corpus]])&gf)/len(gf),
             'ranked_4k':len(set(unf[:K4[corpus]])&gf)/len(gf),
             'focused_4k':len(set(foc[:K4[corpus]])&gf)/len(gf)}
        for k,v in row.items():
            res[k].append(v)
            if corpus=='excalidraw': exr[k].append(v)
for k in res: print('%-14s all %.3f   excalidraw %.3f'%(k,sum(res[k])/len(res[k]),sum(exr[k])/len(exr[k])))
