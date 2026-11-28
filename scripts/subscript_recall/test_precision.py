"""The section 7 pair draw and precision receipt: derived, never hand-written (round-3 finding 2)."""

import contextlib
import io
import json
import tempfile
import unittest
from collections import Counter
from pathlib import Path

import admission as ad

PREPARED_SHA = "p" * 64
ROLES = ("arg", "return", "assign_rhs")


def candidate(i, role):
    key = f"P1|f{i}.py|f@1|{i}:{i + 1}|{i + 5}:{i + 6}|{role}"
    return dict(key=key, corpus="P1", file=f"f{i}.py", owner="f@1", role=role)


def population(counts):
    out, i = [], 0
    for role, n in counts.items():
        for _ in range(n):
            out.append(candidate(i, role))
            i += 1
    return out


def cli(*argv):
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = ad.main([str(a) for a in argv], frozen=None)
    return code, out.getvalue(), err.getvalue()


class PrecisionTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)

    def prepared(self, counts, name="adm"):
        """A prepared directory holding exactly the candidate population (the fields the draw reads)."""
        d = self.root / name
        d.mkdir()
        rows = population(counts)
        text = "".join(ad.dump(r) + "\n" for r in rows)
        (d / "candidates.jsonl").write_text(text)
        meta = dict(prepared_sha256=PREPARED_SHA, frame_sha256="f" * 64, valid=True,
                    candidates_sha256=ad.digest(text.encode()))
        (d / "prepared.json").write_text(json.dumps(meta))
        return d, rows

    def draw(self, d, name="draw.json"):
        out = self.root / name
        self.assertEqual(cli("draw", "--prepared", d, "--out", out)[0], 0)
        return out, json.loads(out.read_text())

    def judge(self, draw, verdict=lambda k, role: "valid", name="j.jsonl", extra=()):
        rows = [dict(key=k, verdict=verdict(k, role), witnessed=verdict(k, role) == "valid",
                     prepared_sha256=PREPARED_SHA) for k, role in sorted(draw["keys"].items())]
        path = self.root / name
        path.write_text("".join(ad.dump(r) + "\n" for r in rows + list(extra)))
        return path

    def precision(self, d, draw_path, judgments, out="precision.json", *extra):
        return cli("precision", "--prepared", d, "--draw", draw_path, "--judgments", judgments,
                   "--out", self.root / out, *extra)

    def test_the_draw_is_derived_deterministically_from_the_candidates(self):
        d, rows = self.prepared({"arg": 130, "return": 300, "assign_rhs": 3})
        out, draw = self.draw(d)
        self.assertEqual(draw["population"], 433)
        self.assertEqual(draw["prepared_sha256"], PREPARED_SHA)
        self.assertEqual(draw["candidates_sha256"], json.loads((d / "prepared.json").read_text())["candidates_sha256"])
        by_role = Counter(draw["keys"].values())
        self.assertEqual(by_role["assign_rhs"], 3)  # the rare role is exhausted by the top-up
        self.assertGreaterEqual(by_role["arg"], 20)
        self.assertEqual(len(draw["core"]), 100)
        self.assertTrue(set(draw["core"]) <= set(draw["keys"]) and set(draw["topup"]).isdisjoint(draw["core"]))
        self.assertEqual(set(draw["keys"]), set(draw["core"]) | set(draw["topup"]))
        self.assertFalse(draw["exhaustive"])
        # same candidates in another order and another run: byte-identical draw
        d2, _ = self.prepared({"arg": 130, "return": 300, "assign_rhs": 3}, name="adm2")
        rev = [json.loads(x) for x in (d2 / "candidates.jsonl").read_text().splitlines()][::-1]
        text = "".join(ad.dump(r) + "\n" for r in rev)
        (d2 / "candidates.jsonl").write_text(text)
        meta = json.loads((d2 / "prepared.json").read_text())
        meta["candidates_sha256"] = ad.digest(text.encode())
        (d2 / "prepared.json").write_text(json.dumps(meta))
        out2, draw2 = self.draw(d2, "draw2.json")
        self.assertEqual(draw2["keys"], draw["keys"])
        self.assertEqual(draw2["core"], draw["core"])

    def test_a_small_population_is_drawn_exhaustively(self):
        d, _ = self.prepared({"return": 3})
        _, draw = self.draw(d)
        self.assertEqual((draw["exhaustive"], len(draw["keys"])), (True, 3))

    def test_a_clean_receipt_binds_the_draw_the_judgments_and_the_prepared_digest(self):
        d, _ = self.prepared({"return": 40, "arg": 70})
        draw_path, draw = self.draw(d)
        j = self.judge(draw)
        code, out, err = self.precision(d, draw_path, j)
        self.assertEqual(code, 0, err)
        receipt = json.loads((self.root / "precision.json").read_text())
        self.assertEqual((receipt["schema"], receipt["population_name"]), ("admission_precision/4", "a-credit"))
        self.assertEqual((receipt["prepared_sha256"], receipt["draw_sha256"], receipt["judgments_sha256"]),
                         (PREPARED_SHA, ad.digest(draw_path.read_bytes()), ad.digest(j.read_bytes())))
        self.assertEqual((receipt["n"], receipt["valid"], receipt["stale"], receipt["unknown"]),
                         (len(draw["keys"]), len(draw["keys"]), 0, 0))
        self.assertEqual(ad.precision.verify(receipt, json.loads((d / "prepared.json").read_text()),
                                             ad.jsonl(d / "candidates.jsonl"), self.root), [])

    def test_a_tiny_exhaustive_population_passes_and_one_invalid_fails_it(self):
        d, _ = self.prepared({"return": 5})
        draw_path, draw = self.draw(d)
        self.assertEqual(self.precision(d, draw_path, self.judge(draw))[0], 0)
        first = sorted(draw["keys"])[0]
        j = self.judge(draw, lambda k, role: "invalid" if k == first else "valid", "bad.jsonl")
        code, out, _ = self.precision(d, draw_path, j, "bad-precision.json")
        self.assertEqual(code, 1, out)
        self.assertIn("every one must be", out)

    def test_a_sampled_population_needs_the_wilson_floor_on_the_core(self):
        d, _ = self.prepared({"return": 300})
        draw_path, draw = self.draw(d)
        core = set(draw["core"])
        bad = set(sorted(core)[:20])
        j = self.judge(draw, lambda k, role: "invalid" if k in bad else "valid")
        code, out, _ = self.precision(d, draw_path, j)
        self.assertEqual(code, 1, out)
        self.assertIn("Wilson", out)
        ok = set(sorted(core)[:3])
        j = self.judge(draw, lambda k, role: "invalid" if k in ok else "valid", "ok.jsonl")
        self.assertEqual(self.precision(d, draw_path, j, "ok-precision.json")[0], 0)

    def test_a_role_below_its_witnessed_floor_fails(self):
        d, _ = self.prepared({"return": 300, "assign_rhs": 20})
        draw_path, draw = self.draw(d)
        rhs = sorted(k for k, r in draw["keys"].items() if r == "assign_rhs")
        j = self.judge(draw, lambda k, role: "unknown" if k in rhs[:5] else "valid")
        code, out, _ = self.precision(d, draw_path, j)
        self.assertEqual(code, 1, out)
        self.assertIn("role assign_rhs", out)
        self.assertIn("unknown", out)

    def test_core_precision_counts_the_witnessed_fraction_not_valid_verdicts(self):
        """100 `valid` core judgments with 90 witnessed: Wilson(90/100) = 0.826 < .85 fails."""
        d, _ = self.prepared({"return": 300})
        draw_path, draw = self.draw(d)
        core = sorted(draw["core"])
        rows = [dict(key=k, verdict="valid", witnessed=k not in set(core[:10]), prepared_sha256=PREPARED_SHA)
                for k in sorted(draw["keys"])]
        path = self.root / "w90.jsonl"
        path.write_text("".join(ad.dump(r) + "\n" for r in rows))
        code, out, _ = self.precision(d, draw_path, path)
        self.assertEqual(code, 1, out)
        self.assertIn("Wilson", out)
        receipt = json.loads((self.root / "precision.json").read_text())
        self.assertEqual((receipt["core"]["valid"], receipt["core"]["witnessed"]), (100, 90))
        # control: 95 of 100 witnessed clears the floor
        rows = [dict(r, witnessed=r["key"] not in set(core[:5])) for r in rows]
        path.write_text("".join(ad.dump(r) + "\n" for r in rows))
        self.assertEqual(self.precision(d, draw_path, path)[0], 0)

    def test_an_exhaustive_family_needs_every_pair_witnessed_not_just_valid(self):
        d, _ = self.prepared({"return": 5})
        draw_path, draw = self.draw(d)
        first = sorted(draw["keys"])[0]
        rows = [dict(key=k, verdict="valid", witnessed=k != first, prepared_sha256=PREPARED_SHA)
                for k in sorted(draw["keys"])]
        path = self.root / "ex.jsonl"
        path.write_text("".join(ad.dump(r) + "\n" for r in rows))
        code, out, _ = self.precision(d, draw_path, path)
        self.assertEqual(code, 1, out)
        self.assertIn("4/5 judgments witnessed", out)

    def test_witnessed_must_be_a_strict_boolean_and_agree_with_the_verdict(self):
        d, _ = self.prepared({"return": 5})
        draw_path, draw = self.draw(d)
        first = sorted(draw["keys"])[0]
        for name, change in (("string", dict(witnessed="true")), ("int", dict(witnessed=1)),
                             ("missing", dict(witnessed=None)),
                             ("witness for an invalid verdict", dict(verdict="invalid", witnessed=True))):
            with self.subTest(name):
                rows = [dict(key=k, verdict="valid", witnessed=True, prepared_sha256=PREPARED_SHA)
                        for k in sorted(draw["keys"])]
                rows[0] = dict(rows[0], **change)
                assert rows[0]["key"] == first
                path = self.root / "bad.jsonl"
                path.write_text("".join(ad.dump(r) + "\n" for r in rows))
                code, _, err = self.precision(d, draw_path, path)
                self.assertEqual(code, 3, err)
                self.assertTrue("witnessed" in err, err)

    def test_judgments_outside_the_draw_are_refused(self):
        """The round-3 repro: 22 all-return judgments cannot stand in for the 100-pair draw."""
        d, _ = self.prepared({"return": 1100, "arg": 100, "assign_rhs": 50})
        draw_path, draw = self.draw(d)
        some = sorted(draw["keys"])[:22]
        path = self.root / "short.jsonl"
        path.write_text("".join(ad.dump(dict(key=k, verdict="valid", witnessed=True,
                                              prepared_sha256=PREPARED_SHA)) + "\n" for k in some))
        code, _, err = self.precision(d, draw_path, path)
        self.assertEqual(code, 3, err)
        self.assertIn("missing", err)
        self.assertFalse((self.root / "precision.json").exists())
        # keys that are candidates but not drawn are extras
        drawn = set(draw["keys"])
        outside = [c["key"] for c in ad.jsonl(d / "candidates.jsonl") if c["key"] not in drawn][:3]
        j = self.judge(draw, extra=[dict(key=k, verdict="valid", witnessed=True, prepared_sha256=PREPARED_SHA)
                                    for k in outside])
        code, _, err = self.precision(d, draw_path, j)
        self.assertEqual(code, 3, err)
        self.assertIn("extra", err)
        # a made-up role: keys not among the candidates at all
        fake = "P1|zz.py|f@1|1:2|3:4|made_up"
        j = self.judge(draw, extra=[dict(key=fake, verdict="valid", witnessed=True, prepared_sha256=PREPARED_SHA)])
        self.assertEqual(self.precision(d, draw_path, j)[0], 3)
        # a duplicated key
        first = sorted(draw["keys"])[0]
        j = self.judge(draw, extra=[dict(key=first, verdict="valid", witnessed=True, prepared_sha256=PREPARED_SHA)])
        code, _, err = self.precision(d, draw_path, j)
        self.assertEqual(code, 3, err)
        self.assertIn("duplicated", err)

    def test_a_draw_that_is_not_the_derived_one_is_refused(self):
        d, _ = self.prepared({"return": 300})
        draw_path, draw = self.draw(d)
        forged = dict(draw, keys=dict(list(sorted(draw["keys"].items()))[:22]))
        forged["core"] = [k for k in draw["core"] if k in forged["keys"]]
        forged["topup"] = []
        path = self.root / "forged.json"
        path.write_text(json.dumps(forged))
        code, _, err = self.precision(d, path, self.judge(forged))
        self.assertEqual(code, 3, err)
        self.assertIn("derived", err)

    def test_stale_judgments_fail_and_a_stale_prepared_digest_counts_as_stale(self):
        d, _ = self.prepared({"return": 5})
        draw_path, draw = self.draw(d)
        first = sorted(draw["keys"])[0]
        j = self.judge(draw, lambda k, role: "stale" if k == first else "valid")
        code, out, _ = self.precision(d, draw_path, j)
        self.assertEqual(code, 1, out)
        self.assertIn("stale", out)
        rows = [json.loads(x) for x in self.judge(draw).read_text().splitlines()]
        rows[0]["prepared_sha256"] = "0" * 64
        path = self.root / "old.jsonl"
        path.write_text("".join(ad.dump(r) + "\n" for r in rows))
        code, out, _ = self.precision(d, draw_path, path)
        self.assertEqual(code, 1, out)
        self.assertIn("stale", out)

    def test_verification_recomputes_every_binding_and_refuses_a_forged_receipt(self):
        d, _ = self.prepared({"return": 300, "arg": 40})
        draw_path, draw = self.draw(d)
        j = self.judge(draw)
        self.assertEqual(self.precision(d, draw_path, j)[0], 0)
        meta = json.loads((d / "prepared.json").read_text())
        cands = ad.jsonl(d / "candidates.jsonl")
        good = json.loads((self.root / "precision.json").read_text())
        verify = lambda receipt: ad.precision.verify(receipt, meta, cands, self.root)
        self.assertEqual(verify(good), [])
        forged = {
            "n": dict(good, n=good["n"] - 1),
            "valid": dict(good, valid=good["n"] + 5),
            "roles": dict(good, roles={"return": dict(n=99, witnessed=99)}),
            "judgments hash": dict(good, judgments_sha256="0" * 64),
            "draw hash": dict(good, draw_sha256="0" * 64),
            "prepared": dict(good, prepared_sha256="0" * 64),
            "frame": dict(good, frame_sha256="0" * 64),
            "schema": dict(good, schema="admission_precision/3"),
            "missing draw file": dict(good, draw_path=str(self.root / "gone.json")),
        }
        for name, receipt in forged.items():
            with self.subTest(name):
                self.assertTrue(verify(receipt), name)
        # the judgments file edited after the receipt was written (a verdict flipped, hash unchanged in the receipt)
        rows = [json.loads(x) for x in j.read_text().splitlines()]
        rows[0]["verdict"] = "invalid"
        j.write_text("".join(ad.dump(r) + "\n" for r in rows))
        self.assertTrue(any("judgments" in p for p in verify(good)))
        # a draw file replaced by a smaller, self-consistent one
        small = dict(draw, keys=dict(list(sorted(draw["keys"].items()))[:22]))
        draw_path.write_text(json.dumps(small))
        self.assertTrue(verify(dict(good, draw_sha256=ad.digest(draw_path.read_bytes()))))


    def with_survivors(self):
        """A prepared directory whose survivors extend the candidates by 30 pairs (the excluded method calls)."""
        d, rows = self.prepared({"return": 40, "arg": 70})
        extra = [dict(candidate(1000 + i, "return"), credited=False) for i in range(30)]
        text = "".join(ad.dump(r) + "\n" for r in rows + extra)
        (d / "survivors.jsonl").write_text(text)
        meta = json.loads((d / "prepared.json").read_text())
        meta["survivors_sha256"] = ad.digest(text.encode())
        (d / "prepared.json").write_text(json.dumps(meta))
        return d, rows, extra

    def test_an_old_schema_receipt_fails_naming_the_schema_not_the_recomputation(self):
        d, _ = self.prepared({"return": 300, "arg": 40})
        draw_path, draw = self.draw(d)
        self.assertEqual(self.precision(d, draw_path, self.judge(draw))[0], 0)
        good = json.loads((self.root / "precision.json").read_text())
        meta, cands = json.loads((d / "prepared.json").read_text()), ad.jsonl(d / "candidates.jsonl")
        old = ad.precision.verify(dict(good, schema="admission_precision/3"), meta, cands, self.root)
        self.assertEqual(len(old), 1, old)
        self.assertIn("schema mismatch", old[0])
        self.assertIn("admission_precision/3", old[0])
        self.assertIn("admission_precision/4", old[0])
        self.assertNotIn("recomputation", old[0])
        other = ad.precision.verify(dict(good, schema="something/else"), meta, cands, self.root)
        self.assertIn("not an admission_precision/4 document", other[0])  # control: not a schema-family document
        self.assertEqual(ad.precision.verify(good, meta, cands, self.root), [])

    def test_the_all_survivors_population_is_drawn_and_verified_from_the_survivors_file(self):
        d, rows, extra = self.with_survivors()
        out = self.root / "draw-all.json"
        self.assertEqual(cli("draw", "--prepared", d, "--out", out, "--population", "all-survivors")[0], 0)
        draw_all = json.loads(out.read_text())
        self.assertEqual(draw_all["population"], 140)
        _, draw_a = self.draw(d)
        self.assertEqual(draw_a["population"], 110)
        j = self.judge(draw_all)
        self.assertEqual(self.precision(d, out, j, "p-all.json", "--population", "all-survivors")[0], 0)
        receipt = json.loads((self.root / "p-all.json").read_text())
        self.assertEqual(receipt["population_name"], "all-survivors")
        meta = json.loads((d / "prepared.json").read_text())
        cands, surv = ad.jsonl(d / "candidates.jsonl"), ad.jsonl(d / "survivors.jsonl")
        verify = lambda r, s=surv: ad.precision.verify(r, meta, cands, self.root, survivors=s)
        self.assertEqual(verify(receipt), [])
        self.assertTrue(any("survivors" in p for p in verify(receipt, None)))  # an old prepared dir has none
        # the same draw named as the a-credit population is not derived from the candidates
        self.assertTrue(verify(dict(receipt, population_name="a-credit")))
        self.assertTrue(verify(dict(receipt, population_name="nonsense")))

    def test_a_prepared_receipt_without_a_survivors_digest_cannot_verify_an_all_survivors_receipt(self):
        d, rows, extra = self.with_survivors()
        out = self.root / "draw-all.json"
        self.assertEqual(cli("draw", "--prepared", d, "--out", out, "--population", "all-survivors")[0], 0)
        self.assertEqual(self.precision(d, out, self.judge(json.loads(out.read_text())), "p-all.json",
                                        "--population", "all-survivors")[0], 0)
        receipt = json.loads((self.root / "p-all.json").read_text())
        meta = json.loads((d / "prepared.json").read_text())
        cands, surv = ad.jsonl(d / "candidates.jsonl"), ad.jsonl(d / "survivors.jsonl")
        self.assertEqual(ad.precision.verify(receipt, meta, cands, self.root, surv), [])  # control
        del meta["survivors_sha256"]
        problems = ad.precision.verify(receipt, meta, cands, self.root, surv)
        self.assertTrue(any("cannot be recomputed" in p and "survivors.jsonl" in p for p in problems), problems)
        with self.assertRaises(ValueError):
            ad.precision.population_meta(meta, "nonsense")
        self.assertEqual(ad.precision.verify("not a dict", meta, cands, self.root),
                         ["precision receipt is not an admission_precision/4 document"])

    def test_all_survivors_without_a_survivors_file_is_refused_by_the_cli(self):
        d, _ = self.prepared({"return": 5})
        code, _, err = cli("draw", "--prepared", d, "--out", self.root / "x.json", "--population", "all-survivors")
        self.assertEqual(code, 3, err)
        self.assertIn("survivors.jsonl", err)
        self.assertFalse((self.root / "x.json").exists())

    def test_wilson_interval_brackets_the_lower_bound(self):
        lo, hi = ad.precision.wilson_interval(100, 100)
        self.assertAlmostEqual(lo, ad.precision.wilson_lower(100, 100))
        self.assertAlmostEqual(hi, 1.0, places=6)
        self.assertEqual(ad.precision.wilson_interval(0, 0), (0.0, 0.0))
        lo, hi = ad.precision.wilson_interval(90, 100)
        self.assertTrue(lo < 0.9 < hi < 1.0)


if __name__ == "__main__":
    unittest.main()
