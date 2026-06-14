"""Pinned probes (spec §2.5) — fixed forever, excluded from stratum denominators."""
from __future__ import annotations

from .compare import site_compare
from .metrics import precision_recall
from .model import CallEdge, FunctionDef
from .sut import SutAmbiguous

PINNED = [
    {"id": "target-c-method", "symbol": "target", "file": "src/algorithms/taint.rs",
     "expected": "known_fail",   # G1(b): reproduce raw P<=0.2 and R<=0.2, or flip-candidate
     "note": "prototype C-method total failure; S3 flip indicator"},
    {"id": "module-deps-feature-gated", "symbol": "module_deps",
     "file": "src/navigation/module_graph.rs", "expected": "oracle_miss_site",
     "oracle_miss_site": "src/mcp/tools.rs:162"},
    {"id": "load-repo-feature-gated", "symbol": "load_repo",
     "file": "src/repo_loader.rs", "expected": "oracle_miss_site",
     "oracle_miss_site": "src/mcp/session.rs:28"},
    {"id": "ambiguous-symbol-contract", "symbol": "slice", "file": None,
     "expected": "ambiguous_symbol_error",
     "note": "bare-symbol seed with no file MUST safe-fail (prototype contract)"},
]


def _site(site: str) -> tuple[str, int]:
    file, line = site.rsplit(":", 1)
    return (file, int(line))


def _sites(edges: list[CallEdge]) -> list[str]:
    return [f"{e.call_site.file}:{e.call_site.start_line}" for e in edges]


def evaluate_pinned(
    probe_cfg: dict,
    prism_edges: list[CallEdge],
    oracle_edges: list[CallEdge],
    ambiguous_raised: bool,
) -> dict:
    """Pure evaluator for the four pinned probe contracts."""
    out = {
        "id": probe_cfg["id"],
        "expected": probe_cfg["expected"],
        "prism_sites": _sites(prism_edges),
        "oracle_sites": _sites(oracle_edges),
    }
    if probe_cfg["expected"] == "ambiguous_symbol_error":
        out["ambiguous_ok"] = ambiguous_raised
        out["outcome"] = "ok" if ambiguous_raised else "regression"
        return out

    diff = site_compare(prism_edges, oracle_edges)
    raw = precision_recall(len(diff.tp), len(diff.fp), len(diff.fn))
    out["raw"] = raw
    out["prism_only"] = [f"{f}:{line}" for f, line in sorted(diff.fp)]
    out["oracle_only"] = [f"{f}:{line}" for f, line in sorted(diff.fn)]

    if probe_cfg["expected"] == "known_fail":
        p, r = raw["precision"][0], raw["recall"][0]
        out["known_fail"] = p <= 0.2 and r <= 0.2
        out["outcome"] = "known_fail" if out["known_fail"] else "flip_candidate"
    elif probe_cfg["expected"] == "oracle_miss_site":
        miss = _site(probe_cfg["oracle_miss_site"])
        out["oracle_miss_site_found"] = miss in diff.fp
        out["outcome"] = "ok" if out["oracle_miss_site_found"] else "missing"
    else:
        out["outcome"] = "unsupported_expected"
    return out


def _resolve(snapshot: list[FunctionDef], probe: dict) -> FunctionDef | None:
    for fd in snapshot:
        if fd.name == probe["symbol"] and fd.location.file == probe.get("file"):
            return fd
    return None


def run_pinned(oracle, sut, snapshot: list[FunctionDef], corpus_root: str,
               prism_by_oracle: dict | None = None) -> list[dict]:
    """Run the pinned probes through oracle+SUT and return per-probe evaluations."""
    out = []
    for probe in PINNED:
        if probe["expected"] == "ambiguous_symbol_error":
            ambiguous_raised = False
            error = None
            try:
                sut.callers_by_symbol(corpus_root, probe["symbol"])
            except SutAmbiguous:
                ambiguous_raised = True
            except Exception as exc:
                error = str(exc)
            result = evaluate_pinned(probe, [], [], ambiguous_raised)
            if error is not None:
                result["error"] = error
            out.append(result)
            continue

        fd = _resolve(snapshot, probe)
        if fd is None:
            out.append({
                "id": probe["id"],
                "expected": probe["expected"],
                "outcome": "seed_missing",
            })
            continue
        # seed the SUT with the M1-matched prism record: the oracle record's
        # range starts at doc comments, which prism's enclosing-function lookup
        # rejects (first live baseline: module_deps pinned probe lost all prism sites)
        pfd = (prism_by_oracle or {}).get(fd, fd)
        try:
            oracle_edges = oracle.callers(fd)
            prism_edges = sut.callers(corpus_root, pfd)
        except Exception as exc:
            out.append({
                "id": probe["id"],
                "expected": probe["expected"],
                "outcome": "error",
                "error": str(exc),
            })
            continue
        result = evaluate_pinned(probe, prism_edges, oracle_edges, False)
        # EFT re-bless: the headline outcome stays on the DEFAULT measurement
        # (flip_candidate, R=1.0/P~0.21). Additionally record the exact-confidence
        # caller P/R as a SUPPLEMENTARY metric — NOT a new `expected` that retires
        # the pin (the default-nav NameOnly FPs are deferred to Phase-IP).
        if probe["id"] == "target-c-method":
            try:
                exact_edges = sut.callers(corpus_root, pfd, confidence="exact")
                ediff = site_compare(exact_edges, oracle_edges)
                result["exact_supplementary"] = precision_recall(
                    len(ediff.tp), len(ediff.fp), len(ediff.fn)
                )
            except Exception as exc:  # supplementary only — never fails the probe
                result["exact_supplementary_error"] = str(exc)
        out.append(result)
    return out
