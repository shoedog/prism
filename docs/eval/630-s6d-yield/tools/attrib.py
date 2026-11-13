import sys, json, collections
sys.path.insert(0,'.')
from join import *
corpora=sys.argv[1].split(',')
out={}
for c in corpora:
    ev=load_events(c)
    for t in ['c1','c1call','c1ctor','c1brace','c2','c3','c3arith','c3cast','c4','c5','c5cmp','c5arith','c5shift','c5other','all']:
        ex=load_extra(c,t)
        un=0; kinds=collections.Counter()
        for e in ex:
            o,evs=edge_events(ev,e)
            if o is None: un+=1; continue
            kinds[tuple(sorted({x[2] for x in evs}))]+=1
        out[(c,t)]=(len(ex),un,kinds)
        print(c,t,len(ex),'unattributed',un)
        if t in('c1','c2','c3','c4','c5'):
            for k,v in kinds.most_common(6): print('    ',v,k)
