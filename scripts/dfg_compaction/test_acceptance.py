"""Synthetic artifact controls; no corpus process is run."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import subprocess
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import read_acceptance as reader
from memreport_fixtures import canned, stage_named


def raw(hit=True):
    r = canned("warm")
    r["build"].update(package_version="3.2", git_sha="base", grammar_fingerprint="grammar", allocator="system")
    r["env"] = {"PRISM_PROBE_NO_CENSUS": "1"}
    r["cache"] = dict.fromkeys(("cpg_hit", "call_edges_loaded"), hit)
    stage_named(r, "after_index")["elapsed_ms"] = 2000
    return {"status": "ok", "report": r}


def side(identity="base", ms=2000, peak=2000):
    r = raw()["report"]
    r["build"]["build_identity"] = identity
    return {"corpus": {"commit": "commit", "path": "/corpus", "dirty": False, "workload": {"sha256": "a", "requests": 1}},
            "report": r, "readings": [{"heap": 1000, "load_ms": ms, "peak": peak}] * 3}


class AcceptanceTests(unittest.TestCase):
    def test_cli_emits_one_json_artifact_and_kernel_completion(self):
        root = Path(__file__).resolve().parent
        failed = subprocess.run([sys.executable,str(root/"read_acceptance.py")],capture_output=True,text=True)
        self.assertEqual(failed.returncode,1)
        self.assertEqual(json.loads(failed.stdout)["overall"],"INVALID")
        kernel = subprocess.run([sys.executable,str(root/"compare_acceptance.py")],capture_output=True,text=True)
        self.assertEqual(kernel.returncode,0)
        self.assertEqual(kernel.stdout,"acceptance classifier: 10 boundary assertions passed\n")

    def test_single_high_candidate_is_not_hidden_by_mean_saving(self):
        self.assertEqual(reader.classify([1000,1000],[699,701],[1000,1000],[1000,1000],2000,2000,300)["heap"], "FAIL")

    def test_sample_and_false_cache_flags(self):
        reader.sample(raw(), True)
        for field in ("cpg_hit", "call_edges_loaded"):
            r = raw(); r["report"]["cache"][field] = False
            with self.assertRaises(ValueError): reader.sample(r, True)

    def test_missing_nonfinite_counters(self):
        for v in (None, float("nan"), float("inf"), -1):
            r = raw(); stage_named(r["report"], "after_index")["heap_in_use_bytes"] = v
            with self.assertRaises(ValueError): reader.sample(r, True)

    def test_census_and_dirty_provenance(self):
        for env, dirty in [({}, False), ({"PRISM_PROBE_NO_CENSUS": "0"}, False),
                           ({"PRISM_PROBE_NO_CENSUS": "1"}, True),
                           ({"PRISM_PROBE_NO_CENSUS": "1", "RAYON_NUM_THREADS": "4"}, False)]:
            r = raw(); r["report"]["env"] = env; r["report"]["build"]["binary_input_dirty"] = dirty
            with self.assertRaises(ValueError): reader.sample(r, True)
        r = raw(); r["report"]["build"]["binary_input_dirty"] = True
        r["report"]["env"]["PRISM_NAV_EDGE_CACHE_LOAD_DIRTY"] = "1"
        reader.sample(r, True)

    def test_diagnostic_cannot_repair_product_load_or_peak(self):
        for candidate, field in [(side("candidate", ms=2250), "load"), (side("candidate", peak=2300), "peak")]:
            a = side()
            self.assertEqual(reader.compare(a, candidate, 0)[field], "FAIL")
            a["diagnostic"] = {"load_ms": 800, "peak": 2500}
            candidate["diagnostic"] = {"load_ms": 400, "peak": 1000}
            self.assertEqual(reader.compare(a, candidate, 0)[field], "FAIL")

    def test_provenance_mismatch(self):
        for section, key, value in [("corpus", "commit", "other"), ("report", "threads", 99)]:
            b = side("candidate"); b[section][key] = value
            with self.assertRaises(ValueError): reader.compare(side(), b, 0)
        with self.assertRaises(ValueError): reader.compare(side(), side(), 0)

    def test_reader_requires_prime_and_probe_population(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d); rd = p / "raw/tiny"; rd.mkdir(parents=True)
            (rd / "corpus.json").write_text(json.dumps({"path":"/tiny", "cache_files":{"cpg_cache_bytes":5,"call_edge_sidecar_bytes":5}}))
            cache = p / "cache/tiny"; cache.mkdir(parents=True)
            nested = cache/"prism/nav/repo"; nested.mkdir(parents=True)
            for f in ("cpg-cache-v2.bin", "resolved-call-edge-index.bin"): (nested/f).write_bytes(b"saved")
            records = []
            for pid, name, hit in [(1,"prime",False), (2,"warm-1",True), (3,"warm-2",True)]:
                entry = raw(hit); entry["kind"] = "warm" if hit else "prime"
                if hit: entry["rep"] = pid-1
                (rd/f"{name}.json").write_text(json.dumps(entry))
                records.extend([{"pid":pid,"kind":"meta","cpg_hit":hit,"repo":"/tiny"}, {"pid":pid,"kind":"stage"}])
            probe = p/"probe.jsonl"
            def write(): probe.write_text("\n".join(map(json.dumps, records)))
            write(); reader.read_side(p,probe,"tiny")
            (cache/"cpg-cache-v2.bin").write_bytes(b"duplicate")
            with self.assertRaises(ValueError): reader.read_side(p,probe,"tiny")
            (cache/"cpg-cache-v2.bin").unlink()
            prime = rd/"prime.json"; saved = prime.read_text()
            invalid = json.loads(saved); invalid["status"] = "failed"
            prime.write_text(json.dumps(invalid))
            with self.assertRaises(ValueError): reader.read_side(p,probe,"tiny")
            prime.write_text(saved)
            records.append({"pid":3,"kind":"census"}); write()
            with self.assertRaises(ValueError): reader.read_side(p,probe,"tiny")
            records.pop(); write(); (rd/"prime.json").unlink()
            with self.assertRaises(ValueError): reader.read_side(p,probe,"tiny")
