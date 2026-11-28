#!/usr/bin/env python3
"""Offline ceiling pilot for item 4 (statement-level dependence over MCP).

Simulates what a `slice_at` query would reach with TODAY's substrate (DFG labels,
CFG, Exact-gated taint descent), using only the existing CLI primitives:
  forward  = `prism nav taint-reaches --source file:line` (parameter Defs are keyed at
             the function start line, so a function-start seed == "all parameters")
  backward = `prism --diff <one-line diff> --algorithm fullflow[,relevant]`
Then scores the reached (file,line) set against gold v2 with the DIRECTED harness's
own recall-ceiling rule (all required citation slots have an accepted location).

Tiers (fixed in the spec before the probes; see the top-level README for how that is witnessed):
  T0  one call per question-text seed (what DIRECTED did, plus the backward step)
  T1  + one mechanical follow-up round driven by the tool's own output:
      - backward: fullflow from every line the depth-1 result contained (depth-2 closure)
      - forward: field-store stop -> grep reads of that field in the same file -> seed
      - forward: callees d2 of the seed function that are unique in-repo functions ->
        seed their start line (simulates `reach: nameonly` descent)
Usage: python3 -I gate_pilot.py <prism-bin> <cache-base> <out-json>

Required environment (the trial harness and gold are NOT in the public repo):
  PRISM_TRIAL_DIR    tracing-trial directory of the private evidence store (match.py,
                     evidence/directed-results.py, gold/v2/)
  PRISM_BENCH_REPOS  directory holding the pinned checkouts httpx/, cobra/, excalidraw/
Optional:
  PRISM_GOLD_DIR     gold v2 directory (default: $PRISM_TRIAL_DIR/gold/v2)
  PRISM_PILOT_UNITS  set to 1 to also write the covered gold-unit ids to <out-json>
                     (the committed report omits them: gold units stay private)
Write <out-json> to a scratch directory, never over the committed pilot-report.json;
the one-line diff files are written next to it.
"""
import json, re, subprocess, sys, os, collections
from pathlib import Path

def _need_dir(var, what, must_contain=()):
    v = os.environ.get(var)
    if not v:
        sys.exit(f'gate_pilot.py: environment variable {var} is not set ({what}). See the README.')
    p = Path(v)
    if not p.is_dir():
        sys.exit(f'gate_pilot.py: {var}={v} is not a directory ({what}).')
    for rel in must_contain:
        if not (p / rel).exists():
            sys.exit(f'gate_pilot.py: {var}={v} has no {rel} ({what}).')
    return p

if len(sys.argv) != 4:
    sys.exit('usage: python3 -I gate_pilot.py <prism-bin> <cache-base> <out-json>')
TRIAL = _need_dir('PRISM_TRIAL_DIR', 'private tracing-trial directory',
                  ('match.py', 'evidence/directed-results.py'))
BENCH = _need_dir('PRISM_BENCH_REPOS', 'directory with the httpx, cobra and excalidraw checkouts',
                  ('httpx', 'cobra', 'excalidraw'))
GOLD = Path(os.environ['PRISM_GOLD_DIR']) if os.environ.get('PRISM_GOLD_DIR') else TRIAL / 'gold' / 'v2'
if not GOLD.is_dir():
    sys.exit(f'gate_pilot.py: gold directory {GOLD} not found (set PRISM_GOLD_DIR).')
KEEP_UNITS = os.environ.get('PRISM_PILOT_UNITS') == '1'
sys.path.insert(0, str(TRIAL)); sys.path.insert(0, str(TRIAL / 'evidence'))
import match  # harness matcher (gold rule accessors)
import importlib.util
_spec = importlib.util.spec_from_file_location('dr', TRIAL / 'evidence' / 'directed-results.py')
dr = importlib.util.module_from_spec(_spec); _spec.loader.exec_module(dr)

PRISM, CACHE, OUT = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
if not os.access(PRISM, os.X_OK):
    sys.exit(f'gate_pilot.py: prism binary {PRISM} is not an executable file.')
OUT.parent.mkdir(parents=True, exist_ok=True)
REPOS = {'httpx': BENCH / 'httpx', 'cobra': BENCH / 'cobra', 'excalidraw': BENCH / 'excalidraw'}
CALLS = collections.Counter()
LOG = []

def run(args, timeout=600):
    p = subprocess.run(args, capture_output=True, text=True, timeout=timeout)
    return p.stdout, p.stderr, p.returncode

def taint(corpus, seed):
    CALLS['taint'] += 1
    out, err, rc = run([PRISM, 'nav', '--cache-dir', str(CACHE / f'cache-{corpus}'), 'taint-reaches',
                        '--repo', str(REPOS[corpus]), '--source', seed, '--format', 'json'])
    try:
        d = json.loads(out)
    except ValueError:
        LOG.append(f'{corpus} taint {seed}: no JSON ({err[:120]!r})'); return set(), [], {}
    if 'error' in d:
        LOG.append(f'{corpus} taint {seed}: {json.dumps(d["error"])[:160]}'); return set(), [], d
    locs = {(i['location']['file'], i['location']['start_line']) for i in d.get('items', [])}
    return locs, d.get('warnings', []), d

def backward(corpus, file, line, algos='fullflow,relevant'):
    CALLS['slice'] += 1
    src = (REPOS[corpus] / file).read_text().splitlines()
    text = src[line - 1]
    diff = OUT.parent / f'diff-{corpus}-{Path(file).name}-{line}.diff'
    diff.write_text(f'diff --git a/{file} b/{file}\n--- a/{file}\n+++ b/{file}\n@@ -{line},1 +{line},1 @@\n-{text}\n+{text}\n')
    out, err, rc = run([PRISM, '--repo', str(REPOS[corpus]), '--diff', str(diff), '--algorithm', algos,
                        '--format', 'json', '--cache-dir', str(CACHE / f'cache-{corpus}')])
    try:
        d = json.loads(out)
    except ValueError:
        LOG.append(f'{corpus} slice {file}:{line}: no JSON ({err[:120]!r})'); return set()
    results = d.get('results') or [d]
    locs = set()
    for r in results:
        for s in r.get('slices', []):
            locs.update((s['file'], n) for n in s.get('slice_lines', []))
    return locs

def callees(corpus, file, line, depth=2):
    CALLS['callees'] += 1
    out, err, rc = run([PRISM, 'nav', '--cache-dir', str(CACHE / f'cache-{corpus}'), 'callees',
                        '--repo', str(REPOS[corpus]), '--location', f'{file}:{line}', '--depth', str(depth), '--format', 'json'])
    try:
        d = json.loads(out)
    except ValueError:
        return []
    fns = []
    for i in d.get('items', []):
        sym = (i.get('symbol') or {}).get('Function')
        if sym and 'test' not in sym['file'] and sym['file'].startswith(('httpx/', 'command', 'args', 'excalidraw-app/', 'packages/')):
            fns.append((sym['file'], sym['start_line'], sym['name'], i.get('score')))
    return fns

def field_store_followups(corpus, data, file):
    """Frontier Defs with a field path (e.g. `c.args`) -> lines in `file` reading that field."""
    seeds = set()
    src = (REPOS[corpus] / file).read_text().splitlines()
    for i in data.get('items', []):
        sym = (i.get('symbol') or {}).get('Variable')
        if not sym or sym.get('access') != 'def':
            continue
        path = sym.get('path', '')
        if '.' not in path:
            continue
        field = path.split('.')[-1]
        pat = re.compile(r'\.' + re.escape(field) + r'\b(?!\s*=[^=])')
        for n, text in enumerate(src, 1):
            if pat.search(text) and n != sym['line']:
                seeds.add(n)
    return sorted(seeds)[:5]

def ceiling(qid, locs):
    gold = json.load(open(GOLD / f'{qid}.json'))
    rules = match.required_rules(gold)
    strs = {f'{f}:{l}' for f, l in locs}
    covered = [r['id'] for r in rules if dr.rule_covered(r, strs)]
    out = dict(covered=len(covered), total=len(rules), share=round(len(covered) / len(rules), 4),
               lines=len(locs))
    if KEEP_UNITS:
        out['covered_units'] = covered
    return out

report = {}

# ---------------- P4 (guards, httpx): backward from the target line 922 ----------------
p4_t0 = backward('httpx', 'httpx/_client.py', 922)
p4_t1 = set(p4_t0)
for f, l in sorted(p4_t0):
    if f == 'httpx/_client.py':
        p4_t1 |= backward('httpx', f, l, 'fullflow')
report['P4'] = dict(seeds=['httpx/_client.py:922 (target line in question)'],
                    T0=ceiling('P4', p4_t0), T1=ceiling('P4', p4_t1),
                    T0_lines=sorted(p4_t0), T1_lines=sorted(p4_t1))

# ---------------- P3 (data, httpx): forward from Client.request params; Request.__init__ ----------------
# question names Client.request (parameter headers) and Request. Parameter Defs are keyed at the
# function start line (data_flow/build/file.rs:155), so the start line is the "all params" seed.
p3_t0 = set()
for seed in ['httpx/_client.py:771', 'httpx/_client.py:781', 'httpx/_models.py:383', 'httpx/_models.py:389']:
    locs, warns, d = taint('httpx', seed); p3_t0 |= locs
p3_t1 = set(p3_t0)
followups = callees('httpx', 'httpx/_client.py', 820, 2)
seen = set()
for f, l, name, score in followups:
    if (f, l) in seen or len(seen) >= 15:
        continue
    seen.add((f, l))
    locs, _, _ = taint('httpx', f'{f}:{l}'); p3_t1 |= locs
report['P3'] = dict(seeds=['Client.request params (771 start line; 781 fails)', 'Request.__init__ params (383)'],
                    T1_followups=[f'{f}:{l} {name} score={score}' for f, l, name, score in followups][:30],
                    T0=ceiling('P3', p3_t0), T1=ceiling('P3', p3_t1))

# ---------------- G3 (data, cobra): forward from SetArgs(a), execute(a), ValidateArgs(args) ----------------
g3_t0 = set(); g3_data = {}
for seed in ['command.go:281', 'command.go:905', 'command.go:1172', 'command.go:1084']:
    locs, warns, d = taint('cobra', seed); g3_t0 |= locs; g3_data[seed] = d
g3_t1 = set(g3_t0)
fs = field_store_followups('cobra', g3_data['command.go:281'], 'command.go')
for n in fs:
    locs, _, _ = taint('cobra', f'command.go:{n}'); g3_t1 |= locs
report['G3'] = dict(seeds=['SetArgs 281', 'execute 905', 'ValidateArgs 1172', 'ExecuteC 1084 (no params)'],
                    field_store_followups=[f'command.go:{n}' for n in fs],
                    T0=ceiling('G3', g3_t0), T1=ceiling('G3', g3_t1))

# ---------------- J3 (data, excalidraw): forward from onChange(elements) at App.tsx:677 ----------------
j3_t0 = set()
for seed in ['excalidraw-app/App.tsx:677', 'excalidraw-app/App.tsx:678']:
    locs, warns, d = taint('excalidraw', seed); j3_t0 |= locs
j3_t1 = set(j3_t0)
followups = callees('excalidraw', 'excalidraw-app/App.tsx', 689, 2)
seen = set()
for f, l, name, score in followups:
    if (f, l) in seen or len(seen) >= 15:
        continue
    seen.add((f, l))
    locs, _, _ = taint('excalidraw', f'{f}:{l}'); j3_t1 |= locs
report['J3'] = dict(seeds=['onChange arrow at App.tsx:677 (678 fails)'],
                    T1_followups=[f'{f}:{l} {name} score={score}' for f, l, name, score in followups][:30],
                    T0=ceiling('J3', j3_t0), T1=ceiling('J3', j3_t1))

report['_calls'] = dict(CALLS)
report['_log'] = LOG
OUT.write_text(json.dumps(report, indent=1))
for q in ['G3', 'J3', 'P3', 'P4']:
    r = report[q]
    print(q, 'T0', r['T0']['covered'], '/', r['T0']['total'], f"({r['T0']['share']}) lines={r['T0']['lines']}",
          '| T1', r['T1']['covered'], '/', r['T1']['total'], f"({r['T1']['share']}) lines={r['T1']['lines']}")
print('calls', dict(CALLS))
for line in LOG:
    print('  log:', line)
