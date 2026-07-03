"""Capability matrix runner (spec §2.7): by-construction ground truth, no LSP.
Outcomes: ok | regression (pass case failing -> fails the run) |
expected_gap | flip_candidate (known_fail now passing -> report, update status).

Three probe types (`[case] probe`, P6bc), selected by `[case] probe`
(default "callers" -- the key is absent on all pre-P6bc fixtures, so they
keep loading/running byte-identically):
  - "callers" (default): existing caller-site oracle. Requires `[seed]`;
    forbids `[taint]`/`[module]`.
  - "taint": `prism nav taint-reaches`. Requires `[taint]`; forbids `[seed]`
    and `[[expect.callers]]`.
  - "module_deps": `prism nav module-deps`. Requires `[module]`; forbids
    `[seed]` and `[[expect.callers]]`.
Mixing sections across probe types, or an unknown probe value, is an
explicit `load_case` error rather than a silently-ignored section.
"""
from __future__ import annotations

import tomllib
from dataclasses import dataclass, field
from pathlib import Path

from .model import FunctionDef, Location


MATRIX_LANGUAGES = ["rust", "go", "python", "javascript", "typescript"]
PROBE_TYPES = ("callers", "taint", "module_deps")


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

    # probe == "module_deps"
    module_file: str | None = None
    expect_module_to: set = field(default_factory=set)
    expect_module_forbid_to: set = field(default_factory=set)
    module_exact: bool = False


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


def load_case(toml_path: Path) -> Case:
    d = tomllib.loads(toml_path.read_text())
    probe = d["case"].get("probe", "callers")
    if probe not in PROBE_TYPES:
        raise ValueError(
            f"{toml_path}: unknown [case] probe {probe!r} (expected one of {PROBE_TYPES})"
        )

    has_seed = "seed" in d
    has_taint = "taint" in d
    has_module = "module" in d
    has_expect_callers = bool(d.get("expect", {}).get("callers"))

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
        expect_callers = _expect_callers(d["expect"])
        expected_resolution_kind = d["expect"].get("resolution_kind")
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
            exact=d["expect"].get("exact", True),
            expected_resolution_kind=expected_resolution_kind,
            forbid_resolution_kind=d["expect"].get("forbid_resolution_kind"),
        )

    if probe == "taint":
        if has_seed or has_expect_callers:
            raise ValueError(
                f"{toml_path}: probe=\"taint\" must not define [seed] or expect.callers"
            )
        if not has_taint:
            raise ValueError(f"{toml_path}: probe=\"taint\" requires [taint]")
        expect = d.get("expect", {})
        return Case(
            **common,
            taint_sources=list(d["taint"]["sources"]),
            taint_sinks=list(d["taint"].get("sinks", [])),
            expect_reachability=expect.get("reachability"),
            expect_warning_kinds_present=list(expect.get("warning_kinds_present", [])),
            expect_sanitizers_present=expect.get("sanitizers_present"),
        )

    # probe == "module_deps"
    if has_seed or has_expect_callers:
        raise ValueError(
            f"{toml_path}: probe=\"module_deps\" must not define [seed] or expect.callers"
        )
    if not has_module:
        raise ValueError(f"{toml_path}: probe=\"module_deps\" requires [module]")
    expect = d.get("expect", {})
    module_edges = expect.get("module_edges", [])
    return Case(
        **common,
        module_file=d["module"]["file"],
        expect_module_to={e["to"] for e in module_edges},
        expect_module_forbid_to=set(expect.get("forbid_to", [])),
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


def _format_taint_summary(reachability, warning_kinds, sanitizers_present) -> str:
    """Deterministic, triage-useful got/expected string for taint probes, e.g.
    "BoundaryExited|warnings=InterproceduralBoundary|sanitizers=false". Any
    field left unconstrained on the expected side renders "any" so a
    regression line shows exactly what was (and wasn't) being asserted
    without needing to re-run anything."""
    r = "any" if reachability is None else reachability
    w = ",".join(sorted(warning_kinds)) if warning_kinds else "none"
    if sanitizers_present is None:
        s = "any"
    else:
        s = "true" if sanitizers_present else "false"
    return f"{r}|warnings={w}|sanitizers={s}"


def _run_taint_case(case: Case, lang: str, sut) -> CaseResult:
    ev = sut.taint_reaches(str(case.path), case.taint_sources, case.taint_sinks or None)
    reasoning = ev.get("reasoning") or {}
    got_reachability = reasoning.get("reachability")
    got_warnings = sorted({_kind_discriminant(w.get("kind")) for w in ev.get("warnings", [])})
    got_sanitizers = _sanitizers_present(reasoning)

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
    matched = reachability_ok and warnings_ok and sanitizers_ok

    got_reachability_str = "None" if got_reachability is None else got_reachability
    got = _format_taint_summary(got_reachability_str, got_warnings, got_sanitizers)
    expected = _format_taint_summary(
        case.expect_reachability, case.expect_warning_kinds_present, case.expect_sanitizers_present
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
            else:
                results.append(_run_module_case(case, lang, sut))
    return results
