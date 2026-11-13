import sys, json, collections, re
sys.path.insert(0,'.')
from join import *
KW={'bool','char','wchar_t','short','int','long','float','double','signed','unsigned','const','size_t','ssize_t','uint8_t','uint16_t','uint32_t','uint64_t','int8_t','int16_t','int32_t','int64_t','intptr_t','uintptr_t','ptrdiff_t','std::size_t'}
def load_defs(c):
    d=collections.defaultdict(list)
    for l in open(f'{S}{c}.defs.tsv'):
        p=l.rstrip('\n').split('\t')
        if len(p)<9: continue
        f,line,name,q,n,tm,me,isdef,kinds=p[:9]
        d[name].append(dict(file=f,line=int(line),q=q,n=int(n),tmpl=int(tm),method=int(me),isdef=int(isdef),kinds=kinds.split('|') if kinds else []))
    return d
RANK={'A':0,'F':1,'B':2,'E':3,'C':4,'D':5}
def classify(defs,callee,i):
    m=re.match(r'callee=([a-z_]+):(.*)',callee)
    if not m: return 'D','no-callee-expr:'+callee
    k,t=m.group(1),m.group(2)
    if k=='field_expression':
        return 'E','method '+t
    if k=='identifier': name=t
    elif k=='qualified_identifier': name=t.rsplit('::',1)[-1]
    elif k=='template_function': name=t.split('<')[0].rsplit('::',1)[-1]
    elif k=='primitive_type' or k=='sized_type_specifier': return 'D','scalar-functional-cast '+t
    else: return 'D',k+':'+t
    ents=defs.get(name,[])
    if not ents: return 'D','external '+t
    def agree():
        ok=True
        for e in ents:
            if i>=len(e['kinds']): ok=False; break
            if e['kinds'][i].split(':',1)[0] not in ('val','ptr','fnptr'): ok=False; break
        return 'all-byval' if ok else 'some-ref-or-short'
    if any(e['tmpl'] for e in ents): return 'C','template '+name+' ['+agree()+']'
    sigs={tuple(e['kinds']) for e in ents}
    if len(sigs)>1: return 'C','overloaded '+name+' ['+agree()+']'
    kinds=ents[0]['kinds']
    if i>=len(kinds):
        return 'F','arity/variadic '+name
    kd,ty=kinds[i].split(':',1)
    if kd in('val','ptr','fnptr'):
        cls='A'
        if kd=='val' and not all(w in KW for w in re.split(r'_|\s',ty) if w) :
            return 'A','byval_class '+name+'('+ty+')'
        return 'A','byval_scalar_or_ptr '+name+'('+kd+':'+ty+')'
    return 'B','byref '+name+'('+kinds[i]+')'
if __name__=='__main__':
    corpora=sys.argv[1].split(',')
    tot_ev=collections.Counter(); tot_edge=collections.Counter(); sub=collections.Counter()
    per={}
    for c in corpora:
        ev=load_events(c); defs=load_defs(c)
        ce=collections.Counter(); cg=collections.Counter(); seen=set()
        for e in load_extra(c,'c1'):
            o,evs=edge_events(ev,e)
            ks=[]
            for x in evs:
                if x[2]!='call_arg': continue
                m=re.search(r';argpos=(\d+)$',x[4]); i=int(m.group(1)) if m else 0
                cls,why=classify(defs,x[4].split(';argpos=')[0],i)
                ks.append(cls)
                key=(o,e['from']['file'],e['from']['path']['base'],x[3],x[4])
                if key not in seen:
                    seen.add(key); ce[cls]+=1; sub[(cls,why.split(' ')[0])]+=1
            if ks:
                worst=max(ks,key=lambda k:RANK[k]); cg['all_A' if worst=='A' else 'worst_'+worst]+=1
            else:
                cg['no_call_arg_event']+=1
        per[c]=(ce,cg)
        print(c,'events',dict(ce),'edges',dict(cg))
        for k,v in ce.items(): tot_ev[k]+=v
        for k,v in cg.items(): tot_edge[k]+=v
    print('TOTAL events',dict(tot_ev),'edges',dict(tot_edge))
    print(sorted(sub.items()))
    json.dump({c:[dict(a),dict(b)] for c,(a,b) in per.items()}|{'_total':[dict(tot_ev),dict(tot_edge)],'_sub':{f'{a}:{b}':v for (a,b),v in sub.items()}},open(S+'c1class.json','w'),indent=1)
