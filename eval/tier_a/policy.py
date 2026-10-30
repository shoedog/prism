"""Tier-A measurement definitions separated from CLI orchestration."""
from __future__ import annotations

import json
import os
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

from .accounting import CorpusAccounting
from .adjudication import apply_verdicts, load_records, reanchor_map
from .boundary import readmit_stored
from .compare import caller_fn_sets, site_compare
from .lsp_client import LspError
from .metrics import precision_recall
from .model import CallEdge, FunctionDef, Location, SelectionMatcher, edge_tier
from .oracles import OracleError, OracleTimeout
from .spotcheck import classify_site, find_call_position, utf16_column
from .sut import SutAmbiguous, SutError, SutTimeout


EVAL_DIR = Path(__file__).resolve().parents[1]


def seed_id(fd, line_counts):
    key = f"{fd.location.file}:{fd.selection_line}"
    if line_counts[(fd.location.file, fd.selection_line)] > 1:
        key += ":" + json.dumps(
            [fd.selection_char, fd.name, fd.kind, fd.container], separators=(",", ":")
        )
    return key


def _site_parts(site) -> tuple[str, int, int, dict]:
    """Read legacy site triples or metadata-bearing site records."""
    if isinstance(site, dict):
        return (
            site["file"],
            site["start_line"],
            site.get("end_line", site["start_line"]),
            site,
        )
    file, start, end, *rest = site
    meta = {}
    if rest:
        raw = rest[0]
        if isinstance(raw, dict):
            meta = raw
        elif raw is not None:
            meta = {"resolution_kind": raw}
    return file, start, end, meta


def _edges(sites: list, direction: str) -> list[CallEdge]:
    """Rebuild minimal CallEdges from stored site triples plus optional metadata."""
    dummy = FunctionDef("_", "function", None, Location("_", 1, 1), 1)
    return [
        CallEdge(
            direction,
            dummy,
            None,
            None,
            Location(f, s, e),
            meta.get("resolution_kind"),
            meta.get("score"),
        )
        for f, s, e, meta in (_site_parts(site) for site in sites)
    ]


def _dispatch_kind_for_site(sites: list, file: str, line: int) -> str | None:
    for f, s, _e, meta in (_site_parts(site) for site in sites):
        if f == file and s == line:
            dispatch = meta.get("dispatch_kind") or meta.get("resolution_kind")
            if dispatch:
                return dispatch
    return None


def _tier_for_site(edges: list[CallEdge], file: str, line: int) -> str | None:
    """P6a: the edge_tier of the prism edge at (file, line), or None if absent."""
    for edge in edges:
        if edge.call_site.file == file and edge.call_site.start_line == line:
            return edge_tier(edge)
    return None


def _site_key(file: str, line: int) -> str:
    return f"{file}:{line}"


def _pair_metrics(prism: set[tuple], oracle: set[tuple]) -> dict:
    tp = len(prism & oracle)
    return precision_recall(tp, len(prism - oracle), len(oracle - prism))


def _site_fingerprints_for_probes(probes: dict, corpus_root: str) -> dict[str, str]:
    """Fingerprint every probe diff-site's call line (+/- 1) from corpus source."""
    from .adjudication import fingerprint

    by_file: dict[str, set] = {}
    for pid, probe in probes.items():
        if pid.startswith("_"):
            continue
        for site in list(probe.get("prism_sites", [])) + list(
            probe.get("oracle_sites", [])
        ):
            file, line, _end, _meta = _site_parts(site)
            by_file.setdefault(file, set()).add(line)
    out: dict[str, str] = {}
    for file, lines in by_file.items():
        try:
            source = (
                (Path(corpus_root) / file)
                .read_text(encoding="utf-8", errors="replace")
                .splitlines()
            )
        except (FileNotFoundError, IsADirectoryError):
            continue
        for line in lines:
            lo = max(1, line - 1)
            hi = min(len(source), line + 1)
            out[f"{file}:{line}"] = fingerprint(source[lo - 1 : hi])
    return out


def check_pinned(cfg: dict, sha: str, allow: bool) -> tuple[bool, str | None]:
    pinned = cfg.get("pinned_sha")
    if allow or not pinned or pinned == sha:
        return True, None
    return False, f"corpus_sha_drift: {sha} != pinned {pinned}"


def is_live_corpus(
    name: str, requested_live: bool, has_identity: bool, has_manifest: bool
) -> bool:
    return requested_live or (name == "prism" and not has_identity and not has_manifest)


def initial_run_invalid_reasons(
    pinned_ok: bool,
    pinned_reason: str | None,
    source_drift: list[str],
    live_tree: bool,
) -> list[str]:
    return (
        ([] if pinned_ok else [pinned_reason])
        + source_drift
        + (["live_corpus"] if live_tree else [])
    )


SCORED_PROBE_STATUSES = frozenset({"ok", "inventory_miss"})
ORACLE_ANSWER_STATUSES = SCORED_PROBE_STATUSES | {"sut_error"}
ORACLE_FAILURE_STATUSES = frozenset(
    {"oracle_error", "oracle_timeout", "seed_unaddressable"}
)
PROBE_STATUS_VOCABULARY = ORACLE_ANSWER_STATUSES | ORACLE_FAILURE_STATUSES


def oracle_failure_outcome(exc: Exception) -> str:
    return "oracle_timeout" if isinstance(exc, OracleTimeout) else "oracle_error"


def sut_failure_outcome(exc: Exception) -> str:
    return "sut_timeout" if isinstance(exc, SutTimeout) else "sut_error"


def sample_per_stratum(requested: int | None, quick: bool, default: int) -> int:
    return requested or (3 if quick else default)


def spotcheck_cap(quick: bool) -> int:
    return 10 if quick else 25


def seed_is_unaddressable(
    prism_function,
    unaddressable_seeds: set,
    duplicate_named_selection: bool,
) -> bool:
    return prism_function in unaddressable_seeds or (
        prism_function is not None and duplicate_named_selection
    )


def probe_outcome(prism_function) -> str:
    return "inventory_miss" if prism_function is None else "ok"


def baseline_error_reasons(
    failures: dict, m3: dict, pinned: list[dict], probes: dict
) -> tuple[list[str], dict[str, dict]]:
    reasons = []
    if any(pid.startswith("docsym:") for pid in failures):
        reasons.append("oracle_inventory_incomplete")
    if any(failure["outcome"] == "oracle_timeout" for failure in failures.values()):
        reasons.append("oracle_timeout")
    if any(failure["outcome"] == "sut_timeout" for failure in failures.values()):
        reasons.append("sut_timeout")
    if any(
        failure["outcome"] == "seed_unaddressable" for failure in failures.values()
    ):
        reasons.append("seed_unaddressable")
    if any(pid.startswith("member:") for pid in failures):
        reasons.append("member_sample_error")
    if any(
        checked.get("verdict") in ("oracle_timeout", "oracle_error")
        for checked in m3["checked"]
    ):
        reasons.append("m3_oracle_error")
    pinned_failures = {}
    for row in pinned:
        if row.get("outcome") in (
            "error",
            "oracle_error",
            "oracle_timeout",
            "sut_error",
            "sut_timeout",
        ):
            reasons.append("pinned_" + row["outcome"])
            pinned_failures["pinned:" + row["id"]] = row
    if not any(
        probe.get("outcome") in ("ok", "inventory_miss")
        for pid, probe in probes.items()
        if not pid.startswith("_")
    ):
        reasons.append("no_scored_probes")
    return reasons, pinned_failures


def invalid_exception_metrics(exc: Exception) -> tuple[float, float, str]:
    return (
        float(not isinstance(exc, (SutError, OSError, ValueError))),
        float(isinstance(exc, SutError)),
        "oracle_timeout"
        if isinstance(exc, OracleTimeout)
        else "sut_error"
        if isinstance(exc, (SutError, SutAmbiguous))
        else "input_unavailable"
        if isinstance(exc, (OSError, ValueError))
        else "oracle_unavailable",
    )


def _pinned_accounting_outcome(row: dict) -> str | None:
    outcome = row.get("outcome")
    if outcome == "error":
        side = row.get("error_side")
        return f"{side}_error" if side in {"oracle", "sut"} else None
    if outcome in {"sut_error", "sut_timeout"}:
        return "sut_error"
    if outcome in {"oracle_error", "oracle_timeout"}:
        return outcome
    return None


def _pending_for_probe(
    probe: dict, corpus: str, adjudications: list, site_fps: dict | None = None
) -> list[dict]:
    site_fps = site_fps or {}
    if probe.get("outcome") == "inventory_miss":
        return []
    direction = probe["direction"]
    prism_edges = _edges(probe["prism_sites"], direction)
    diff = site_compare(prism_edges, _edges(probe["oracle_sites"], direction))
    scoped = [
        record
        for record in adjudications
        if record.corpus == corpus
        and record.measurement == direction
        and record.seed_def == probe["seed_def"]
    ]
    adjudicated = {(record.direction, record.site) for record in scoped}
    reanchored = reanchor_map(diff.fp, diff.fn, scoped, site_fps)
    pending = []
    for file, line in sorted(diff.fp):
        key = ("prism_only", _site_key(file, line))
        if key not in adjudicated and key not in reanchored:
            record = {
                "corpus": corpus,
                "measurement": direction,
                "direction": "prism_only",
                "seed_def": probe["seed_def"],
                "site": key[1],
                "site_fingerprint": site_fps.get(key[1]),
            }
            dispatch_kind = _dispatch_kind_for_site(
                probe.get("prism_sites", []), file, line
            )
            if dispatch_kind:
                record["dispatch_kind"] = dispatch_kind
            if _tier_for_site(prism_edges, file, line) == "candidate":
                record["tier"] = "candidate"
            pending.append(record)
    for file, line in sorted(diff.fn):
        key = ("oracle_only", _site_key(file, line))
        if key not in adjudicated and key not in reanchored:
            record = {
                "corpus": corpus,
                "measurement": direction,
                "direction": "oracle_only",
                "seed_def": probe["seed_def"],
                "site": key[1],
                "site_fingerprint": site_fps.get(key[1]),
            }
            dispatch_kind = _dispatch_kind_for_site(
                probe.get("oracle_sites", []), file, line
            )
            if dispatch_kind:
                record["dispatch_kind"] = dispatch_kind
            pending.append(record)
    return pending


def _compute_m2_and_pending(
    probes: dict, adjudications: list, site_fps: dict | None = None
) -> tuple[dict, list[dict], int]:
    """Pure replay over stored probes plus the current adjudication store."""
    site_fps = site_fps or {}
    corpus = probes.get("_corpus", "?")
    shortfalls = {
        name: max(0, meta.get("target", 0) - meta.get("eligible_symbols", 0))
        for name, meta in probes.get("_strata", {}).items()
    }
    pending_records: list[dict] = []
    out: dict = {}
    stale_adjudications = 0
    scored = {"ok", "inventory_miss"}
    for direction in ("callers", "callees"):
        strata: dict = {}
        for pid, probe in sorted(probes.items()):
            if (
                pid.startswith("_")
                or probe.get("outcome") not in scored
                or probe.get("direction") != direction
            ):
                continue
            prism_edges = _edges(probe["prism_sites"], direction)
            oracle_edges = _edges(probe["oracle_sites"], direction)
            result = site_compare(prism_edges, oracle_edges)
            exact_edges = [edge for edge in prism_edges if edge_tier(edge) == "exact"]
            candidate_edges = [
                edge for edge in prism_edges if edge_tier(edge) == "candidate"
            ]
            exact_result = site_compare(exact_edges, oracle_edges)
            candidate_result = site_compare(candidate_edges, oracle_edges)
            prism_functions = {tuple(value) for value in probe.get("prism_functions", [])}
            oracle_functions = {
                tuple(value) for value in probe.get("oracle_functions", [])
            }
            function_raw = _pair_metrics(prism_functions, oracle_functions)
            stratum = strata.setdefault(
                probe["stratum"],
                {
                    "tp": 0,
                    "fp": 0,
                    "fn": 0,
                    "corr_tp": 0,
                    "corr_fp": 0,
                    "corr_fn": 0,
                    "pending": 0,
                    "function_tp": 0,
                    "function_fp": 0,
                    "function_fn": 0,
                    "exact_tp": 0,
                    "exact_fp": 0,
                    "exact_fn": 0,
                    "candidate_count": 0,
                    "candidate_confirmed": 0,
                    "candidate_unconfirmed": 0,
                },
            )
            stratum["tp"] += len(result.tp)
            stratum["fp"] += len(result.fp)
            stratum["fn"] += len(result.fn)
            stratum["exact_tp"] += len(exact_result.tp)
            stratum["exact_fp"] += len(exact_result.fp)
            stratum["exact_fn"] += len(exact_result.fn)
            stratum["candidate_count"] += len(candidate_result.tp) + len(
                candidate_result.fp
            )
            stratum["candidate_confirmed"] += len(candidate_result.tp)
            stratum["candidate_unconfirmed"] += len(candidate_result.fp)
            corrected = apply_verdicts(
                tp=len(result.tp),
                fp_sites=result.fp,
                fn_sites=result.fn,
                records=adjudications,
                corpus=corpus,
                measurement=direction,
                seed_def=probe["seed_def"],
                site_fps=site_fps,
            )
            stratum["corr_tp"] += corrected.tp
            stratum["corr_fp"] += corrected.fp
            stratum["corr_fn"] += corrected.fn
            stratum["pending"] += corrected.pending
            stale_adjudications += corrected.stale
            stratum["function_tp"] += function_raw["tp"]
            stratum["function_fp"] += function_raw["fp"]
            stratum["function_fn"] += function_raw["fn"]
            pending_records.extend(
                _pending_for_probe(probe, corpus, adjudications, site_fps)
            )
        out[direction] = {}
        for name, stratum in sorted(strata.items()):
            function_metrics = precision_recall(
                stratum["function_tp"],
                stratum["function_fp"],
                stratum["function_fn"],
            )
            out[direction][name] = {
                "raw": precision_recall(stratum["tp"], stratum["fp"], stratum["fn"]),
                "corrected": precision_recall(
                    stratum["corr_tp"], stratum["corr_fp"], stratum["corr_fn"]
                ),
                "function": {"raw": function_metrics, "corrected": function_metrics},
                "pending": stratum["pending"],
                "shortfall": shortfalls.get(name, 0),
                "exact_tier": {
                    "raw": precision_recall(
                        stratum["exact_tp"], stratum["exact_fp"], stratum["exact_fn"]
                    )
                },
                "candidate_tier": {
                    "count": stratum["candidate_count"],
                    "oracle_confirmed": stratum["candidate_confirmed"],
                    "oracle_unconfirmed": stratum["candidate_unconfirmed"],
                },
            }
    return out, pending_records, stale_adjudications


def compute_m2_from_probes(
    probes: dict, adjudications: list, site_fps: dict | None = None
) -> dict:
    """Pure: stored raw sites -> per-direction per-stratum raw+corrected metrics."""
    return _compute_m2_and_pending(probes, adjudications, site_fps)[0]


def recompute_metrics_from_stored(
    stored: dict, eval_dir: Path | None = None
) -> dict:
    eval_dir = eval_dir or EVAL_DIR
    stored = readmit_stored(stored, eval_dir)
    if "probes" not in stored:
        return stored
    adjudications = load_records(eval_dir / "adjudications.jsonl")
    site_fps = stored.get("site_fingerprints", {})
    m2, pending, stale = _compute_m2_and_pending(
        stored["probes"], adjudications, site_fps
    )
    meta = {**stored.get("meta", {}), "stale_adjudications": stale}
    return {**stored, "meta": meta, "m2": m2, "pending": pending}


def resolve_capability(oracle, overlay_probe_ok: bool, inventory: list) -> bool:
    """Two-stage capability check: overlay probe, then one real symbol."""
    if overlay_probe_ok:
        return True
    for function in inventory:
        if function.name is not None:
            try:
                oracle.callers(function)
                return True
            except OracleTimeout:
                raise
            except (OracleError, LspError):
                return False
    return False


@dataclass(frozen=True)
class OracleInventoryResult:
    inventory: list[FunctionDef]
    failed_files: list[str]

    @property
    def failure_count(self) -> int:
        return len(self.failed_files)


def oracle_inventory(
    oracle, files: list[str], accounting: CorpusAccounting | None = None
) -> OracleInventoryResult:
    inventory = []
    failed_files = []
    for file in files:
        try:
            inventory.extend(oracle.document_symbols(file))
        except OracleError:
            failed_files.append(file)
            if accounting is not None:
                accounting.record(f"docsym:{file}", "oracle_error")
    return OracleInventoryResult(inventory, failed_files)


def _stored_sites(edges: list[CallEdge], direction: str) -> list[list]:
    out = []
    for edge in edges:
        if direction != "callers" and edge.other_def is None:
            continue
        site = [
            edge.call_site.file,
            edge.call_site.start_line,
            edge.call_site.end_line,
        ]
        meta = {}
        if edge.resolution_kind is not None:
            meta["resolution_kind"] = edge.resolution_kind
        if edge.score is not None:
            meta["score"] = edge.score
        dispatch_kind = getattr(edge, "dispatch_kind", None)
        if dispatch_kind is not None:
            meta["dispatch_kind"] = dispatch_kind
        if meta:
            site.append(meta)
        out.append(site)
    return out


def _stored_functions(
    edges: list[CallEdge], inventory: list[FunctionDef]
) -> list[list]:
    return [list(pair) for pair in sorted(caller_fn_sets(edges, inventory))]


def package_dirs(root: str) -> set[str]:
    dirs = set()
    for dirpath, _dirnames, filenames in os.walk(root):
        if "__init__.py" not in filenames:
            continue
        relative = os.path.relpath(dirpath, root).replace(os.sep, "/")
        if relative != ".":
            dirs.add(relative)
    return dirs


def match_snapshot_to_prism(
    snapshot: list[FunctionDef], prism_inventory: list[FunctionDef]
) -> dict[FunctionDef, FunctionDef]:
    matcher = SelectionMatcher(prism_inventory)
    matched = {}
    for function in snapshot:
        match = matcher.match(function)
        if match is not None:
            matched[function] = match
    return matched


def matrix_result_to_json(result) -> dict:
    probe = getattr(result, "probe", "callers")
    if probe == "callers":
        got = [list(site) for site in sorted(result.got)]
        expected = [list(site) for site in sorted(result.expected)]
        got_kinds = {
            f"{file}:{line}": kind
            for (file, line), kind in sorted(result.got_kinds.items())
        }
    else:
        got = result.got
        expected = result.expected
        got_kinds = {}
    row = {
        "capability": result.capability,
        "language": result.language,
        "outcome": result.outcome,
        "probe": probe,
        "got": got,
        "expected": expected,
        "got_kinds": got_kinds,
        "expected_resolution_kind": result.expected_resolution_kind,
        "forbid_resolution_kind": result.forbid_resolution_kind,
    }
    fixture_id = getattr(result, "fixture_id", None)
    if fixture_id is not None:
        row["id"] = fixture_id
    return row


def run_m3_spotcheck(
    probes: dict,
    oracle,
    snapshot: list[FunctionDef],
    corpus_root: str,
    cap: int,
) -> dict:
    """Spot-check prism-only caller sites against oracle definitions."""
    line_counts = Counter(
        (function.location.file, function.selection_line) for function in snapshot
    )
    by_seed = {seed_id(function, line_counts): function for function in snapshot}
    counts = {
        "confirmed_tp": 0,
        "confirmed_fp": 0,
        "ambiguous": 0,
        "alias_site": 0,
    }
    checked = []
    for pid, probe in sorted(probes.items()):
        if len(checked) >= cap:
            break
        if pid.startswith("_") or probe.get("direction") != "callers":
            continue
        if probe.get("outcome") not in {"ok", "inventory_miss"}:
            continue
        diff = site_compare(
            _edges(probe["prism_sites"], "callers"),
            _edges(probe["oracle_sites"], "callers"),
        )
        seed = by_seed.get(probe["seed_def"])
        if seed is None or seed.name is None:
            continue
        for file, line in sorted(diff.fp):
            if len(checked) >= cap:
                break
            source = (
                (Path(corpus_root) / file)
                .read_text(encoding="utf-8", errors="replace")
                .splitlines()
            )
            text = source[line - 1] if 1 <= line <= len(source) else ""
            char = find_call_position(text, seed.name)
            definitions = []
            if char is not None:
                try:
                    definitions = oracle.definitions_at(
                        snapshot, file, line, utf16_column(text, char)
                    )
                except OracleError as exc:
                    verdict = (
                        "oracle_timeout"
                        if isinstance(exc, OracleTimeout)
                        else "oracle_error"
                    )
                else:
                    verdict = classify_site(text, seed.name, definitions, seed)
            else:
                verdict = classify_site(text, seed.name, definitions, seed)
            counts[verdict] = counts.get(verdict, 0) + 1
            checked.append(
                {"probe": pid, "site": f"{file}:{line}", "verdict": verdict}
            )
    return {"cap": cap, "checked": checked, "counts": counts}


def summarize_m2(m2: dict) -> dict:
    result = {}
    for direction, strata in m2.items():
        result[direction] = {}
        for tier in ("raw", "exact_tier"):
            rows = [
                stratum["raw"] if tier == "raw" else stratum[tier]["raw"]
                for stratum in strata.values()
            ]
            result[direction][tier] = precision_recall(
                *(sum(row[key] for row in rows) for key in ("tp", "fp", "fn"))
            )
    return result


def select_corpora(cfg, args):
    names = (
        list(cfg["corpus"])
        if args.corpus == "all"
        else [args.corpus]
        if args.corpus
        else cfg.get("quick", {}).get("corpora", ["prism"])
        if args.lang
        else ["prism"]
    )
    if args.lang:
        languages = set(args.lang.split(","))
        names = [
            name for name in names if cfg["corpus"][name]["lang"] in languages
        ]
    if not names:
        raise ValueError("no corpora match the requested languages")
    return names
