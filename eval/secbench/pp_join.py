#!/usr/bin/env python3
"""Lane-4 benchmark join: the proto-writes listing against the frozen strict conversions.

usage: python3 -m eval.secbench.pp_join <harness out dir> <packages dir> <prism binary>
                                        [--out records.jsonl] [--cases converted.json] [--s2]

Read-only: package sources are parsed by the prism binary (`nav proto-writes`), never executed or
imported. One record per strict conversion (DESIGN 5.4):

  structural_match   a fact of the expected class at the designated target with a selector operand
  route1 / route2    credit flags from the saved raw witness (Reached Uses with a data root joined to the
                     fact's selector operands by name, owner, binding and version, DESIGN 5.3)
  finding_equivalent_frozen_line   the write-line diagnostic (frozen-line Uses against any selector operand)
  sink_finding       null before S2; from S2 the actual finding joined to the S2 match trace
  credited_route     1 when route 1 holds, else 2 when route 2 holds (a case counts once)
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import re
import subprocess
import sys
import tarfile
from collections import Counter, defaultdict
from pathlib import Path

from .run import base_name, data_root, variable

CLASS = 'prototype-pollution'
RULE = 'js-proto-write'

# ---------------------------------------------------------------------------------------------
# JSON field names of the listing (PLAN S1a-1 interface). Adjust here once the real output exists.
# ---------------------------------------------------------------------------------------------
F = {
    'facts': 'facts', 'name_reads': 'name_reads',
    # ProtoWriteRow (ProtoWrite flattened)
    'file': 'file', 'line': 'line', 'end_line': 'end_line', 'start': 'start_byte', 'end': 'end_byte',
    'class': 'class', 'guard': 'guard', 'operands': 'key_operands', 'owner': 'owner', 'function': 'function',
    # KeyOperand
    'op_name': 'name', 'op_base': 'base', 'op_role': 'role', 'op_line': 'line',
    'op_start': 'start_byte', 'op_end': 'end_byte', 'op_owner': 'owner', 'op_binding': 'binding',
    'op_versions': 'reaching_versions', 'role_selector': 'selector', 'role_property': 'property',
    # LineReads / NameRead
    'lr_file': 'file', 'lr_line': 'line', 'lr_reads': 'reads',
    'rd_path': 'path', 'rd_base': 'base', 'rd_span': 'span', 'rd_owner': 'owner', 'rd_binding': 'binding',
    'rd_versions': 'reaching_versions', 'rd_complete': 'resolution_complete',
    # AstSpan
    'span_start': 'start_byte', 'span_end': 'end_byte',
    # guard values
    'guard_sufficient': 'sufficient',
}

# ---------------------------------------------------------------------------------------------
# S2 match trace (PLAN S2, optional measurement trace; format not yet fixed): field-name table.
# The trace is looked up per case in raw/prototype-pollution/<entry>/<one of TRACE_FILES>; it is a list
# of records or an object holding the list under T['records'].
# ---------------------------------------------------------------------------------------------
TRACE_FILES = ('proto-match-trace.json.gz', 'proto-match-trace.json')
T = {
    'records': 'matches', 'file': 'file', 'target': 'target', 'route': 'route', 'selector': 'selector',
    'evidence': 'evidence', 'owner': 'owner', 'binding': 'binding', 'versions': 'reaching_versions',
    'survives': 'emitted', 'ambiguous': 'ambiguous', 'competing': 'competing', 'code': 'code',
    'route_target_line': ('target_line', 'TargetLine', 'target-line'),
    'route_earlier': ('earlier_selection', 'EarlierSelection', 'earlier-selection'),
}

# DESIGN 5.4 route table: (class, expected route, case ids).
ROUTE_TABLE = [
    ('nested_keys', 1, ['cached-path-relative_1.0.1', 'dot-object_2.1.2']),
    ('path_walk', 1, ['madlib-object-utils_0.1.6', 'set-value_3.0.0']),
    ('path_walk', 2, ['confucious_0.0.12', 'eivindfjeldstad-dot_0.0.1', 'grunt-util-property_0.0.2',
                      'json-pointer_0.6.0', 'json-pointer_0.6.1', 'just-safe-set_1.0.0']),
    ('path_walk', 2, ['minimist_1.0.0']),
    ('path_walk', 2, ['tiny-conf_1.1.0', 'keyd_1.3.4', 'set-deep-prop_1.0.0', 'dot-notes_3.2.0']),
    ('path_walk', 2, ['deeps_1.4.5', 'jsonpointer_4.0.0', 'libnested_1.5.0', 'nodee-utils_1.2.2',
                      'arr-flatten-unflatten_1.1.4', 'flat_5.0.0']),
    ('recursive_merge', 1, ['json8-merge-patch_1.0.1']),
    ('recursive_merge', 2, ['assign-deep_1.0.0', 'dotty_0.0.1', 'field_1.0.1', 'keyget_2.2.0',
                            'object-path-set_1.0.0', 'paypal-adaptive_0.4.1']),
    ('recursive_merge', 2, ['doc-path_2.0.0']),
]
EXPECTED = {case: (cls, route) for cls, route, cases in ROUTE_TABLE for case in cases}
CLASSES = ('nested_keys', 'path_walk', 'recursive_merge')

# operator gap between the end of the written member and the coordinate: closing parens, TS non-null
# `!`, whitespace, then the leading characters of a (compound) assignment operator.
GAP = re.compile(rb'^[\s)!]*[-+*/%<>&|^?]*$')
LINE_BREAK = re.compile('\r\n|\n|\r| | ')

# stage ranks for first_failed_condition: the furthest failing stage wins
STAGES = ['no_reached_selector_use', 'earlier_site_absent_from_witness', 'use_on_other_line',
          'path_incompatible', 'M-AMBIG']


# ------------------------------------------------------------------------------------------ helpers
def span_of(v):
    """AstSpan as {start_byte,end_byte} or [start,end] -> (start, end) or None."""
    if isinstance(v, dict):
        a, b = v.get(F['span_start']), v.get(F['span_end'])
    elif isinstance(v, (list, tuple)) and len(v) == 2:
        a, b = v
    else:
        return None
    return (a, b) if isinstance(a, int) and isinstance(b, int) else None


def spans_of(vs):
    return tuple(sorted(s for s in (span_of(v) for v in (vs or [])) if s is not None))


def identity(owner, binding, versions):
    return (span_of(owner), span_of(binding), spans_of(versions))


def segments(path):
    return [s for s in re.split(r'[.\[\]]+', path or '') if s]


def path_compatible(a, b):
    """One path is a prefix of the other (same base, compatible member paths)."""
    sa, sb = segments(a), segments(b)
    n = min(len(sa), len(sb))
    return n > 0 and sa[:n] == sb[:n]


def read_json(path: Path):
    raw = path.read_bytes()
    if raw[:2] == b'\x1f\x8b':
        raw = gzip.decompress(raw)
    return json.loads(raw)


def digest(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


# ----------------------------------------------------------------------- sink file and the listing
def package_dir(packages: Path, entry: str) -> Path:
    return packages / CLASS / entry / 'src' / 'package'


def sink_text(packages: Path, row: dict):
    """(bytes, error): the frozen sink file, from src/package, else the tarball member (in memory)."""
    sink = row['sink']
    p = package_dir(packages, row['entry']) / sink['file']
    if p.is_file():
        return p.read_bytes(), None
    entry_dir = packages / CLASS / row['entry']
    for tb in sorted(entry_dir.glob('*.tgz')) + sorted(entry_dir.glob('*.tar.gz')):
        try:
            with tarfile.open(tb) as t:
                m = t.getmember('package/' + sink['file'])
                if m.isfile():
                    return t.extractfile(m).read(), None
        except (KeyError, tarfile.TarError, OSError):
            continue
    return None, f'sink file {sink["file"]} not found under {entry_dir}'


def run_listing(binary: Path, repo: Path, timeout: int = 600):
    """-> (parsed listing | None, error | None, sha256 of the raw stdout)."""
    try:
        r = subprocess.run([str(binary), 'nav', 'proto-writes', '--repo', str(repo), '--format', 'json'],
                           capture_output=True, timeout=timeout)
    except (OSError, subprocess.SubprocessError) as e:
        return None, f'listing invocation failed: {e}', None
    sha = digest(r.stdout)
    if r.returncode != 0:
        return None, f'listing exited {r.returncode}: {r.stderr.decode("utf-8", "replace")[:300]}', sha
    try:
        doc = json.loads(r.stdout)
    except ValueError as e:
        return None, f'listing is not JSON: {e}', sha
    if not isinstance(doc, dict) or not isinstance(doc.get(F['facts']), list):
        return None, 'listing has no facts array', sha
    return doc, None, sha


def norm_file(f):
    f = str(f or '').replace('\\', '/')
    return f[2:] if f.startswith('./') else f


def file_facts(listing: dict, file: str):
    return [f for f in listing.get(F['facts'], []) if norm_file(f.get(F['file'])) == norm_file(file)]


def line_reads(listing: dict, file: str):
    out = defaultdict(list)
    for lr in listing.get(F['name_reads'], []) or []:
        if norm_file(lr.get(F['lr_file'])) == norm_file(file):
            out[lr.get(F['lr_line'])].extend(lr.get(F['lr_reads'], []) or [])
    return out


# --------------------------------------------------------------------------- coordinate mapping
def coordinate_offset(text: str, line: int, column: int):
    """Frozen (line, column) -> UTF-8 byte offset. Line starts follow TypeScript's line-break set and
    the column counts UTF-16 units (1-based), as in the stage-M lookup (key_index.mjs). None when out
    of range."""
    starts, pos = [0], 0
    for m in LINE_BREAK.finditer(text):
        pos = m.end()
        starts.append(pos)
    if line < 1 or line > len(starts):
        return None
    start = starts[line - 1]
    end = starts[line] if line < len(starts) else len(text)
    body = LINE_BREAK.split(text[start:end])[0]
    units = len(body.encode('utf-16-le')) // 2
    if column < 1 or column > units + 1:
        return None
    # advance (column-1) UTF-16 units from the line start
    u, i = 0, 0
    while u < column - 1:
        u += 2 if ord(body[i]) > 0xFFFF else 1
        i += 1
    if u != column - 1:
        return None  # inside a surrogate pair
    return len(text[:start].encode('utf-8')) + len(body[:i].encode('utf-8'))


def map_designated(text_bytes: bytes, line: int, column: int, facts: list):
    """Map the frozen coordinate (it points at the assignment operator) to the fact whose target span is
    the written member of the enclosing assignment. Returns {fact, how, offset, competing, note}.

    how: 'operator_gap' (the member ends before the operator, only whitespace, `)`, `!` between) or
    'contains' (the coordinate lies inside a member span); fact None when neither holds."""
    try:
        text = text_bytes.decode('utf-8')
    except UnicodeDecodeError:
        return {'fact': None, 'how': None, 'offset': None, 'competing': [], 'note': 'sink file is not UTF-8'}
    off = coordinate_offset(text, line, column)
    if off is None:
        return {'fact': None, 'how': None, 'offset': None, 'competing': [],
                'note': 'coordinates out of range of the sink file'}
    gap, cont = [], []
    for f in facts:
        s, e = f.get(F['start']), f.get(F['end'])
        if not (isinstance(s, int) and isinstance(e, int)):
            continue
        if s <= off < e:
            cont.append(f)
        elif e <= off and GAP.match(text_bytes[e:off]):
            gap.append(f)
    pool, how = (gap, 'operator_gap') if gap else (cont, 'contains') if cont else ([], None)
    if not pool:
        return {'fact': None, 'how': None, 'offset': off, 'competing': [], 'note': 'no fact at the coordinate'}
    if how == 'operator_gap':
        pool.sort(key=lambda f: (-f[F['end']], f[F['start']]))   # nearest end, then outermost
        near = [f for f in pool if f[F['end']] == pool[0][F['end']]]
    else:
        pool.sort(key=lambda f: (f[F['end']] - f[F['start']], f[F['start']]))  # smallest containing member
        near = pool[:1]
    chosen = near[0] if how == 'operator_gap' else pool[0]
    competing = [[f[F['start']], f[F['end']]] for f in (near if how == 'operator_gap' else pool) if f is not chosen]
    note = None if off < len(text_bytes) and text_bytes[off:off + 1] in (b'=', b'+', b'-', b'*', b'/', b'%', b'<', b'>',
                                                                            b'&', b'|', b'^', b'?') else \
        'coordinate is not at an assignment operator character'
    return {'fact': chosen, 'how': how, 'offset': off, 'competing': competing, 'note': note}


# ------------------------------------------------------------------------------ witness and match
def reached_uses(row: dict, witness: dict):
    """Reached Uses in the sink file from a data root (raw witness; the harness sink_value filter is not
    applied). The saved witness holds the ground-truth line only; a witness with other lines is joined
    the same way."""
    sink = row['sink']
    found = {}
    for s in witness.get('reasoning', {}).get('per_sink', []):
        u = variable(s.get('sink'))
        if u.get('file') != sink['file'] or u.get('access', 'use') != 'use':
            continue
        for src in s.get('sources', []):
            root = variable(src.get('source'))
            if src.get('reachability') == 'Reached' and data_root(root, row['source']):
                path = u.get('path', '')
                k = (u.get('line'), path, u.get('start_byte'), u.get('end_byte'))
                rec = found.setdefault(k, {'line': k[0], 'path': path, 'base': base_name(path),
                                           'start_byte': k[2], 'end_byte': k[3], 'roots': []})
                rec['roots'].append(root.get('path'))
    return sorted(found.values(), key=lambda u: (u['line'] or 0, u['start_byte'] or 0, u['path']))


def operand_view(op: dict):
    return {'name': op.get(F['op_name']), 'base': op.get(F['op_base']) or base_name(op.get(F['op_name']) or ''),
            'role': op.get(F['op_role']), 'line': op.get(F['op_line']),
            'span': (op.get(F['op_start']), op.get(F['op_end'])),
            'identity': identity(op.get(F['op_owner']), op.get(F['op_binding']), op.get(F['op_versions']))}


def use_zero_width(u):
    return u['start_byte'] is None or u['end_byte'] is None or u['end_byte'] <= u['start_byte']


def match_use(u: dict, op: dict, reads: dict):
    """One reached Use against one operand under DESIGN 5.3.

    -> (verdict, detail). verdict: 'exact' | 'name' | 'ambiguous' | 'mismatch'. 'name' admits by name only
    when every indistinguishable read on the Use's line (same base) denotes the operand's one owner, one
    binding and one version; otherwise 'ambiguous' with the competing spans (code M-AMBIG)."""
    if u['base'] != op['base'] or not path_compatible(u['path'], op['name'] or op['base']):
        return 'mismatch', {'why': 'name_or_path'}
    s, e = op['span']
    if (not use_zero_width(u)) and isinstance(s, int) and u['line'] == op['line'] \
            and s <= u['start_byte'] and u['end_byte'] <= e:
        return 'exact', {'span': [u['start_byte'], u['end_byte']]}
    group = [r for r in reads.get(u['line'], []) if r.get(F['rd_base']) == u['base']]
    if not group:
        return 'ambiguous', {'why': 'no_read_inventory', 'competing': []}
    want = op['identity']
    bad = []
    for r in group:
        ident = identity(r.get(F['rd_owner']), r.get(F['rd_binding']), r.get(F['rd_versions']))
        sp = span_of(r.get(F['rd_span']))
        problem = None
        if not r.get(F['rd_complete'], True):
            problem = 'incomplete_resolution'
        elif ident[1] is None or want[1] is None:
            problem = 'unknown_binding'
        elif ident[0] != want[0]:
            problem = 'owner'
        elif ident[1] != want[1]:
            problem = 'binding'
        elif len(ident[2]) != 1 or ident[2] != want[2]:
            problem = 'version'
        if problem:
            bad.append((problem, sp))
    if bad:
        return 'ambiguous', {'why': bad[0][0], 'competing': [list(sp) for _, sp in bad if sp]}
    return 'name', {'span': [u['start_byte'], u['end_byte']], 'reads': len(group)}


def evaluate(fact: dict, uses: list, reads: dict, frozen_line: int):
    """Route flags and diagnostics for one fact."""
    ops = [operand_view(o) for o in fact.get(F['operands'], []) or []]
    sels = [o for o in ops if o['role'] == F['role_selector']]
    props = [o for o in ops if o['role'] == F['role_property']]
    t0, t1 = fact.get(F['line']), fact.get(F['end_line']) or fact.get(F['line'])
    target_lines = set(range(t0, t1 + 1)) | {frozen_line} if isinstance(t0, int) else {frozen_line}
    witness_lines = {u['line'] for u in uses}
    out = {'route1': False, 'route2': False, 'finding_equivalent_frozen_line': False, 'matches': [],
           'competing_spans': [], 'ambiguous': [], 'property_only_reach': False, 'stage': None}
    best = -1

    def note_stage(name):
        nonlocal best
        i = STAGES.index(name)
        if i > best:
            best = i
            out['stage'] = name

    for op in sels:
        on_target = op['line'] in target_lines
        base_uses = [u for u in uses if u['base'] == op['base']]
        if not base_uses:
            note_stage('earlier_site_absent_from_witness' if not on_target and op['line'] not in witness_lines
                       else 'no_reached_selector_use')
            continue
        for u in base_uses:
            verdict, detail = match_use(u, op, reads)
            rec = {'operand': op['name'], 'operand_line': op['line'], 'use_line': u['line'],
                   'use_path': u['path'], 'verdict': verdict, **detail}
            if verdict in ('exact', 'name'):
                if u['line'] == frozen_line:
                    out['finding_equivalent_frozen_line'] = True
                if u['line'] == op['line']:
                    out['route1' if on_target else 'route2'] = True
                    out['matches'].append(rec)
                else:
                    note_stage('use_on_other_line')
            elif verdict == 'ambiguous':
                out['ambiguous'].append(rec)
                out['competing_spans'].extend(detail.get('competing', []))
                note_stage('M-AMBIG')
            else:
                note_stage('path_incompatible')
        if not on_target and op['line'] not in witness_lines:
            note_stage('earlier_site_absent_from_witness')
    credited = out['route1'] or out['route2']
    if not credited and uses:
        for op in props:
            for u in uses:
                if u['line'] == frozen_line and match_use(u, op, reads)[0] in ('exact', 'name'):
                    out['property_only_reach'] = True
    return out


# ---------------------------------------------------------------------------------- S2 trace join
def span_equal(v, fact):
    return span_of(v) == (fact.get(F['start']), fact.get(F['end']))


def load_trace(raw_dir: Path):
    for name in TRACE_FILES:
        p = raw_dir / name
        if p.is_file():
            try:
                doc = read_json(p)
            except (OSError, ValueError) as e:
                return {'error': f'{p.name}: {e}'}
            recs = doc.get(T['records']) if isinstance(doc, dict) else doc
            return {'records': recs if isinstance(recs, list) else []}
    return None


def is_rule_finding(f: dict) -> bool:
    for r in f.get('rules', []) or []:
        if (r.get('id') if isinstance(r, dict) else r) == RULE:
            return True
    return RULE in (f.get('description') or '')


def s2_join(raw_dir: Path, sink: dict, fact: dict, trace):
    """Actual rule finding at the designated target joined to the pre-dedup match record by exact target."""
    res = {'status': None, 'finding': None, 'trace_records': 0, 'route1': False, 'route2': False,
           'credited_route': None, 'ambiguous': [], 'code': None}
    try:
        taint = read_json(raw_dir / 'taint.json.gz')
        findings = taint.get('findings', []) if isinstance(taint, dict) else []
    except (OSError, ValueError) as e:
        res['status'] = 'taint_output_unreadable'
        res['error'] = str(e)
        return res
    lines = set(range(fact[F['line']], (fact.get(F['end_line']) or fact[F['line']]) + 1)) | {sink['line']}
    hits = [f for f in findings if f.get('file') == sink['file'] and f.get('line') in lines
            and f.get('category') == 'taint_sink' and is_rule_finding(f)]
    if hits:
        f = hits[0]
        res['finding'] = {k: f.get(k) for k in ('file', 'line', 'severity', 'description', 'category')}
    recs = []
    if trace and 'records' in trace:
        recs = [r for r in trace['records'] if (r.get(T['file']) in (None, sink['file'])) and span_equal(r.get(T['target']), fact)]
    res['trace_records'] = len(recs)
    for r in recs:
        if r.get(T['ambiguous']):
            res['ambiguous'].append({'competing': r.get(T['competing']), 'code': r.get(T['code'])})
            continue
        if not r.get(T['survives'], True):
            continue
        if r.get(T['route']) in T['route_target_line']:
            res['route1'] = True
        elif r.get(T['route']) in T['route_earlier']:
            res['route2'] = True
    res['credited_route'] = 1 if res['route1'] else 2 if res['route2'] else None
    if not hits:
        res['status'] = 'no_finding'
    elif not recs:
        res['status'] = 'finding_unverified'      # a finding with no match-trace record enters no numerator
    elif res['credited_route'] is None:
        res['status'] = 'finding_without_route_credit'
    else:
        res['status'] = 'finding_verified'
    return res


# --------------------------------------------------------------------------------------- the join
def read_rows(out_dir: Path):
    rows = {}
    for line in (out_dir / 'entries.jsonl').read_text().splitlines():
        if line.strip():
            r = json.loads(line)
            if r.get('class') == CLASS:
                rows[r['entry']] = r
    return rows


def join_case(entry: str, row: dict, out_dir: Path, packages: Path, binary: Path, s2: bool, listing_fn=None):
    cls, route = EXPECTED.get(entry, (None, None))
    sink = {k: row['sink'].get(k) for k in ('file', 'line', 'column')}
    rec = {'entry': entry, 'id': row.get('id'), 'expected_class': cls, 'expected_route': route, 'sink': sink,
           'fact': None, 'fact_class': None, 'guard': None, 'selector_operands': [], 'mapping': None,
           'structural_match': False, 'route1': False, 'route2': False, 'credited_route': None,
           'finding_equivalent_frozen_line': False, 'sink_finding': None, 'first_failed_condition': None,
           'competing_spans': [], 'property_only_reach': False, 'reached_uses': [], 'matches': [], 'errors': []}
    raw_dir = out_dir / 'raw' / CLASS / entry
    text, err = sink_text(packages, row)
    if err:
        rec['errors'].append(err)
    listing, lerr, sha = (listing_fn or run_listing)(binary, package_dir(packages, entry))
    rec['listing_sha256'] = sha
    if lerr:
        rec['errors'].append(lerr)
    uses, werr = [], None
    try:
        uses = reached_uses(row, read_json(raw_dir / 'witness.json.gz'))
    except (OSError, ValueError) as e:
        werr = f'witness {raw_dir / "witness.json.gz"}: {e}'
        rec['errors'].append(werr)
    rec['reached_uses'] = uses
    fact = None
    if listing is not None and text is not None:
        facts = file_facts(listing, sink['file'])
        m = map_designated(text, sink['line'], sink['column'], facts)
        fact = m['fact']
        rec['mapping'] = {k: m[k] for k in ('how', 'offset', 'competing', 'note')}
        if fact is not None:
            rec['mapping']['member_span'] = [fact[F['start']], fact[F['end']]]
    if fact is not None:
        sels = [operand_view(o) for o in fact.get(F['operands'], []) or []]
        rec.update(fact={'line': fact.get(F['line']), 'end_line': fact.get(F['end_line']),
                         'start_byte': fact.get(F['start']), 'end_byte': fact.get(F['end']),
                         'function': fact.get(F['function'])},
                   fact_class=fact.get(F['class']), guard=fact.get(F['guard']),
                   selector_operands=[{k: o[k] for k in ('name', 'base', 'line', 'span')}
                                      for o in sels if o['role'] == F['role_selector']])
        rec['structural_match'] = rec['fact_class'] == cls and bool(rec['selector_operands'])
        ev = evaluate(fact, uses, line_reads(listing, sink['file']), sink['line'])
        for k in ('route1', 'route2', 'finding_equivalent_frozen_line', 'property_only_reach', 'competing_spans',
                  'matches'):
            rec[k] = ev[k]
        if ev['ambiguous']:
            rec['ambiguous'] = ev['ambiguous']
        rec['credited_route'] = 1 if rec['route1'] else 2 if rec['route2'] else None
        rec['failed_stage'] = ev['stage'] if rec['credited_route'] is None else None
        if rec['credited_route'] is None:
            if not rec['selector_operands']:
                rec['first_failed_condition'] = 'no_selector_operand'
            elif werr or not uses:
                rec['first_failed_condition'] = 'witness_unreadable' if werr else 'no_reached_data_root_use'
            elif ev['property_only_reach']:
                rec['first_failed_condition'] = 'property_only_reach'
            else:
                rec['first_failed_condition'] = ev['stage'] or 'no_reached_selector_use'
        if s2:
            trace = load_trace(raw_dir)
            rec['sink_finding'] = s2_join(raw_dir, sink, fact, trace)
    else:
        rec['first_failed_condition'] = ('listing_unavailable' if listing is None else
                                         'sink_file_unavailable' if text is None else 'no_fact_at_target')
        if s2:
            rec['sink_finding'] = {'status': 'unverified_no_fact', 'finding': None, 'credited_route': None}
    return rec


def run_join(out_dir: Path, packages: Path, binary: Path, cases=None, s2=None, listing_fn=None):
    rows = read_rows(out_dir)
    names = list(cases) if cases else list(EXPECTED)
    if s2 is None:   # S2 mode when any case carries a match trace
        s2 = any(load_trace(out_dir / 'raw' / CLASS / n) is not None for n in names)
    recs = []
    for n in names:
        if n not in rows:
            recs.append({'entry': n, 'expected_class': EXPECTED.get(n, (None, None))[0],
                         'expected_route': EXPECTED.get(n, (None, None))[1], 'structural_match': False,
                         'route1': False, 'route2': False, 'credited_route': None, 'sink_finding': None,
                         'first_failed_condition': 'no_frozen_row', 'errors': [f'no {CLASS} row for {n} in entries.jsonl']})
            continue
        recs.append(join_case(n, rows[n], out_dir, packages, binary, s2, listing_fn))
    return recs, s2


def totals(recs, s2):
    lines = []
    cnt = lambda rs, f: sum(1 for r in rs if f(r))
    lines.append(f'cases={len(recs)} s2_trace_mode={s2}')
    lines.append('per class: n structural route1 route2 credited frozen_line_equiv property_only'
                 + (' sink_verified' if s2 else ''))
    for c in CLASSES + (None,):
        rs = [r for r in recs if r.get('expected_class') == c]
        if not rs:
            continue
        tail = f' {cnt(rs, lambda r: (r.get("sink_finding") or {}).get("status") == "finding_verified")}' if s2 else ''
        lines.append(f'  {c}: {len(rs)} {cnt(rs, lambda r: r["structural_match"])} {cnt(rs, lambda r: r["route1"])} '
                     f'{cnt(rs, lambda r: r["route2"])} {cnt(rs, lambda r: r["credited_route"])} '
                     f'{cnt(rs, lambda r: r.get("finding_equivalent_frozen_line"))} '
                     f'{cnt(rs, lambda r: r.get("property_only_reach"))}{tail}')
    lines.append('per expected route: n credited_route1 credited_route2 uncredited')
    for er in (1, 2, None):
        rs = [r for r in recs if r.get('expected_route') == er]
        if rs:
            lines.append(f'  expected {er}: {len(rs)} {cnt(rs, lambda r: r["credited_route"] == 1)} '
                         f'{cnt(rs, lambda r: r["credited_route"] == 2)} {cnt(rs, lambda r: not r["credited_route"])}')
    diff = [f'{r["entry"]} expected {r["expected_route"]} credited {r["credited_route"]}'
            for r in recs if r['credited_route'] and r['credited_route'] != r.get('expected_route')]
    lines.append(f'route differences from the table: {len(diff)}' + ''.join(f'\n  {d}' for d in diff))
    fails = Counter(r['first_failed_condition'] for r in recs if r.get('first_failed_condition'))
    lines.append('first failed condition: ' + ', '.join(f'{k}={v}' for k, v in sorted(fails.items())))
    return '\n'.join(lines)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('out_dir', help='harness out dir (entries.jsonl, raw/)')
    ap.add_argument('packages', help='packages dir (prototype-pollution/<entry>/src/package/)')
    ap.add_argument('binary', help='prism binary')
    ap.add_argument('--out', help='records JSONL (default stdout)')
    ap.add_argument('--cases', help='converted.json; its "strict" list names the cases (default: the 29 of the table)')
    ap.add_argument('--s2', action='store_true', help='force S2 mode (read match traces and findings)')
    a = ap.parse_args(argv)
    cases = None
    if a.cases:
        doc = json.loads(Path(a.cases).read_text())
        cases = doc['strict'] if isinstance(doc, dict) else doc
    recs, s2 = run_join(Path(a.out_dir), Path(a.packages), Path(a.binary), cases, True if a.s2 else None)
    text = ''.join(json.dumps(r, sort_keys=True) + '\n' for r in recs)
    if a.out:
        Path(a.out).write_text(text)
    else:
        sys.stdout.write(text)
    print(totals(recs, s2), file=sys.stderr)


if __name__ == '__main__':
    main()
