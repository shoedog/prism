import json,re,collections,sys
sys.path.insert(0,'.')
from join import S
from c1class import load_defs, classify
rs=json.load(open(S+'samples_raw2.json'))
decl={('C1',6),('C2',2),('C2',3),('C2',4),('C2',5),('C2',11),('C2',13),('C3',1),('C3',2),('C3',4),('C3',5),('C3',7),('C5',1),('C5',2),('C5',3),('C5',4),('C5',10),('C5',12)}
lval={('C1',0),('C1',3),('C1',11),('C1',13),('C1',4),('C2',0),('C2',14),('C3',0),('C3',6),('C4',4),('C4',5),('C5',9),('C5',11),('C5',13),('C5',14)}
back={('C1',2),('C1',5),('C1',10),('C2',10),('C4',0),('C4',2),('C4',3),('C4',10),('C4',11),('C4',13),('C4',14),('C5',0),('C5',6),('C5',7),('C5',8)}
killed={('C1',1),('C1',7),('C1',8),('C1',9),('C1',12),('C1',14),('C2',1),('C2',6),('C2',7),('C2',8),('C2',9),('C2',12),('C4',1),('C4',6),('C4',7),('C4',8),('C4',9),('C4',12),('C5',5)}
loopk={('C3',3)}
assert decl|lval|back|killed|loopk==set((r['candidate'],r['idx']) for r in rs), set((r['candidate'],r['idx']) for r in rs)-(decl|lval|back|killed|loopk)
dfc={}
def why_dead(k,r):
    if k in decl: return 'yes: the use line is only a declarator/initialiser of the variable (write-only anchor), so the earlier def cannot be observed there (rule U)'+('; inside a loop the declaration re-executes each iteration' if r['common_loop'] else '')
    if k in lval: return 'yes: the use line only assigns the variable (plain lvalue span, write-only anchor), rule U; the real reads on later lines keep their own edges'
    if k in back: return 'yes: backward pair (use line before def line) with no loop enclosing both lines in the source, so the def cannot reach the use'
    if k in killed: return 'yes: an unconditional same-block overwrite lies between def and use (rule D)'
    return 'yes: the loop body redeclares `code` (block-scoped) at line 1188 before the use, so a def from a previous iteration cannot reach it'
PROOF={'call':'the callee must be resolved to a function whose return type is a built-in scalar',
 'global/unknown name':'the other operand (not an owner local: global, class member, enumerator, macro or constant) needs a census-backed proof that it is a built-in scalar object/enumerator',
 'local with non-keyword type':'the other operand is an owner local whose declared type is not a keyword scalar (size_t/uintN_t spelling or class): C2-style typing needed',
 'other operand name escaped (cascade)':'resolves by itself once the other operand name stops escaping (cascade of another candidate)',
 'member':'type of the member expression needed (field type from in-repo class)','deref/addr-of':'pointee type needed'}
def parse(x):
    m=re.search(r'op=(\w+):([^;]*)',x); fam=m.group(1); op=m.group(2)
    m2=re.search(r'why=([\w:]+)',x)
    why=m2.group(1) if m2 else (re.search(r'(dead:\w+)',x).group(1) if 'dead:' in x else 'cascade')
    return fam,op,why
CAT={'dead:global_or_unknown':'global/unknown name','dead:local_nonscalar_type':'local with non-keyword type','dead:escaped_name':'other operand name escaped (cascade)','cascade':'other operand name escaped (cascade)'}
out=[]
for r in rs:
    k=(r['candidate'],r['idx']); c=r['corpus']
    if c not in dfc: dfc[c]=load_defs(c)
    rec=dict(candidate=r['candidate'],corpus=c,site=f"{r['file']}:{r['def_line']}->{r['use_line']}",file=r['file'],def_line=r['def_line'],use_line=r['use_line'],variable=r['var'],
             edge=dict(confidence=r['confidence'],doubt=r['doubt']),def_text=r['def_text'],use_text=r['use_text'])
    rec['blocking_events']=sorted({(e[0],e[2][:160]) for e in r['events']})[:12]
    rec['dead']=why_dead(k,r)
    cand=r['candidate']
    if cand=='C1':
        cl=[]
        for ln,cls,why in (r.get('c1_callee_class') or []): cl.append(cls+(':allbyval' if cls=='C' and 'all-byval' in why else ''))
        cnt=collections.Counter(cl); rec['c1_callee_classes']=dict(cnt)
        bad=[x for x in cnt if x not in('A','C:allbyval')]
        if not cnt: rec['proof']='no call_arg event (constructor/brace kind)'; rec['bounded_local_proof']='unclear'
        elif not bad and set(cnt)=={'A'}: rec['proof']='every blocking callee is a single in-repo definition taking the argument by value / T*'; rec['bounded_local_proof']='yes'
        elif not bad: rec['proof']='in-repo overload sets in which every overload takes the argument by value/T*; needs a closed-world proof of the overload set (no ADL, template or external overload)'; rec['bounded_local_proof']='partly'
        else:
            what=[]
            if 'E' in bad: what.append('method call (needs receiver typing and class method lookup)')
            if 'D' in bad: what.append('external/unresolved callee (stdlib/libc/macro, no in-repo signature)')
            if 'B' in bad: what.append('by-reference parameter')
            if 'F' in bad: what.append('arity/variadic')
            if 'C' in bad: what.append('overload/template with a reference or short overload')
            rec['proof']='; '.join(what); rec['bounded_local_proof']='partly' if set(bad)<={'E'} else 'no'
    elif cand=='C2':
        m=re.match(r'^(?:const\s+)?((?:::)?[\w:]+)',r['def_text'] if not r['def_text'].startswith('(') else '')
        rec['proof']='type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enum spells it, plus an owner-scope check for block-local aliases'
        rec['bounded_local_proof']='yes'
    elif cand=='C3':
        rec['proof']='scalar local cast to a keyword scalar (static_cast<char>(code)) or an arithmetic right side whose own operators pass the operand certificate: the result is a prvalue and cannot bind int&'
        rec['bounded_local_proof']='yes'
    elif cand=='C4':
        rec['proof']='pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T * x;` cannot be a product; the declaration here has an initialiser (init_declarator)' if '=' in r['def_text'] or 'Item*' in r['def_text'] or True else ''
        rec['bounded_local_proof']='partly' if r['file'].endswith('_unittest.cc') else 'yes'
        if r['file'].endswith('_unittest.cc'): rec['proof']+=' [the sampled type is a generated protobuf test message (unittest .pb.h is not in the corpus), so the census cannot prove it: the proof fails here]'
    else:
        ps=[]
        for e in r['events']:
            if e[0]!='operand_certificate': continue
            fam,op,why=parse(e[2]); cat=CAT.get(why,why)
            ps.append(f'{fam}:{op}/{cat}')
        rec['c5_events']=sorted(set(ps))[:8]
        cats={p.split('/',1)[1] for p in ps}
        rec['proof']='; '.join(sorted({PROOF.get(c,c) for c in cats}))
        rec['bounded_local_proof']='yes' if cats<= {'other operand name escaped (cascade)'} else ('partly' if cats<={'global/unknown name','local with non-keyword type','other operand name escaped (cascade)'} else 'partly (needs callee/type proof)')
    out.append(rec)
with open(S+'samples.jsonl','w') as f:
    for r in out: f.write(json.dumps(r,sort_keys=True)+'\n')
print(len(out)); print(collections.Counter((r['candidate'],r['bounded_local_proof']) for r in out))
