import json,sys,collections
a=json.load(open(sys.argv[1]))
print({k:a[k] for k in ('sites','unlocated','paths_from','program_files')})
cls=['repo_impl_named','repo_impl_anonymous','repo_impl_not_in_inventory','type_only_signature','outside_repo','no_declaration','unlocated']
t=collections.defaultdict(collections.Counter)
for k,v in a['tally'].items():
    b,part,verdict,c=k.split('|')
    if part!='product': continue
    t[(b,verdict)][c]+=v
print(f"{'binding / census verdict (product)':66}"+''.join(f'{c[:11]:>12}' for c in cls))
tot=collections.Counter()
for (b,verdict),c in sorted(t.items()):
    print(f"{b+' / '+verdict:66}"+''.join(f'{c[x] or "":>12}' for x in cls))
    if verdict not in('correct',): 
        for x in cls: tot[x]+=c[x]
print(f"{'TOTAL not already correct':66}"+''.join(f'{tot[x]:>12}' for x in cls))
