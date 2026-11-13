import json,re
S='/tmp/claude-0/-home-user-prism/817300ea-683f-51d6-b28c-8f12e9d2d739/scratchpad/'
root=S+'corpora/'
rs=json.load(open(S+'out/samples_raw.json'))
def lines(c,f): return open(f'{root}{c}/{f}',errors='replace').read().split('\n')
out=[]
for r in rs:
    L=lines(r['corpus'],r['file']); f=r['def_line']; t=r['use_line']
    out.append(f"## {r['candidate']}#{r['idx']} {r['corpus']}:{r['file']} var={r['var']}{'.'.join(['']+r['fields'])} def@{f} use@{t} {r['confidence']}/{r['doubt']}")
    def show(a,b):
        for ln in range(max(1,a),min(len(L),b)+1): out.append(f'  {ln}| {L[ln-1].strip()[:120]}')
    show(f-1,f+1); out.append('  ..'); show(t-2,t+2)
    ev={}
    for k,l,x in r['events']:
        ev.setdefault(k,[]).append(x)
    for k,v in ev.items(): out.append(f'  EV {k} x{len(v)}: '+' || '.join(v[:3])[:260])
    if 'c1_callee_class' in r: out.append('  C1: '+str([c for c in r['c1_callee_class']][:4])[:300])
open(S+'out/dossier2.txt','w').write('\n'.join(out))
