from .comparator_fixtures import ComparatorFixture


class DiffStatsTests(ComparatorFixture):
    def stats(self, base, head, *args, call=None, **options):
        a, b = call or (None, None)
        self.arm("base", stats=base, call=a)
        self.arm("head", stats=head, call=b)
        return self.invoke("diff-stats", *args, **options)

    def allow(self, data):
        path = self.work / "allow.txt"
        path.write_bytes(data)
        return path

    def test_admitted_large_decimals_have_finite_exact_delta(self):
        self.stats(b'{"a":-1e308}', b'{"a":1e308}', diagnostic="(+2E+308)")
        stdout = self.stats(b'{"a":1e308}', b'{"a":1e308}')
        self.assertIn("changed=0", stdout)

    def test_distinct_decimal_and_integer_counter_values_do_not_round_to_parity(self):
        allow = self.allow(b"")
        self.stats(b'{"a":9007199254740992}', b'{"a":9007199254740993.0}',
                   "--allow", allow, expected=1, diagnostic="(+1.0)")
        self.stats(b'{"a":9007199254740992}', b'{"a":9007199254740992.0}', "--allow", allow)

    def test_no_allow_list_is_descriptive_and_zero_forbidden(self):
        from measure.corpora import load_manifest
        from measure.diff_stats import compare_stats

        base = self.arm("base", stats={"x": 1})
        head = self.arm("head", stats={"x": 2})
        result = compare_stats(base, head, which="dfg", entries=load_manifest(self.manifest))
        self.assertEqual((len(result["dfg"]["changed"]), result["dfg"]["forbidden"]), (1, []))
        self.invoke("diff-stats", diagnostic="x: 1 -> 2 (+1)")
        self.invoke("diff-stats", "--allow", self.allow(b""), expected=1,
                    diagnostic="forbidden counter delta: dfg x")

    def test_nested_and_escaped_literal_paths_have_distinct_allow_rules(self):
        base = {"dfg_loop_plan": {"overlap.placement": 1, "overlap": {"placement": 7}, "a\\.b": 1}}
        head = {"dfg_loop_plan": {"overlap.placement": 2, "overlap": {"placement": 7}, "a\\.b": 2}}
        call = ({"dfg_labels": base}, {"dfg_labels": head})
        allow = self.allow(br"""dfg_loop_plan.overlap\.placement
dfg_labels.dfg_loop_plan.overlap\.placement
dfg_loop_plan.a\\\.b
dfg_labels.dfg_loop_plan.a\\\.b
""")
        self.stats(base, head, "--allow", allow, call=call,
                   diagnostic=r"dfg_labels.dfg_loop_plan.overlap\.placement: 1 -> 2 (+1)")
        self.stats(base, head, "--allow", self.allow(b"dfg_loop_plan.overlap.placement\n"),
                   call=call, expected=1)

    def test_counter_presence_is_preserved_and_prefix_needs_component_boundary(self):
        self.stats({"x": 0, "xy": {"a": 1}}, {"x": {"a": 0}, "xy": {"a": 2}},
                   "--allow", self.allow(b"x.*\n"), expected=1, diagnostic="x: 0 -> missing (+0)")
        self.stats({"x": {"a": 0}}, {"x": {"a": 0, "b": 1}}, "--allow", self.allow(b"x.*\n"))

    def test_counter_input_and_allow_refusals_have_controls(self):
        for bad in (b'[]', b'{"a":NaN}', b'{"a":1e1001}', b'{"a":1,"a":2}',
                    b'{"\\ud800":1}', b'{"a":' + b'1' * 1001 + b'}'):
            self.stats(bad, {"a": 1}, expected=2, diagnostic="counter input")
            self.stats({"a": 1}, {"a": 1})
        for bad in (b"\xff", b"a..b\n", b"a.\\q\n", b"a.\\u{020}\n", b"a*x\n",
                    b"\\u{D800}\n", b"\\eX\n", b"a.\\u{41}\n"):
            self.stats({}, {"a": 1}, "--allow", self.allow(bad), expected=2, diagnostic="allow list")
            self.stats({}, {"a": 1}, "--allow", self.allow(b"a # comment\n"))
        self.stats({}, {"": {"#*\t": 1}}, "--allow", self.allow(br"\e.\#\*\u{9} # comment"))

    def test_only_selected_counter_transport_is_decoded(self):
        self.stats({"x": 1}, {"x": 2}, "--which", "dfg", call=(b"[]", {}))
        self.stats({"x": 1}, {"x": 2}, "--which", "call", call=(b"[]", {}),
                   expected=2, diagnostic="counter input: call-stats.json")
        self.stats({"x": 1}, {"x": 2}, "--which", "call", call=({}, {}))

    def test_extreme_decimal_quantum_span_preserves_all_digits(self):
        from decimal import Decimal
        from measure.corpora import load_manifest
        from measure.diff_stats import compare_stats

        base = self.arm("base", stats=b'{"x":1e1000}')
        head = self.arm("head", stats=b'{"x":1e-1000}')
        result = compare_stats(base, head, which="dfg", entries=load_manifest(self.manifest))
        self.assertEqual(result["dfg"]["changed"][0]["delta"], Decimal("-" + "9" * 2000 + "e-1000"))

    def test_missing_allow_file_refuses_before_reporting_deltas(self):
        self.arm("base", stats={"x": 1})
        self.arm("head", stats={"x": 2})
        allow = self.work / "allow.txt"
        stdout = self.invoke("diff-stats", "--allow", allow, expected=2, diagnostic="allow list")
        self.assertEqual(stdout, "")
        allow.write_text("x")
        self.invoke("diff-stats", "--allow", allow)
