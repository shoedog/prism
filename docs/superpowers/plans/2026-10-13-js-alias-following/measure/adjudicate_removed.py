#!/usr/bin/env python3
"""Adjudicate edges REMOVED between two site dumps, without types.
  adjudicate_removed.py <edge-diff-rows.jsonl> <census sites.jsonl> <callresult-rows.jsonl>
For each changed site that lost an Exact target (or NameOnly candidates), join the census row and the
syntactic value-following row for the site and classify the removed target:
  matches_followed_value   the removed target IS the callee the syntactic follow derives (a correct edge lost)
  differs_from_followed    the follow derives a different in-repo callee (a wrong edge removed)
  no_in_repo_callee        the binding's value comes from React state, an npm hook, a tagged template… (wrong edge removed)
  value_unknown:<class>    the value is a parameter member, a context value, a method-call result… (truth unknown)
Prints aggregate counts only."""
import json, sys, collections
diff_p, sites_p, cr_p = sys.argv[1:4]
census = {}
for l in open(sites_p):
    r = json.loads(l)
    span = (r.get('join_spans') or [r['span']])[0]
    census[(r['file'], span[0], span[1], r['name'])] = r
cr = {}
for l in open(cr_p):
    c = json.loads(l)
    cr[(c['file'], c['span'][0], c['span'][1], c['name'])] = c
NONE = {'react_state_setter', 'react_state_value', 'tagged_template', 'callee_external_package', 'react_ref', 'literal_nullish', 'new_instance', 'object_value', 'array_value'}
agg = collections.Counter(); nameonly_sites = 0; nameonly_edges = 0; added = collections.Counter()
for l in open(diff_p):
    r = json.loads(l)
    if r.get('change') == 'record':
        agg[('RECORD ' + ('removed' if r.get('head') is None else 'added'), '', '')] += 1
        continue
    file, sb, eb, callee, origin = r['key']
    bx = [t for t in r['base_targets'] if t[0] == 'exact']; hx = [t for t in r['head_targets'] if t[0] == 'exact']
    hk = {tuple(t[2:]) for t in hx}
    removed = [t for t in bx if tuple(t[2:]) not in hk]
    bk = {tuple(t[2:]) for t in bx}
    for t in hx:
        if tuple(t[2:]) not in bk: added[(r['part'], t[1])] += 1
    bn = [t for t in r['base_targets'] if t[0] != 'exact']; hn = [t for t in r['head_targets'] if t[0] != 'exact']
    if len(bn) > len(hn) and not removed:
        nameonly_sites += 1; nameonly_edges += len(bn) - len(hn)
    if not removed:
        continue
    cen = census.get((file, sb, eb, callee)); c = cr.get((file, sb, eb, callee))
    for t in removed:
        kind = t[1]
        if origin != 'Source':
            cat = 'indirect_record'
        elif c is None:
            cat = 'site_not_in_local_census:' + (cen['group'] if cen else 'no_census_row')
        elif c['cls'].startswith('fn') and c.get('target'):
            cat = 'matches_followed_value' if (t[2] == c['target']['file'] and t[4] == c['target']['line']) else 'differs_from_followed'
            cat += f" (hops {c['hops']})"
        elif c['cls'] in NONE:
            cat = 'no_in_repo_callee'
        else:
            cat = 'value_unknown:' + c['cls']
        agg[(r['part'], f"{origin}:{kind}", cat, (cen or {}).get('verdict', '-'))] += 1
print('REMOVED exact targets: (part, origin:kind, category, census verdict on main)')
for k, v in sorted(agg.items(), key=lambda kv: (str(kv[0][0]), str(kv[0][1]), -kv[1])):
    print(f"{v:6d}  " + ' | '.join(str(x) for x in k))
print(f"sites that only lost NameOnly candidates: {nameonly_sites} (candidate edges removed: {nameonly_edges})")
print('ADDED exact targets:', dict(added))
