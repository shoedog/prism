#!/usr/bin/env python3
"""Unit tests for the mutgate rewriter (schema planning, not execution).

Run with:
    python3 -m unittest scripts/mutgate/test_mutgate.py

These tests exercise code_mask, fn_bodies, plan_schemata and render directly
on small synthetic Rust snippets. They do not touch cargo, the scratch tree,
or any lane registry file, so they run in well under a second.
"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import mutgate  # noqa: E402


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

    def test_macro_rules_body_nested_in_fn_uses_enclosing_fn_unit(self):
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


if __name__ == '__main__':
    unittest.main()
