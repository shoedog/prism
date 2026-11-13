import sys,json,re,collections
sys.path.insert(0,'.')
from join import *
S2=S
corp=['leveldb','re2','snappy','protobuf','protobuf-x']
cfgs=['c1','c1call','c1ctor','c1brace','c2','c3','c3arith','c3cast','c4','c5','c5cmp','c5arith','c5shift','c5other','all','allbut1','allbut2','allbut3','allbut4','allbut5']
cur={}
rows=[]
def nlines(c,t): return sum(1 for _ in open(f'{S}{c}.{t}.sorted'))
res={}
for c in corp:
    d=nlines(c,'default'); off=nlines(c,'cpp_off')
    # cpp_off present edges: multiset difference computed earlier -> recompute properly
    import collections as C
    D=C.Counter(open(f'{S}{c}.default.sorted')); O=C.Counter(open(f'{S}{c}.cpp_off.sorted'))
    cur[c]=(sum((O-D).values()),sum((D-O).values()))
    for t in cfgs:
        ex=load_extra(c,t)
        try: added=sum(1 for _ in open(f'{S}{c}.{t}.added'))
        except FileNotFoundError: added=0
        # distinct variables and generated split
        ev=load_events(c); vars_=set(); gen=0
        for e in ex:
            o,_=edge_events(ev,e)
            vars_.add((e['from']['file'],o,e['from']['path']['base']))
            if re.search(r'\.pb\.(cc|h)$',e['from']['file']): gen+=1
        res[(c,t)]=dict(edges=len(ex),added=added,vars=len(vars_),gen=gen)
json.dump({'cur':cur,'res':{f'{c}|{t}':v for (c,t),v in res.items()}},open(S+'final.json','w'),indent=1)
def row(label,keys,tag):
    return label
with open(S+'table_per_corpus.tsv','w') as f:
    f.write('corpus\tswitch\textra_removed_edges\tadded_edges\tpct_of_6c_yield\tdistinct_vars\tgenerated_pb_edges\n')
    for c in corp:
        f.write(f'{c}\tcpp_off(6c yield; edges present with C++ rules off)\t{cur[c][0]}\t{cur[c][1]}\t100\t\t\n')
        for t in cfgs:
            r=res[(c,t)]; pct=(100*r['edges']/cur[c][0]) if cur[c][0] else float('nan')
            f.write(f"{c}\t{t}\t{r['edges']}\t{r['added']}\t{pct:.0f}\t{r['vars']}\t{r['gen']}\n")
    for label,cs in (('SUM official (leveldb+re2+snappy+protobuf)',['leveldb','re2','snappy','protobuf']),('SUM supplementary (leveldb+re2+snappy+protobuf-x)',['leveldb','re2','snappy','protobuf-x'])):
        y=sum(cur[c][0] for c in cs)
        f.write(f'{label}\tcpp_off(6c yield)\t{y}\t{sum(cur[c][1] for c in cs)}\t100\t\t\n')
        for t in cfgs:
            e=sum(res[(c,t)]['edges'] for c in cs); a=sum(res[(c,t)]['added'] for c in cs); v=sum(res[(c,t)]['vars'] for c in cs); g=sum(res[(c,t)]['gen'] for c in cs)
            f.write(f"{label}\t{t}\t{e}\t{a}\t{100*e/y:.0f}\t{v}\t{g}\n")
print(open(S+'table_per_corpus.tsv').read())
