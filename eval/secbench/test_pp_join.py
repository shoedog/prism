"""Offline tests for pp_join: a fake harness out dir, a fake package dir and a fake `prism` script that
prints a canned listing. No corpus, no benchmark package code."""
from __future__ import annotations

import gzip
import json
import os
import io
import stat
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path

from . import pp_join as J

FILE = 'index.js'


def gz(path: Path, doc):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(gzip.compress(json.dumps(doc).encode()))


def span(a, b):
    return {'start_byte': a, 'end_byte': b}


def at(text: str, sub: str, nth: int = 0):
    """(start, end) byte offsets of the nth occurrence of sub (UTF-8)."""
    b, s = text.encode(), sub.encode()
    i = -1
    for _ in range(nth + 1):
        i = b.index(s, i + 1)
    return i, i + len(s)


def operator_coord(text: str, line: int, nth: int = 0):
    """(line, 1-based column) of the nth `=` in a one-based line."""
    body = text.split('\n')[line - 1]
    i = -1
    for _ in range(nth + 1):
        i = body.index('=', i + 1)
    return line, len(body[:i].encode('utf-16-le')) // 2 + 1


def ident(owner, binding, versions):
    return {'owner': span(*owner), 'binding': span(*binding) if binding else None,
            'reaching_versions': [span(*v) for v in versions]}


def operand(text, sub, line, role='selector', nth=0, base=None, name=None, **idn):
    s, e = at(text, sub, nth)
    return {'name': name or sub, 'base': base or sub.split('.')[0].split('[')[0], 'role': role, 'line': line,
            'start_byte': s, 'end_byte': e, **idn}


def read(text, sub, nth=0, path=None, complete=True, **idn):
    s, e = at(text, sub, nth)
    return {'path': path or sub, 'base': (path or sub).split('.')[0], 'span': span(s, e),
            'resolution_complete': complete, **idn}


class Harness:
    """One temp harness out dir + packages dir + fake binary."""

    def __init__(self, test: unittest.TestCase):
        tmp = tempfile.TemporaryDirectory(prefix='pp-join-')
        test.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.out, self.packages = self.root / 'out', self.root / 'pk'
        self.out.mkdir()
        self.canned = {}
        self.rows = []
        self.binary = self.root / 'prism'
        self.binary.write_text(
            f'#!{sys.executable}\n'
            'import json, os, sys\n'
            'a = sys.argv[1:]\n'
            "if a == ['--version']:\n"
            "    print('prism 9.9.9-test'); sys.exit(0)\n"
            "assert a[:2] == ['nav', 'proto-writes'] and a[a.index('--format') + 1] == 'json', a\n"
            "entry = a[a.index('--repo') + 1].rstrip('/').split('/')[-3]\n"
            "canned = json.load(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'canned.json')))\n"
            "print(json.dumps(canned[entry]))\n")
        self.binary.chmod(self.binary.stat().st_mode | stat.S_IXUSR)

    def case(self, entry, text, op_coord, uses, facts, reads=(), data_param=('key', 0)):
        """uses: [(line, path, start, end)] Reached data-root Uses; facts/reads: canned listing parts."""
        pkg = J.package_dir(self.packages, entry)
        pkg.mkdir(parents=True)
        (pkg / FILE).write_bytes(text.encode())
        ps, pe = at(text, data_param[0], data_param[1])
        source = {'file': FILE, 'start_line': 1,
                  'data_parameters': [{'names': [data_param[0]], 'line': 1, 'start_byte': ps, 'end_byte': pe}]}
        self.rows.append({'class': J.CLASS, 'entry': entry, 'id': 'X-' + entry, 'source': source,
                          'sink': {'file': FILE, 'line': op_coord[0], 'column': op_coord[1]}})
        (self.out / 'entries.jsonl').write_text(''.join(json.dumps(r) + '\n' for r in self.rows))
        root = {'Variable': {'file': FILE, 'access': 'def', 'path': data_param[0], 'line': 1, 'start_byte': ps}}
        per_sink = [{'sink': {'Variable': {'file': FILE, 'line': ln, 'path': p, 'access': 'use',
                                           'start_byte': s, 'end_byte': e}},
                     'sources': [{'source': root, 'reachability': 'Reached'}]} for ln, p, s, e in uses]
        gz(self.out / 'raw' / J.CLASS / entry / 'witness.json.gz', {'reasoning': {'per_sink': per_sink}})
        by_line = {}
        for ln, r in reads:
            by_line.setdefault(ln, []).append(r)
        self.canned[entry] = {'facts': facts, 'name_reads': [
            {'file': FILE, 'line': ln, 'reads': rs} for ln, rs in sorted(by_line.items())], 'extra_field': 1}
        (self.root / 'canned.json').write_text(json.dumps(self.canned))

    def run(self, cases, s2=None):
        recs, mode = J.run_join(self.out, self.packages, self.binary, cases, s2)
        return {r['entry']: r for r in recs}, mode


def fact(text, member, line, cls, operands, guard='none', end_line=None, nth=0):
    s, e = at(text, member, nth)
    return {'file': FILE, 'line': line, 'end_line': end_line or line, 'start_byte': s, 'end_byte': e,
            'function': 'f', 'owner': span(0, len(text.encode())), 'function_start_line': 1,
            'function_end_line': text.count('\n') + 1, 'class': cls, 'key_operands': operands, 'guard': guard}


OWN = (0, 10_000)

NESTED = 'function set(obj, key, v) {\n    obj[key] = v;\n}\n'


def nested_fixture(h, entry='dot-object_2.1.2', uses=None, extra_reads=(), key_versions=None):
    """`obj[key] = v` with `key` a formal: the selector is on the write line (route 1)."""
    decl = at(NESTED, 'key')
    idn = ident(OWN, decl, key_versions or [decl])
    op = operand(NESTED, 'key', 2, nth=1, **idn)
    reads = [(2, read(NESTED, 'key', nth=1, **idn))] + list(extra_reads)
    f = fact(NESTED, 'obj[key]', 2, 'nested_keys', [op])
    h.case(entry, NESTED, operator_coord(NESTED, 2), uses if uses is not None else [(2, 'key', *op_span(op))],
           [f], reads)
    return op, decl


def op_span(op):
    return op['start_byte'], op['end_byte']


class MappingTests(unittest.TestCase):
    """The frozen coordinate points at the assignment operator; the fact's span covers the written member."""

    def test_two_case_fixture_operator_column_to_member_span(self):
        ascii_text = 'function f(o, k) {\n    o.a[k] = 1;\n    o[k] = 2;\n}\n'
        facts = [fact(ascii_text, 'o.a[k]', 2, 'path_walk', []), fact(ascii_text, 'o[k]', 3, 'path_walk', [])]
        for line, member in ((2, 'o.a[k]'), (3, 'o[k]')):
            m = J.map_designated(ascii_text.encode(), *operator_coord(ascii_text, line), facts)
            self.assertEqual(m['how'], 'operator_gap')
            self.assertEqual((m['fact']['start_byte'], m['fact']['end_byte']), at(ascii_text, member))
            self.assertIsNone(m['note'])
        # astral characters before the write: the column counts UTF-16 units, the spans are UTF-8 bytes
        wide = 'var s = "\U0001F600é"; o.a[k] = 1;\n'
        wf = [fact(wide, 'o.a[k]', 1, 'path_walk', [])]
        line, col = operator_coord(wide, 1, nth=1)
        self.assertEqual(col, len('var s = "') + 2 + 1 + len('"; o.a[k] ') + 1)  # the emoji is 2 UTF-16 units
        m = J.map_designated(wide.encode(), line, col, wf)
        self.assertEqual((m['fact']['start_byte'], m['fact']['end_byte']), at(wide, 'o.a[k]'))
        self.assertEqual(m['offset'], wide.encode().index(b'= 1'))   # 3 bytes further than the column suggests

    def test_coordinate_on_a_non_member_maps_to_no_fact(self):
        text = 'function f(o, k, v) {\n    o[k] = v;\n}\n'
        facts = [fact(text, 'o[k]', 2, 'path_walk', [])]
        line, col = operator_coord(text, 2)
        for c in (col + 2, 1):   # the right-hand `v`, the file start
            m = J.map_designated(text.encode(), line if c != 1 else 1, c, facts)
            self.assertIsNone(m['fact'])
        self.assertIsNone(J.map_designated(text.encode(), 99, 1, facts)['fact'])   # out of range

    def test_compound_operator_wrapper_and_nearest_member(self):
        text = 'function f(o, k) {\n    (o[k]) ||= 1;\n    o[k] += 2;\n}\n'
        facts = [fact(text, 'o[k]', 2, 'path_walk', []), fact(text, 'o[k]', 3, 'path_walk', [], nth=1)]
        m = J.map_designated(text.encode(), 2, text.split('\n')[1].index('||=') + 1, facts)
        self.assertEqual(m['fact']['line'], 2)
        m = J.map_designated(text.encode(), 3, text.split('\n')[2].index('+=') + 1, facts)
        self.assertEqual(m['fact']['line'], 3)


class RouteTests(unittest.TestCase):
    def test_exact_selector_on_the_write_line(self):
        h = Harness(self)
        op, _ = nested_fixture(h)
        r = h.run(['dot-object_2.1.2'])[0]['dot-object_2.1.2']
        self.assertTrue(r['structural_match'])
        self.assertEqual((r['expected_class'], r['expected_route']), ('nested_keys', 1))
        self.assertTrue(r['route1'] and not r['route2'])
        self.assertEqual(r['credited_route'], 1)
        self.assertTrue(r['finding_equivalent_frozen_line'])
        self.assertIsNone(r['first_failed_condition'])
        self.assertIsNone(r['sink_finding'])          # null before S2
        self.assertEqual(r['mapping']['member_span'], list(at(NESTED, 'obj[key]')))

    def test_unique_zero_width_anchor_is_admitted_by_name(self):
        h = Harness(self)
        anchor = at(NESTED, 'key')[0]
        nested_fixture(h, uses=[(2, 'key', anchor, anchor)])
        r = h.run(['dot-object_2.1.2'])[0]['dot-object_2.1.2']
        self.assertTrue(r['route1'])
        self.assertEqual(r['matches'][0]['verdict'], 'name')

    def test_zero_width_anchor_with_a_conflated_shadow_is_ambiguous(self):
        h = Harness(self)
        anchor = at(NESTED, 'key')[0]
        other = ident((40, 60), (41, 44), [(41, 44)])
        shadow = read(NESTED, 'key', nth=1, **other)   # same line, same spelling, other owner/binding
        shadow['span'] = span(30, 33)
        nested_fixture(h, uses=[(2, 'key', anchor, anchor)], extra_reads=[(2, shadow)])
        r = h.run(['dot-object_2.1.2'])[0]['dot-object_2.1.2']
        self.assertFalse(r['route1'] or r['route2'])
        self.assertEqual(r['first_failed_condition'], 'M-AMBIG')
        self.assertIn([30, 33], r['competing_spans'])
        self.assertEqual(r['ambiguous'][0]['why'], 'owner')

    def test_multiple_reaching_versions_defeat_the_fallback(self):
        h = Harness(self)
        decl = at(NESTED, 'key')
        anchor = decl[0]
        nested_fixture(h, uses=[(2, 'key', anchor, anchor)], key_versions=[decl, (50, 53)])
        r = h.run(['dot-object_2.1.2'])[0]['dot-object_2.1.2']
        self.assertFalse(r['route1'])
        self.assertEqual(r['ambiguous'][0]['why'], 'version')

    def test_stable_shared_rhs_outside_the_operand(self):
        text = 'function set(obj, key, v) {\n    obj[key] = key;\n}\n'
        h = Harness(self)
        decl = at(text, 'key')
        idn = ident(OWN, decl, [decl])
        op = operand(text, 'key', 2, nth=1, **idn)
        reads = [(2, read(text, 'key', nth=1, **idn)), (2, read(text, 'key', nth=2, **idn))]
        rhs = at(text, 'key', 2)
        h.case('dot-object_2.1.2', text, operator_coord(text, 2), [(2, 'key', *rhs)],
               [fact(text, 'obj[key]', 2, 'nested_keys', [op])], reads)
        r = h.run(['dot-object_2.1.2'])[0]['dot-object_2.1.2']
        self.assertTrue(r['route1'])
        self.assertEqual(r['matches'][0]['verdict'], 'name')
        # a rebind on the same line makes the RHS read another version: no credit
        h2 = Harness(self)
        other = ident(OWN, decl, [(60, 63)])
        h2.case('dot-object_2.1.2', text, operator_coord(text, 2), [(2, 'key', *rhs)],
                [fact(text, 'obj[key]', 2, 'nested_keys', [op])],
                [(2, read(text, 'key', nth=1, **idn)), (2, read(text, 'key', nth=2, **other))])
        r2 = h2.run(['dot-object_2.1.2'])[0]['dot-object_2.1.2']
        self.assertFalse(r2['route1'])
        self.assertEqual(r2['first_failed_condition'], 'M-AMBIG')

    WALK = ('function set(obj, path, v) {\n    var cur = obj;\n    for (var i = 0; i < path.length; i++) {\n'
            '        cur = cur[path[i]];\n    }\n    cur[last] = v;\n}\n')

    def walk_case(self, h, uses_extra=(), leaf_use=True):
        t = self.WALK
        decl = at(t, 'path')
        step = operand(t, 'path[i]', 4, base='path', name='path[i]', **ident(OWN, decl, [decl]))
        leaf = operand(t, 'last', 6, role='property', **ident(OWN, None, []))
        frozen = [(6, 'last', *at(t, 'last'))] if leaf_use else [(6, 'v', *at(t, 'v', 1))]
        h.case('flat_5.0.0', t, operator_coord(t, 6), frozen + list(uses_extra),
               [fact(t, 'cur[last]', 6, 'path_walk', [step, leaf])],
               [(4, read(t, 'path[i]', path='path[i]', **ident(OWN, decl, [decl])))], data_param=('path', 0))
        return step

    def test_earlier_site_absent_from_the_frozen_line(self):
        h = Harness(self)
        self.walk_case(h, leaf_use=False)
        r = h.run(['flat_5.0.0'])[0]['flat_5.0.0']
        self.assertEqual((r['expected_class'], r['expected_route']), ('path_walk', 2))
        self.assertTrue(r['structural_match'])
        self.assertFalse(r['route1'] or r['route2'])
        self.assertFalse(r['finding_equivalent_frozen_line'])
        self.assertEqual(r['first_failed_condition'], 'earlier_site_absent_from_witness')
        self.assertFalse(r['property_only_reach'])

    def test_final_property_only_reach(self):
        h = Harness(self)
        self.walk_case(h)
        r = h.run(['flat_5.0.0'])[0]['flat_5.0.0']
        self.assertTrue(r['property_only_reach'])
        self.assertEqual(r['first_failed_condition'], 'property_only_reach')
        self.assertEqual(r['failed_stage'], 'earlier_site_absent_from_witness')   # the selector side, kept apart
        self.assertIsNone(r['credited_route'])

    def test_earlier_selection_in_the_witness_earns_route_2(self):
        h = Harness(self)
        step = self.walk_case(h, uses_extra=[(4, 'path[i]', *op_span(operand(self.WALK, 'path[i]', 4)))], leaf_use=False)
        r = h.run(['flat_5.0.0'])[0]['flat_5.0.0']
        self.assertTrue(r['route2'] and not r['route1'])
        self.assertEqual(r['credited_route'], 2)
        self.assertIsNone(r['first_failed_condition'])

    def test_both_routes_count_once_route_1_first(self):
        h = Harness(self)
        t = self.WALK.replace('cur[last] = v;', 'cur[last] = path;')
        decl = at(t, 'path')
        idn = ident(OWN, decl, [decl])
        step = operand(t, 'path[i]', 4, base='path', name='path[i]', **idn)
        shared = operand(t, 'path', 6, nth=3, name='path', **idn)
        uses = [(4, 'path[i]', *op_span(step)), (6, 'path', *op_span(shared))]
        h.case('flat_5.0.0', t, operator_coord(t, 6), uses, [fact(t, 'cur[last]', 6, 'path_walk', [step, shared])],
               [], data_param=('path', 0))
        recs, _ = h.run(['flat_5.0.0'])
        r = recs['flat_5.0.0']
        self.assertTrue(r['route1'] and r['route2'])
        self.assertEqual(r['credited_route'], 1)
        text = J.totals(list(recs.values()), False)
        self.assertIn('path_walk: 1 1 1 1 1', text)
        self.assertIn('route differences from the table: 1', text)   # expected route 2, credited 1

    def test_missing_fact_and_listing_failure(self):
        h = Harness(self)
        nested_fixture(h)
        h.canned['dot-object_2.1.2']['facts'] = []
        (h.root / 'canned.json').write_text(json.dumps(h.canned))
        r = h.run(['dot-object_2.1.2'])[0]['dot-object_2.1.2']
        self.assertFalse(r['structural_match'])
        self.assertEqual(r['first_failed_condition'], 'no_fact_at_target')
        h.binary.write_text('#!/bin/sh\nexit 3\n')
        r = h.run(['dot-object_2.1.2'])[0]['dot-object_2.1.2']
        self.assertEqual(r['first_failed_condition'], 'listing_unavailable')
        self.assertIn('exited 3', r['errors'][0])


class S2Tests(unittest.TestCase):
    def setUp(self):
        self.h = Harness(self)
        self.op, _ = nested_fixture(self.h)
        self.raw = self.h.out / 'raw' / J.CLASS / 'dot-object_2.1.2'
        self.target = list(at(NESTED, 'obj[key]'))

    def finding(self, line=2, rules=True):
        f = {'file': FILE, 'line': line, 'severity': 'warning', 'category': 'taint_sink',
             'description': 'tainted key from x reaches a prototype-capable write at line 2 (CWE-1321, js-proto-write)'}
        if rules:
            f['rules'] = [{'id': J.RULE}]
        gz(self.raw / 'taint.json.gz', {'findings': [f]})

    def trace(self, **rec):
        base = {'file': FILE, 'target': span(*self.target), 'route': 'target_line', 'emitted': True}
        gz(self.raw / 'proto-match-trace.json.gz', {'matches': [{**base, **rec}]})

    def join(self):
        return self.h.run(['dot-object_2.1.2'])

    def test_finding_joined_to_its_trace_record(self):
        self.finding()
        self.trace()
        recs, mode = self.join()
        sf = recs['dot-object_2.1.2']['sink_finding']
        self.assertTrue(mode)
        self.assertEqual((sf['status'], sf['credited_route'], sf['route1'], sf['route2']), ('finding_verified', 1, True, False))

    def test_finding_without_a_match_record_is_unverified(self):
        self.finding()
        self.trace(target=span(0, 1))                     # another target: no record for this one
        sf = self.join()[0]['dot-object_2.1.2']['sink_finding']
        self.assertEqual(sf['status'], 'finding_unverified')
        self.assertIsNone(sf['credited_route'])

    def test_other_line_finding_gives_no_credit_and_two_routes_one_case(self):
        self.finding(line=3)
        gz(self.raw / 'proto-match-trace.json.gz', {'matches': [
            {'file': FILE, 'target': span(*self.target), 'route': 'target_line'},
            {'file': FILE, 'target': span(*self.target), 'route': 'EarlierSelection'}]})
        sf = self.join()[0]['dot-object_2.1.2']['sink_finding']
        self.assertEqual(sf['status'], 'no_finding')
        self.finding()
        sf = self.join()[0]['dot-object_2.1.2']['sink_finding']
        self.assertTrue(sf['route1'] and sf['route2'])
        self.assertEqual(sf['credited_route'], 1)

    def test_ambiguous_or_filtered_record_gives_no_route_credit(self):
        self.finding()
        self.trace(ambiguous=True, competing=[[1, 2]], code='M-AMBIG')
        sf = self.join()[0]['dot-object_2.1.2']['sink_finding']
        self.assertEqual(sf['status'], 'finding_without_route_credit')
        self.assertEqual(sf['ambiguous'][0]['code'], 'M-AMBIG')
        self.trace(emitted=False)
        self.assertIsNone(self.join()[0]['dot-object_2.1.2']['sink_finding']['credited_route'])

    def test_description_form_accepted_and_s2_flag_without_traces(self):
        self.finding(rules=False)
        self.trace()
        self.assertEqual(self.join()[0]['dot-object_2.1.2']['sink_finding']['status'], 'finding_verified')
        os.remove(self.raw / 'proto-match-trace.json.gz')
        recs, mode = self.h.run(['dot-object_2.1.2'])
        self.assertFalse(mode)
        self.assertIsNone(recs['dot-object_2.1.2']['sink_finding'])
        recs, mode = self.h.run(['dot-object_2.1.2'], s2=True)
        self.assertEqual(recs['dot-object_2.1.2']['sink_finding']['status'], 'finding_unverified')


CROSS = ('function set(obj, path, v) {\n    var cur = obj;\n    for (var i = 0; i < path.length; i++) {\n'
         '        cur = cur[path[i]];\n    }\n    cur[last] = path;\n}\n')


class CrossLineTests(unittest.TestCase):
    """Credit needs the Use on the operand's own line: a line-6 Use against a line-4 operand never credits,
    so an ambiguity on line 6 is a frozen-line diagnostic, not the case's failure reason."""

    def case(self, h, shadow):
        t = CROSS
        decl = at(t, 'path')
        idn = ident(OWN, decl, [decl])
        step = operand(t, 'path[i]', 4, base='path', name='path[i]', **idn)
        leaf = operand(t, 'last', 6, role='property', **ident(OWN, None, []))
        rhs = at(t, 'path', 3)
        other = ident(OWN, decl, [(70, 73)]) if shadow else idn
        reads = [(6, read(t, 'path', nth=3, **other))]
        h.case('flat_5.0.0', t, operator_coord(t, 6), [(6, 'path', *rhs)],
               [fact(t, 'cur[last]', 6, 'path_walk', [step, leaf])], reads, data_param=('path', 0))

    def test_rebound_frozen_line_read_is_not_the_failure_reason(self):
        h = Harness(self)
        self.case(h, shadow=True)
        r = h.run(['flat_5.0.0'])[0]['flat_5.0.0']
        self.assertIsNone(r['credited_route'])
        self.assertEqual(r['first_failed_condition'], 'earlier_site_absent_from_witness')
        self.assertEqual(r['ambiguous'], [])
        self.assertEqual(r['competing_spans'], [])
        self.assertEqual(len(r['frozen_line_ambiguous']), 1)
        self.assertEqual(r['frozen_line_ambiguous'][0]['why'], 'version')

    def test_unambiguous_cross_line_use_is_use_on_other_line(self):
        h = Harness(self)
        self.case(h, shadow=False)
        r = h.run(['flat_5.0.0'])[0]['flat_5.0.0']
        self.assertIsNone(r['credited_route'])
        self.assertEqual(r['first_failed_condition'], 'use_on_other_line')
        self.assertEqual(r['frozen_line_ambiguous'], [])

    def test_same_line_ambiguity_is_still_m_ambig(self):
        h = Harness(self)
        decl = at(NESTED, 'key')
        anchor = decl[0]
        other = ident((40, 60), (41, 44), [(41, 44)])
        nested_fixture(h, uses=[(2, 'key', anchor, anchor)], extra_reads=[(2, read(NESTED, 'key', nth=1, **other))])
        r = h.run(['dot-object_2.1.2'])[0]['dot-object_2.1.2']
        self.assertEqual(r['first_failed_condition'], 'M-AMBIG')
        self.assertEqual(r['frozen_line_ambiguous'], [])
        self.assertTrue(r['ambiguous'])


def use_of(op, line=None, span_=None, path=None):
    s, e = span_ or op['span']
    p = path or op['name']
    return {'line': line if line is not None else op['line'], 'path': p, 'base': J.base_name(p),
            'start_byte': s, 'end_byte': e}


class MatchUseTests(unittest.TestCase):
    """One Use against one operand: every branch of DESIGN 5.3."""

    def setUp(self):
        self.decl = at(NESTED, 'key')
        self.idn = ident(OWN, self.decl, [self.decl])
        self.op = J.operand_view(operand(NESTED, 'key', 2, nth=1, **self.idn))
        self.anchor = (self.op['span'][0],) * 2

    def verdict(self, use, reads):
        by_line = {}
        for r in reads:
            by_line.setdefault(2, []).append(r)
        return J.match_use(use, self.op, by_line)

    def own_read(self, **kw):
        base = dict(self.idn)
        base.update(kw)
        return read(NESTED, 'key', nth=1, **base)

    def test_exact_needs_no_inventory(self):
        v, d = J.match_use(use_of(self.op), self.op, {})
        self.assertEqual(v, 'exact')

    def test_exact_is_vetoed_by_an_incomplete_inventory_read_in_the_operand(self):
        v, d = self.verdict(use_of(self.op), [read(NESTED, 'key', nth=1, complete=False, **self.idn)])
        self.assertEqual((v, d['why']), ('ambiguous', 'incomplete_resolution'))

    def test_exact_is_vetoed_by_two_versions_owner_or_binding_in_the_operand(self):
        for kw, why in (({'reaching_versions': [span(*self.decl), span(50, 53)]}, 'version'),
                        ({'owner': span(1, 2)}, 'owner'), ({'binding': span(41, 44)}, 'binding')):
            v, d = self.verdict(use_of(self.op), [self.own_read(**kw)])
            self.assertEqual((v, d['why']), ('ambiguous', why))

    def test_exact_survives_an_unknown_binding_read(self):
        v, _ = self.verdict(use_of(self.op), [self.own_read(binding=None)])
        self.assertEqual(v, 'exact')

    def test_exact_ignores_reads_outside_the_operand_span(self):
        stray = read(NESTED, 'key', nth=0, complete=False, **self.idn)   # the formal, elsewhere
        v, _ = self.verdict(use_of(self.op), [stray, self.own_read()])
        self.assertEqual(v, 'exact')

    def test_base_and_path_mismatch(self):
        v, d = self.verdict(use_of(self.op, path='obj'), [self.own_read()])
        self.assertEqual((v, d['why']), ('mismatch', 'name_or_path'))
        v, _ = self.verdict(use_of(self.op, path='key.x'), [self.own_read()])   # compatible path: a prefix
        self.assertNotEqual(v, 'mismatch')
        v, _ = self.verdict(use_of(self.op, path='other.key'), [self.own_read()])
        self.assertEqual(v, 'mismatch')

    def test_zero_width_name_branch_reasons(self):
        use = use_of(self.op, span_=self.anchor)
        self.assertEqual(self.verdict(use, [self.own_read()])[0], 'name')
        cases = (({'binding': span(41, 44)}, 'binding'), ({'owner': span(1, 2)}, 'owner'),
                 ({'binding': None}, 'unknown_binding'),
                 ({'reaching_versions': [span(*self.decl), span(50, 53)]}, 'version'),
                 ({'reaching_versions': []}, 'version'))
        for kw, why in cases:
            v, d = self.verdict(use, [self.own_read(**kw)])
            self.assertEqual((v, d['why']), ('ambiguous', why), kw)
        v, d = self.verdict(use, [read(NESTED, 'key', nth=1, complete=False, **self.idn)])
        self.assertEqual((v, d['why']), ('ambiguous', 'incomplete_resolution'))
        v, d = J.match_use(use, self.op, {})
        self.assertEqual((v, d['why']), ('ambiguous', 'no_read_inventory'))

    def test_unknown_operand_binding_never_admits_by_name(self):
        op = dict(self.op, identity=(span_t(OWN), None, ()))
        v, d = J.match_use(use_of(op, span_=self.anchor), op, {2: [self.own_read()]})
        self.assertEqual((v, d['why']), ('ambiguous', 'unknown_binding'))

    def test_cross_line_ambiguity_is_flagged(self):
        use = use_of(self.op, line=3, span_=self.anchor)
        v, d = J.match_use(use, self.op, {3: [self.own_read(owner=span(1, 2))]})
        self.assertEqual((v, d['cross_line']), ('ambiguous', True))


def span_t(sp):
    return (sp[0], sp[1])


class IsolationTests(unittest.TestCase):
    """An unreadable witness, listing or taint output fails its own case only."""

    def setUp(self):
        self.h = Harness(self)
        nested_fixture(self.h)
        self.witness = self.h.out / 'raw' / J.CLASS / 'dot-object_2.1.2' / 'witness.json.gz'

    def one(self):
        return self.h.run(['dot-object_2.1.2'])[0]['dot-object_2.1.2']

    def test_truncated_gzip_witness(self):
        self.witness.write_bytes(self.witness.read_bytes()[:-12])
        r = self.one()
        self.assertEqual(r['first_failed_condition'], 'witness_unreadable')
        self.assertTrue(r['structural_match'])
        self.assertTrue(any('witness' in e for e in r['errors']))

    def test_corrupt_gzip_body_and_bad_json(self):
        blob = bytearray(gzip.compress(b'{"a": 1}'))
        blob[12:18] = b'\xff' * 6
        self.witness.write_bytes(bytes(blob))
        self.assertEqual(self.one()['first_failed_condition'], 'witness_unreadable')
        self.witness.write_bytes(gzip.compress(b'{not json'))
        self.assertEqual(self.one()['first_failed_condition'], 'witness_unreadable')

    def test_non_dict_witness_shapes(self):
        for doc in ([1, 2], 'str', None, {'reasoning': None}, {'reasoning': []}, {'reasoning': {'per_sink': None}},
                    {'reasoning': {'per_sink': [3]}}, {'reasoning': {'per_sink': [{'sink': None, 'sources': [4]}]}}):
            gz(self.witness, doc)
            r = self.one()
            self.assertEqual(r['first_failed_condition'], 'witness_unreadable', doc)
            self.assertFalse(r['credited_route'])

    def test_non_dict_listing_elements_are_skipped(self):
        c = self.h.canned['dot-object_2.1.2']
        c['facts'] = [None, 5, 'x'] + c['facts']
        c['name_reads'] = [None, 7] + c['name_reads']
        (self.h.root / 'canned.json').write_text(json.dumps(self.h.canned))
        r = self.one()
        self.assertEqual(r['credited_route'], 1)

    def test_listing_with_bad_operand_elements_does_not_abort(self):
        c = self.h.canned['dot-object_2.1.2']
        c['facts'][0]['key_operands'].append(None)
        (self.h.root / 'canned.json').write_text(json.dumps(self.h.canned))
        self.assertEqual(self.one()['credited_route'], 1)

    def test_one_failing_case_does_not_abort_the_run(self):
        ok = 'cached-path-relative_1.0.1'
        nested_fixture(self.h, entry=ok)

        def listing(binary, repo):
            if 'dot-object' in str(repo):
                raise RuntimeError('boom')
            return J.run_listing(binary, repo)

        recs, _ = J.run_join(self.h.out, self.h.packages, self.h.binary, ['dot-object_2.1.2', ok], False, listing)
        self.assertEqual(recs[0]['first_failed_condition'], 'join_error')
        self.assertIn('boom', recs[0]['errors'][0])
        self.assertEqual(recs[1]['credited_route'], 1)

    def test_truncated_taint_and_trace_in_s2(self):
        raw = self.h.out / 'raw' / J.CLASS / 'dot-object_2.1.2'
        gz(raw / 'taint.json.gz', {'findings': [{'file': FILE, 'line': 2, 'category': 'taint_sink',
                                                 'rules': [{'id': J.RULE}]}, 'junk', None]})
        (raw / 'proto-match-trace.json.gz').write_bytes(gzip.compress(b'{"matches": []}')[:-9])
        recs, mode = self.h.run(['dot-object_2.1.2'])
        self.assertTrue(mode)   # a trace file exists, though unreadable
        sf = recs['dot-object_2.1.2']['sink_finding']
        self.assertEqual(sf['status'], 'finding_unverified')
        self.assertIn('trace_error', sf)
        (raw / 'taint.json.gz').write_bytes((raw / 'taint.json.gz').read_bytes()[:-9])
        sf = self.one()['sink_finding']
        self.assertEqual(sf['status'], 'taint_output_unreadable')

    def test_taint_non_dict_findings_and_rules(self):
        raw = self.h.out / 'raw' / J.CLASS / 'dot-object_2.1.2'
        gz(raw / 'taint.json.gz', {'findings': [{'file': FILE, 'line': 2, 'category': 'taint_sink', 'rules': 'x',
                                                 'description': 'js-proto-write'}]})
        sf = self.h.run(['dot-object_2.1.2'], s2=True)[0]['dot-object_2.1.2']['sink_finding']
        self.assertEqual(sf['status'], 'finding_unverified')
        gz(raw / 'taint.json.gz', {'findings': 'oops'})
        sf = self.h.run(['dot-object_2.1.2'], s2=True)[0]['dot-object_2.1.2']['sink_finding']
        self.assertEqual(sf['status'], 'no_finding')


class SchemaTests(unittest.TestCase):
    def test_every_record_has_the_same_keys_and_binary_identity(self):
        h = Harness(self)
        nested_fixture(h)
        nested_fixture(h, entry='cached-path-relative_1.0.1')
        h.canned['cached-path-relative_1.0.1']['facts'] = []
        (h.root / 'canned.json').write_text(json.dumps(h.canned))
        recs, _ = J.run_join(h.out, h.packages, h.binary, ['dot-object_2.1.2', 'cached-path-relative_1.0.1',
                                                            'dotty_0.0.1'])
        self.assertEqual(len({tuple(sorted(r)) for r in recs}), 1)
        self.assertEqual([r['first_failed_condition'] for r in recs], [None, 'no_fact_at_target', 'no_frozen_row'])
        for r in recs:
            self.assertEqual(r['binary_version'], 'prism 9.9.9-test')
            self.assertEqual(r['binary_path'], str(h.binary.resolve()))
            self.assertEqual(r['ambiguous'] if r['entry'] != 'x' else [], r['ambiguous'])
            self.assertIn('frozen_line_ambiguous', r)

    def test_version_failure_is_recorded_not_raised(self):
        h = Harness(self)
        h.binary.write_text('#!/bin/sh\nexit 3\n')
        ident_ = J.binary_identity(h.binary)
        self.assertEqual(ident_['binary_version'], '--version exited 3')
        self.assertIn('failed', J.binary_identity(h.root / 'missing')['binary_version'])


class SinkTextTests(unittest.TestCase):
    def row(self, entry='dot-object_2.1.2'):
        return {'entry': entry, 'sink': {'file': FILE}}

    def test_source_tree_then_tarball_fallback_then_error(self):
        tmp = tempfile.TemporaryDirectory(prefix='pp-join-tgz-')
        self.addCleanup(tmp.cleanup)
        packages = Path(tmp.name)
        entry_dir = packages / J.CLASS / 'dot-object_2.1.2'
        entry_dir.mkdir(parents=True)
        text, err = J.sink_text(packages, self.row())
        self.assertIsNone(text)
        self.assertIn('not found', err)
        data = b'tarball body\n'
        with tarfile.open(entry_dir / 'pkg.tgz', 'w:gz') as t:
            for name in ('package/other.js', 'package/' + FILE):
                ti = tarfile.TarInfo(name)
                body = data if name.endswith(FILE) else b'other'
                ti.size = len(body)
                t.addfile(ti, io.BytesIO(body))
        self.assertEqual(J.sink_text(packages, self.row()), (data, None))
        src = J.package_dir(packages, 'dot-object_2.1.2')
        src.mkdir(parents=True)
        (src / FILE).write_bytes(b'source tree body\n')
        self.assertEqual(J.sink_text(packages, self.row()), (b'source tree body\n', None))

    def test_corrupt_tarball_is_skipped(self):
        tmp = tempfile.TemporaryDirectory(prefix='pp-join-tgz-')
        self.addCleanup(tmp.cleanup)
        entry_dir = Path(tmp.name) / J.CLASS / 'dot-object_2.1.2'
        entry_dir.mkdir(parents=True)
        (entry_dir / 'bad.tgz').write_bytes(b'not a tarball')
        text, err = J.sink_text(Path(tmp.name), self.row())
        self.assertIsNone(text)
        self.assertIsNotNone(err)


class CoordinateTests(unittest.TestCase):
    def test_crlf_lone_cr_and_unicode_line_separators_start_lines(self):
        for sep in ('\r\n', '\r', ' ', ' ', '\n'):
            text = 'ab' + sep + 'cd = 1;' + sep + 'ef'
            first = len(('ab' + sep).encode())
            self.assertEqual(J.coordinate_offset(text, 1, 1), 0, repr(sep))
            self.assertEqual(J.coordinate_offset(text, 2, 1), first, repr(sep))
            self.assertEqual(J.coordinate_offset(text, 2, 4), first + 3, repr(sep))
            self.assertEqual(J.coordinate_offset(text, 3, 1), first + len(('cd = 1;' + sep).encode()), repr(sep))
            self.assertIsNone(J.coordinate_offset(text, 4, 1), repr(sep))

    def test_column_past_the_line_body_is_out_of_range(self):
        text = 'ab\r\ncd\r\n'
        self.assertEqual(J.coordinate_offset(text, 1, 3), 2)       # just past `ab`, before the CR
        self.assertIsNone(J.coordinate_offset(text, 1, 4))         # the line break is not part of the line
        self.assertIsNone(J.coordinate_offset(text, 1, 0))
        self.assertEqual(J.coordinate_offset(text, 3, 1), len(text))   # empty last line

    def test_bom_counts_as_one_utf16_unit_and_three_bytes(self):
        text = '﻿x = 1;\n'
        self.assertEqual(J.coordinate_offset(text, 1, 1), 0)
        self.assertEqual(J.coordinate_offset(text, 1, 4), 3 + 2)    # the `=`: BOM, `x`, space precede it

    def test_inside_a_surrogate_pair_is_none(self):
        text = '\U0001F600x = 1;'
        self.assertEqual(J.coordinate_offset(text, 1, 3), 4)
        self.assertIsNone(J.coordinate_offset(text, 1, 2))

    def test_crlf_file_maps_to_the_member(self):
        text = 'function f(o, k) {\r\n    o[k] = 1;\r\n}\r\n'
        f = fact(text, 'o[k]', 2, 'path_walk', [])
        m = J.map_designated(text.encode(), 2, 10, [f])
        self.assertEqual((m['how'], m['fact']), ('operator_gap', f))
        self.assertIsNone(m['note'])

    def test_bom_file_maps_to_the_member(self):
        text = '﻿o[k] = 1;\n'
        f = fact(text, 'o[k]', 1, 'path_walk', [])
        m = J.map_designated(text.encode(), 1, 7, [f])
        self.assertEqual(m['fact'], f)

    def test_comment_between_member_and_operator(self):
        for text, line, col in (('o[k] /* c */ = 1;\n', 1, 14), ('o[k] // c\n    = 1;\n', 2, 5),
                                ('o[k] /* a */ /* b */ += 1;\n', 1, 22)):
            f = fact(text, 'o[k]', 1, 'path_walk', [])
            m = J.map_designated(text.encode(), line, col, [f])
            self.assertEqual((m['how'], m['fact']), ('operator_gap', f), text)
        # a comment-looking gap that holds code is no gap
        text = 'o[k] /* c */ x = 1;\n'
        self.assertIsNone(J.map_designated(text.encode(), 1, 16, [fact(text, 'o[k]', 1, 'path_walk', [])])['fact'])

    def test_two_same_line_members_choose_the_nearer_end(self):
        text = '((o[k])) = 1;\n'
        inner = fact(text, 'o[k]', 1, 'path_walk', [])
        wrapped = fact(text, '(o[k])', 1, 'path_walk', [])
        m = J.map_designated(text.encode(), 1, 10, [inner, wrapped])
        self.assertEqual(m['fact'], wrapped)                      # ends nearest the operator
        self.assertEqual(m['competing'], [])                      # the inner one ends earlier: not a tie
        # equal ends: the outermost wins and the other is listed as competing
        text2 = 'a.o[k] = 1;\n'
        outer = fact(text2, 'a.o[k]', 1, 'path_walk', [])
        inner2 = fact(text2, 'o[k]', 1, 'path_walk', [])
        m = J.map_designated(text2.encode(), 1, 8, [inner2, outer])
        self.assertEqual(m['fact'], outer)
        self.assertEqual(m['competing'], [[inner2['start_byte'], inner2['end_byte']]])

    def test_member_far_from_the_operator_is_not_chosen(self):
        text = 'x[a] = y[b] = 1;\n'
        a = fact(text, 'x[a]', 1, 'path_walk', [])
        b = fact(text, 'y[b]', 1, 'path_walk', [])
        m1 = J.map_designated(text.encode(), 1, 6, [a, b])
        m2 = J.map_designated(text.encode(), 1, 13, [a, b])
        self.assertEqual((m1['fact'], m2['fact']), (a, b))

    def test_containing_member_when_no_gap(self):
        text = 'o[k = 1];\n'
        f = fact(text, 'o[k = 1]', 1, 'path_walk', [])
        m = J.map_designated(text.encode(), 1, 5, [f])
        self.assertEqual((m['how'], m['fact']), ('contains', f))


class S2TotalsTests(unittest.TestCase):
    def setUp(self):
        self.h = Harness(self)
        nested_fixture(self.h)
        self.raw = self.h.out / 'raw' / J.CLASS / 'dot-object_2.1.2'
        self.target = list(at(NESTED, 'obj[key]'))

    def write(self, finding_line, route):
        gz(self.raw / 'taint.json.gz', {'findings': [{'file': FILE, 'line': finding_line, 'category': 'taint_sink',
                                                      'rules': [{'id': J.RULE}]}]})
        gz(self.raw / 'proto-match-trace.json.gz',
           {'matches': [{'file': FILE, 'target': span(*self.target), 'route': route, 'emitted': True}]})

    def test_no_finding_carries_no_route_fields(self):
        self.write(3, 'target_line')
        sf = self.h.run(['dot-object_2.1.2'])[0]['dot-object_2.1.2']['sink_finding']
        self.assertEqual(sf['status'], 'no_finding')
        self.assertEqual((sf['route1'], sf['route2'], sf['credited_route']), (None, None, None))
        self.assertEqual(sf['trace_records'], 1)

    def test_unverified_and_unrouted_carry_no_route_fields(self):
        self.write(2, 'target_line')
        gz(self.raw / 'proto-match-trace.json.gz', {'matches': [{'file': FILE, 'target': span(0, 1)}]})
        sf = self.h.run(['dot-object_2.1.2'])[0]['dot-object_2.1.2']['sink_finding']
        self.assertEqual((sf['status'], sf['route1'], sf['credited_route']), ('finding_unverified', None, None))
        self.write(2, 'unknown-route')
        sf = self.h.run(['dot-object_2.1.2'])[0]['dot-object_2.1.2']['sink_finding']
        self.assertEqual((sf['status'], sf['route2'], sf['credited_route']), ('finding_without_route_credit', None, None))

    def test_totals_tally_sink_routes_and_differences(self):
        self.write(2, 'target_line')
        recs, mode = self.h.run(['dot-object_2.1.2'])
        text = J.totals(list(recs.values()), mode)
        self.assertIn('S2 sink finding, per class', text)
        self.assertIn('nested_keys: 1 1 1 0 0', text)
        self.assertIn('S2 route differences from the table: 0', text)
        self.write(2, 'EarlierSelection')    # expected route 1, the sink credits route 2
        recs, mode = self.h.run(['dot-object_2.1.2'])
        text = J.totals(list(recs.values()), mode)
        self.assertIn('nested_keys: 1 1 0 1 1', text)
        self.assertIn('S2 route differences from the table: 1\n  dot-object_2.1.2 expected 1 credited 2', text)
        # not S2: no S2 lines
        self.assertNotIn('S2 sink finding', J.totals(list(recs.values()), False))


# Copy of the `strict` list of the frozen conversions (~/prism-evidence/pp/m/converted.json, key `strict`).
STRICT = ["arr-flatten-unflatten_1.1.4", "assign-deep_1.0.0", "cached-path-relative_1.0.1", "confucious_0.0.12",
          "deeps_1.4.5", "doc-path_2.0.0", "dot-notes_3.2.0", "dot-object_2.1.2", "dotty_0.0.1",
          "eivindfjeldstad-dot_0.0.1", "field_1.0.1", "flat_5.0.0", "grunt-util-property_0.0.2",
          "json-pointer_0.6.0", "json-pointer_0.6.1", "json8-merge-patch_1.0.1", "jsonpointer_4.0.0",
          "just-safe-set_1.0.0", "keyd_1.3.4", "keyget_2.2.0", "libnested_1.5.0", "madlib-object-utils_0.1.6",
          "minimist_1.0.0", "nodee-utils_1.2.2", "object-path-set_1.0.0", "paypal-adaptive_0.4.1",
          "set-deep-prop_1.0.0", "set-value_3.0.0", "tiny-conf_1.1.0"]


class TableTests(unittest.TestCase):
    def test_route_table_ids_are_exactly_the_strict_list(self):
        self.assertEqual(len(STRICT), 29)
        self.assertEqual(sorted(J.EXPECTED), sorted(STRICT))
        listed = [c for _, _, cases in J.ROUTE_TABLE for c in cases]
        self.assertEqual(len(listed), len(set(listed)))   # no id in two table rows

    def test_route_table_covers_the_29_strict_conversions(self):
        self.assertEqual(len(J.EXPECTED), 29)
        counts = {}
        for cls, route in J.EXPECTED.values():
            counts[(cls, route)] = counts.get((cls, route), 0) + 1
        self.assertEqual(counts, {('nested_keys', 1): 2, ('path_walk', 1): 2, ('path_walk', 2): 17,
                                  ('recursive_merge', 1): 1, ('recursive_merge', 2): 7})

    def test_cli_writes_records_and_totals(self):
        h = Harness(self)
        nested_fixture(h)
        cases = h.root / 'converted.json'
        cases.write_text(json.dumps({'strict': ['dot-object_2.1.2']}))
        out = h.root / 'records.jsonl'
        J.main([str(h.out), str(h.packages), str(h.binary), '--out', str(out), '--cases', str(cases)])
        recs = [json.loads(line) for line in out.read_text().splitlines()]
        self.assertEqual([r['entry'] for r in recs], ['dot-object_2.1.2'])
        self.assertEqual(recs[0]['credited_route'], 1)


if __name__ == '__main__':
    unittest.main()


class GapRegexTests(unittest.TestCase):
    def test_comment_cannot_swallow_code_between_two_comments(self):
        self.assertIsNone(pp_join.GAP.match(b' /* c1 */ ; b.x /* c2 */ '))

    def test_adjacent_comments_do_not_blow_up(self):
        import time
        start = time.monotonic()
        self.assertIsNone(pp_join.GAP.match(b' ' + b'/* a */' * 40 + b'x'))
        self.assertLess(time.monotonic() - start, 1.0)

    def test_a_comment_between_member_and_operator_is_allowed(self):
        self.assertIsNotNone(pp_join.GAP.match(b' /* c */ '))
