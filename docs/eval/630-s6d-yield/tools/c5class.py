import sys,json,re,collections
sys.path.insert(0,'.')
from join import *
corp=['leveldb','re2','snappy','protobuf-x']
def parse(x):
    fam=re.search(r'op=(\w+):',x).group(1)
    m=re.search(r'why=([\w:]+)',x)
    if m: why=m.group(1)
    elif 'dead:' in x: why=re.search(r'(dead:\w+)',x).group(1)
    elif x.endswith('cascade'): why='cascade'
    else: why='?'
    return fam,why
CAT={'call':'call','member':'member','deref_or_addr':'deref/addr-of','dead:global_or_unknown':'global/unknown name','dead:local_nonscalar_type':'local with non-keyword type',
 'dead:escaped_name':'other operand name escaped (cascade)','cascade':'other operand name escaped (cascade)','qualified_global':'qualified/global','this':'this','string':'string literal'}
ev_cnt=collections.Counter(); fam_ev=collections.Counter(); edge_fam=collections.Counter(); edge_why=collections.Counter(); edge_pure=collections.Counter()
nedges=0
for scope in ['pooled']:
  for c in corp:
    ev=load_events(c); seen=set()
    for e in load_extra(c,'c5'):
        o,evs=edge_events(ev,e); nedges+=1
        oc=[x for x in evs if x[2]=='operand_certificate']
        fams=set(); whys=set()
        for x in oc:
            fam,why=parse(x[4]); cat=CAT.get(why,why)
            fams.add(fam); whys.add(cat)
            key=(c,o,e['from']['file'],e['from']['path']['base'],x[3],x[4])
            if key not in seen: seen.add(key); ev_cnt[(fam,cat)]+=1
        edge_fam['+'.join(sorted(fams)) or 'none']+=1
        for w in whys: edge_why[w]+=1
        edge_pure[next(iter(whys)) if len(whys)==1 else 'mixed:'+'|'.join(sorted(whys))]+=1
print('edges',nedges)
print('events by (family,reason):'); 
for k,v in sorted(ev_cnt.items(),key=lambda x:-x[1]): print('  ',v,k)
print('edges by family set:',dict(edge_fam))
print('edges having reason (not exclusive):',dict(edge_why))
print('edges by pure/mixed reason:',dict(edge_pure))
json.dump(dict(edges=nedges,events={f'{a}|{b}':v for (a,b),v in ev_cnt.items()},edge_fam=edge_fam,edge_why=edge_why,edge_pure=edge_pure),open(S+'c5class.json','w'),indent=1)
