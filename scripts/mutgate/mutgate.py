#!/usr/bin/env python3
"""prism mutation gate: generated function-body mutant schemata + text-mode fallback.

One command:
    python3 scripts/mutgate/mutgate.py                 # full gate (every lane file)
    python3 scripts/mutgate/mutgate.py --since main    # scoped: mutants whose anchor
                                                       # touches lines changed since main
Options: --lane FILE (repeatable), --only ID (repeatable), --jobs N, --mode schema|text,
         --scope line|fn, --timeout SECS, --out DIR, --plan-only.
Build state lives in $CARGO_TARGET_DIR (default ./target) under mutgate/; it is reused across
rounds for incremental builds. Reclaim it with `rm -rf $CARGO_TARGET_DIR/mutgate`.

Production source is never edited. The schemata are generated into a scratch copy
of the tree under $CARGO_TARGET_DIR/mutgate/tree; each mutated function body becomes

    { match crate::__mutant() { "ID" => { <mutated body> } _ => { <original body> } } }

so the active arm is byte-identical to the text mutant's body. A mutant whose edit
falls outside a function body (struct fields, attributes, consts), or whose unit
fails to compile as a schema, runs in text mode (one incremental build per mutant).
"""
import argparse, concurrent.futures as cf, json, os, re, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LANES_GLOB = 'mutants/*.json'
LIB_PREFIXES_DEFAULT = ()
COPIED = ['src', 'vendor', 'Cargo.toml', 'Cargo.lock', 'build.rs']
HELPER = '''
#[doc(hidden)]
#[inline(never)]
pub fn __mutant() -> &'static str {
    static M: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    M.get_or_init(|| std::env::var("PRISM_MUTANT").unwrap_or_default()).as_str()
}
'''
ALLOW = '#![allow(unreachable_code, unused_variables, unused_mut, unused_assignments, dead_code, unused_imports)]\n'


# ---------------------------------------------------------------- Rust lexing
def code_mask(s):
    """Return a list of bools: True where s[i] is code (not comment/string/char)."""
    n = len(s); m = [True] * n; i = 0
    while i < n:
        c = s[i]
        if s.startswith('//', i):
            j = s.find('\n', i); j = n if j < 0 else j
            for k in range(i, j): m[k] = False
            i = j; continue
        if s.startswith('/*', i):
            depth = 1; j = i + 2
            while j < n and depth:
                if s.startswith('/*', j): depth += 1; j += 2
                elif s.startswith('*/', j): depth -= 1; j += 2
                else: j += 1
            for k in range(i, j): m[k] = False
            i = j; continue
        r = re.match(r'b?r(#*)"', s[i:i + 300]) if c in 'br' and (i == 0 or not (s[i - 1].isalnum() or s[i - 1] == '_')) else None
        if r:
            close = '"' + r.group(1); j = s.find(close, i + r.end()); j = n if j < 0 else j + len(close)
            for k in range(i, j): m[k] = False
            i = j; continue
        if c == '"' or (c == 'b' and s.startswith('b"', i) and (i == 0 or not (s[i - 1].isalnum() or s[i - 1] == '_'))):
            j = i + (2 if c == 'b' else 1)
            while j < n and s[j] != '"':
                j += 2 if s[j] == '\\' else 1
            j += 1
            for k in range(i, min(j, n)): m[k] = False
            i = j; continue
        if c == "'":
            ch = re.match(r"'(\\(x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|.)|[^\\'])'", s[i:i + 16])
            if ch:
                for k in range(i, i + ch.end()): m[k] = False
                i += ch.end(); continue
        i += 1
    return m


def fn_bodies(s, mask):
    """[(fn_start, body_open, body_close)] for every `fn name ... { ... }` item."""
    out = []
    for f in re.finditer(r'\bfn\s+[A-Za-z_][A-Za-z0-9_]*', s):
        if not mask[f.start()]: continue
        depth = 0; j = f.end(); body = None
        while j < len(s):
            if mask[j]:
                ch = s[j]
                if ch in '([': depth += 1
                elif ch in ')]': depth -= 1
                elif depth == 0 and ch == ';': break
                elif depth == 0 and ch == '{': body = j; break
            j += 1
        if body is None: continue
        d = 0; k = body
        while k < len(s):
            if mask[k]:
                if s[k] == '{': d += 1
                elif s[k] == '}':
                    d -= 1
                    if d == 0: break
            k += 1
        out.append((f.start(), body, k))
    return out


def min_diff(a, b):
    p = 0
    while p < min(len(a), len(b)) and a[p] == b[p]: p += 1
    q = 0
    while q < min(len(a), len(b)) - p and a[-1 - q] == b[-1 - q]: q += 1
    return p, len(a) - q, b[p:len(b) - q]


def occurrences(s, a):
    out = []; i = s.find(a)
    while i >= 0:
        out.append(i); i = s.find(a, i + len(a))
    return out


# ---------------------------------------------------------------- population
def load_population(lane_files):
    muts = {}
    for lf in lane_files:
        d = json.loads(Path(lf).read_text())
        extra = d.get('extra', {})
        for mid, (file, a, b, test) in d['mutations'].items():
            assert mid not in muts, ('duplicate mutant id', mid, lf)
            edits = [(file, a, b)] + [tuple(e) for e in extra.get(mid, [])]
            tests = test if isinstance(test, list) else [test]
            muts[mid] = {'id': mid, 'lane': Path(lf).stem, 'edits': edits, 'tests': tests,
                         'lib_prefixes': tuple(d.get('lib_test_prefixes', LIB_PREFIXES_DEFAULT))}
    return muts


def test_target(m, test):
    return 'lib' if test.startswith(m['lib_prefixes']) else m.get('test_target', 'integration')


# ---------------------------------------------------------------- schemata
def plan_schemata(muts, src_text):
    """Return (units, static). units: (file, body_open, body_close) -> {id: [(start, end, replacement)]}."""
    static = {}
    for m in muts.values():
        for file, a, b in m['edits']:
            s = src_text.get(file)
            if s is None or a not in s:
                static[m['id']] = 'anchor-missing'; break
    bodies = {}
    for file, s in src_text.items():
        bodies[file] = fn_bodies(s, code_mask(s))
    units = {}  # (file, open, close) -> {id: [(start,end,repl)]}
    for m in muts.values():
        if m['id'] in static: continue
        placed = []
        for file, a, b in m['edits']:
            s = src_text[file]
            p, e_rel, repl = min_diff(a, b)
            for occ in occurrences(s, a):
                st, en = occ + p, occ + e_rel
                outer = [bd for bd in bodies[file] if bd[1] < st and en <= bd[2]]
                if not outer:
                    placed = None; break
                o = min(outer, key=lambda bd: bd[1])  # outermost enclosing fn body
                placed.append(((file, o[1], o[2]), (st, en, repl)))
            if placed is None: break
        if not placed:
            static[m['id']] = 'outside-fn-body'; continue
        for unit, ed in placed:
            units.setdefault(unit, {}).setdefault(m['id'], []).append(ed)
    # outermost-unit normalisation: drop units nested in other units of the same file
    keys = sorted(units)
    for u in keys:
        for v in keys:
            if u != v and u[0] == v[0] and v[1] < u[1] and u[2] <= v[2] and u in units and v in units:
                for mid, eds in units.pop(u).items():
                    units[v].setdefault(mid, []).extend(eds)
    return units, static


def render(src_text, units, accessor):
    out = dict(src_text)
    by_file = {}
    for (file, o, c), arms in units.items():
        by_file.setdefault(file, []).append((o, c, arms))
    for file, us in by_file.items():
        s = src_text[file]
        for o, c, arms in sorted(us, reverse=True):
            inner = s[o + 1:c]
            parts = [f'{{ match {accessor(file)}() {{']
            for mid, eds in sorted(arms.items()):
                mi = inner
                for st, en, repl in sorted(eds, reverse=True):
                    mi = mi[:st - o - 1] + repl + mi[en - o - 1:]
                parts.append(f' {json.dumps(mid)} => {{{mi}}}')
            parts.append(f' _ => {{{inner}}} }} }}')
            s = s[:o] + ''.join(parts) + s[c + 1:]
        out[file] = s
    return out


def accessor_for(file):
    return 'prism::__mutant' if file in ('src/main.rs',) or file.startswith('src/bin/') else 'crate::__mutant'


# ---------------------------------------------------------------- tree sync
def sync_tree(tree, overrides):
    """Build the scratch tree: real copies of src/ (rendered) and the manifest files, symlinks
    to the live repo for every other top-level entry (tests, fixtures, README, eval, docs...).
    Only files whose bytes differ are written, so mtimes stay stable and cargo stays incremental."""
    tree.mkdir(parents=True, exist_ok=True)
    want = set()
    for item in COPIED:
        src = ROOT / item
        files = [src] if src.is_file() else [p for p in src.rglob('*') if p.is_file()]
        for p in files:
            rel = str(p.relative_to(ROOT)); want.add(rel)
            data = overrides[rel].encode() if rel in overrides else p.read_bytes()
            dst = tree / rel
            if not dst.exists() or dst.read_bytes() != data:
                dst.parent.mkdir(parents=True, exist_ok=True); dst.write_bytes(data)
    for p in [q for d in ('src', 'vendor') for q in (tree / d).rglob('*')]:
        if p.is_file() and str(p.relative_to(tree)) not in want: p.unlink()
    for entry in ROOT.iterdir():
        if entry.name in COPIED or entry.name in ('target', '.git'): continue
        link = tree / entry.name
        if not link.is_symlink(): link.symlink_to(entry)


def cargo_env(target):
    env = os.environ.copy()
    env.update(CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL='1', CARGO_PROFILE_DEV_DEBUG='0')
    env.setdefault('PRISM_TYPESCRIPT', '/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js')
    env.pop('PRISM_MUTANT', None)
    return env


def build(tree, target, targets):
    args = ['cargo', 'test', '--offline', '--manifest-path', str(tree / 'Cargo.toml'), '--no-run', '--message-format=json']
    for t in sorted(targets): args += ['--lib'] if t == 'lib' else ['--test', t]
    p = subprocess.run(args, capture_output=True, text=True, env=cargo_env(target))
    exes, diags = {}, []
    for l in p.stdout.splitlines():
        try: j = json.loads(l)
        except ValueError: continue
        if j.get('reason') == 'compiler-artifact' and j.get('executable') and j.get('profile', {}).get('test'):
            name = 'lib' if 'lib' in j['target']['kind'] else j['target']['name']
            exes[name] = j['executable']
        if j.get('reason') == 'compiler-message' and j['message'].get('level') == 'error':
            for sp in j['message'].get('spans', []):
                if sp.get('is_primary'): diags.append((sp['file_name'], sp['byte_start']))
    return p.returncode, exes, diags, p.stderr


def run_test(exe, test, mutant, timeout, env):
    e = dict(env)
    if mutant: e['PRISM_MUTANT'] = mutant
    t = time.monotonic()
    try:
        p = subprocess.run([exe, test, '--exact', '--test-threads=1'], capture_output=True, text=True, env=e, timeout=timeout)
        log = p.stdout + p.stderr; status = p.returncode
    except subprocess.TimeoutExpired as ex:
        log = (ex.stdout or b'').decode(errors='replace') if isinstance(ex.stdout, bytes) else (ex.stdout or ''); status = 'timeout'
    ran = 'running 1 test' in log
    killed = status == 'timeout' or (ran and 'test result: FAILED.' in log)
    survived = ran and 'test result: ok.' in log
    return {'killed': killed, 'timeout': status == 'timeout', 'admissible': killed or survived,
            'status': status, 'seconds': round(time.monotonic() - t, 3), 'log_tail': log[-1500:]}


# ---------------------------------------------------------------- scoping
def changed_lines(since):
    diff = subprocess.run(['git', '-C', str(ROOT), 'diff', '-U0', since, '--', 'src'], capture_output=True, text=True, check=True).stdout
    out = {}; cur = None
    for l in diff.splitlines():
        if l.startswith('+++ '):
            cur = l[6:] if l.startswith('+++ b/') else None
        elif l.startswith('@@') and cur:
            h = re.match(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@', l)
            o, oc, n, nc = int(h.group(1)), int(h.group(2) or 1), int(h.group(3)), int(h.group(4) or 1)
            # pure deletions touch the line they were deleted after
            out.setdefault(cur, set()).update(range(n, n + nc) if nc else {n, n + 1})
    return out


def select_scoped(muts, src_text, since, scope):
    ch = changed_lines(since); sel = []
    bodies = {f: fn_bodies(s, code_mask(s)) for f, s in src_text.items() if f in ch}
    for m in muts.values():
        hit = False
        for file, a, b in m['edits']:
            if file not in ch: continue
            s = src_text[file]
            if a not in s: hit = True; break  # stale anchor inside a changed file: must be rebound
            for occ in occurrences(s, a):
                lo, hi = s.count('\n', 0, occ) + 1, s.count('\n', 0, occ + len(a)) + 1
                if scope == 'fn':
                    enc = [bd for bd in bodies[file] if bd[0] <= occ < bd[2]]
                    if enc:
                        bd = min(enc, key=lambda x: x[2] - x[0])
                        lo, hi = s.count('\n', 0, bd[0]) + 1, s.count('\n', 0, bd[2]) + 1
                if any(lo <= x <= hi for x in ch[file]): hit = True; break
            if hit: break
        if hit: sel.append(m['id'])
    return sel


# ---------------------------------------------------------------- text mode
def text_mode(muts, ids, target, timeout, log, tree=None):
    # Static mutants run on the already-built schema tree when there is one: their edits lie outside
    # every rendered unit (or their unit was dropped), so with PRISM_MUTANT unset the tree is the
    # original program plus the text edit, and the build stays incremental in one target dir.
    if tree is None:
        tree = target / 'mutgate' / 'tree'
        sync_tree(tree, {})
    res = {}
    for mid in ids:
        m = muts[mid]; saved = {}
        try:
            for file, a, b in m['edits']:
                p = tree / file; s = p.read_text()
                if a not in s: raise RuntimeError(f'anchor missing in {file}')
                saved.setdefault(file, p.read_bytes()); p.write_text(s.replace(a, b))
            tgts = {test_target(m, t) for t in m['tests']}
            t0 = time.monotonic(); rc, exes, _, err = build(tree, target, tgts); bt = time.monotonic() - t0
            if rc != 0:
                res[mid] = {'killed': False, 'admissible': False, 'error': 'compile', 'build_seconds': round(bt, 2), 'log_tail': err[-1500:]}
            else:
                runs = [run_test(exes[test_target(m, t)], t, None, timeout, cargo_env(target)) for t in m['tests']]
                res[mid] = {'killed': any(r['killed'] for r in runs), 'admissible': all(r['admissible'] for r in runs),
                            'build_seconds': round(bt, 2), 'runs': runs}
        except Exception as e:
            res[mid] = {'killed': False, 'admissible': False, 'error': str(e)}
        finally:
            for f, data in saved.items(): (tree / f).write_bytes(data)
        res[mid]['mode'] = 'text'
        log(f'{mid} {verdict(res[mid])} (text)')
    return res


def verdict(r):
    return 'TIMEOUT' if any(x.get('timeout') for x in r.get('runs', [])) else 'KILLED' if r['killed'] else 'SURVIVED' if r['admissible'] else 'INADMISSIBLE'


# ---------------------------------------------------------------- main
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--lane', action='append'); ap.add_argument('--only', action='append')
    ap.add_argument('--since'); ap.add_argument('--scope', choices=['line', 'fn'], default='line')
    ap.add_argument('--mode', choices=['schema', 'text'], default='schema')
    ap.add_argument('--jobs', type=int, default=max(1, (os.cpu_count() or 4) // 2))
    ap.add_argument('--timeout', type=float, default=120.0)
    ap.add_argument('--out', default=None)
    ap.add_argument('--plan-only', action='store_true')
    a = ap.parse_args()
    T0 = time.monotonic()
    target = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target'))
    out = Path(a.out or target / 'mutgate' / 'report'); out.mkdir(parents=True, exist_ok=True)
    def log(msg): print(f'[{time.monotonic() - T0:7.1f}s] {msg}', flush=True)
    lanes = a.lane or sorted(str(p) for p in ROOT.glob(LANES_GLOB))
    muts = load_population(lanes)
    src_text = {str(p.relative_to(ROOT)): p.read_text() for p in (ROOT / 'src').rglob('*.rs')}
    ids = list(muts)
    if a.since: ids = select_scoped(muts, src_text, a.since, a.scope)
    if a.only: ids = [i for i in ids if i in set(a.only)]
    sel = {i: muts[i] for i in ids}
    log(f'{len(muts)} mutants in {len(lanes)} lane file(s); selected {len(sel)}')
    timings = {}
    results = {}
    if a.mode == 'schema' and sel:
        units, static = plan_schemata(sel, src_text)
        for _ in range(3):
            rendered = render(src_text, units, accessor_for)
            lib = rendered['src/lib.rs']; rendered['src/lib.rs'] = ALLOW + lib + HELPER
            log(f'schemata: {len(units)} fn units, {len(sel) - len(static)} schema mutants, {len(static)} static: {static}')
            if a.plan_only:
                (out / 'schemata-plan.json').write_text(json.dumps({'units': len(units), 'static': static}, indent=2)); return 0
            tree = target / 'mutgate' / 'tree'; sync_tree(tree, {k: v for k, v in rendered.items() if v != src_text.get(k)})
            tgts = {test_target(m, t) for m in sel.values() for t in m['tests']}
            t = time.monotonic(); rc, exes, diags, err = build(tree, target, tgts); timings['schema_build'] = round(time.monotonic() - t, 2)
            log(f'schemata build rc={rc} in {timings["schema_build"]}s')
            if rc == 0: break
            # demote every mutant whose unit contains a compile error, then retry
            bad = set()
            for fname, pos in diags:
                rel = str(Path(fname).resolve().relative_to(tree.resolve())) if Path(fname).is_absolute() else fname
                for (f, o, c), arms in units.items():
                    if f == rel: bad.add((f, o, c))
            if not bad: sys.exit('schemata build failed outside mutant units:\n' + err[-3000:])
            for u in bad:
                for mid in units.pop(u): static[mid] = 'schema-compile-error'
                # a mutant demoted in one unit must leave all units
            for u in list(units):
                for mid in list(units[u]):
                    if mid in static: del units[u][mid]
                if not units[u]: del units[u]
        else:
            sys.exit('schemata build did not converge')
        env = cargo_env(target)
        tests = sorted({(test_target(m, t), t) for m in sel.values() for t in m['tests']})
        t = time.monotonic()
        with cf.ThreadPoolExecutor(a.jobs) as ex:
            base = dict(zip(tests, ex.map(lambda tt: run_test(exes[tt[0]], tt[1], None, a.timeout, env), tests)))
        timings['baseline'] = round(time.monotonic() - t, 2)
        red = [tt for tt, r in base.items() if r['killed'] or not r['admissible']]
        log(f'baseline {len(tests)} selectors, {len(red)} red, {timings["baseline"]}s')
        if red:
            (out / 'baseline.json').write_text(json.dumps({str(k): v for k, v in base.items()}, indent=2))
            sys.exit(f'baseline red: {red}')
        jobs = [(mid, test_target(m, tt), tt) for mid, m in sel.items() if mid not in static for tt in m['tests']]
        t = time.monotonic()
        with cf.ThreadPoolExecutor(a.jobs) as ex:
            rr = list(ex.map(lambda j: (j, run_test(exes[j[1]], j[2], j[0], a.timeout, env)), jobs))
        timings['schema_runs'] = round(time.monotonic() - t, 2)
        for (mid, _, _), r in rr:
            agg = results.setdefault(mid, {'mode': 'schema', 'runs': [], 'killed': False, 'admissible': True})
            agg['runs'].append(r); agg['killed'] |= r['killed']; agg['admissible'] &= r['admissible']
        for mid in sel:
            if mid in results: log(f'{mid} {verdict(results[mid])}')
        log(f'schema runs: {len(jobs)} in {timings["schema_runs"]}s (jobs={a.jobs})')
        text_ids = [i for i in sel if i in static and static[i] != 'anchor-missing']
        for i in sel:
            if static.get(i) == 'anchor-missing':
                results[i] = {'mode': 'none', 'killed': False, 'admissible': False, 'error': 'anchor-missing (rebind)'}
                log(f'{i} INADMISSIBLE anchor-missing')
    else:
        text_ids = list(sel)
    if text_ids:
        t = time.monotonic(); results.update(text_mode(muts, text_ids, target, a.timeout, log, tree if a.mode == 'schema' and sel else None)); timings['text_mode'] = round(time.monotonic() - t, 2)
    timings['total'] = round(time.monotonic() - T0, 2)
    killed = sum(r['killed'] for r in results.values()); adm = sum(r['admissible'] for r in results.values())
    summary = {'selected': len(sel), 'killed': killed, 'admissible': adm, 'timings': timings,
               'results': {k: {kk: vv for kk, vv in v.items()} for k, v in results.items()}}
    (out / 'summary.json').write_text(json.dumps(summary, indent=2))
    log(f'TOTAL killed {killed}/{len(sel)} admissible {adm} timings {timings}')
    return 0 if killed == len(sel) else 1


if __name__ == '__main__':
    sys.exit(main())
