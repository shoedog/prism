"""Capability matrix runner (spec §2.7): by-construction ground truth, no LSP.
Outcomes: ok | regression (pass case failing -> fails the run) |
expected_gap | flip_candidate (known_fail now passing -> report, update status).

Four probe types (`[case] probe`, P6bc), selected by `[case] probe`
(default "callers" -- the key is absent on all pre-P6bc fixtures, so they
keep loading/running byte-identically):
  - "callers" (default): existing caller-site oracle. Requires `[seed]`;
    forbids `[taint]`/`[module]`.
  - "taint": `prism nav taint-reaches`. Requires `[taint]`; forbids `[seed]`
    and `[[expect.callers]]`.
  - "module_deps": `prism nav module-deps`. Requires `[module]`; forbids
    `[seed]` and `[[expect.callers]]`.
  - "dfg": `prism nav dfg-stats --edges`. Requires a non-empty
    `[[expect.edges]]`; forbids `[seed]`, `[taint]`, and `[module]`.
Mixing sections across probe types, or an unknown probe value, is an
explicit `load_case` error rather than a silently-ignored section.
"""
from __future__ import annotations

import json
import subprocess
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

from .model import FunctionDef, Location


MATRIX_LANGUAGES = ["rust", "go", "python", "javascript", "typescript"]
PROBE_TYPES = ("callers", "taint", "module_deps", "dfg")


@dataclass
class Case:
    path: Path
    language: str
    capability: str
    status: str
    probe: str = "callers"

    # probe == "callers"
    seed_symbol: str | None = None
    seed_file: str | None = None
    seed_line: int | None = None
    expect_callers: set = field(default_factory=set)
    exact: bool = True
    expected_resolution_kind: str | None = None
    forbid_resolution_kind: str | None = None

    # probe == "taint"
    taint_sources: list = field(default_factory=list)
    taint_sinks: list = field(default_factory=list)
    expect_reachability: str | None = None
    expect_warning_kinds_present: list = field(default_factory=list)
    expect_sanitizers_present: bool | None = None
    frontier_count_min: int | None = None

    # probe == "module_deps"
    module_file: str | None = None
    expect_module_to: set = field(default_factory=set)
    expect_module_forbid_to: set = field(default_factory=set)
    module_exact: bool = False

    # probe == "dfg"
    expect_dfg_edges: list = field(default_factory=list)
    expect_dfg_stats: dict = field(default_factory=dict)
    expect_dfg_findings: list = field(default_factory=list)


@dataclass
class CaseResult:
    capability: str
    language: str
    outcome: str
    got: object
    expected: object
    got_kinds: dict
    expected_resolution_kind: str | None
    forbid_resolution_kind: str | None
    probe: str = "callers"


def _expect_callers(expect: dict) -> set:
    callers = expect.get("callers", [])
    return {(c["file"], c["line"]) for c in callers}


# F2 (codex BLOCKER 2 + opus m2 + controller finding): schema whitelists, so a
# typo'd or foreign key is a load_case ValueError rather than a silently
# ignored key that leaves a fixture assert-nothing or empty-subset (which
# trivially "passes"). `KNOWN_TOP_SECTIONS` catches any top-level TOML table
# outside the whole known vocabulary; the per-probe checks further reject a
# *known* section that belongs to a different probe (e.g. [module] on a taint
# fixture) -- "both directions" per finding (a).
# `[[expect.edges]]` is nested below the known top-level `expect` table.
KNOWN_TOP_SECTIONS = {"case", "seed", "taint", "module", "expect"}
TAINT_SECTION_KEYS = {"sources", "sinks"}
MODULE_SECTION_KEYS = {"file"}
EXPECT_KEYS_BY_PROBE = {
    "callers": {"callers", "exact", "resolution_kind", "forbid_resolution_kind"},
    "taint": {"reachability", "warning_kinds_present", "sanitizers_present", "frontier_count_min"},
    "module_deps": {"module_edges", "forbid_to", "exact"},
    "dfg": {"edges", "stats", "findings"},
}
DFG_EDGE_KEYS = {"from", "to", "confidence", "doubt", "kill_line", "present"}
DFG_STATS_KEYS = {"dfg_label_loop_carried_min"}
DFG_FINDING_KEYS = {"from", "to", "confidence", "crossed_unlabeled", "present"}
# Controller adjudication (e): the only wire `Reachability` variants plus this
# harness's own "None" (JSON null / frontier mode) sentinel. Anything else is
# a typo'd sentinel that would otherwise silently never match. "Sanitized" (P10)
# is a witness-mode-only downgrade of "Reached", proven path-specific by a
# chain-window walk -- see src/reasoning/sanitizer_walk.rs.
VALID_REACHABILITY_VALUES = {"Reached", "NotReached", "BoundaryExited", "Sanitized", "None"}


def _reject_unknown_keys(toml_path: Path, probe: str, section: str, present: dict, allowed: set) -> None:
    unknown = set(present) - allowed
    if unknown:
        raise ValueError(
            f'{toml_path}: probe="{probe}" [{section}] has unknown key(s) {sorted(unknown)} '
            f"(allowed: {sorted(allowed)})"
        )


def load_case(toml_path: Path) -> Case:
    d = tomllib.loads(toml_path.read_text())
    probe = d["case"].get("probe", "callers")
    if probe not in PROBE_TYPES:
        raise ValueError(
            f"{toml_path}: unknown [case] probe {probe!r} (expected one of {PROBE_TYPES})"
        )

    unknown_sections = set(d) - KNOWN_TOP_SECTIONS
    if unknown_sections:
        raise ValueError(
            f"{toml_path}: unknown top-level section(s) {sorted(unknown_sections)} "
            f"(allowed: {sorted(KNOWN_TOP_SECTIONS)})"
        )

    has_seed = "seed" in d
    has_taint = "taint" in d
    has_module = "module" in d
    # (d): key PRESENCE, not `bool(...)` -- an empty `callers = []` on a
    # taint/module_deps probe must still be rejected as expect.callers defined
    # where it must not be.
    has_expect_callers = "callers" in d.get("expect", {})

    common = dict(
        path=toml_path.parent,
        language=d["case"]["language"],
        capability=d["case"]["capability"],
        status=d["case"]["status"],
        probe=probe,
    )

    if probe == "callers":
        if has_taint or has_module:
            raise ValueError(
                f"{toml_path}: probe=\"callers\" must not define [taint] or [module]"
            )
        if not has_seed:
            raise ValueError(f"{toml_path}: probe=\"callers\" requires [seed]")
        if "expect" not in d:
            raise ValueError(f'{toml_path}: probe="callers" requires [expect]')
        expect = d["expect"]
        _reject_unknown_keys(toml_path, probe, "expect", expect, EXPECT_KEYS_BY_PROBE["callers"])
        expect_callers = _expect_callers(expect)
        expected_resolution_kind = expect.get("resolution_kind")
        if expected_resolution_kind is not None and not expect_callers:
            raise ValueError(
                f"{toml_path}: expect.resolution_kind requires at least one expect.callers entry"
            )
        return Case(
            **common,
            seed_symbol=d["seed"]["symbol"],
            seed_file=d["seed"]["file"],
            seed_line=d["seed"]["line"],
            expect_callers=expect_callers,
            exact=expect.get("exact", True),
            expected_resolution_kind=expected_resolution_kind,
            forbid_resolution_kind=expect.get("forbid_resolution_kind"),
        )

    if probe == "taint":
        if has_seed or has_expect_callers:
            raise ValueError(
                f"{toml_path}: probe=\"taint\" must not define [seed] or expect.callers"
            )
        if has_module:
            raise ValueError(f'{toml_path}: probe="taint" must not define [module]')
        if not has_taint:
            raise ValueError(f"{toml_path}: probe=\"taint\" requires [taint]")

        taint = d["taint"]
        _reject_unknown_keys(toml_path, probe, "taint", taint, TAINT_SECTION_KEYS)
        expect = d.get("expect", {})
        _reject_unknown_keys(toml_path, probe, "expect", expect, EXPECT_KEYS_BY_PROBE["taint"])

        expect_reachability = expect.get("reachability")
        if expect_reachability is not None and expect_reachability not in VALID_REACHABILITY_VALUES:
            raise ValueError(
                f"{toml_path}: expect.reachability {expect_reachability!r} is not one of "
                f"{sorted(VALID_REACHABILITY_VALUES)}"
            )
        warning_kinds_present = list(expect.get("warning_kinds_present", []))
        sanitizers_present = expect.get("sanitizers_present")
        frontier_count_min = expect.get("frontier_count_min")
        # F1(b) (codex MAJOR): `bool` is a subclass of `int` in Python, so an
        # unchecked `frontier_count_min = true` typo would silently load and
        # then (via the runtime `type(...) is int` check below) never match
        # -- a confusing schema-adjacent failure mode. Reject at load time,
        # same tier as the other schema checks. A floor of 0 is likewise
        # rejected: `frontier_count >= 0` is always true for a present int
        # field, so it asserts nothing (consistent with this harness's
        # >=1-assertion doctrine for optional expect fields).
        if frontier_count_min is not None:
            if type(frontier_count_min) is not int:
                raise ValueError(
                    f"{toml_path}: expect.frontier_count_min must be an int, "
                    f"got {frontier_count_min!r} ({type(frontier_count_min).__name__})"
                )
            if frontier_count_min < 1:
                raise ValueError(
                    f"{toml_path}: expect.frontier_count_min must be >= 1 "
                    f"(a floor of 0 asserts nothing), got {frontier_count_min}"
                )
        if (
            expect_reachability is None
            and not warning_kinds_present
            and sanitizers_present is None
            and frontier_count_min is None
        ):
            raise ValueError(
                f'{toml_path}: probe="taint" [expect] asserts nothing -- provide at least one of '
                "reachability, warning_kinds_present, sanitizers_present, frontier_count_min"
            )

        return Case(
            **common,
            taint_sources=list(taint["sources"]),
            taint_sinks=list(taint.get("sinks", [])),
            expect_reachability=expect_reachability,
            expect_warning_kinds_present=warning_kinds_present,
            expect_sanitizers_present=sanitizers_present,
            frontier_count_min=frontier_count_min,
        )

    if probe == "dfg":
        if has_seed or has_taint or has_module or has_expect_callers:
            raise ValueError(
                f'{toml_path}: probe="dfg" must not define [seed], [taint], [module], '
                "or expect.callers"
            )
        if "expect" not in d:
            raise ValueError(f'{toml_path}: probe="dfg" requires [expect]')
        expect = d["expect"]
        _reject_unknown_keys(toml_path, probe, "expect", expect, EXPECT_KEYS_BY_PROBE["dfg"])
        edges = list(expect.get("edges", []))
        if not edges:
            raise ValueError(
                f'{toml_path}: probe="dfg" requires at least one [[expect.edges]] entry'
            )
        for index, edge in enumerate(edges):
            _reject_unknown_keys(
                toml_path, probe, f"expect.edges[{index}]", edge, DFG_EDGE_KEYS
            )
            if not isinstance(edge.get("from"), str) or not isinstance(edge.get("to"), str):
                raise ValueError(
                    f"{toml_path}: expect.edges[{index}] requires string from/to endpoints"
                )
            if "present" in edge and type(edge["present"]) is not bool:
                raise ValueError(
                    f"{toml_path}: expect.edges[{index}].present must be a bool"
                )
        stats = dict(expect.get("stats", {}))
        _reject_unknown_keys(
            toml_path, probe, "expect.stats", stats, DFG_STATS_KEYS
        )
        loop_carried_min = stats.get("dfg_label_loop_carried_min")
        if loop_carried_min is not None and (
            type(loop_carried_min) is not int or loop_carried_min < 1
        ):
            raise ValueError(
                f"{toml_path}: expect.stats.dfg_label_loop_carried_min "
                f"must be an int >= 1, got {loop_carried_min!r}"
            )
        findings = list(expect.get("findings", []))
        for index, finding in enumerate(findings):
            _reject_unknown_keys(
                toml_path, probe, f"expect.findings[{index}]", finding,
                DFG_FINDING_KEYS,
            )
            if not isinstance(finding.get("from"), str) or not isinstance(
                finding.get("to"), str
            ):
                raise ValueError(
                    f"{toml_path}: expect.findings[{index}] requires string from/to endpoints"
                )
            for bool_key in ("crossed_unlabeled", "present"):
                if bool_key in finding and type(finding[bool_key]) is not bool:
                    raise ValueError(
                        f"{toml_path}: expect.findings[{index}].{bool_key} must be a bool"
                    )
        return Case(
            **common,
            expect_dfg_edges=edges,
            expect_dfg_stats=stats,
            expect_dfg_findings=findings,
        )

    # probe == "module_deps"
    if has_seed or has_expect_callers:
        raise ValueError(
            f"{toml_path}: probe=\"module_deps\" must not define [seed] or expect.callers"
        )
    if has_taint:
        raise ValueError(f'{toml_path}: probe="module_deps" must not define [taint]')
    if not has_module:
        raise ValueError(f"{toml_path}: probe=\"module_deps\" requires [module]")

    module = d["module"]
    _reject_unknown_keys(toml_path, probe, "module", module, MODULE_SECTION_KEYS)
    expect = d.get("expect", {})
    _reject_unknown_keys(toml_path, probe, "expect", expect, EXPECT_KEYS_BY_PROBE["module_deps"])

    module_edges = expect.get("module_edges", [])
    forbid_to = set(expect.get("forbid_to", []))
    if not module_edges and not forbid_to:
        raise ValueError(
            f'{toml_path}: probe="module_deps" [expect] asserts nothing -- provide at least one of '
            "module_edges, forbid_to"
        )
    return Case(
        **common,
        module_file=module["file"],
        expect_module_to={e["to"] for e in module_edges},
        expect_module_forbid_to=forbid_to,
        module_exact=expect.get("exact", False),
    )


def _status_outcome(status: str, matched: bool) -> str:
    if status == "pass":
        return "ok" if matched else "regression"
    return "flip_candidate" if matched else "expected_gap"


def _run_callers_case(case: Case, lang: str, sut) -> CaseResult:
    seed = FunctionDef(case.seed_symbol, "function", None,
                       Location(case.seed_file, case.seed_line, case.seed_line),
                       case.seed_line)
    edges = sut.callers(str(case.path), seed)
    got = {(e.call_site.file, e.call_site.start_line) for e in edges}
    got_kinds = {
        (e.call_site.file, e.call_site.start_line): e.resolution_kind
        for e in edges
        if e.resolution_kind is not None
    }
    matched = got == case.expect_callers if case.exact \
        else case.expect_callers <= got
    if matched and case.expected_resolution_kind is not None:
        matched = all(
            got_kinds.get(site) == case.expected_resolution_kind
            for site in case.expect_callers
        )
    if matched and case.forbid_resolution_kind is not None:
        matched = all(
            kind != case.forbid_resolution_kind
            for kind in got_kinds.values()
        )
    outcome = _status_outcome(case.status, matched)
    return CaseResult(case.capability, lang, outcome, got, case.expect_callers, got_kinds,
                      case.expected_resolution_kind, case.forbid_resolution_kind, probe="callers")


def _kind_discriminant(kind) -> str:
    """Reduce serde's externally-tagged `WarningKind` JSON to one discriminant
    string. Fieldless variants serialize as a bare string (e.g.
    "ParseQuality"). `WarningKind::Reasoning(ReasoningWarning)` is the one
    variant carrying a payload, and that payload is itself an externally
    tagged enum: {"Reasoning": {"Cleansed": {fields...}}}. Unwrap exactly the
    "Reasoning" wrapper to reach the ReasoningWarning variant name
    ("Cleansed") -- do NOT recurse into `fields`, which may itself contain a
    single string field and falsely look like another tag to unwrap."""
    if isinstance(kind, str):
        return kind
    if isinstance(kind, dict) and len(kind) == 1:
        tag, inner = next(iter(kind.items()))
        if tag == "Reasoning" and isinstance(inner, dict) and len(inner) == 1:
            return next(iter(inner.keys()))
        return tag
    return "unknown"


def _sanitizers_present(reasoning: dict) -> bool:
    for sink in reasoning.get("per_sink", []):
        for source in sink.get("sources", []):
            if source.get("sanitizers_present_in_source_fn"):
                return True
    return False


def _format_taint_summary(reachability, warning_kinds, sanitizers_present, frontier=None) -> str:
    """Deterministic, triage-useful got/expected string for taint probes, e.g.
    "BoundaryExited|warnings=InterproceduralBoundary|sanitizers=false". Any
    field left unconstrained on the expected side renders "any" so a
    regression line shows exactly what was (and wasn't) being asserted
    without needing to re-run anything. `frontier` is only appended when the
    case actually asserts on it (frontier_count_min), to keep the format
    byte-identical for every fixture that doesn't -- see F1(c)."""
    r = "any" if reachability is None else reachability
    w = ",".join(sorted(warning_kinds)) if warning_kinds else "none"
    if sanitizers_present is None:
        s = "any"
    else:
        s = "true" if sanitizers_present else "false"
    base = f"{r}|warnings={w}|sanitizers={s}"
    if frontier is not None:
        base += f"|frontier={frontier}"
    return base


def _run_taint_case(case: Case, lang: str, sut) -> CaseResult:
    """F1 (codex BLOCKER 1): a taint probe must not report `ok` when the wire
    evidence doesn't actually carry a taint_reaches reasoning payload. Missing
    `query`, missing/null `reasoning`, or a `reasoning` dict without a
    `reachability` key are all hard mismatches -- never coerced to `{}` and
    never compared as "null equals the None sentinel". Only once evidence is
    confirmed to actually be reasoning-bearing taint_reaches output do the
    per-field comparisons (including the optional frontier_count_min floor)
    run."""
    ev = sut.taint_reaches(str(case.path), case.taint_sources, case.taint_sinks or None)
    reasoning = ev.get("reasoning")
    reasoning_ok = isinstance(reasoning, dict) and "reachability" in reasoning
    query_ok = ev.get("query") == "taint_reaches"
    # F2/Item 2: shared triage formatting for the frontier floor -- reused by
    # both the missing-evidence branch (below) and the normal mismatch branch
    # so a reader sees "frontier=>=N" consistently in either case.
    frontier_expected = (
        f">={case.frontier_count_min}" if case.frontier_count_min is not None else None
    )

    if not (query_ok and reasoning_ok):
        reasons = []
        if not query_ok:
            reasons.append(f"query={ev.get('query')!r} (expected 'taint_reaches')")
        if not reasoning_ok:
            reasons.append("reasoning absent/null/missing-reachability-key")
        got = "MISSING_EVIDENCE|" + ";".join(reasons)
        expected = _format_taint_summary(
            case.expect_reachability, case.expect_warning_kinds_present,
            case.expect_sanitizers_present, frontier_expected,
        )
        outcome = _status_outcome(case.status, False)
        return CaseResult(case.capability, lang, outcome, got, expected, {}, None, None, probe="taint")

    got_reachability = reasoning.get("reachability")
    got_warnings = sorted({_kind_discriminant(w.get("kind")) for w in ev.get("warnings", [])})
    got_sanitizers = _sanitizers_present(reasoning)
    got_frontier_count = reasoning.get("frontier_count")

    if case.expect_reachability is None:
        reachability_ok = True
    elif case.expect_reachability == "None":
        reachability_ok = got_reachability is None
    else:
        reachability_ok = got_reachability == case.expect_reachability
    warnings_ok = set(case.expect_warning_kinds_present) <= set(got_warnings)
    sanitizers_ok = (
        case.expect_sanitizers_present is None
        or case.expect_sanitizers_present == got_sanitizers
    )
    # F1(a) (codex MAJOR): `bool` is a subclass of `int` in Python
    # (`isinstance(True, int) is True`), so a malformed wire payload
    # `frontier_count: true` would satisfy `isinstance(..., int) and True >=
    # N` for any N <= 1. Require the exact type so a bool value is always a
    # mismatch, never coerced into a passing int comparison.
    frontier_ok = (
        case.frontier_count_min is None
        or (type(got_frontier_count) is int and got_frontier_count >= case.frontier_count_min)
    )
    matched = reachability_ok and warnings_ok and sanitizers_ok and frontier_ok

    got_reachability_str = "None" if got_reachability is None else got_reachability
    got = _format_taint_summary(
        got_reachability_str, got_warnings, got_sanitizers,
        got_frontier_count if case.frontier_count_min is not None else None,
    )
    expected = _format_taint_summary(
        case.expect_reachability, case.expect_warning_kinds_present, case.expect_sanitizers_present,
        frontier_expected,
    )
    outcome = _status_outcome(case.status, matched)
    return CaseResult(case.capability, lang, outcome, got, expected, {}, None, None, probe="taint")


def _run_module_case(case: Case, lang: str, sut) -> CaseResult:
    ev = sut.module_deps(str(case.path), case.module_file)
    got_files = {it["location"]["file"] for it in ev.get("items", [])}

    edges_ok = got_files == case.expect_module_to if case.module_exact \
        else case.expect_module_to <= got_files
    forbid_ok = not (case.expect_module_forbid_to & got_files)
    matched = edges_ok and forbid_ok

    got = ",".join(sorted(got_files)) if got_files else "none"
    expected = ",".join(sorted(case.expect_module_to)) if case.expect_module_to else "none"
    if case.expect_module_forbid_to:
        expected += f"|forbid={','.join(sorted(case.expect_module_forbid_to))}"
    outcome = _status_outcome(case.status, matched)
    return CaseResult(case.capability, lang, outcome, got, expected, {}, None, None, probe="module_deps")


def _dfg_endpoint(item: dict) -> str:
    path = item.get("path")
    if isinstance(path, dict):
        base = path.get("base", "?")
        fields = path.get("fields", [])
        path = ".".join([base, *fields])
    return f'{item.get("file", "?")}:{item.get("line", "?")}:{path}'


def _dfg_edge_matches(expected: dict, actual: dict) -> bool:
    if _dfg_endpoint(actual.get("from", {})) != expected["from"]:
        return False
    if _dfg_endpoint(actual.get("to", {})) != expected["to"]:
        return False
    return all(
        actual.get(key) == value
        for key, value in expected.items()
        if key not in {"from", "to", "present"}
    )


def _dfg_same_endpoints(expected: dict, actual: dict) -> bool:
    return (
        _dfg_endpoint(actual.get("from", {})) == expected["from"]
        and _dfg_endpoint(actual.get("to", {})) == expected["to"]
    )


def _dfg_expectation_matches(expected: dict, actual: list[dict]) -> bool:
    endpoint_rows = [edge for edge in actual if _dfg_same_endpoints(expected, edge)]
    if not expected.get("present", True):
        return not endpoint_rows
    return bool(endpoint_rows) and all(
        _dfg_edge_matches(expected, edge) for edge in endpoint_rows
    )


def _dfg_expected(case: Case) -> object:
    if not case.expect_dfg_stats and not case.expect_dfg_findings:
        return case.expect_dfg_edges
    return {
        "edges": case.expect_dfg_edges,
        "stats": case.expect_dfg_stats,
        "findings": case.expect_dfg_findings,
    }


def _run_dfg_case(case: Case, lang: str, sut) -> CaseResult:
    if not case.expect_dfg_edges:
        raise ValueError(f'{case.path}: probe="dfg" expected-edge list is empty')

    cache_args = ["--no-cache"] if getattr(sut, "no_cache", False) else []
    command = [
        getattr(sut, "bin", "prism"), "nav", *cache_args, "dfg-stats",
        "--repo", str(case.path), "--edges",
    ]
    try:
        completed = subprocess.run(command, capture_output=True, text=True)
    except OSError as exc:
        got = f"DFG_ORACLE_LAUNCH_FAILED|{exc}"
        return CaseResult(
            case.capability, lang, "regression", got,
            _dfg_expected(case), {}, None, None, probe="dfg",
        )
    if completed.returncode != 0:
        detail = "\n".join(
            part.strip() for part in (completed.stdout, completed.stderr) if part.strip()
        )
        oracle_missing = (
            completed.returncode == 2
            and "unrecognized subcommand 'dfg-stats'" in detail
        )
        kind = "DFG_ORACLE_UNAVAILABLE" if oracle_missing else "DFG_ORACLE_COMMAND_FAILED"
        got = f"{kind}|exit={completed.returncode}|{detail}"
        return CaseResult(
            case.capability, lang,
            "expected_gap" if oracle_missing else _status_outcome(case.status, False), got,
            _dfg_expected(case), {}, None, None, probe="dfg",
        )

    try:
        actual = [json.loads(line) for line in completed.stdout.splitlines() if line.strip()]
    except json.JSONDecodeError as exc:
        got = f"DFG_ORACLE_INVALID_JSONL|{exc}"
        return CaseResult(
            case.capability, lang, _status_outcome(case.status, False), got,
            _dfg_expected(case), {}, None, None, probe="dfg",
        )

    edges_matched = all(
        _dfg_expectation_matches(expected, actual)
        for expected in case.expect_dfg_edges
    )
    got_edges = [
        {
            "from": _dfg_endpoint(edge.get("from", {})),
            "to": _dfg_endpoint(edge.get("to", {})),
            "confidence": edge.get("confidence"),
            "doubt": edge.get("doubt"),
            "kill_line": edge.get("kill_line"),
        }
        for edge in actual
    ]
    supplemental_staged = bool(case.expect_dfg_stats or case.expect_dfg_findings)
    matched = edges_matched and not supplemental_staged
    got = (
        {
            "edges": got_edges,
            "supplemental": "DFG_ORACLE_SUPPLEMENTAL_EXPECTATIONS_STAGED",
        }
        if supplemental_staged
        else got_edges
    )
    return CaseResult(
        case.capability, lang, _status_outcome(case.status, matched), got,
        _dfg_expected(case), {}, None, None, probe="dfg",
    )


def test_dfg_launch_oserror_is_a_regression() -> None:
    """Contract test kept here because §9 permits this matrix module only."""
    from types import SimpleNamespace

    case = Case(
        path=Path("."), language="python", capability="dfg_launch_oserror",
        status="pass", probe="dfg",
        expect_dfg_edges=[{"from": "a.py:1:x", "to": "a.py:2:x"}],
    )
    original = subprocess.run
    try:
        def raise_permission(*_args, **_kwargs):
            raise PermissionError(13, "permission denied", "prism")

        subprocess.run = raise_permission
        result = _run_dfg_case(
            case, "python", SimpleNamespace(bin="prism", no_cache=False)
        )
    finally:
        subprocess.run = original
    assert result.outcome == "regression"
    assert str(result.got).startswith("DFG_ORACLE_LAUNCH_FAILED|")


def test_dfg_unknown_subcommand_requires_prism_usage_exit_code() -> None:
    from types import SimpleNamespace

    case = Case(
        path=Path("."), language="python", capability="dfg_missing_subcommand",
        status="pass", probe="dfg",
        expect_dfg_edges=[{"from": "a.py:1:x", "to": "a.py:2:x"}],
    )
    sut = SimpleNamespace(bin="prism", no_cache=False)
    original = subprocess.run
    try:
        subprocess.run = lambda *_args, **_kwargs: SimpleNamespace(
            returncode=1,
            stdout="",
            stderr="error: unrecognized subcommand 'dfg-stats'",
        )
        wrong_exit = _run_dfg_case(case, "python", sut)
        subprocess.run = lambda *_args, **_kwargs: SimpleNamespace(
            returncode=2,
            stdout="",
            stderr="error: unrecognized subcommand 'dfg-stats'",
        )
        prism_usage = _run_dfg_case(case, "python", sut)
    finally:
        subprocess.run = original
    assert wrong_exit.outcome == "regression"
    assert prism_usage.outcome == "expected_gap"


def test_dfg_mixed_payload_jsonl_is_a_regression() -> None:
    expected = {
        "from": "main.py:2:x", "to": "main.py:3:x",
        "confidence": "nameonly", "doubt": "sameline", "present": True,
    }
    mixed_jsonl = "\n".join([
        json.dumps({
            "from": {"file": "main.py", "line": 2, "path": "x"},
            "to": {"file": "main.py", "line": 3, "path": "x"},
            "confidence": "nameonly", "doubt": "sameline", "kill_line": None,
        }),
        json.dumps({
            "from": {"file": "main.py", "line": 2, "path": "x"},
            "to": {"file": "main.py", "line": 3, "path": "x"},
            "confidence": "exact", "doubt": None, "kill_line": None,
        }),
    ])
    actual = [json.loads(line) for line in mixed_jsonl.splitlines()]
    assert not _dfg_expectation_matches(expected, actual)


def run_matrix(fixtures_root: Path, sut, languages: list[str]) -> list[CaseResult]:
    # Fixture eval must be deterministic: bypass the per-repo nav cache, which
    # otherwise persists pre-change results across binary versions and fakes
    # regressions (S3 Task 13). Restore the SUT's prior setting afterward so
    # corpus measurements (sharing the SUT) keep caching. FakeSut in the
    # self-tests lacks the attribute — guard with getattr/setattr.
    prev_no_cache = getattr(sut, "no_cache", None)
    if prev_no_cache is not None:
        sut.no_cache = True
    try:
        return _run_matrix_inner(fixtures_root, sut, languages)
    finally:
        if prev_no_cache is not None:
            sut.no_cache = prev_no_cache


def _run_matrix_inner(fixtures_root: Path, sut, languages: list[str]) -> list[CaseResult]:
    results = []
    for lang in languages:
        for toml_path in sorted((fixtures_root / lang).glob("*/expected.toml")):
            case = load_case(toml_path)
            if case.probe == "callers":
                results.append(_run_callers_case(case, lang, sut))
            elif case.probe == "taint":
                results.append(_run_taint_case(case, lang, sut))
            elif case.probe == "dfg":
                results.append(_run_dfg_case(case, lang, sut))
            else:
                results.append(_run_module_case(case, lang, sut))
    return results
