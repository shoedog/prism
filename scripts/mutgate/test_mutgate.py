#!/usr/bin/env python3
"""Planner tests and compiled gate regressions (offline, dependency-free Rust).

Run with:
    python3 -m unittest scripts/mutgate/test_mutgate.py

The gate regressions use temporary Cargo projects and exercise the same main
entry point, baseline, compilation and verdict aggregation as a real lane.
"""
import contextlib
import io
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import mutgate  # noqa: E402

LANE = Path(__file__).resolve().parents[2] / 'mutants/lane-p-tsconfig-paths.json'


def one_mutant(mid, file, a, b, test='t'):
    return {mid: {'id': mid, 'edits': [(file, a, b)], 'tests': [test], 'lib_prefixes': ()}}


class CodeMaskTests(unittest.TestCase):
    def test_raw_string_braces_are_masked(self):
        s = 'let x = r#"{ not a brace }"#;'
        mask = mutgate.code_mask(s)
        start = s.index('r#"')
        end = s.index('"#', start + 3) + 2
        self.assertTrue(all(not mask[i] for i in range(start, end)))
        self.assertTrue(mask[0])  # 'l' of "let" is real code

    def test_byte_string_braces_are_masked(self):
        s = 'let x = b"{}";'
        mask = mutgate.code_mask(s)
        start = s.index('b"')
        end = s.index('"', start + 2) + 1
        self.assertTrue(all(not mask[i] for i in range(start, end)))


class FnBodiesTests(unittest.TestCase):
    def test_const_fn_is_detected(self):
        s = 'const fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n'
        bodies = mutgate.fn_bodies(s, mutgate.code_mask(s))
        self.assertEqual(len(bodies), 1)
        _, body_open, body_close = bodies[0]
        self.assertEqual(s[body_open], '{')
        self.assertEqual(s[body_close], '}')

    def test_raw_string_with_unbalanced_brace_does_not_confuse_depth_counting(self):
        s = (
            'fn f() -> &\'static str {\n'
            '    let s = r#"{ unbalanced"#;\n'
            '    s\n'
            '}\n'
        )
        mask = mutgate.code_mask(s)
        bodies = mutgate.fn_bodies(s, mask)
        self.assertEqual(len(bodies), 1)
        _, body_open, body_close = bodies[0]
        self.assertEqual(s[body_close], '}')
        self.assertEqual(body_close, len(s.rstrip('\n')) - 1)

    def test_nested_fn_bodies_both_discovered_outer_first(self):
        s = (
            'fn outer() -> i32 {\n'
            '    fn inner() -> i32 { 1 + 1 }\n'
            '    inner() + 1\n'
            '}\n'
        )
        bodies = mutgate.fn_bodies(s, mutgate.code_mask(s))
        self.assertEqual(len(bodies), 2)
        outer, inner = bodies
        self.assertLess(outer[1], inner[1])
        self.assertGreater(outer[2], inner[2])


class PlanSchemataTests(unittest.TestCase):
    def test_mutation_outside_any_fn_body_demotes_to_static(self):
        s = 'struct Point { x: i32, y: i32 }\n'
        muts = one_mutant('m1', 'src/lib.rs', 'x: i32', 'x: i64')
        units, static = mutgate.plan_schemata(muts, {'src/lib.rs': s})
        self.assertEqual(units, {})
        self.assertIn('m1', static)

    def test_macro_rules_body_at_module_scope_demotes_to_static(self):
        s = (
            'macro_rules! add_one {\n'
            '    ($x:expr) => { $x + 1 };\n'
            '}\n'
        )
        muts = one_mutant('m1', 'src/lib.rs', '$x + 1', '$x + 2')
        units, static = mutgate.plan_schemata(muts, {'src/lib.rs': s})
        self.assertEqual(units, {})
        self.assertIn('m1', static)

    def test_safe_macro_rules_body_nested_in_fn_stays_in_schema(self):
        s = (
            'fn two() -> i32 {\n'
            '    macro_rules! lit { () => { 2 }; }\n'
            '    lit!()\n'
            '}\n'
        )
        muts = one_mutant('m1', 'src/lib.rs', '{ 2 }', '{ 3 }')
        units, static = mutgate.plan_schemata(muts, {'src/lib.rs': s})
        self.assertEqual(static, {})
        self.assertEqual(len(units), 1)

    def test_const_fn_schema_included(self):
        s = 'const fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n'
        muts = one_mutant('m1', 'src/lib.rs', 'a + b', 'a - b')
        units, static = mutgate.plan_schemata(muts, {'src/lib.rs': s})
        self.assertEqual(static, {})
        self.assertEqual(len(units), 1)

    def test_static_inside_fn_body_schema_included(self):
        s = (
            'fn counter() -> i32 {\n'
            '    static N: i32 = 5;\n'
            '    N + 1\n'
            '}\n'
        )
        muts = one_mutant('m1', 'src/lib.rs', 'static N: i32 = 5;', 'static N: i32 = 6;')
        units, static = mutgate.plan_schemata(muts, {'src/lib.rs': s})
        self.assertEqual(static, {})
        self.assertEqual(len(units), 1)

    def test_nested_fn_mutation_attaches_to_outermost_enclosing_fn(self):
        s = (
            'fn outer() -> i32 {\n'
            '    fn inner() -> i32 { 1 + 1 }\n'
            '    inner() + 1\n'
            '}\n'
        )
        outer_open = mutgate.fn_bodies(s, mutgate.code_mask(s))[0][1]
        muts = one_mutant('m1', 'src/lib.rs', '1 + 1', '2 + 2')
        units, static = mutgate.plan_schemata(muts, {'src/lib.rs': s})
        self.assertEqual(static, {})
        self.assertEqual(len(units), 1)
        (file, o, c), arms = next(iter(units.items()))
        self.assertEqual(o, outer_open)
        self.assertIn('m1', arms)

    def test_closure_inside_fn_uses_enclosing_fn_body(self):
        s = (
            'fn outer() -> i32 {\n'
            '    let f = |x: i32| x + 1;\n'
            '    f(1)\n'
            '}\n'
        )
        muts = one_mutant('m1', 'src/lib.rs', 'x + 1', 'x + 2')
        units, static = mutgate.plan_schemata(muts, {'src/lib.rs': s})
        self.assertEqual(static, {})
        self.assertEqual(len(units), 1)

    def test_impl_trait_return_demotes_to_static(self):
        # Two copies of the same closure literal (one per match arm) are two
        # distinct anonymous types; RPIT inference needs exactly one, so this
        # unit must never be rendered as a schema.
        s = (
            'fn make_adder(n: i32) -> impl Fn(i32) -> i32 {\n'
            '    move |x| x + n\n'
            '}\n'
        )
        muts = one_mutant('m1', 'src/lib.rs', 'x + n', 'x - n')
        units, static = mutgate.plan_schemata(muts, {'src/lib.rs': s})
        self.assertEqual(units, {})
        self.assertIn('m1', static)

    def test_non_impl_trait_fn_after_impl_trait_fn_is_unaffected(self):
        s = (
            'fn make_adder(n: i32) -> impl Fn(i32) -> i32 {\n'
            '    move |x| x + n\n'
            '}\n'
            'fn add(a: i32, b: i32) -> i32 {\n'
            '    a + b\n'
            '}\n'
        )
        muts = one_mutant('m1', 'src/lib.rs', 'a + b', 'a - b')
        units, static = mutgate.plan_schemata(muts, {'src/lib.rs': s})
        self.assertEqual(static, {})
        self.assertEqual(len(units), 1)

    def test_multiple_mutants_in_one_function_become_separate_arms(self):
        s = (
            'fn calc(a: i32, b: i32) -> i32 {\n'
            '    let sum = a + b;\n'
            '    let prod = a * b;\n'
            '    sum + prod\n'
            '}\n'
        )
        muts = {
            **one_mutant('m1', 'src/lib.rs', 'a + b', 'a - b'),
            **one_mutant('m2', 'src/lib.rs', 'a * b', 'a / b'),
        }
        units, static = mutgate.plan_schemata(muts, {'src/lib.rs': s})
        self.assertEqual(static, {})
        self.assertEqual(len(units), 1)
        (_, _, _), arms = next(iter(units.items()))
        self.assertEqual(set(arms), {'m1', 'm2'})

        rendered = mutgate.render({'src/lib.rs': s}, units, mutgate.accessor_for)
        out = rendered['src/lib.rs']
        self.assertIn('"m1"', out)
        self.assertIn('"m2"', out)
        self.assertIn('a - b', out)
        self.assertIn('a / b', out)
        # the default arm must be byte-identical to the original body
        self.assertIn('_ => {\n    let sum = a + b;\n    let prod = a * b;\n    sum + prod\n}', out)


class RenderTests(unittest.TestCase):
    def test_accessor_for_main_and_bin_use_prism_path(self):
        self.assertEqual(mutgate.accessor_for('src/main.rs'), 'prism::__mutant')
        self.assertEqual(mutgate.accessor_for('src/bin/foo.rs'), 'prism::__mutant')
        self.assertEqual(mutgate.accessor_for('src/lib.rs'), 'crate::__mutant')
        self.assertEqual(mutgate.accessor_for('src/ast.rs'), 'crate::__mutant')


class GateRegressionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='mutgate-regression-')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / 'src').mkdir()
        (self.root / 'Cargo.toml').write_text(
            '[package]\nname="prism"\nversion="0.0.0"\nedition="2021"\n')

    def gate(self, source, mutations, mode=None, extra=None, timeout=5, flags=()):
        (self.root / 'src/lib.rs').write_text(source)
        lane = self.root / 'lane.json'
        lane.write_text(json.dumps({'mutations': mutations, 'extra': extra or {},
                                    'lib_test_prefixes': ['']}))
        out = self.root / ('report-' + str(mode))
        args = ['mutgate', '--lane', str(lane), '--jobs', '2',
                '--timeout', str(timeout), '--out', str(out), *flags]
        if mode: args += ['--mode', mode]
        if mode == 'schema': args += ['--since', 'HEAD', '--scope', 'file']
        output = io.StringIO()
        with patch.object(mutgate, 'ROOT', self.root), patch.object(sys, 'argv', args), \
                patch.dict(os.environ, {'CARGO_TARGET_DIR': str(self.root / 'target')}), \
                patch.object(mutgate, 'changed_lines', return_value={'src/lib.rs': {1}}), \
                contextlib.redirect_stdout(output):
            try:
                status = mutgate.main()
            except SystemExit as e:
                status = e.code
        self.last_output = output.getvalue()
        summary = json.loads((out / 'summary.json').read_text()) if (out / 'summary.json').exists() else {}
        return status, summary

    def test_six_indirect_observers_are_authoritative_survivors_and_advisory_kills(self):
        fixtures = Path(__file__).with_name('fixtures')
        mutation = {'m': ['src/lib.rs', 'let ignored = 1;', 'let ignored = 2;', 't']}
        for fixture in sorted(fixtures.iterdir()):
            with self.subTest(fixture=fixture.name):
                observer = self.root / 'src/observer.rs'
                observer.unlink(missing_ok=True)
                for p in (fixture / 'src').glob('*.rs'):
                    (self.root / 'src' / p.name).write_bytes(p.read_bytes())
                source = (fixture / 'src/lib.rs').read_text()
                for mode, flags in ((None, ()), (None, ('--since', 'HEAD', '--scope', 'file', '--authoritative')),
                                    ('schema', ())):
                    status, summary = self.gate(source, mutation, mode, flags=flags)
                    advisory = mode == 'schema'
                    self.assertEqual(summary['selected'], 1)
                    self.assertEqual(status, 0 if advisory else 1)
                    self.assertEqual(summary['authoritative'], not advisory)
                    self.assertIn('ADVISORY' if advisory else 'AUTHORITATIVE', self.last_output)
                    self.assertEqual(summary['results']['m']['verdict'], 'KILLED' if advisory else 'SURVIVED')
                    self.assertEqual(summary['results']['m']['mode'], 'schema' if advisory else 'text')
                    self.assertTrue(summary['results']['m']['admissible'])
                    self.assertTrue(all(r['admissible'] and not r['killed']
                                        for r in summary['baselines']['source'].values()))

    def test_schema_without_scope_and_authoritative_schema_are_rejected(self):
        source = 'fn f() -> u32 { 1 }\n#[test]\nfn t() { assert_eq!(f(), 1); }\n'
        mutation = {'m': ['src/lib.rs', '{ 1 }', '{ 2 }', 't']}
        for flags in (('--mode', 'schema'), ('--since', 'HEAD', '--mode', 'schema', '--authoritative'),
                      ('--jobs', '0')):
            with self.subTest(flags=flags):
                status, _ = self.gate(source, mutation, flags=flags)
                self.assertEqual(status, 2)

    def test_scoped_default_is_advisory_and_explicit_text_is_authoritative(self):
        source = 'fn f() -> u32 { 1 }\n#[test]\nfn t() { assert_eq!(f(), 1); }\n'
        mutation = {'m': ['src/lib.rs', '{ 1 }', '{ 2 }', 't']}
        for mode in (None, 'text'):
            with self.subTest(mode=mode):
                status, summary = self.gate(source, mutation, mode, flags=('--since', 'HEAD'))
                self.assertEqual(status, 0)
                self.assertEqual(summary['authoritative'], mode == 'text')

    def test_parallel_text_workers_have_private_trees_targets_and_cleanup(self):
        source = 'fn f() -> u32 { 1 }\n#[test]\nfn t() { assert_eq!(f(), 1); }\n'
        mutations = {str(n): ['src/lib.rs', '{ 1 }', '{ ' + str(n) + ' }', 't'] for n in (2, 3, 4)}
        builds = []
        original = mutgate.build
        def record(tree, target, targets):
            builds.append((tree, target, (tree / 'src/lib.rs').read_text()))
            return original(tree, target, targets)
        with patch.object(mutgate, 'build', side_effect=record):
            status, summary = self.gate(source, mutations)
        self.assertEqual(status, 0)
        worker_builds = builds[1:]
        self.assertEqual(len({target for _, target, _ in worker_builds}), 2)
        self.assertEqual(len({tree for tree, _, _ in worker_builds}), 2)
        self.assertEqual(sum(text == source for _, _, text in worker_builds), 2)
        self.assertEqual({text for _, _, text in worker_builds if text != source},
                         {source.replace('{ 1 }', '{ ' + str(n) + ' }') for n in (2, 3, 4)})
        self.assertTrue(summary['resources']['workers_cleaned'])
        self.assertGreater(summary['resources']['worker_disk_peak_bytes'], 0)
        self.assertFalse(list((self.root / 'target/mutgate').glob('workers-*')))
        self.assertEqual((self.root / 'src/lib.rs').read_text(), source)

    def test_worker_relocation_failure_cannot_kill_an_equivalent_mutant(self):
        self.root = self.root.resolve()
        warm_path = str(self.root / 'target/mutgate/tree')
        mutation = {'m': ['src/lib.rs', 'let ignored = 1;', 'let ignored = 2;', 't']}
        for compile_failure in (False, True):
            with self.subTest(compile_failure=compile_failure):
                source = 'fn f() { let ignored = 1; }\n'
                if compile_failure:
                    source += f'const _: [(); {len(warm_path)}] = [(); env!("CARGO_MANIFEST_DIR").len()];\n'
                    source += '#[test]\nfn t() { f(); }\n'
                else:
                    source += '#[test]\nfn t() { f(); assert_eq!(env!("CARGO_MANIFEST_DIR"), ' + json.dumps(warm_path) + '); }\n'
                status, summary = self.gate(source, mutation)
                self.assertEqual(status, 1)
                self.assertEqual(summary['selected'], 1)
                self.assertEqual(summary['results']['m']['verdict'], 'INADMISSIBLE')
                self.assertIn('baseline', summary['results']['m']['error'])
                self.assertTrue(all(r['admissible'] and not r['killed'] for r in summary['baselines']['source'].values()))
                self.assertEqual(len(summary['baselines']['workers']), 1)
                self.assertTrue(summary['resources']['workers_cleaned'])

    def test_seed_failure_refuses_entire_population_and_cleans_workers(self):
        source = 'fn f() -> u32 { 1 }\n#[test]\nfn t() { assert_eq!(f(), 1); }\n'
        mutations = {str(n): ['src/lib.rs', '{ 1 }', '{ ' + str(n) + ' }', 't'] for n in (2, 3)}
        with patch.object(mutgate, 'seed_target', side_effect=OSError('seed refused')):
            status, summary = self.gate(source, mutations)
        self.assertEqual(status, 1)
        self.assertEqual(summary['selected'], 2)
        self.assertTrue(all(r['verdict'] == 'INADMISSIBLE' and 'seed refused' in r['error']
                            for r in summary['results'].values()))
        self.assertTrue(summary['resources']['workers_cleaned'])
        self.assertFalse(list((self.root / 'target/mutgate').glob('workers-*')))

    def test_w1_constructed_false_kill_survives_both_modes(self):
        source = ('pub fn f() -> u32 {\n    let ignored = 1;\n    line!()\n}\n'
                  'const AFTER: u32 = line!();\n#[test]\n'
                  'fn t() { assert_eq!(AFTER - f(), 2); }\n')
        mutation = {'m': ['src/lib.rs', 'let ignored = 1;', 'let ignored = 2;', 't']}
        for mode in ('text', 'schema'):
            with self.subTest(mode=mode):
                status, summary = self.gate(source, mutation, mode)
                self.assertEqual(status, 1)
                self.assertEqual(mutgate.verdict(summary['results']['m']), 'SURVIVED')

    def test_w1_hidden_track_caller_false_kill_requires_text_confirmation(self):
        source = ('pub fn f() -> u32 {\n    let ignored = 1;\n    location()\n}\n'
                  'const AFTER: u32 = line!();\n#[track_caller]\n'
                  'fn location() -> u32 { std::panic::Location::caller().line() }\n'
                  '#[test]\nfn t() { assert_eq!(AFTER - f(), 2); }\n')
        mutation = {'m': ['src/lib.rs', 'let ignored = 1;', 'let ignored = 2;', 't']}
        status, summary = self.gate(source, mutation, 'schema')
        self.assertEqual(status, 1)
        self.assertEqual(mutgate.verdict(summary['results']['m']), 'SURVIVED')
        self.assertTrue(summary['results']['m']['schema_observation']['killed'])
        self.assertEqual(summary['text_confirmations'], 1)
        self.assertIn('location-sensitive-test: t', summary['results']['m']['schema_observation']['confirmation_reasons'])

    def test_plain_unwrap_genuine_kills_stay_in_schema_without_confirmation(self):
        source = 'fn f() -> u32 { Some(1).unwrap() }\n#[test]\nfn t() { assert_eq!(f(), 1); }\n'
        # Both a wrong value and an actual unwrap panic are genuine, location-free
        # kills. The automatic libtest panic header must not force a text build.
        for replacement in ('Some(2)', 'None::<u32>'):
            with self.subTest(replacement=replacement):
                status, summary = self.gate(source, {'m': ['src/lib.rs', 'Some(1)', replacement, 't']}, 'schema')
                self.assertEqual(status, 0)
                self.assertEqual(summary['static'], {})
                self.assertEqual(summary['results']['m']['mode'], 'schema')
                self.assertEqual(summary['results']['m']['verdict'], 'KILLED')
                self.assertEqual(summary['text_confirmations'], 0)
                self.assertNotIn('text_mode', summary['timings'])

    def test_location_in_failure_payload_requires_confirmation(self):
        source = ('fn f() -> bool { true }\n#[test]\n'
                  'fn t() { assert!(f(), "observed src/lib.rs:12:3"); }\n')
        status, summary = self.gate(source, {'m': ['src/lib.rs', '{ true }', '{ false }', 't']}, 'schema')
        self.assertEqual(status, 0)
        self.assertEqual(summary['results']['m']['mode'], 'schema+text')
        self.assertEqual(summary['text_confirmations'], 1)
        self.assertEqual(summary['results']['m']['schema_observation']['confirmation_reasons'],
                         ['failure-payload-location: src/lib.rs:12:3'])

    def test_location_expected_by_should_panic_requires_confirmation(self):
        source = ('fn f() -> bool { true }\n#[test]\n#[should_panic(expected="src/lib.rs:")]\n'
                  'fn t() { if f() { panic!("src/lib.rs:"); } else { panic!("wrong"); } }\n')
        status, summary = self.gate(source, {'m': ['src/lib.rs', '{ true }', '{ false }', 't']}, 'schema')
        self.assertEqual(status, 0)
        self.assertEqual(summary['results']['m']['mode'], 'schema+text')
        self.assertEqual(summary['text_confirmations'], 1)
        self.assertIn('location-sensitive-test: t', summary['results']['m']['schema_observation']['confirmation_reasons'])

    def test_safe_local_macro_genuine_kill_stays_in_schema(self):
        source = ('macro_rules! lit { () => { 1 }; }\nfn f() -> u32 { lit!() + 1 }\n'
                  '#[test]\nfn t() { assert_eq!(f(), 2); }\n')
        status, summary = self.gate(source, {'m': ['src/lib.rs', 'lit!() + 1', 'lit!() + 2', 't']}, 'schema')
        self.assertEqual(status, 0)
        self.assertEqual(summary['static'], {})
        self.assertEqual(summary['results']['m']['mode'], 'schema')
        self.assertEqual(summary['text_confirmations'], 0)

    def test_w2_timeout_never_kills(self):
        source = ('fn f() -> u32 { let ignored = 1; '
                  'if ignored == 2 { std::thread::sleep(std::time::Duration::from_millis(150)); } '
                  '1 }\n#[test]\nfn t() { assert_eq!(f(), 1); }\n')
        mutation = {'m': ['src/lib.rs', 'let ignored = 1;', 'let ignored = 2;', 't']}
        for mode in ('text', 'schema'):
            with self.subTest(mode=mode):
                status, summary = self.gate(source, mutation, mode, timeout=0.05)
                self.assertEqual(status, 1)
                self.assertEqual(mutgate.verdict(summary['results']['m']), 'TIMEOUT')
                self.assertFalse(summary['results']['m']['killed'])
                # The same finite, equivalent mutant completes successfully
                # with enough time; the short timeout cannot stand in for a kill.
                status, summary = self.gate(source, mutation, mode, timeout=1)
                self.assertEqual(status, 1)
                self.assertEqual(mutgate.verdict(summary['results']['m']), 'SURVIVED')

    def test_w3_red_baseline_is_inadmissible_both_modes(self):
        source = 'fn f() -> u32 { 1 }\n#[test]\nfn t() { assert_eq!(f(), 2); }\n'
        mutation = {'m': ['src/lib.rs', '{ 1 }', '{ 1 + 0 }', 't']}
        for mode in ('text', 'schema'):
            with self.subTest(mode=mode):
                status, summary = self.gate(source, mutation, mode)
                self.assertEqual(status, 1)
                self.assertEqual(mutgate.verdict(summary['results']['m']), 'INADMISSIBLE')
                self.assertIn('baseline', summary['results']['m']['error'])

    def test_w4_missing_selector_fails_even_with_killing_selector(self):
        source = 'fn f() -> u32 { 1 }\n#[test]\nfn t() { assert_eq!(f(), 1); }\n'
        mutation = {'m': ['src/lib.rs', '{ 1 }', '{ 2 }', ['t', 't_missing']]}
        for mode in ('text', 'schema'):
            with self.subTest(mode=mode):
                status, summary = self.gate(source, mutation, mode)
                self.assertEqual(status, 1)
                self.assertEqual(mutgate.verdict(summary['results']['m']), 'INADMISSIBLE')

    def test_w4_selector_removed_or_ignored_by_mutant_is_inadmissible(self):
        source = ('fn f() -> u32 { 1 }\n#[test]\nfn t() { assert_eq!(f(), 1); }\n'
                  '#[test]\nfn other() { assert_eq!(f(), 1); }\n')
        mutation = {'m': ['src/lib.rs', '{ 1 }', '{ 2 }', ['t', 'other']]}
        for replacement in ('fn other()', '#[test]\n#[ignore]\nfn other()'):
            extra = {'m': [['src/lib.rs', '#[test]\nfn other()', replacement]]}
            for mode in ('text', 'schema'):
                with self.subTest(replacement=replacement, mode=mode):
                    status, summary = self.gate(source, mutation, mode, extra)
                    self.assertEqual(status, 1)
                    self.assertEqual(mutgate.verdict(summary['results']['m']), 'INADMISSIBLE')
                    self.assertTrue(summary['results']['m']['runs'][0]['killed'])

    def test_w3_bad_baseline_does_not_disappear_from_denominator(self):
        source = ('fn f() -> u32 { 1 }\nfn g() -> u32 { 3 }\n'
                  '#[test]\nfn t() { assert_eq!(f(), 1); }\n'
                  '#[test]\nfn bad() { assert_eq!(g(), 9); }\n')
        mutations = {'good': ['src/lib.rs', '{ 1 }', '{ 2 }', 't'],
                     'bad': ['src/lib.rs', '{ 3 }', '{ 4 }', 'bad']}
        for mode in ('text', 'schema'):
            with self.subTest(mode=mode):
                status, summary = self.gate(source, mutations, mode)
                self.assertEqual(status, 1)
                self.assertEqual(summary['selected'], 2)
                self.assertEqual(summary['killed'], 1)
                self.assertEqual(mutgate.verdict(summary['results']['bad']), 'INADMISSIBLE')

    def test_w5_failed_extra_edit_is_restored_before_next_mutant(self):
        source = 'fn f() -> u32 { 1 }\n#[test]\nfn t() { assert_eq!(f(), 1); }\n'
        mutations = {'bad': ['src/lib.rs', '{ 1 }', '{ missing }', 't'],
                     'good': ['src/lib.rs', '{ 1 }', '{ 2 }', 't']}
        status, summary = self.gate(source, mutations, 'text')
        self.assertEqual(status, 1)
        self.assertEqual(mutgate.verdict(summary['results']['bad']), 'INADMISSIBLE')
        self.assertEqual(mutgate.verdict(summary['results']['good']), 'KILLED')
        self.assertEqual((self.root / 'target/mutgate/tree/src/lib.rs').read_text(), source)

    def test_empty_selector_list_and_missing_anchor_fail(self):
        source = 'fn f() -> u32 { 1 }\n'
        for mutation in (['src/lib.rs', '{ 1 }', '{ 2 }', []],
                         ['src/lib.rs', 'gone', 'other', 't']):
            with self.subTest(mutation=mutation):
                status, summary = self.gate(source, {'m': mutation}, 'text')
                self.assertEqual(status, 1)
                self.assertEqual(mutgate.verdict(summary['results']['m']), 'INADMISSIBLE')

    def test_w5_static_fallback_contains_only_its_own_edits(self):
        source = ('struct S { a: u32 }\nfn f() -> S { S { a: 1 } }\n'
                  '#[test]\nfn t() { assert_eq!(f().a, 1); }\n')
        mutations = {'A': ['src/lib.rs', 'S { a: 1 }', 'S { a: 2 }', 't'],
                     'B': ['src/lib.rs', 'struct S { a: u32 }', 'struct S { a: u32, b: u32 }', 't']}
        extra = {'B': [['src/lib.rs', 'S { a: 1 }', 'S { a: 3, b: 0 }']]}
        for mode in ('text', 'schema'):
            with self.subTest(mode=mode):
                status, summary = self.gate(source, mutations, mode, extra)
                self.assertEqual(status, 0)
                self.assertEqual(mutgate.verdict(summary['results']['B']), 'KILLED')
                tree = self.root / 'target/mutgate/tree/src/lib.rs'
                self.assertEqual(tree.read_text(), source)


class WorkerSeedTests(unittest.TestCase):
    def test_copy_and_failed_clone_fallback_are_private_and_exclude_workers(self):
        with tempfile.TemporaryDirectory(prefix='mutgate-seed-') as tmp:
            root = Path(tmp); source = root / 'warm'
            (source / 'debug').mkdir(parents=True)
            (source / 'debug/artifact').write_bytes(b'original')
            (source / 'mutgate/workers').mkdir(parents=True)
            (source / 'mutgate/workers/secret').write_text('do not copy')
            for platform in ('linux', 'darwin'):
                with self.subTest(platform=platform), patch.object(mutgate.sys, 'platform', platform), \
                        patch.object(mutgate.subprocess, 'run') as run:
                    def failed_clone(*args, **kwargs):
                        (root / platform / 'partial').write_text('partial clone')
                        return type('Result', (), {'returncode': 1})()
                    run.side_effect = failed_clone
                    target = root / platform
                    self.assertEqual(mutgate.seed_target(source, target), 'copy')
                    self.assertFalse((target / 'mutgate').exists())
                    self.assertFalse((target / 'partial').exists())
                    (target / 'debug/artifact').write_bytes(b'changed')
                    self.assertEqual((source / 'debug/artifact').read_bytes(), b'original')


class AdmissibilityTests(unittest.TestCase):
    def test_inadmissibility_has_priority_over_a_kill(self):
        self.assertEqual(mutgate.verdict({'killed': True, 'admissible': False}), 'INADMISSIBLE')

    def test_ignored_selector_is_not_a_survivor(self):
        output = ('running 1 test\ntest t ... ignored\n'
                  'test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out;\n')
        with patch.object(mutgate.subprocess, 'run') as run:
            run.return_value.stdout = output
            run.return_value.stderr = ''
            run.return_value.returncode = 0
            result = mutgate.run_test('exe', 't', None, 1, {})
        self.assertFalse(result['admissible'])

    def test_location_observers_in_original_or_mutated_body_demote(self):
        for expression in ('line!()', 'column!()', 'file!()', 'module_path!()', 'dbg!(1)',
                           'let _: Option<std::panic::Location> = None',
                           'std::panic::Location::caller().line()'):
            for in_mutant in (False, True):
                with self.subTest(expression=expression, in_mutant=in_mutant):
                    source = 'fn f() { let ignored = 1; ' + ('' if in_mutant else expression + ';') + ' }'
                    replacement = 'let ignored = 2; ' + (expression + ';' if in_mutant else '')
                    muts = one_mutant('m', 'src/lib.rs', 'let ignored = 1;', replacement)
                    units, static = mutgate.plan_schemata(muts, {'src/lib.rs': source})
                    self.assertEqual(units, {})
                    self.assertEqual(static['m'], 'source-location')

    def test_plain_panics_unwraps_and_safe_macros_do_not_demote(self):
        for expression in ('panic!("x")', 'assert!(true)', 'Some(1).unwrap()', 'Some(1).expect("x")',
                           'vec![1]', 'format!("{}", 1)', 'write!(w, "x")', 'debug_assert!(true)',
                           'matches!(1, 1)', 'std::assert_eq!(1, 1)', 'serde_json::json!({"x": 1})'):
            for in_mutant in (False, True):
                with self.subTest(expression=expression, in_mutant=in_mutant):
                    source = 'fn f() { let ignored = 1; ' + ('' if in_mutant else expression + ';') + ' }'
                    replacement = 'let ignored = 2; ' + (expression + ';' if in_mutant else '')
                    units, static = mutgate.plan_schemata(one_mutant('m', 'src/lib.rs', 'let ignored = 1;', replacement),
                                                         {'src/lib.rs': source})
                    self.assertEqual(static, {})
                    self.assertEqual(len(units), 1)

    def test_unknown_macro_is_inspected_before_demotion(self):
        for definition, expected in (('macro_rules! custom { () => { vec![1] }; }', False),
                                     ('macro_rules! custom { () => { line!() }; }', True),
                                     ('macro_rules! custom { () => { helper!() }; }\n'
                                      'macro_rules! helper { () => { Location::caller() }; }', True),
                                     ('macro_rules! custom { () => { external::wrap!() }; }', True),
                                     ('', True)):
            with self.subTest(definition=definition):
                source = 'fn f() { let ignored = 1; custom!(); }'
                units, static = mutgate.plan_schemata(one_mutant('m', 'src/lib.rs', 'let ignored = 1;', 'let ignored = 2;'),
                                                     {'src/lib.rs': source, 'src/macros.rs': definition})
                self.assertEqual(bool(static), expected)
                self.assertEqual(bool(units), not expected)

    def test_macro_std_name_shadowing_and_external_qualifiers(self):
        sources = {'src/lib.rs': 'macro_rules! vec { () => { line!() }; }\nfn f() { let n = 1; vec!(); }'}
        muts = one_mutant('m', 'src/lib.rs', 'let n = 1;', 'let n = 2;')
        self.assertIn('m', mutgate.plan_schemata(muts, sources)[1])
        sources['src/lib.rs'] = sources['src/lib.rs'].replace('vec!();', 'std::vec!();')
        self.assertEqual(mutgate.plan_schemata(muts, sources)[1], {})
        sources['src/lib.rs'] = sources['src/lib.rs'].replace('std::vec!();', 'external::vec!();')
        self.assertEqual(mutgate.plan_schemata(muts, sources)[1], {'m': 'unaudited-macro: external::vec'})

    def test_new_nested_macro_observer_and_safe_macro_arguments_demote(self):
        for replacement in ('macro_rules! lit { () => { line!() }; } lit!();', 'format!("{}", line!());'):
            source = 'fn f() { let ignored = 1; }'
            units, static = mutgate.plan_schemata(one_mutant('m', 'src/lib.rs', 'let ignored = 1;', replacement),
                                                 {'src/lib.rs': source})
            self.assertEqual(units, {})
            self.assertEqual(static['m'], 'source-location')

    def test_observer_text_in_strings_comments_does_not_demote(self):
        source = 'fn f() { let n = 1; let text = "line!() Location::caller()"; /* wrapped!() */ }'
        self.assertEqual(mutgate.plan_schemata(one_mutant('m', 'src/lib.rs', 'let n = 1;', 'let n = 2;'),
                                              {'src/lib.rs': source})[1], {})

    def test_track_caller_on_nested_callee_demotes(self):
        source = 'fn f() { let n = 1; #[track_caller] fn helper() {} helper(); }'
        self.assertEqual(mutgate.plan_schemata(one_mutant('m', 'src/lib.rs', 'let n = 1;', 'let n = 2;'),
                                              {'src/lib.rs': source})[1], {'m': 'source-location'})

    def test_negated_conditions_are_not_macro_invocations(self):
        source = 'fn f() { let n = 1; if !(n == 1) {} while ![true].contains(&true) {} }'
        self.assertEqual(mutgate.plan_schemata(one_mutant('m', 'src/lib.rs', 'let n = 1;', 'let n = 2;'),
                                              {'src/lib.rs': source})[1], {})

    def test_track_caller_attribute_demotes(self):
        source = '#[track_caller]\nfn f() -> u32 { 1 }'
        units, static = mutgate.plan_schemata(one_mutant('m', 'src/lib.rs', '{ 1 }', '{ 2 }'),
                                             {'src/lib.rs': source})
        self.assertEqual(units, {})
        self.assertEqual(static['m'], 'source-location')

    def test_scope_file_includes_other_functions_in_changed_file(self):
        source = 'fn f() -> u32 { 1 }\nfn g() -> u32 { 2 }\n'
        muts = one_mutant('m', 'src/lib.rs', '{ 1 }', '{ 3 }')
        with patch.object(mutgate, 'changed_lines', return_value={'src/lib.rs': {2}}):
            self.assertEqual(mutgate.select_scoped(muts, {'src/lib.rs': source}, 'main', 'file'), ['m'])

    def test_scope_stale_anchor_is_not_silently_omitted(self):
        muts = one_mutant('m', 'src/gone.rs', '{ 1 }', '{ 2 }')
        with patch.object(mutgate, 'changed_lines', return_value={}):
            self.assertEqual(mutgate.select_scoped(muts, {}, 'main', 'line'), ['m'])


class ConfirmationTests(unittest.TestCase):
    def test_panic_header_is_metadata_but_payload_locations_are_retained(self):
        log = ("thread 't' panicked at /scratch/src/lib.rs:91:3:\n"
               'assertion failed: wrong value\n')
        self.assertEqual(mutgate.failure_locations(log), [])
        self.assertEqual(mutgate.failure_locations(log.replace("'t' panicked", "'t' (1234) panicked")), [])
        self.assertEqual(mutgate.failure_locations(log + 'observed /scratch/src/lib.rs:5:2\n'),
                         ['/scratch/src/lib.rs:5:2'])
        old = "thread 't' panicked at 'observed src/lib.rs:5:2', src/lib.rs:91:3\n"
        self.assertEqual(mutgate.failure_locations(old), ['src/lib.rs:5:2'])

    def test_run_test_scans_full_payload_before_tail_truncation(self):
        output = ('running 1 test\n'
                  "thread 't' panicked at src/lib.rs:91:3:\nobserved src/lib.rs:5:2\n" + 'x' * 2000 + '\n'
                  'test result: FAILED. 0 passed; 1 failed; 0 ignored;\n')
        with patch.object(mutgate.subprocess, 'run') as run:
            run.return_value.stdout = output; run.return_value.stderr = ''; run.return_value.returncode = 101
            result = mutgate.run_test('exe', 't', 'm', 1, {})
        self.assertTrue(result['killed'])
        self.assertNotIn('src/lib.rs:5:2', result['log_tail'])
        self.assertEqual(result['failure_locations'], ['src/lib.rs:5:2'])

    def test_only_killing_test_and_locations_in_own_edited_files_confirm(self):
        m = one_mutant('m', 'src/lib.rs', 'a', 'b')['m']; m['tests'] = ['kill', 'pass']
        runs = [{'killed': True, 'failure_locations': ['src/other.rs:2:1']},
                {'killed': False, 'failure_locations': []}]
        observation = {'runs': runs}
        self.assertEqual(mutgate.confirmation_reasons(m, observation, {'kill': False, 'pass': True}), [])
        runs[0]['failure_locations'] = ['/scratch/src/lib.rs:2:1']
        self.assertEqual(mutgate.confirmation_reasons(m, observation, {}),
                         ['failure-payload-location: /scratch/src/lib.rs:2:1'])
        runs[0]['failure_locations'] = ['src/notlib.rs:2:1']
        self.assertEqual(mutgate.confirmation_reasons(m, observation, {}), [])
        self.assertEqual(mutgate.confirmation_reasons(m, observation, {'kill': True}),
                         ['location-sensitive-test: kill'])

    def test_test_api_scan_includes_helpers_constants_and_should_panic(self):
        for source in ('#[test] fn t() { line!(); }', '#[test] fn t() { column!(); }',
                       '#[test] fn t() { let _: Option<Location> = None; }',
                       'const AFTER: u32 = line!(); #[test] fn t() { use_value(AFTER); }',
                       'fn helper() { std::panic::Location::caller(); } #[test] fn t() { helper(); }',
                       '#[test] #[should_panic(expected="src/lib.rs:")] fn t() { panic!("x"); }'):
            with self.subTest(source=source):
                self.assertEqual(mutgate.location_sensitive_tests(['m::t', 'calm'], {'src/lib.rs': source}),
                                 {'m::t': True, 'calm': False})

    def test_unrelated_observers_and_literals_do_not_make_a_test_sensitive(self):
        source = ('fn unused() { line!(); } const UNUSED: u32 = line!();\n'
                  '#[test] fn t() { let text = "Location line!() src/lib.rs:"; /* line!() */ assert!(true); }')
        self.assertEqual(mutgate.location_sensitive_tests(['t'], {'src/lib.rs': source}), {'t': False})


class RegistryIntentTests(unittest.TestCase):
    def test_r4_04_preserves_a_distinct_decoding_obligation(self):
        population = mutgate.load_population([LANE])
        mid = 'R4-04-non-utf8-declines'
        # The original registry silently duplicated unreadable-file coverage.
        # Preserve a distinct obligation, backed by the existing LE/BE fixture
        # and its non-declaration/other-encoding negative cases.
        self.assertNotEqual(population[mid]['edits'],
                            population['R4-11-unread-file-declines']['edits'])
        self.assertIn('js_paths_r3_test::native_decoding_keeps_ambient_in_both_grammars',
                      population[mid]['tests'])
        revisions = json.loads(LANE.read_text()).get('intent_revisions', {})
        self.assertIn(mid, revisions)
        self.assertEqual(revisions[mid]['disposition'], 'redefined')
        self.assertTrue(revisions[mid]['reason'])


if __name__ == '__main__':
    unittest.main()


class TestExecutionSerialization(unittest.TestCase):
    """C2-W1: concurrent workers must never execute two tests at once (shared runtime state)."""

    def test_run_test_executions_never_overlap(self):
        import threading as th, time as tm, unittest.mock as um, subprocess as sp
        active = {'now': 0, 'max': 0}
        guard = th.Lock()

        def fake_run(*a, **k):
            with guard:
                active['now'] += 1
                active['max'] = max(active['max'], active['now'])
            tm.sleep(0.05)
            with guard:
                active['now'] -= 1
            return sp.CompletedProcess(a[0], 0, 'running 1 test\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n', '')

        with um.patch.object(mutgate.subprocess, 'run', side_effect=fake_run):
            threads = [th.Thread(target=mutgate.run_test, args=('exe', 't', None, 5, {})) for _ in range(6)]
            for x in threads: x.start()
            for x in threads: x.join()
        self.assertEqual(active['max'], 1)

