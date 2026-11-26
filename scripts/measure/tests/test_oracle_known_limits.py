"""Keep the known-limit ledger and the self-check goldens in agreement."""

from collections import Counter
import copy
import json
import unittest

from measure.oracle_inputs import fixture_set

from .test_oracle import KIT, read_jsonl


DECIDED = {"CORRECT", "WRONG"}
DETECTORS = {
    "annex-b": {"annexb": "reliable-detector"},
    "direct-eval": {"eval": "reliable-detector"},
    "catch-var": {"catch-var": "reliable-detector"},
    "host-mode": {"host-mode": "proxy"},
    "property-origin": {
        "property-origin": "proxy",
        "spread-getter/__proto__": "reliable-detector",
        "declare/merged": "reliable-detector",
    },
    "parameter-default": {"param-default-closure": "reliable-detector"},
    "for-let-init": {"for-let-init": "reliable-detector"},
    "for-in-of-head": {"for-in-of-head": "reliable-detector"},
    "module-resolution": {"ext-substitution": "reliable-detector", "package-main/json/d.ts": "proxy"},
    "html-comment": {"html-comment": "reliable-detector"},
    "receiver-dispatch": {"receiver-override": "proxy"},
    "computed-member-name": {"computed-name": "reliable-detector"},
    "static-initialization": {"static-init": "proxy"},
    "member-order": {"member-order": "proxy"},
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
        self.assertEqual(set(cases), set(registry["classes"]))
        self.assertEqual({slug: entry["prevalence"] for slug, entry in registry["classes"].items()},
                         DETECTORS)

        programs = Counter((entry["source_pass"], entry["id"]) for entry in registry["unrecovered"])
        members = Counter(entry["source_pass"] for entry in registry["unrecovered"])
        for slug, entry in registry["classes"].items():
            with self.subTest(known_limit=slug):
                case = cases[slug]
                goldens = read_jsonl(case / "expected.jsonl")
                claimed_rows = Counter()
                claimed_files = Counter()
                for program in entry["programs"]:
                    programs[program["source_pass"], program["id"]] += 1
                    if program["pinned"]:
                        members[program["source_pass"]] += 1
                    self.assertTrue(program["pinned"] or program["controls"])
                    self.assertTrue(program["reason"].strip())
                    self.assertEqual(program["node"]["status"], 0)
                    claimed_files.update(program["files"])
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

                self.assertTrue(any(program["pinned"] for program in entry["programs"]))
                self.assertTrue(any(program["controls"] for program in entry["programs"]))
                self.assertEqual(claimed_rows, Counter(range(1, len(goldens) + 1)))
                self.assertEqual(claimed_files,
                                 Counter(p.relative_to(case).as_posix() for p in case.rglob("*")
                                         if p.is_file() and p.parent != case))
        self.assertEqual(set(programs.values()), {1})
        self.assertEqual(members, {1: 67, 2: 19, 3: 24, "round-7-opus": 3})
