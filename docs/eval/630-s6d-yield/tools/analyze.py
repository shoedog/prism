import sys, collections, json, os
S='/tmp/claude-0/-home-user-prism/817300ea-683f-51d6-b28c-8f12e9d2d739/scratchpad/out/'
corpora=sys.argv[1].split(',')
tags=[l.split('\t')[0] for l in open('/tmp/claude-0/-home-user-prism/817300ea-683f-51d6-b28c-8f12e9d2d739/scratchpad/configs.txt')]
def load(c,t):
    return collections.Counter(l.rstrip('\n') for l in open(f'{S}{c}.{t}.sorted'))
rows=[]
for c in corpora:
    d=load(c,'default')
    cur=None
    cache={}
    for t in tags:
        m=load(c,t)
        removed=d-m; added=m-d
        nr=sum(removed.values()); na=sum(added.values())
        if t=='cpp_off':
            cur=nr_cur=sum((m-d).values())  # edges present with rules off but not in default = 6c removals
            # rules off should only have more edges
            cur=sum((m-d).values()); cur_added=sum((d-m).values())
            rows.append((c,'cpp_off(6c yield)',cur,cur_added)); 
            continue
        # extra removed = edges in default not in muted (muted removes more)
        extra=d-m  # present in default, absent when muted
        extra_n=sum(extra.values()); add_n=sum((m-d).values())
        rows.append((c,t,extra_n,add_n))
        with open(f'{S}{c}.{t}.extra','w') as f:
            for k in sorted(extra.elements()): f.write(k+'\n')
        with open(f'{S}{c}.{t}.added','w') as f:
            for k in sorted((m-d).elements()): f.write(k+'\n')
with open(sys.argv[2],'w') as f:
    f.write('corpus\tswitch\textra_removed\tadded\n')
    for r in rows: f.write('\t'.join(map(str,r))+'\n')
for r in rows: print(*r,sep='\t')
