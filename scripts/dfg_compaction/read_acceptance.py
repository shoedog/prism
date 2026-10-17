"""Read one census-free base/candidate corpus pair, never diagnostic census runs.

Usage: read_acceptance.py BASE_DIR CAND_DIR BASE_JSONL CAND_JSONL CORPUS MINIMUM_BYTES
The driver directories must contain raw/<corpus>/{prime,warm-1,warm-2}.json.
Receipts are source evidence, not authentication; preserve their checksums in custody.
"""
from pathlib import Path
import json
import math
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "memreport"))
from memreport_validate import validate
from memreport import cache_file_sizes
from compare_acceptance import classify


def number(value):
    if type(value) not in (int, float) or not math.isfinite(value) or value < 0:
        raise ValueError("missing/nonfinite/negative counter")
    return value


def sample(raw, hit):
    if raw.get("status") != "ok":
        raise ValueError("unsuccessful raw sample")
    report = raw["report"]
    error = validate("warm" if hit else "prime", report)
    if error:
        raise ValueError(error)
    if any(report["cache"].get(k) is not hit for k in ("cpg_hit", "call_edges_loaded")):
        raise ValueError("cache flags")
    env = report["env"]
    if env.get("PRISM_PROBE_NO_CENSUS") != "1":
        raise ValueError("census must be disabled")
    if env.get("RAYON_NUM_THREADS") is not None:
        raise ValueError("RAYON_NUM_THREADS must be unset")
    dirty = report["build"]["binary_input_dirty"]
    if dirty and env.get("PRISM_NAV_EDGE_CACHE_LOAD_DIRTY") != "1":
        raise ValueError("dirty binary without explicit sidecar override")
    stages = {r["name"]: r for r in report["stages"]}
    if len(stages) != len(report["stages"]):
        raise ValueError("duplicate stages")
    index, loaded = stages["after_index"], stages["after_load_repo"]
    heap = number(index["heap_in_use_bytes"])
    delta = number(index["elapsed_ms"]) - number(loaded["elapsed_ms"])
    if delta <= 0:
        raise ValueError("nonpositive load interval")
    peak = number(stages["after_call_edges"]["peak_bytes"])
    return report, {"heap": heap, "load_ms": delta, "peak": peak,
                    "loader_delta": heap - number(loaded["heap_in_use_bytes"]),
                    "physical": index}


def read_side(directory, probe_path, corpus):
    directory = Path(directory)
    if "diagnostic-census" in directory.parts:
        raise ValueError("diagnostic namespace is not acceptance")
    corpus_info = json.loads((directory / "raw" / corpus / "corpus.json").read_text())
    records = [json.loads(line) for line in Path(probe_path).read_text().splitlines()]
    if any(r.get("kind") == "census" for r in records):
        raise ValueError("census-enabled acceptance probe")
    pids = list(dict.fromkeys(r["pid"] for r in records))
    if len(pids) != 3:
        raise ValueError("expected three distinct probe processes")
    reports, readings = [], []
    rawdir = directory / "raw" / corpus
    if {p.name for p in rawdir.glob("*.json")} != {"corpus.json", "prime.json", "warm-1.json", "warm-2.json"}:
        raise ValueError("unexpected or missing raw population")
    for name, hit, pid in zip(("prime", "warm-1", "warm-2"), (False, True, True), pids):
        raw = json.loads((rawdir / f"{name}.json").read_text())
        if raw.get("kind") != ("warm" if hit else "prime") or (hit and raw.get("rep") != int(name[-1])):
            raise ValueError("raw kind/repetition does not match its filename")
        report, reading = sample(raw, hit)
        meta = [r for r in records if r["pid"] == pid and r.get("kind") == "meta"]
        if len(meta) != 1 or meta[0].get("cpg_hit") is not hit or meta[0].get("repo") != corpus_info["path"]:
            raise ValueError("probe meta cache hit")
        if not any(r["pid"] == pid and r.get("kind") == "stage" for r in records):
            raise ValueError("missing probe timers")
        if reports and any(report[k] != reports[0][k] for k in ("build", "env", "threads", "platform")):
            raise ValueError("within-build provenance drift")
        reports.append(report)
        readings.append(reading)
    cache = directory / "cache" / corpus
    sizes = cache_file_sizes(cache)
    if any(type(v) is not int or v <= 0 for v in sizes.values()):
        raise ValueError("prime did not save both unique caches")
    if sizes != corpus_info.get("cache_files"):
        raise ValueError("cache inventory differs from prime receipt")
    for path in rawdir.glob("*.stderr"):
        if any(s in path.read_text() for s in ("failed to save CPG cache", "nav edge cache: failed to save sidecar")):
            raise ValueError("cache save failure")
    return {"corpus": corpus_info,
            "report": reports[0], "readings": readings}


def compare(base, candidate, minimum):
    # Diagnostic input is deliberately absent from this API.
    if any(base["corpus"][k] != candidate["corpus"][k] for k in ("commit", "path", "dirty", "workload")):
        raise ValueError("corpus/workload provenance differs")
    a, b = base["report"], candidate["report"]
    for field in ("threads", "platform"):
        if a[field] != b[field]:
            raise ValueError(f"unmatched {field}")
    ea, eb = (dict(r["env"]) for r in (a,b))
    for env in (ea,eb): env.pop("PRISM_PROBE_OUT", None)
    if ea != eb: raise ValueError("unmatched environment")
    if a["build"]["build_identity"] == b["build"]["build_identity"]:
        raise ValueError("base/candidate build identities must be distinct")
    for field in ("package_version", "grammar_fingerprint", "binary_input_dirty", "allocator"):
        if a["build"][field] != b["build"][field]:
            raise ValueError(f"unmatched build {field}")
    x, y = base["readings"], candidate["readings"]
    return classify([r["heap"] for r in x[1:]], [r["heap"] for r in y[1:]],
                    [r["load_ms"] for r in x[1:]], [r["load_ms"] for r in y[1:]],
                    x[0]["peak"], y[0]["peak"], minimum)


if __name__ == "__main__":
    try:
        base, cand, bp, cp, corpus, minimum = sys.argv[1:]
        a, b = read_side(base, bp, corpus), read_side(cand, cp, corpus)
        print(json.dumps({"verdict": compare(a, b, int(minimum)), "base": a, "candidate": b}, indent=2))
    except (ValueError, KeyError, TypeError, OSError) as e:
        print(json.dumps({"overall": "INVALID", "reason": str(e)}))
        sys.exit(1)
