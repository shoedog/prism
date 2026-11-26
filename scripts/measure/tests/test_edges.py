from collections import Counter
import copy
import random

from .comparator_fixtures import ComparatorFixture, edge, lines


class EdgesTests(ComparatorFixture):
    def test_changes_rebuild_both_input_multisets(self):
        from measure.edges import residual_rows

        rng = random.Random(731)
        attributes = [b"a", b"b", b"c", b"d"]
        for index in range(200):
            arms = [Counter(rng.choices(attributes, k=rng.randrange(16))) for _ in range(2)]
            cancelled, changes = residual_rows(*arms)
            for i, expected in enumerate(arms):
                recovered = cancelled.copy()
                for _, base, head in changes:
                    recovered.update((base, head)[i])
                self.assertEqual(recovered, expected, index)

    def test_surrogate_path_refuses_before_sorting(self):
        bad = edge()
        bad["from"]["path"]["fields"] = ["\ud800"]
        self.diff([bad], [bad], expected=2, diagnostic="edge input")
        self.assertFalse((self.out / "certificate.json").exists())
        self.diff([edge()], [edge()])

    def test_wire_schema_refusals_have_equal_dump_controls(self):
        original = edge()
        defects = [
            ("from", None), ("confidence", "unknown"), ("doubt", "exact-with-doubt"),
            ("kill_line", False), ("kill_line", 0), ("confidence", None), ("doubt", []),
        ]
        for name, value in defects:
            with self.subTest(name=name, value=value):
                bad = dict(original, **{name: value})
                self.diff([bad], [bad], expected=2, diagnostic="edge input")
                self.diff([original], [original])
                self.next_output()
        for name, value in [
                ("file", 1), ("line", True), ("line", 0), ("access", "other"), ("path", {}),
                ("path", {"base": 1, "fields": []}), ("path", {"base": "x", "fields": [1]}),
                ("path", {"base": "x", "fields": "a"}), ("start_byte", -1),
                ("start_byte", True), ("start_byte", 156), ("end_byte", "155")]:
            with self.subTest(name=name, value=value):
                bad = copy.deepcopy(original)
                bad["to"][name] = value
                self.diff([bad], [bad], expected=2, diagnostic="edge input")
                self.diff([original], [original])
                self.next_output()
        for bad in ([], ""):
            self.diff([dict(original, confidence="nameonly", doubt=bad)], [original],
                      expected=2, diagnostic="edge input")
        self.diff([dict(original, confidence="nameonly")], [dict(original, confidence="nameonly")])

    def test_json_transport_refusals_have_matching_controls(self):
        good = lines([edge()])
        for bad in (b"\xef\xbb\xbf" + good, b"[]\n", b'{"a":1,"a":2}\n',
                    b"\xff\n", good[:-2] + b',"extra":NaN}\n'):
            with self.subTest(bad=bad[:15]):
                self.diff(bad, good, expected=2, diagnostic="edge input")
                self.diff(good, good.rstrip(b"\n") + b"\r\n")
                self.next_output()
