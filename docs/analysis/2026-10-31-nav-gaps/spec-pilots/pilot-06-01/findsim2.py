import json, re, collections, os, sys
# Simulated find_symbol ranking (spec 01 section 8.3). Run from any directory:
#   PRISM_BENCH_REPOS  directory with the pinned checkouts httpx/ ripgrep/ cobra/ excalidraw/ (required)
#   PRISM_PILOT_INPUTS directory holding the regenerated fn-<corpus>.json and cs-<corpus>.jsonl (default: cwd)
# seeds.tsv is read from this script's directory. Any missing or unreadable file stops the run.
HERE=os.path.dirname(os.path.abspath(__file__))
BR=os.environ.get('PRISM_BENCH_REPOS') or sys.exit('findsim2.py: PRISM_BENCH_REPOS is not set (directory with the pinned corpus checkouts)')
if not os.path.isdir(BR): sys.exit('findsim2.py: PRISM_BENCH_REPOS=%s is not a directory' % BR)
def inp(name): return os.path.join(os.environ.get('PRISM_PILOT_INPUTS','.'),name)
TEST_COMP={'test','tests','__tests__','testdata'}
def is_test(p):
    comps=[c.lower() for c in p.split('/') if c]; b=comps[-1]
    return any(c in TEST_COMP for c in comps) or (b.startswith('test_') and b.endswith('.py')) or b.endswith(('_test.py','_test.go','_test.rs')) or '.test.' in b or '.spec.' in b
src_cache={}
def lines(c,f):
    k=(c,f)
    if k not in src_cache:
        path=os.path.join(BR,c,f)
        try: src_cache[k]=open(path,errors='replace').read().split('\n')
        except OSError as e: sys.exit('findsim2.py: cannot read source file %s (%s); wrong PRISM_BENCH_REPOS or wrong corpus revision?' % (path,e))
    return src_cache[k]
def owner(c,r):
    L=lines(c,r['file']); i=r['start_line']-1
    if not L or i>=len(L): return (None,None)
    f=r['file']
    if f.endswith('.go'):
        m=re.match(r'\s*func\s*\(\s*\w*\s*\*?\s*(\w+)',L[i]); return (m.group(1) if m else None,None)
    if f.endswith('.py'):
        ind=len(L[i])-len(L[i].lstrip())
        for j in range(i-1,-1,-1):
            s=L[j]; 
            if not s.strip(): continue
            jind=len(s)-len(s.lstrip())
            if jind<ind:
                m=re.match(r'\s*class\s+(\w+)',s)
                if m: return (m.group(1),None)
                if re.match(r'\s*(async\s+)?def\s',s): return (None,None)
                ind=jind
        return (None,None)
    if f.endswith('.rs'):
        for j in range(i-1,-1,-1):
            s=L[j]
            if s.startswith('}'): return (None,None)
            m=re.match(r'\s*(?:unsafe\s+)?impl(?:<[^>]*>)?\s+(?:(\S+)\s+for\s+)?&?(?:mut\s+)?([A-Za-z_][\w:]*)',s)
            if m and (s.startswith('impl') or s.startswith('unsafe impl') or re.match(r'\s{4}impl',s)):
                tr=m.group(1); ty=m.group(2).split('::')[-1]
                return (ty, tr.split('<')[0].split('::')[-1] if tr else None)
            m=re.match(r'\s*(?:pub(?:\([^)]*\))?\s+)?trait\s+(\w+)',s)
            if m: return (m.group(1),None)
        return (None,None)
    if f.endswith(('.ts','.tsx','.js','.jsx')):
        for j in range(i-1,-1,-1):
            m=re.match(r'\s*(?:export\s+)?(?:default\s+)?(?:abstract\s+)?class\s+(\w+)',L[j])
            if m: return (m.group(1),None)
            if L[j].startswith('}'): return (None,None)
        return (None,None)
    return (None,None)
def words(n):
    parts=re.split(r'_+',n); out=[]
    for p in parts: out+= re.findall(r'[A-Z]+(?=[A-Z][a-z])|[A-Z]?[a-z0-9]+|[A-Z]+',p)
    return [w.lower() for w in out if w]
def name_tier(q,n):
    if n==q: return 0
    if n.lower()==q.lower(): return 1
    if n.lower().startswith(q.lower()): return 2
    qw=words(q); nw=words(n)
    # word-boundary: query words are prefixes of consecutive-or-ordered name words
    it=iter(nw)
    if qw and all(any(w.startswith(x) for w in it) for x in qw): return 3
    if q.lower() in n.lower(): return 4
    return None
def parse(q):
    q=re.sub(r'\(\*?(\w+)\)\.',r'\1.',q)
    segs=[s for s in re.split(r'::|\.|#',q) if s]
    return segs[-1], segs[:-1]
def search(c,query,fns,callers,ffilter=None):
    name,quals=parse(query); res=[]
    for r in fns:
        if ffilter and r['file']!=ffilter: continue
        if not r['name']: continue
        t=name_tier(name,r['name'])
        if t is None: continue
        ow,tr=r['_owner']
        qmatch=0
        if quals:
            ql=quals[-1]
            stem=r['file'].split('/')[-1].split('.')[0]
            comps=set(r['file'].replace('.','/').split('/'))
            if ql in (ow,tr): qmatch=0
            elif ql==stem or ql.lstrip('_')==stem.lstrip('_') or ql==r['file'].split('/')[-2:][0]: qmatch=1
            else: qmatch=2
        key=(t+ (0 if qmatch==0 else (1 if qmatch==1 else 3)), is_test(r['file']), -callers.get((r['file'],r['name'],r['start_line']),0), r['file'].count('/'), r['file'], r['start_line'])
        res.append((key,r))
    res.sort(key=lambda x:x[0])
    return res
allres=[]
for line in open(os.path.join(HERE,'seeds.tsv')):
    q,c,name,file,target,spelled=line.rstrip('\n').split('\t')
    if not os.path.isdir(os.path.join(BR,c)): sys.exit('findsim2.py: corpus checkout %s not found under PRISM_BENCH_REPOS' % os.path.join(BR,c))
    fns=json.load(open(inp(f'fn-{c}.json')))
    for r in fns: r['_owner']=owner(c,r)
    callers=collections.Counter()
    for l in open(inp(f'cs-{c}.jsonl')):
        d=json.loads(l)
        for t in d.get('resolved_targets',[]):
            fid=t['function_id']; callers[(fid['file'],fid['name'],fid['start_line'])]+=1
    res=search(c,spelled,fns,callers)
    if file!='-':
        res2=search(c,spelled,fns,callers,file)
        u2=len(res2)>0 and (len(res2)==1 or res2[0][0][0]<res2[1][0][0]); r2=res2[0][1]['file']+':'+str(res2[0][1]['start_line']) if res2 else None
        print('   with file filter:',r2==target and u2)
    top=[(r['file']+':'+str(r['start_line']), r['_owner'], k[0]) for k,r in res[:3]]
    rank=next((i+1 for i,(k,r) in enumerate(res) if r['file']+':'+str(r['start_line'])==target),None)
    uniq = len(res)>0 and (len(res)==1 or res[0][0][0]<res[1][0][0])
    print(q,spelled,'target',target,'rank',rank,'n',len(res),'uniqueTopTier',uniq,top)
