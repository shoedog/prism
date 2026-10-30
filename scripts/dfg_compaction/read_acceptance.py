"""Read one census-free base/candidate corpus pair, never diagnostic census runs.

Usage: read_acceptance.py BASE_DIR CAND_DIR BASE_JSONL CAND_JSONL CORPUS MINIMUM_BYTES
The driver directories must contain raw/<corpus>/{prime,warm-1,warm-2}.json.
Receipts are source evidence, not authentication; preserve their checksums in custody.
"""
from pathlib import Path
import argparse
import json
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "memreport"))
from memreport_validate import validate
from memreport import cache_file_sizes
from compare_acceptance import classify
import evidence
from evidence import number


def sample(raw, hit):
    if raw.get("status") != "ok":
        raise ValueError("unsuccessful raw sample")
    report = raw["report"]
    error = validate("warm" if hit else "prime", report)
    if error:
        raise ValueError(error)
    evidence.build_fields(report)
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


def read_side(directory, probe_path, corpus, expected=None, binary=None, rescore=False):
    directory, probe_path = Path(directory).resolve(), Path(probe_path).resolve()
    if "diagnostic-census" in directory.parts:
        raise ValueError("diagnostic namespace is not acceptance")
    corpus_info = evidence.load(directory / "raw" / corpus / "corpus.json")
    evidence.corpus_fields(corpus_info)
    run = None
    if expected is not None:
        evidence.inventory(directory, corpus, corpus_info, rescore)
        run = evidence.load(directory / "run.json")
        if (run['reps'] != 2 or run['kinds'] != ['warm'] or run['bin'] != str(binary)
                or type(run['settle']) is not int or run['settle'] < 0):
            raise ValueError('unexpected driver settings or binary path')
        number(run['timeout'])
    records = [json.loads(line) for line in Path(probe_path).read_text().splitlines()]
    if any(not isinstance(r, dict) for r in records):
        raise ValueError('probe record is not an object')
    if any(r.get("kind") == "census" for r in records):
        raise ValueError("census-enabled acceptance probe")
    pids = list(dict.fromkeys(r["pid"] for r in records))
    if len(pids) != 3:
        raise ValueError("expected three distinct probe processes")
    reports, readings, configs, raw_samples = [], [], [], []
    rawdir = directory / "raw" / corpus
    if {p.name for p in rawdir.glob("*.json")} != {"corpus.json", "prime.json", "warm-1.json", "warm-2.json"}:
        raise ValueError("unexpected or missing raw population")
    for name, hit, pid in zip(("prime", "warm-1", "warm-2"), (False, True, True), pids):
        raw = evidence.load(rawdir / f"{name}.json")
        if raw.get("kind") != ("warm" if hit else "prime") or (hit and raw.get("rep") != int(name[-1])):
            raise ValueError("raw kind/repetition does not match its filename")
        report, reading = sample(raw, hit)
        if expected is not None:
            evidence.bind_build(report, expected)
        config = evidence.settings(report, directory, dict(corpus_info, name=corpus),
                                   probe_path, hit, run, raw)
        meta = [r for r in records if r["pid"] == pid and r.get("kind") == "meta"]
        if len(meta) != 1 or meta[0].get("cpg_hit") is not hit or meta[0].get("repo") != corpus_info["path"]:
            raise ValueError("probe meta cache hit")
        evidence.timers([r for r in records if r['pid'] == pid], hit, report['platform'])
        if reports and any(report[k] != reports[0][k] for k in ("build", "env", "threads", "platform")):
            raise ValueError("within-build provenance drift")
        reports.append(report)
        readings.append(reading)
        configs.append(config)
        raw_samples.append(raw)
    # Prime alone uses settle=0; every other setting must stay fixed in this side.
    if any({k: v for k, v in c.items() if k != 'settle_secs'} !=
           {k: v for k, v in configs[0].items() if k != 'settle_secs'} for c in configs):
        raise ValueError('within-build settings drift')
    if configs[1] != configs[2]:
        raise ValueError('warm settings drift')
    cache = directory / "cache" / corpus
    sizes = cache_file_sizes(cache)
    if any(type(v) is not int or v <= 0 for v in sizes.values()):
        raise ValueError("prime did not save both unique caches")
    if sizes != corpus_info.get("cache_files"):
        raise ValueError("cache inventory differs from prime receipt")
    save = evidence.save_evidence(directory, raw_samples, sizes)
    return {"corpus": corpus_info,
            "report": reports[0], "readings": readings, "configs": configs,
            "run_settings": None if run is None else {k: v for k, v in run.items() if k != 'bin'},
            "save_evidence": save}


def compare(base, candidate, minimum):
    # Diagnostic input is deliberately absent from this API.
    for side in (base, candidate):
        evidence.corpus_fields(side['corpus'])
        evidence.build_fields(side['report'])
    if any(base["corpus"][k] != candidate["corpus"][k] for k in ("commit", "path", "dirty", "workload")):
        raise ValueError("corpus/workload provenance differs")
    a, b = base["report"], candidate["report"]
    for field in ('configs', 'run_settings'):
        if base.get(field) != candidate.get(field):
            raise ValueError(f'unmatched {field}')
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


class Parser(argparse.ArgumentParser):
    def error(self, message):
        raise ValueError(message)


def main():
    try:
        parser = Parser(description=__doc__)
        for name in ('base', 'candidate', 'base_probe', 'candidate_probe', 'corpus'):
            parser.add_argument(name)
        parser.add_argument('minimum', type=int)
        parser.add_argument('--expect-receipts', type=Path,
                            help='controller binary-receipts.json; defaults to BASE_DIR/../../binary-receipts.json')
        parser.add_argument('--rescore', action='store_true', help='require final after-inventory; never writes evidence')
        args = parser.parse_args()
        if args.minimum < 0:
            raise ValueError('minimum must be nonnegative')
        if args.corpus in ('.', '..') or Path(args.corpus).name != args.corpus:
            raise ValueError('corpus must be a directory name')
        receipt_path = (args.expect_receipts or Path(args.base).resolve().parent.parent / 'binary-receipts.json').resolve()
        receipts = evidence.receipts(receipt_path)
        sides = []
        for name, directory, probe in [('base', args.base, args.base_probe),
                                        ('candidate', args.candidate, args.candidate_probe)]:
            sides.append(read_side(directory, probe, args.corpus, receipts[name]['probe'],
                                   receipt_path.parent / 'bin' / name / 'prism-mcp', args.rescore))
        a, b = sides
        print(json.dumps({"verdict": compare(a, b, args.minimum), "base": a, "candidate": b,
                          "expected_receipts": str(receipt_path)}, indent=2))
        return 0
    except (ValueError, KeyError, TypeError, OSError) as e:
        print(json.dumps({"overall": "INVALID", "reason": str(e)}))
        return 1


if __name__ == "__main__":
    sys.exit(main())
