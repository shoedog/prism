import copy
import gzip
import hashlib
import json
import tracemalloc

from .comparator_fixtures import CLASSIFIER, TRACE_CLASSIFIER, ComparatorFixture, edge, lines


class DiffEdgesTests(ComparatorFixture):
    def test_identical_dumps_pass_with_zero_changes(self):
        stdout = self.diff([edge()], [edge()], "--list")
        self.assertEqual(self.cert()["changed_keys"], 0)
        self.assertEqual(self.changes(), [])
        self.assertEqual(stdout.splitlines()[0], "edges base=1 head=1 added=0 removed=0 label-moved keys=0")

    def test_legacy_move_keys_and_occurrences_have_distinct_totals(self):
        stdout = self.diff([edge()] * 2, [edge(label="sameline")] * 2)
        self.assertTrue(stdout.startswith("edges base=2 head=2 added=0 removed=0 label-moved keys=1\n"
                                          "occurrences: added=0 removed=0 label_moved=2 ambiguous=0\n"))

    def test_ambiguous_occurrences_do_not_count_legacy_key_surplus(self):
        stdout = self.diff([edge()], [edge(label="sameline"), edge(label="killed")])
        self.assertTrue(stdout.startswith("edges base=1 head=2 added=1 removed=0 label-moved keys=1\n"
                                          "occurrences: added=0 removed=0 label_moved=0 ambiguous=2\n"))

    def test_base_source_pin_precedes_attribution(self):
        base = self.arm("base", [edge()])
        self.arm("head")
        self.classifier.write_text(CLASSIFIER + """
from pathlib import Path
def classify(change, context):
    return 'fixture' if (Path(context.base_receipt["corpus"]["root"]) / "a.ts").exists() else None
""")
        (base / "snapshot/a.ts").write_text("changed")
        self.compare(expected=2, diagnostic="source binding: base")
        self.assertFalse((self.out / "certificate.json").exists())
        (base / "snapshot/a.ts").write_text("export {};\n")
        self.compare()

    def test_missing_snapshot_refuses_only_when_source_is_requested(self):
        self.arm("base", [edge()])
        head = self.arm("head")
        (head / "snapshot/a.ts").unlink()
        (head / "snapshot").rmdir()
        self.compare()
        self.next_output()
        self.classifier.write_text(CLASSIFIER + "\ndef classify(change, context):\n"
                                   "    return 'fixture' if context.corpus_root.is_dir() else None\n")
        self.compare(expected=2, diagnostic="source binding: head")
        self.arm("head")
        self.compare()

    def test_source_access_rechecked_after_callback(self):
        self.arm("base", [edge()])
        self.arm("head")
        prefix = CLASSIFIER + "\ndef validate(bound):\n    source = bound.corpus_root / 'a.ts'\n"
        self.classifier.write_text(prefix + "    source.write_text('changed')\n    return []\n")
        self.compare(expected=2, diagnostic="source binding: head")
        self.arm("head")
        self.classifier.write_text(prefix + "    source.read_text()\n    return []\n")
        self.compare()

    def test_projection_records_reason_and_loss(self):
        self.arm("base", [edge()])
        self.arm("head", [edge()])
        self.compare("--key", "line", "--line-reason", "  ", expected=2, diagnostic="key mode")
        self.compare("--key", "line", "--line-reason", "historical byte-for-byte control")
        self.assertTrue(self.cert()["line_projection_loss"])
        self.assertEqual(self.cert()["line_reason"], "historical byte-for-byte control")
        for arm in ("base", "head"):
            native = edge()
            for side in ("from", "to"):
                for bound in ("start_byte", "end_byte"):
                    native[side].pop(bound)
            self.arm(arm, [native])
        self.next_output()
        self.compare("--key", "line")
        self.assertFalse(self.cert()["line_projection_loss"])

    def test_line_projection_does_not_waive_receipt_options(self):
        self.arm("base", [edge()])
        self.arm("head", [edge()], options={"bytes": False})
        args = ("--key", "line", "--line-reason", "projection")
        self.compare(*args, expected=2, diagnostic="run pairing")
        self.arm("head", [edge()])
        self.compare(*args)

    def test_counter_and_trace_admission_precede_classification(self):
        self.arm("base", [edge()])
        self.arm("head", trace=b"\xff", stats=b"[]")
        marker = self.work / "classified"
        self.classifier.write_text(TRACE_CLASSIFIER + f"""
def classify(change, context):
    from pathlib import Path
    Path({str(marker)!r}).touch()
    return "fixture"
""")
        self.compare(expected=2, diagnostic="counter input")
        self.assertFalse(marker.exists())
        self.arm("head", trace=b"\xff")
        self.compare(expected=2, diagnostic="trace input: head line 1")
        self.assertFalse(marker.exists())
        self.arm("head", trace=b"\r\nnull\r\n".lstrip())
        self.compare()
        self.assertTrue(marker.exists())

    def test_classifier_interface_and_callback_failures(self):
        self.arm("base", [edge()])
        self.arm("head")
        for bad in ("raise RuntimeError('load')", "CLASSES = []", "CLASSES = {}",
                    "CLASSES = ['x', 'x']", "CLASSES = ['x\\ty']", "CLASSES = ['']",
                    "def classify(change): return None", "parse_trace = 0", "validate = 0",
                    "STRAYS = None", "LEGACY_MECHANISM_ORDER = []",
                    "LEGACY_MECHANISM_ORDER = {'Z': ['a', 'a']}"):
            with self.subTest(bad=bad):
                self.classifier.write_text(CLASSIFIER + "\n" + bad if bad != "CLASSES = []"
                                           else bad)
                self.compare(expected=2, diagnostic="classifier interface")
                self.classifier.write_text(CLASSIFIER)
                self.compare()
                self.next_output()
        for expression in ("None", "'diagnostic'", "['']", "[1]", "1 / 0"):
            self.classifier.write_text(CLASSIFIER + f"def validate(bound): return {expression}\n")
            self.compare(expected=1, diagnostic="validation")
            self.next_output()
        self.classifier.write_text(CLASSIFIER + "def validate(bound): return []\n")
        self.compare()

    def test_trace_schema_refusals_have_positive_controls(self):
        self.arm("base")
        self.classifier.write_text(TRACE_CLASSIFIER)
        good = {"tag": "tag", "kind": "edge", "key": {"to": {"line": 6}}}
        node = {"kind": "node", "tag": "node", "key": {
            "file": "a.ts", "line": 6, "start_byte": 154, "end_byte": 155, "access": "use"}}
        legacy = {**good, "mechanism": "test", "legacy_key": ["a.ts", "x", 5, "x", 6]}
        invalid_endpoint = [{"line": False}, {"unknown": 6}, {"file": 1}, {"base": 1},
                            {"fields": "x"}, {"fields": [1]}, {"start_byte": -1},
                            {"start_byte": 155, "end_byte": 154}, []]
        bad_nodes = [{**node, "key": {**node["key"], key: value}}
                     for key, value in (("owner", 9), ("access", "write"), ("end_byte", 153))]
        bad_legacy = [{**legacy, "mechanism": ""}, {**legacy, "legacy_key": []},
                      {**legacy, "legacy_key": [1, "x", 5, "x", 6]},
                      {**legacy, "legacy_key": ["a.ts", "x", False, "x", 6]}]
        for bad in ([], {**good, "arm": "base"}, {**good, "tag": ""}, {**good, "kind": 4},
                    {**good, "key": []}, {**good, "key": {}}, {**good, "key": {"to": {}}},
                    {**good, "key": {"side": {"line": 6}}}, {**good, "extra": "\ud800"},
                    {**good, "kind": "node"}, {**good, "extra": float("nan")},
                    *({**good, "key": {"to": part}} for part in invalid_endpoint),
                    *bad_nodes, *bad_legacy):
            self.arm("head", [edge()], trace=lines([bad]))
            self.compare(expected=2, diagnostic="trace input")
            self.arm("head", [edge()], trace=lines([legacy, node]))
            self.compare()
            self.assertEqual(self.cert()["records"]["attached"], 1)
            self.next_output()

    def test_projected_trace_uses_surviving_fields_and_checks_original_bounds(self):
        self.classifier.write_text(TRACE_CLASSIFIER)
        self.arm("base")
        for key, expected in (
            ({"start_byte": 154}, "unmatched"),
            ({"line": 6, "start_byte": 999}, "unmatched"),
            ({"line": 6, "end_byte": 999}, "unmatched"),
            ({"line": 6, "start_byte": 154}, "attached"),
            ({"line": 6}, "attached"),
        ):
            record = {"tag": "tag", "kind": "edge", "key": {"to": key}}
            self.arm("head", [edge()], trace=lines([record]))
            self.compare("--key", "line", "--line-reason", "test")
            self.assertEqual(self.cert()["records"][expected], 1, key)
            self.next_output()

    def test_node_inventory_reaches_validation_with_zero_changed_edges(self):
        self.classifier.write_text(TRACE_CLASSIFIER + """
def validate(bound):
    records = list(bound.unjoined)
    nodes = list(bound.nodes("head", "a.ts", 154, 154, "use"))
    assert records == list(bound.records) and len(records) == 3
    assert [record["arm"] for record in records] == ["base", "head", "head"]
    assert records[1:] == nodes and list(bound.nodes("base", "a.ts", 154, 154, "use")) == []
    assert len(list(bound.unchanged_keys)) == 1
    return []
""")
        node = {"tag": "node", "kind": "node", "key": {
            "file": "a.ts", "line": 2, "start_byte": 154, "end_byte": 154, "access": "use"}}
        defining = copy.deepcopy(node)
        defining["key"]["access"] = "def"
        self.arm("base", [edge()], trace=lines([defining]))
        self.arm("head", [edge()], trace=lines([node, node]))
        self.compare()
        self.assertEqual(self.cert()["unjoined"], 3)

    def test_output_ownership_and_list_are_consistent(self):
        self.arm("base")
        self.arm("head", [edge()])
        self.out.mkdir()
        marker = self.out / "keep"
        marker.write_text("keep")
        self.compare(expected=2, diagnostic="output")
        self.assertEqual(marker.read_text(), "keep")
        marker.unlink()
        stdout = self.compare("--list")
        printed = [json.loads(row) for row in stdout.splitlines() if row.startswith("{")]
        self.assertEqual(printed, self.changes())

    def test_diff_edges_streams_large_gzip_within_memory_bound(self):
        self.large_dump(408954, 96 * 1024 * 1024)

    def large_dump(self, count, bound):
        file = self.work / "large.gz"
        template = lines([edge(line=987654321)])
        prefix, suffix = template.split(b"987654321")
        with gzip.open(file, "wb", compresslevel=1) as stream:
            for index in range(1, count + 1):
                stream.write(prefix + str(index).encode() + suffix)
        # Keep fixture construction out of the comparator memory measurement.
        payload = file.read_bytes()
        self.arm("base", transport=payload)
        self.arm("head", transport=payload)
        tracemalloc.start()
        try:
            self.compare()
            peak = tracemalloc.get_traced_memory()[1]
        finally:
            tracemalloc.stop()
        self.assertLess(peak, bound)
        self.assertEqual((self.cert()["base_edges"], self.cert()["head_edges"],
                          self.cert()["changed_keys"]), (count, count, 0))

    def test_optional_inventory_terminal_and_structured_path_policy(self):
        self.classifier.write_text(CLASSIFIER + """
from collections import Counter
from measure.trace import inventory_failures, parse_subscript as parse_trace
def validate(bound):
    failures = inventory_failures(bound, ("base", "head"))
    if failures:
        return failures
    def bag(arm):
        return Counter((r["recorded_path"]["base"], tuple(r["recorded_path"]["fields"]))
                       for r in bound.unjoined if r["arm"] == arm and r["kind"] == "node")
    return [] if bag("base") == bag("head") else ["spelling-leak"]
""")
        def write(arm, trace, mode="all"):
            self.arm(arm, trace=trace, trace_env={"PRISM_SUBSCRIPT_TRACE": mode})

        zero = b"subscript-inventory-end\tall\t0\t0\n"
        one = b"subscript-inventory-end\tall\t0\t1\n"
        prefix = b"subscript-site\ta.ts\tg\t2\t154\t154\tuse\t"
        structured = b'{"base":"o.x","fields":[]}\t{"base":"refused","fields":["a\\tb","a\\\\b","a\\"b"]}\n'
        node = prefix + structured
        write("base", zero)
        for trace, reason, mode in (
            (b"", "inventory-terminal-missing", "all"),
            (zero + zero, "inventory-terminal-count", "all"),
            (one + node, "inventory-terminal-order", "all"),
            (one, "inventory-count-mismatch", "all"),
            (zero, "inventory-mode", "1"),
            (b"subscript-inventory-end\tall\tzero\t0\n", "inventory-record", "all"),
            (prefix + b"o.x\t-\n" + one, "inventory-record", "all"),
            (prefix + b"o.x\t-\n", "inventory-terminal-missing", "all"),
        ):
            write("head", trace, mode)
            self.compare(expected=1, diagnostic=reason)
            self.next_output()
            write("head", zero)
            self.compare()
            self.next_output()
        write("base", node + one)
        write("head", prefix + structured.replace(b'"o.x","fields":[]', b'"o","fields":["x"]') + one)
        self.compare(expected=1, diagnostic="spelling-leak")
        self.assertEqual(self.cert()["changed_keys"], 0)
        self.next_output()
        write("head", node + one)
        self.compare()

    def test_validator_checks_cross_key_endpoint_in_attributed_ambiguous_bag(self):
        self.classifier.write_text(TRACE_CLASSIFIER + """
def validate(bound):
    for change in bound.changes:
        if change["kind"] != "ambiguous":
            continue
        if bound.key_mode != "bytes":
            return ["byte mode required"]
        for attr in change["head_bag"]:
            paths = []
            for end in ("from", "to"):
                span = change["key"][end]
                nodes = list(bound.nodes("head", span["file"], span["start_byte"],
                                        span["end_byte"], attr[end + "_access"]))
                if len(nodes) != 1:
                    return ["unique node required"]
                paths.append(nodes[0]["path"])
            if paths[0] != paths[1]:
                return ["cross-key"]
    return []
""")
        base, head = [edge(), edge(label="a")], [edge(label="b"), edge(label="c")]
        head[1]["to"]["access"] = "def"
        def node(start, role, path):
            return {"kind": "node", "tag": "node", "path": path, "key": {
                "file": "a.ts", "line": 99, "start_byte": start, "end_byte": start + 1,
                "access": role}}
        self.arm("base", base)
        nodes = [node(132, "def", ["x"]), node(154, "use", ["x"])]
        self.arm("head", head, trace=lines(nodes + [node(154, "def", ["y"])]))
        self.compare(expected=1, diagnostic="cross-key")
        self.assertEqual((self.cert()["ambiguous"], self.cert()["unattributed"]), (1, 0))
        self.next_output()
        self.arm("head", head, trace=lines(nodes))
        self.compare(expected=1, diagnostic="unique node required")
        self.next_output()
        self.arm("head", head, trace=lines(nodes + [node(154, "def", ["x"])] * 2))
        self.compare(expected=1, diagnostic="unique node required")
        self.next_output()
        self.arm("head", head, trace=lines(nodes + [node(154, "def", ["x"])]))
        self.compare("--key", "line", "--line-reason", "control", expected=1, diagnostic="byte mode")
        self.next_output()
        self.compare()

    def test_callback_views_are_reiterable_ordered_sequences(self):
        self.classifier.write_text(TRACE_CLASSIFIER + """
def validate(bound):
    assert len(bound.changes) == 1 and bound.changes[-1]["kind"] == "added"
    assert bound.changes[:] == list(bound.changes)
    assert len(bound.unchanged_keys) == 1 and bound.unchanged_keys[0]["key"]["to"]["line"] == 6
    assert len(bound.records) == 2 and bound.records[::-1] == list(reversed(list(bound.records)))
    assert [join["change_id"] for join in bound.joins] == [1, 1]
    assert list(bound.joins) == list(bound.joins)
    try:
        bound.records[2]
    except IndexError:
        return []
    return ["invalid sequence index was accepted"]
""")
        record = {"tag": "test", "kind": "edge", "key": {"to": {"line": 8}}}
        self.arm("base", [edge()])
        self.arm("head", [edge(), edge(line=8)], trace=lines([record, record]))
        self.compare()

    def test_erased_trace_or_contradiction_cannot_classify(self):
        self.classifier.write_text(TRACE_CLASSIFIER + """
def classify(change, context):
    return "fixture" if context.records(change) else None
""")
        trace = lambda key: lines([{"tag": "tag", "kind": "edge", "key": {"to": key}}])
        args = ("--key", "line", "--line-reason", "test")
        self.arm("base")
        for part in ({"start_byte": 154}, {"line": 6, "start_byte": 999},
                     {"line": 6, "end_byte": 999}):
            self.arm("head", [edge()], trace=trace(part))
            self.compare(*args, expected=1, diagnostic="unattributed")
            self.assertEqual(self.cert()["records"], dict(attached=0, stray=0, ambiguous=0, unmatched=1))
            self.next_output()
        self.arm("head", [edge()], trace=trace({"line": 6, "start_byte": 154}))
        self.compare(*args)
        self.next_output()
        self.arm("base", [edge()])
        self.arm("head", [edge()], trace=trace({"line": 6, "start_byte": 999}))
        self.compare(*args)
        self.assertEqual(self.cert()["records"]["unmatched"], 1)
        self.assertEqual(self.cert()["records"]["stray"], 0)

    def test_gzip_bad_crc_and_utf8_member_split_use_the_declared_transport(self):
        payload = lines([edge()])
        transport = gzip.compress(payload)
        bad = transport[:-8] + bytes([transport[-8] ^ 1]) + transport[-7:]
        self.arm("base", transport=transport)
        self.arm("head", transport=bad)
        self.compare(expected=2, diagnostic="edge input: head")
        wire = edge()
        wire["to"]["file"] = "file\u2028\u00e9.ts"
        raw = json.dumps(wire, ensure_ascii=False).encode()
        split = raw.index(b"\xc3") + 1
        self.arm("base", transport=gzip.compress(raw[:split]) + gzip.compress(raw[split:]))
        self.arm("head", [wire])
        self.compare()
        self.assertEqual(self.cert()["changed_keys"], 0)

    def test_certificate_digests_cover_exact_source_artifacts(self):
        self.arm("base", [edge()])
        self.arm("head", [edge(label="killed")])
        self.compare()
        certificate = self.cert()
        for arm in ("base", "head"):
            binding = certificate["inputs"][arm]
            self.assertEqual((self.out / binding["run"]).resolve(), self.work / arm)
            self.assertEqual(binding["receipt_sha256"], hashlib.sha256(
                (self.work / arm / "receipt.json").read_bytes()).hexdigest())
        self.assertEqual(certificate["classifier_sha256"],
                         hashlib.sha256(self.classifier.read_bytes()).hexdigest())
        self.assertEqual(certificate["changes_sha256"],
                         hashlib.sha256((self.out / "changes.jsonl").read_bytes()).hexdigest())

    def test_trace_bom_refuses_even_without_parser(self):
        self.arm("base")
        self.arm("head", trace=b"\xef\xbb\xbfplain stderr\n")
        self.compare(expected=2, diagnostic="trace input: head line 1")
        self.assertFalse((self.out / "certificate.json").exists())
        self.arm("head", trace=b"plain stderr\n")
        self.compare()

    def test_classifier_module_stays_registered_for_standard_library_callbacks(self):
        self.classifier.write_text("""
from __future__ import annotations
from dataclasses import dataclass
CLASSES = ["fixture"]
def classify(change, context):
    @dataclass
    class Attribution:
        name: str = "fixture"
    return Attribution().name
""")
        self.diff([], [edge()])
        self.assertEqual(self.changes()[0]["class"], "fixture")
