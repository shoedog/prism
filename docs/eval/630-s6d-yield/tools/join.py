# join extra-removed edges to escape events (by file, owner range, name)
import json, collections, sys
S='/tmp/claude-0/-home-user-prism/817300ea-683f-51d6-b28c-8f12e9d2d739/scratchpad/out/'
def load_events(c):
    ev=collections.defaultdict(list)  # (file,name)-> list of (ostart,oend,kind,line,extra)
    import os
    if not os.path.exists(f'{S}{c}.log'): return ev
    for l in open(f'{S}{c}.log'):
        p=l.rstrip('\n').split('\t')
        if p[0]!='E': continue
        _,f,os_,oe,kind,name,line,extra=(p+[''])[:8]
        ev[(f,name)].append((int(os_),int(oe),kind,int(line),extra))
    return ev
def edge_events(ev,e):
    f=e['from']['file']; b=e['from']['path']['base']
    lo=min(e['from']['line'],e['to']['line']); hi=max(e['from']['line'],e['to']['line'])
    cands=[x for x in ev.get((f,b),[]) if x[0]<=lo and x[1]>=hi]
    if not cands: return None,[]
    # narrowest owner
    best=min(cands,key=lambda x:x[1]-x[0])
    owner=(best[0],best[1])
    return owner,[x for x in cands if (x[0],x[1])==owner]
def load_extra(c,t):
    try:
        return [json.loads(l) for l in open(f'{S}{c}.{t}.extra')]
    except FileNotFoundError:
        return []
