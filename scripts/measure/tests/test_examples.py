from collections import Counter
import gzip
import importlib.util
import json

from .comparator_fixtures import ComparatorFixture, edge, read_rows
from .helpers import MEASURE

FIXTURE = MEASURE / "fixtures" / "repro-4b"
CLASSIFIER = MEASURE / "examples" / "classify_783_4b.py"


def load_file(path):
    spec = importlib.util.spec_from_file_location(path.stem, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ExampleTests(ComparatorFixture):
    def test_783_classifier_attributes_kill_alias_and_alias_d3_and_refuses_untagged_move(self):
        self.classifier = CLASSIFIER
        for doubt, mechanism in (("alias_unstable", "kill:alias"), ("sameline", "alias-d3")):
            self.arm("base", [edge()])
            self.arm("head", [edge(label=doubt)])
            self.compare(expected=1, diagnostic="unattributed")
            self.next_output()
            trace = f"rd-census\t{mechanism}\ta.ts\tg\tx@5:132\tx@6:154\n".encode()
            self.arm("head", [edge(label=doubt)], trace=trace)
            self.compare()
            self.assertEqual(self.changes()[0]["class"], mechanism)
            self.next_output()

    def test_r4b_capture_matches_independent_truth_siblings_and_five_controls(self):
        self.classifier = CLASSIFIER
        self.key = "R4B"
        source = {path.relative_to(FIXTURE / "repo").as_posix(): path.read_bytes()
                  for path in (FIXTURE / "repo").rglob("*") if path.is_file()}
        self.assertEqual(len(source), 115)
        raw = FIXTURE / "capture"
        base = (raw / "base.edges.jsonl.gz").read_bytes()
        head = (raw / "head.edges.jsonl.gz").read_bytes()
        trace = gzip.decompress((raw / "head.edges.stderr.gz").read_bytes())
        self.arm("base", transport=base, source=source, version="4f9e3a3eb8d2")
        self.arm("head", transport=head, source=source, version="1f2ad1cc49e2",
                 trace=trace, trace_env={"PRISM_RD_CENSUS_TRACE": "1"})
        registered = json.loads((MEASURE / "corpora.json").read_bytes())["corpora"]["R4B"]
        self.assertEqual(json.loads(self.manifest.read_bytes())["corpora"]["R4B"]["tree_sha256"],
                         registered["tree_sha256"])
        self.compare()
        cert = self.cert()
        self.assertEqual((cert["base_edges"], cert["head_edges"], cert["changed_keys"],
                          cert["attributed"], cert["unattributed"]), (950, 950, 78, 78, 0))
        self.assertEqual(cert["records"], dict(attached=78, stray=0, ambiguous=0, unmatched=0))
        self.assertEqual({k: v["attached"] for k, v in cert["trace_by_tag"].items() if v["attached"]},
                         {"kill:alias": 23, "alias-d3": 55})
        self.assertEqual(cert["by_class"], {"kill:alias": {"keys": 23, "occurrences": 23},
                                           "alias-d3": {"keys": 55, "occurrences": 55}})
        changes = self.changes()
        arm_rows = [[json.loads(line) for line in gzip.decompress(data).splitlines()]
                    for data in (base, head)]

        def projected(endpoint, wire):
            path = wire["path"] if "path" in wire else wire
            return {key: part for key, part in dict(base=path["base"], fields=path["fields"],
                                                   line=wire["line"]).items()} == endpoint

        def matches(truth, key):
            return all(key[end]["file"] == truth["file"] and projected(truth[end], key[end])
                       for end in ("from", "to"))

        independent = read_rows(FIXTURE / "truth.jsonl")
        siblings = read_rows(FIXTURE / "truth-extra.jsonl")
        self.assertEqual(Counter(r["class"] for r in independent),
                         {"kill:alias": 22, "alias-d3": 14, "unchanged": 103})
        self.assertEqual(sum(r["base_inferred"] for r in independent), 14)
        self.assertEqual(Counter(r["class"] for r in siblings), {"kill:alias": 1, "alias-d3": 41})
        for truth in independent + siblings:
            with self.subTest(file=truth["file"], edge=truth.get("edge")):
                for side, dump in zip(("base", "head"), arm_rows):
                    labels = [item["confidence"] + ("/" + item["doubt"] if item["doubt"] else "")
                              for item in dump if matches(truth, item)]
                    self.assertIn(truth[side], labels)
                attributed = [row["class"] for row in changes if matches(truth, row["key"])]
                self.assertEqual(attributed, [] if truth["class"] == "unchanged" else [truth["class"]])

        controls = load_file(FIXTURE / "controls.py").controls(arm_rows[0], arm_rows[1], trace)
        expected = {"c1": (1, 0, 1), "c2": (0, 0, 1), "c3": (0, 0, 2),
                    "c4": (1, 0, 0), "c5": (0, 1, 1)}
        self.assertEqual(set(controls), set(expected))
        for name, edited in controls.items():
            with self.subTest(control=name):
                self.next_output()
                self.arm("head", transport=head, source=source, version="1f2ad1cc49e2",
                         trace=edited, trace_env={"PRISM_RD_CENSUS_TRACE": "1"})
                self.compare(expected=1)
                cert = self.cert()
                self.assertEqual((cert["records"]["stray"], cert["records"]["unmatched"],
                                  cert["unattributed"]), expected[name])
                self.assertEqual(cert["records"]["ambiguous"], 0)

    def test_historical_trace_trials_stop_per_corpus_and_cannot_restore_identity(self):
        lane = load_file(CLASSIFIER)
        heads = ("1f2ad1cc", "32ad7c09b", "3510ed4f2")
        expected = {"X": b"X\n", "EX": b"EX\n", "T": b"T\n"}
        matches = dict(zip(expected, heads))
        trials = []

        def measure(corpus, head):
            trials.append((corpus, head))
            return ("2" * 64, expected[corpus] if matches.get(corpus) == head else b"miss\n")

        primary = dict.fromkeys(expected, True)
        result = lane.historical_trials(expected, primary, True, measure)
        self.assertTrue(result["pass"])
        self.assertEqual(trials, [(corpus, head) for corpus, match in matches.items()
                                  for head in heads[:heads.index(match) + 1]])
        self.assertEqual({key: values[-1]["head"] for key, values in result["trials"].items()}, matches)
        for corpus, values in result["trials"].items():
            self.assertTrue(values[0]["identity"])
            self.assertTrue(all(item["identity"] is None for item in values[1:]))
            self.assertEqual(values[-1]["sha256"], "2" * 64)
            self.assertEqual(values[-1]["trace_line"], expected[corpus])
        self.assertFalse(lane.historical_trials(expected, {**primary, "X": False}, True, measure)["pass"])
        self.assertFalse(lane.historical_trials(expected, primary, False, measure)["pass"])
        matches.pop("T")
        self.assertFalse(lane.historical_trials(expected, primary, True, measure)["pass"])

    def test_historical_mechanisms_are_head_only_deduplicated_and_in_frozen_order(self):
        self.key = "X"
        self.classifier = CLASSIFIER
        record = b"rd-census\tcapture-body\ta.ts\tg\tx@5:132\tx@6:154\n"
        second = record.replace(b"capture-body", b"d2c")
        extra = record.replace(b"capture-body", b"aaaa")
        self.arm("base", [edge()], trace=record.replace(b"capture-body", b"base-only"))
        self.arm("head", [edge()], trace=record + second + record + extra)
        stdout = self.compare()
        self.assertIn("trace mechanism lines: {'d2c': 1, 'capture-body': 1, 'aaaa': 1}\n", stdout)
        self.assertEqual(self.cert()["unjoined"], 5)

    def test_783_refuses_malformed_recognized_trace_with_valid_control(self):
        self.classifier = CLASSIFIER
        self.arm("base")
        good = b"rd-census\td2c\ta.ts\tg\tx@5:132\tx@6:154\n"
        for bad in (good.rsplit(b"\t", 1)[0], good.replace(b"@5:", b"@?:")):
            self.arm("head", trace=bad)
            self.compare(expected=2, diagnostic="trace input")
            self.arm("head", trace=good)
            self.compare()
            self.next_output()
