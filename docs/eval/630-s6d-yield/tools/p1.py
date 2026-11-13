import re,glob,collections,os,json
S='/tmp/claude-0/-home-user-prism/817300ea-683f-51d6-b28c-8f12e9d2d739/scratchpad/'
root=S+'corpora/redis/src/'
cands=[l.rstrip('\n').split('\t') for l in open(S+'out/redis_src.log') if l.startswith('P1')]
def strip_comments(t):
    t=re.sub(r'/\*.*?\*/',lambda m:'\n'*m.group(0).count('\n'),t,flags=re.S)
    return re.sub(r'//[^\n]*','',t)
objs=collections.defaultdict(set); types=collections.defaultdict(set); enums=collections.defaultdict(set); macros=collections.defaultdict(set)
hdrs=[p for p in glob.glob(root+'**/*.h',recursive=True)]
for h in hdrs:
    t=strip_comments(open(h,errors='replace').read())
    rel=os.path.relpath(h,root)
    # object-like macros
    for m in re.finditer(r'^\s*#\s*define\s+([A-Za-z_]\w*)(?!\()',t,flags=re.M):
        macros[m.group(1)].add(rel)
    # typedef names and struct/union/enum tags
    for m in re.finditer(r'\btypedef\b[^;{]*?\b([A-Za-z_]\w*)\s*(?:\[[^\]]*\])?\s*;',t): types[m.group(1)].add(rel)
    for m in re.finditer(r'\b(?:struct|union|enum)\s+([A-Za-z_]\w*)\s*\{',t): types[m.group(1)].add(rel)
    for m in re.finditer(r'\}\s*([A-Za-z_]\w*)\s*;',t):  # typedef struct {...} name;
        types[m.group(1)].add(rel)
    # enumerators
    for m in re.finditer(r'\benum\b[^{;]*\{(.*?)\}',t,flags=re.S):
        for e in re.finditer(r'(?:^|,|\{)\s*([A-Za-z_]\w*)\s*(?:=[^,}]*)?(?=,|$)',m.group(1)):
            enums[e.group(1)].add(rel)
    # variables: extern decls, and file-scope definitions at column 0
    for m in re.finditer(r'^\s*extern\s+(?!"C")([^;()]*?)\b([A-Za-z_]\w*)\s*(?:\[[^\]]*\])*\s*;',t,flags=re.M):
        objs[m.group(2)].add(rel)
    for m in re.finditer(r'^(?!typedef|extern|#|\s|struct\s+\w+\s*\{|union|enum|static\s+inline|return)([A-Za-z_][\w \t\*]*?)\b([A-Za-z_]\w*)\s*(?:\[[^\]]*\])*\s*(?:=[^;]*)?;',t,flags=re.M):
        if '(' in m.group(0): continue
        if re.fullmatch(r'\s*(struct|union|enum)\s+\w+\s*;',m.group(0)): continue
        if re.match(r'\s*(struct|union|enum)\b',m.group(0)) and '*' in m.group(1) and False: continue
        objs[m.group(2)].add(rel)
tn=collections.Counter(c[3] for c in cands)
rows=[]
def cls(n):
    o=n in objs; t=n in types; e=n in enums; mc=n in macros
    return ('obj' if o else '')+('+type' if t and o else ('type' if t else ''))+('enum' if e else '')+('macro' if mc else '') or 'none'
cc=collections.Counter(); cands_single=[c for c in cands if re.match(r'^\*\s*\w+$',c[4].strip())]
byname=collections.defaultdict(list)
for c in cands: byname[c[3]].append(c)
for n,v in byname.items(): cc[cls(n)]+=len(v)
print('total candidates (ident*ident; shape, first ident not spelled as object in file):',len(cands),' single-star:',len(cands_single))
print('distinct first idents',len(byname))
print(cc.most_common())
sing=collections.Counter(cls(c[3]) for c in cands_single); print('single-star by class',sing.most_common())
obj_firing=[c for c in cands if cls(c[3]) in('obj','enum','obj+type','objenum')]
print('FIRING (first ident is an object/enumerator in a header, not a type):',len([c for c in cands if cls(c[3]) in ('obj','enum')]),' ; also type:',len([c for c in cands if cls(c[3])=='obj+type']))
for k in ('obj','enum','obj+type','macro'):
    names=sorted({c[3] for c in cands if cls(c[3])==k}); print(k,len(names),names[:25])
json.dump({'total':len(cands),'single_star':len(cands_single),'by_class':cc,'single_by_class':sing,
  'examples':{k:[c for c in cands if cls(c[3])==k][:5] for k in ('obj','enum','obj+type','macro','none','type')}},open(S+'out/p1.json','w'),indent=1)
print(sorted(tn.items(),key=lambda x:-x[1])[:25])
