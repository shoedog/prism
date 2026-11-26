"""Keep the known-limit ledger and the self-check goldens in agreement."""

from collections import Counter
import copy
import json
import unittest

from measure.oracle_inputs import fixture_set

from .test_oracle import KIT, read_jsonl


DECIDED = {"CORRECT", "WRONG"}
FORMER_CLASSES = {"property-origin", "receiver-dispatch", "member-order"}
DETECTORS = {
    "annex-b": {"annexb": "reliable-detector"},
    "direct-eval": {"eval": "reliable-detector"},
    "catch-var": {"catch-var": "reliable-detector"},
    "host-mode": {"host-mode": "proxy"},
    "parameter-default": {"param-default-closure": "reliable-detector"},
    "for-let-init": {"for-let-init": "reliable-detector"},
    "for-in-of-head": {"for-in-of-head": "reliable-detector"},
    "module-resolution": {"ext-substitution": "reliable-detector", "package-main/json/d.ts": "proxy"},
    "html-comment": {"html-comment": "reliable-detector"},
    "computed-member-name": {"computed-name": "reliable-detector"},
    "static-initialization": {"static-init": "proxy"},
    "ambient-erasure": {"ambient-erased": "reliable-detector"},
}


class OracleKnownLimitsTests(unittest.TestCase):
    def test_grouped_case_admission_requires_one_program_per_claim(self):
        root = KIT / "selfcheck"
        case = root / "known-limit-for-let-init"
        raw = {p.relative_to(root).as_posix(): p.read_bytes()
               for p in case.rglob("*") if p.is_file()}
        self.assertEqual(len(fixture_set(root, raw, "kit")), 1)
        rows = read_jsonl(case / "input.jsonl")
        crossed = copy.deepcopy(rows[0])
        crossed["to"]["file"] = next(row["to"]["file"] for row in rows
                                     if row["to"]["file"] != crossed["from"]["file"])
        for path, data, reason in (
            ("repo/loose.ts", b"export {};", "known-limit program directory required"),
            ("input.jsonl", json.dumps(crossed).encode(), "known-limit row must use one program"),
        ):
            with self.subTest(path=path), self.assertRaisesRegex(ValueError, reason):
                fixture_set(root, dict(raw, **{f"{case.name}/{path}": data}), "kit")

    def test_registry_covers_every_case_and_decided_row(self):
        registry = json.loads((KIT / "known_limits.json").read_text())
        root = KIT / "selfcheck"
        cases = {path.name.removeprefix("known-limit-"): path
                 for path in root.glob("known-limit-*") if path.is_dir()}
        self.assertEqual(set(cases), set(registry["classes"]) | FORMER_CLASSES)
        self.assertTrue(FORMER_CLASSES.isdisjoint(registry["classes"]))
        self.assertEqual({slug: entry["prevalence"] for slug, entry in registry["classes"].items()},
                         DETECTORS)

        programs = Counter((entry["source_pass"], entry["id"]) for entry in registry["unrecovered"])
        members = Counter(entry["source_pass"] for entry in registry["unrecovered"])
        totals = Counter()
        for slug, case in cases.items():
            with self.subTest(known_limit=slug):
                entry = registry["classes"].get(slug)
                goldens = read_jsonl(case / "expected.jsonl")
                decided = Counter()
                moved_programs = set()
                for i, golden in enumerate(goldens, 1):
                    if golden["verdict"] in DECIDED:
                        decided[i] += 1
                    else:
                        self.assertEqual((golden["relation"], golden["verdict"], golden["why"]),
                                         ("lexical_binding", "UNDECIDED", "member-path-not-judged"))
                        moved_programs.add(golden["key"]["from"]["file"].split("/", 1)[0])
                        totals["moved"] += 1
                if entry is None:
                    self.assertFalse(decided)
                claimed_rows = Counter()
                claimed_files = Counter()
                for program in entry["programs"] if entry else ():
                    programs[program["source_pass"], program["id"]] += 1
                    if program["pinned"]:
                        members[program["source_pass"]] += 1
                    self.assertTrue(program["pinned"] or program["controls"])
                    self.assertTrue(program["reason"].strip())
                    self.assertEqual(program["node"]["status"], 0)
                    claimed_files.update(program["files"])
                    for kind in ("pinned", "controls"):
                        totals[kind] += sum(len(ordinals) for verdicts in program[kind].values()
                                            for ordinals in verdicts.values())
                    for relation_map in (program["pinned"], program["controls"]):
                        for relation, verdicts in relation_map.items():
                            for verdict, ordinals in verdicts.items():
                                self.assertIn(verdict, DECIDED)
                                self.assertTrue(ordinals)
                                for ordinal in ordinals:
                                    self.assertIn(ordinal, range(1, len(goldens) + 1))
                                    claimed_rows[ordinal] += 1
                                    golden = goldens[ordinal - 1]
                                    self.assertEqual((golden["relation"], golden["verdict"]),
                                                     (relation, verdict))
                                    for endpoint in golden["key"].values():
                                        self.assertTrue(endpoint["file"].startswith(program["id"] + "/"))
                                        self.assertIn("repo/" + endpoint["file"], program["files"])

                if entry:
                    self.assertTrue(any(program["pinned"] for program in entry["programs"]))
                    self.assertTrue(any(program["controls"] for program in entry["programs"]))
                self.assertEqual(claimed_rows, decided)
                files = Counter(p.relative_to(case).as_posix() for p in case.rglob("*")
                                if p.is_file() and p.parent != case)
                self.assertEqual(claimed_files, Counter({p: count for p, count in files.items()
                                                       if p in claimed_files}))
                self.assertTrue(all(path.split("/", 2)[1] in moved_programs
                                    for path in files - claimed_files))
        self.assertEqual(set(programs.values()), {1})
        self.assertEqual(members, {1: 59, 2: 13, 3: 14, "round-7-opus": 2})
        self.assertEqual(totals, {"pinned": 310, "controls": 36, "moved": 52})
