import json, collections, os
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)),'rank.py')).read().split("gold=json.load")[0])
for corpus in ['httpx','ripgrep','cobra','excalidraw']:
    R,nodes,fcallers=rankings(corpus)
    nodes_,mdeps,calls,_,_=load(corpus)
    indeg=collections.Counter(b for a,b in mdeps); outdeg=collections.Counter(a for a,b in mdeps)
    byfile=collections.defaultdict(list)
    for (f,n,l),c in fcallers.items(): byfile[f].append((c,n,l))
    lines=[]
    for f in R['degree_test']:
        syms=sorted(byfile[f],key=lambda x:(-x[0],x[2]))[:3]
        s=', '.join(f'{n}:{l} ({c})' for c,n,l in syms)
        lines.append(f'{f} [in {indeg[f]} out {outdeg[f]}] {s}')
    sizes=[len(x.encode())+1 for x in lines]
    for budget in (1000,2000,4000):
        tot=0;k=0
        for s in sizes:
            if (tot+s)/3.5>budget-150: break
            tot+=s;k+=1
        print(corpus,len(nodes),'budget',budget,'files',k)
    print('  sample:',lines[0][:160])
    # today's MCP default canonical JSON bytes for first 50 nodes + edges
    rm=json.load(open(inp(f'rm-{corpus}.json')))
    n=min(50,len(rm['graph']['nodes']))
    nodes50=rm['graph']['nodes'][:n]; edges50=[e for e in rm['graph']['edges'] if e['from']<n and e['to']<n]
    b=len(json.dumps({'nodes':nodes50,'edges':edges50}))
    print('  today first-50 canonical bytes',b,'~tokens',b//4)
