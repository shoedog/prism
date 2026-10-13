#!/usr/bin/env python3
"""Summarise out/matrix.jsonl into MATRIX.md (deterministic)."""
import json, collections, sys
import sys
SRC = sys.argv[1] if len(sys.argv) > 1 else 'out/matrix.jsonl'
rows = [json.loads(l) for l in open(SRC)]
bins = list(rows[0]['results'].keys())
order = ['CORRECT', 'PARTIAL', 'DEMOTED', 'MISSING', 'WRONG', 'OVERCLAIM', 'MAY-NAMEONLY', 'MAY-NONE', 'NONE-OK', 'NONE+NAMEONLY', 'NO-RECORD', 'ERROR']
out = []
out.append('# Alias-following variant matrix\n')
ORACLE_NOTES = {
    'destructured-props-default-overridden': 'in JavaScript the checker types a defaulted destructured parameter by its default, so it names the default; the caller passes a different function',
    'named-js-suffix-dts-only': 'the checker names the `.d.ts` signature, which has no body; there is no in-repo callable',
    'hook-result-two-slots': 'in JavaScript the checker types the returned array literal as an array of a union and names its first member; the destructure takes the second slot',
    'named-js-suffix-literal-and-ts': 'policy row: TypeScript binds `x.ts`; prism keeps the literal `x.js` as main does',
}
def oracle_ok(r):
    o, t = r['oracle'], r['truth']
    if t['t'] == 'one': return o == t['target']
    if t['t'] == 'set': return o in t['targets']
    if t['t'] == 'may': return (o in t['targets']) or not t['targets']
    if t['t'] == 'anon': return o.startswith(t['file'] + ':')
    return False
typed = [r for r in rows if ':' in r['oracle']]
differ = [r for r in typed if not oracle_ok(r)]
unexplained = [r['id'] for r in differ if r['shape'] not in ORACLE_NOTES]
assert not unexplained, unexplained
out.append(f'{len(rows)} enumerated fixtures, each a small repo with one marked call site and an authored ground truth. '
           f'The pinned TypeScript 5.9.3 checker can type the callee on {len(typed)} of them (the rest go through `any`, a parameter or a wrapper) '
           f'and agrees with the authored truth on {len(typed) - len(differ)}. The {len(differ)} that differ are shapes where the checker answers a different question: '
           + '; '.join(f"`{k}` ({sum(1 for r in differ if r['shape'] == k)}): {v}" for k, v in ORACLE_NOTES.items()) + '. '
           'Binaries: `main` = frozen base (main `bab0dfbd`), `e5` = E5 step-2b head' + (', `i1` = the import-suffix branch `aa74eae0`' if len(bins) > 2 else '') + '.\n')
out.append('Verdicts: CORRECT (Exact edge to the true callee), PARTIAL (Exact to some of a closed set), MISSING (no edge, a true in-repo callee exists), '
           'WRONG (Exact edge to something else), OVERCLAIM (Exact where the callee is only possible), MAY-NONE (no edge where the callee is only possible), '
           'NONE-OK (no edge, no in-repo callee), NO-RECORD (prism has no call-site record there).\n')
out.append('## Totals by lookup step\n')
steps = ['local', 'returned', 'param-args', 'param-lookup', 'import', 'import-path', 'same-file', 'global', 'none']
for b in bins:
    tally = collections.Counter((r['step'], r['results'][b]['verdict']) for r in rows)
    used = [v for v in order if any(tally[(s, v)] for s in steps)]
    out.append(f'**{b}**\n')
    out.append('| Step | Fixtures | ' + ' | '.join(used) + ' |')
    out.append('|---|---:|' + '---:|' * len(used))
    for s in steps:
        n = sum(tally[(s, v)] for v in order)
        out.append(f'| {s} | {n} | ' + ' | '.join(str(tally[(s, v)] or '') for v in used) + ' |')
    out.append('')
def short(res):
    v = res['verdict']
    if v in ('WRONG', 'CORRECT', 'PARTIAL', 'OVERCLAIM'):
        return v + ' via ' + ','.join(sorted(set(t['kind'] for t in res['exact'])))
    if res.get('drop'):
        return v + ' (' + res['drop'] + ')'
    return v
out.append('## Per shape\n')
out.append('One row per shape, scope, invocation form and naming; `ts` and `js` are merged when they agree. Naming: `same` = the alias has the true callee\'s name; '
           '`trap` = the alias has the name of a different, repo-unique callable; `fresh` = the alias name matches nothing.\n')
out.append('| Step | Shape | Scope | Form | Naming | Truth | ' + ' | '.join(bins) + ' | Contract question |')
out.append('|---|---|---|---|---|---|' + '---|' * len(bins) + '---|')
groups = collections.OrderedDict()
for r in rows:
    groups.setdefault((r['step'], r['shape'], r['scope'], r['form'], r['naming']), []).append(r)
for (step, shape, scope, form, naming), rs in groups.items():
    def cell(b):
        vals = collections.OrderedDict()
        for r in rs:
            vals.setdefault(short(r['results'][b]), []).append(r['lang'])
        if len(vals) == 1:
            return next(iter(vals))
        return '; '.join(f"{'/'.join(l)}: {v}" for v, l in vals.items())
    t = rs[0]['truth']
    truth = {'one': 'one callee', 'set': 'closed set', 'may': 'possible only', 'none': 'no in-repo callee', 'anon': 'inline callable'}[t['t']]
    out.append(f"| {step} | {shape} | {scope} | {form} | {naming} | {truth} | " + ' | '.join(cell(b) for b in bins) + f" | {rs[0]['contract'] or ''} |")
# --- population per census class (product sites; private corpus as aggregate counts only)
import os
cmap = json.load(open('census_map.json'))
B = os.path.expanduser('~/prism-evidence/js-alias/step0b')
srcs = {'X': B + '/X/decision-summary.json', 'T': B + '/T/decision-summary.json', 'F': B + '/f-alias-step0b.stdout.json'}
pop = {}
for corp, path in srcs.items():
    if not os.path.exists(path):
        continue
    det = json.load(open(path))['part1a']['detailed']
    for k, v in det.items():
        cls, part = k.split('|')
        if part != 'product':
            continue
        pop.setdefault(cls, {})[corp] = v
have = set(r['shape'] for r in rows)
def key_for(cls):
    if cls in cmap:
        return cls
    for pre in ('local.let', 'local.var'):
        if cls.startswith(pre):
            return pre
    return None
out.append('\n## Population per census class\n')
out.append('Recorded product sites per binding class from the census (X = Excalidraw, T = TypeScript compiler, F = private corpus, aggregate counts only), '
           'with what main does there today (C correct / W wrong / other) and the matrix shapes that cover the class. A class with sites and no shape is a hole in the matrix.\n')
out.append('| Census class | X sites (C/W) | T sites (C/W) | F sites (C/W) | Matrix shapes |')
out.append('|---|---|---|---|---|')
holes = []
for cls in sorted(pop):
    cells = []
    for corp in ('X', 'T', 'F'):
        v = pop[cls].get(corp)
        cells.append(f"{v['sites']} ({v.get('correct', 0)}/{v.get('wrong', 0)})" if v else '0')
    k = key_for(cls)
    shapes = [x for x in (cmap.get(k) or []) if x in have]
    if not shapes:
        holes.append(cls)
    out.append(f"| {cls} | {' | '.join(cells)} | {', '.join(shapes) or '**none**'} |")
out.append('')
out.append('Classes with sites and no fixture: ' + (', '.join(holes) or 'none') + '.')
unmapped = sorted(have - set(x for v in cmap.values() for x in v))
out.append('\nShapes with no census class (constructed, or outside the bare-name census): ' + (', '.join(unmapped) or 'none') + '.')
open('MATRIX.md', 'w').write('\n'.join(out) + '\n')
print('wrote MATRIX.md', len(out), 'lines')
