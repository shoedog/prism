import unittest
from textwrap import dedent
from unittest.mock import patch

from inventory import analyze


class InventoryTests(unittest.TestCase):
    def check(self, lang, source, edges, *, owner="f@1", raw=None, flag=None):
        source = dedent(source).strip().encode() + b"\n"
        result = analyze(source, lang, owner)
        self.assertEqual(result["status"], "ok", result)
        self.assertEqual(len(result["would_add"]), edges, result)
        if raw is not None:
            self.assertEqual(result["counts"]["raw_pairs"], raw, result)
        if flag:
            self.assertGreater(result["flags"].get(flag, 0), 0, result)
        for pair in result["would_add"]:
            write = source[slice(*pair["w"])]
            read = source[slice(*pair["r"])]
            self.assertIn(b"[", write)
            self.assertNotIn(b"[", read)
        return result

    def test_python_straight_line_and_utf8_spans(self):
        result = self.check("python", """
            def f(d, v):
                café = 1; d["é"] = v
                return (d)
        """, 1, raw=1)
        self.assertEqual([r["role"] for r in result["reads"] if r["path"] == ("d",)], ["return"])

    def test_two_stores_and_branch_join(self):
        self.check("javascript", """
            function f(d, v, c) {
                d.a[0] = v;
                if (c) {
                    d.a[1] = v;
                } else {
                    d.b = {};
                }
                sink(d);
            }
        """, 2, raw=2)

    def test_no_matching_branch_completion(self):
        self.check("python", """
            def f(d, v, c, b):
                if c:
                    d[0] = v
                    if b:
                        d = {}
                    else:
                        d = {}
                sink(d)
        """, 0, flag="replacement")

    def test_return_from_arm_prevents_join(self):
        self.check("typescript", """
            function f(d: number[], v: number, c: boolean) {
                if (c) {
                    d[0] = v;
                    return;
                }
                sink(d);
            }
        """, 0)

    def test_replacement_before_read(self):
        self.check("typescript", """
            function f(d: number[], v: number) {
                d[0] = v;
                d = [];
                sink(d);
            }
        """, 0, raw=1, flag="replacement")

    def test_python_loop_else_and_break(self):
        for jump, expected in [("continue", 0), ("break", 1)]:
            with self.subTest(jump=jump):
                self.check("python", f"""
                    def f(d, values):
                        for v in values:
                            d[0] = v
                            {jump}
                        else:
                            d = {{}}
                        sink(d)
                """, expected)

    def test_go_loop_normal_exit(self):
        self.check("go", """
            package p
            func f(d []int, n int) {
                for i := 0; i < n; i++ {
                    d[i] = n
                    continue
                }
                sink(d)
            }
        """, 1, owner="f@2")

    def test_check_before_store(self):
        self.check("python", """
            def f(d, v):
                while True:
                    if ready():
                        break
                    d[0] = v
                sink(d)
        """, 0, flag="reentry_required")

    def test_loop_jump_cannot_bypass_finally(self):
        self.check("python", """
            def f(d):
                while True:
                    d[0] = 1
                    try:
                        break
                    finally:
                        d = {}
                sink(d)
        """, 0, flag="completion_unproven")
        self.check("javascript", """
            function f(d) {
                while (true) {
                    d[0] = 1;
                    try { break; } finally { d = {}; }
                }
                sink(d);
            }
        """, 0, flag="completion_unproven")

    def test_unproven_eager_class_effect_at_read(self):
        self.check("javascript", """
            function f(d) {
                d[0] = 1;
                sink(class { [(d = {}, "field")]() {} }, d);
            }
        """, 0, flag="eager_effect_at_read")

    def test_read_before_assignment(self):
        self.check("python", """
            def f(d, v):
                d[0] = v
                d = take(d)
        """, 1, raw=1)

    def test_projection_is_not_read(self):
        self.check("javascript", """
            function f(d, v) {
                d[0] = v;
                sink(d[1]);
            }
        """, 0, raw=0)

    def test_fanin_is_all_or_nothing(self):
        for size, expected in [(8, 8), (9, 0)]:
            source = "def f(d, v):\n" + "".join(
                f"    d[{i}] = v\n" for i in range(size)
            ) + "    sink(d)\n"
            result = self.check("python", source, expected, raw=size)
            self.assertEqual(result["counts"]["reads_over_fanin"], size == 9)

    def test_go_map_and_slice(self):
        result = self.check("go", """
            package p
            func f(m map[string]int, s []int, v int) {
                m["key"] = v
                s[0] += v
                sink(m)
                sink(s)
            }
        """, 2, owner="f@2", raw=2)
        self.assertEqual([w["container_kind"] for w in result["writes"]], ["map", "slice"])

    def test_js_this_root(self):
        self.check("javascript", """
            class C {
                f(v) {
                    this.d[0] = v;
                    sink(this.d);
                }
            }
        """, 0, owner="f@2", flag="this_root")

    def test_ts_as_wrapper_and_tsx(self):
        self.check("typescript", """
            function f(d: number[], v: number) {
                d[0] = v;
                return d as number[];
            }
        """, 0, flag="ts_as_wrapper")
        self.check("tsx", """
            function f(d: number[], v: number) {
                d[0] = v;
                const markup = <a>{d}</a>;
                return d;
            }
        """, 1, raw=1)

    def test_same_line_use_collision_with_projection(self):
        self.check("python", """
            def f(d, v):
                d[0] = v
                sink(d[1], d)
        """, 0, flag="same_line_collision")

    def test_nested_child_detached_during_store(self):
        for target, edges in [("d[0][(d.clear(), 0)[1]]", 0), ("d[(d.clear(), 0)[1]]", 1)]:
            with self.subTest(target=target):
                self.check("python", f"""
                    def f(d, v):
                        {target} = v
                        sink(d)
                """, edges, flag="eager_effect_in_target" if not edges else None)

    def test_python_rhs_rebind_before_capture(self):
        self.check("python", """
            def f(d, v):
                d[0] = (d := {}) or v
                sink(d)
        """, 1)

    def test_js_capture_and_read_order(self):
        self.check("javascript", """
            function f(d, v) {
                d[(d = {}, 0)] = v;
                sink(d);
            }
        """, 0, flag="eager_effect_in_target")
        for args, edges in [("(d = {}), d", 0), ("d, (d = {})", 1)]:
            with self.subTest(args=args):
                self.check("javascript", f"""
                    function f(d, v) {{
                        d[0] = v;
                        sink({args});
                    }}
                """, edges)

    def test_completion_unknown_and_alternate(self):
        for other, edges in [("return", 0), ("pass", 1)]:
            self.check("python", f"""
                def f(d, v, c):
                    d[0] = v
                    if c:
                        try:
                            noop()
                        finally:
                            noop()
                    else:
                        {other}
                    sink(d)
            """, edges, flag="completion_unproven" if not edges else None)

    def test_block_shadowing_and_module_root(self):
        self.check("javascript", """
            const o = {};
            function f(d) {
                d[0] = 1;
                o[0] = 1;
                {
                    let d = [];
                    sink(d);
                }
                sink(o);
            }
        """, 0, owner="f@2", flag="module_root")

    def test_raw_cap_before_any_route(self):
        source = "def f(d):\n    d[0] = 1\n    sink(d)\n    sink(d)\n"
        with patch("flow.RAW_LIMIT", 2):
            self.check("python", source, 2, raw=2)
        with patch("flow.RAW_LIMIT", 1), patch("flow.Graph.reachable", side_effect=AssertionError):
            result = self.check("python", source, 0, raw=2)
        self.assertEqual(result["counts"]["raw_pair_cap"], 1)

    def test_owner_cap_at_256_and_257(self):
        for count, expected in [(256, 256), (257, 0)]:
            source = "def f(d):\n    d[0] = 1\n" + "    sink(d)\n" * count
            result = self.check("python", source, expected, raw=count)
            self.assertEqual(result["counts"]["owner_edge_cap"], int(count == 257))

    def test_indexed_child_replacement_depth(self):
        for target, expected in [("d[0][1]", 2), ("d[0]", 1)]:
            with self.subTest(target=target):
                self.check("javascript", f"""
                    function f(d, v) {{
                        d[0][(
                            {target} = v, 0)] = v;
                        sink(d);
                    }}
                """, expected, flag="eager_effect_in_target" if expected == 1 else None)

    def test_python_unpacking_is_not_whole(self):
        self.check("python", """
            def f(d, v):
                d[0] = v
                a, b = d
        """, 0, raw=0)

    def test_eval_binding_is_uncertain(self):
        self.check("javascript", """
            function f(d) {
                d[0] = 1;
                eval("d = []");
                sink(d);
            }
        """, 0, flag="dynamic_scope")

    def test_go_select_only_chooses_one_target(self):
        for target, expected in [("d", 0), ("scratch", 1)]:
            self.check("go", f"""
                package p
                func f(d []int, c chan []int, scratch []int) {{
                    d[0] = 1
                    select {{
                    case {target} = <-c:
                        break
                    }}
                    sink(d)
                }}
            """, expected, owner="f@2")

    def test_go_unsupported_fallthrough_and_goto(self):
        for transfer in ["""
                switch {
                case true:
                    fallthrough
                default:
                }
            """, """
                goto out
                d = nil
            out:
            """]:
            code = "package p\nfunc f(d []int) {\n    d[0] = 1\n"
            code += dedent(transfer).strip() + "\n    sink(d)\n}"
            self.check("go", code, 0, owner="f@2", flag="completion_unproven")

    def test_js_loop_update_break_and_reentry(self):
        for jump, expected in [("break", 1), ("continue", 0)]:
            self.check("javascript", f"""
                function f(d, n) {{
                    for (; n; d = []) {{
                        d[0] = 1;
                        {jump};
                    }}
                    sink(d);
                }}
            """, expected)
        self.check("javascript", """
            function f(d, n) {
                while (true) {
                    if (n) break;
                    d[0] = 1;
                }
                sink(d);
            }
        """, 0, flag="reentry_required")

    def test_collision_with_other_callable_on_line(self):
        self.check("javascript", """
            function f(d) {
                d[0] = 1;
                const g = () => take(d); sink(d);
            }
        """, 0, flag="same_line_collision")

    def test_js_numeric_literal_loop_has_no_false_exit(self):
        self.check("javascript", """
            function f(d) {
                while (1) {
                    d[0] = 1;
                }
                sink(d);
            }
        """, 0)

    def test_literal_short_circuit_prevents_entering_arm(self):
        self.check("python", """
            def f(d, test):
                if False and test:
                    d[0] = 1
                sink(d)
        """, 0)
        self.check("javascript", """
            function f(d, test) {
                if (false && test) {
                    d[0] = 1;
                }
                sink(d);
            }
        """, 0)

    def test_js_excluded_callee_uses_static_spelling(self):
        self.check("javascript", """
            function f(d, value) {
                d[0] = value;
                Object . is(d, value);
            }
        """, 0, raw=0)

    def test_annex_b_initializer_has_uncertain_exit(self):
        self.check("javascript", """
            function f(d) {
                d[0] = 1;
                for (var x = (d = {}) in {}) {}
                sink(d);
            }
        """, 0, flag="completion_unproven")

    def test_counts_only_agrees_after_fanin(self):
        source = b"def f(d, e):\n" + b"".join(
            f"    d[{i}] = 1\n".encode() for i in range(9)
        ) + b"    e[0] = 2\n    sink(d)\n    sink(e)\n"
        summary = analyze(source, "python", "f@1", detailed=False)
        detail = analyze(source, "python", "f@1")
        self.assertEqual(summary["counts"]["would_add"], 1)
        self.assertEqual(summary["counts"]["reads_over_fanin"], 1)
        self.assertEqual(summary, {key: detail[key] for key in summary})
        self.assertNotIn("pairs", summary)

    def test_ts_compound_store_and_declarator(self):
        result = self.check("typescript", """
            function f(d: Record<string, number>, k: string, v: number) {
                d[k] += v;
                const copy = (d);
                return copy;
            }
        """, 1, raw=1)
        self.assertEqual(result["writes"][0]["role"], "aug_write")
        self.assertEqual(result["would_add"][0]["role"], "assign_rhs")

    def test_go_named_and_direct_conversions_are_excluded(self):
        self.check("go", """
            package p
            type named []int
            func f(d []int) {
                d[0] = 1
                sink(named(d))
                sink([]int(d))
                defer sink(d)
                go sink(d)
            }
        """, 0, owner="f@3", raw=0)


    def test_python_comprehension_is_an_ordinary_expression(self):
        self.check("python", """
            def f(d, v, xs):
                d[0] = v
                ys = [x for x in xs]
                return d
        """, 1)

    def test_python_walrus_in_comprehension_stays_unproven(self):
        self.check("python", """
            def f(d, v, xs):
                d[0] = v
                ys = [d := x for x in xs]
                return d
        """, 0, flag="completion_unproven")

    def test_write_after_an_unsupported_compound_is_not_refused_for_that(self):
        self.check("python", """
            def f(d, v, xs):
                try:
                    pass
                finally:
                    pass
                d[0] = v
                return d
        """, 1)

    def test_write_and_read_inside_one_unsupported_arm(self):
        self.check("python", """
            def f(d, v):
                with v:
                    d[0] = v
                    return d
        """, 1)


if __name__ == "__main__":
    unittest.main()
