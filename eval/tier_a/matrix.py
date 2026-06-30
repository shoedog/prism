"""Capability matrix runner (spec §2.7): by-construction ground truth, no LSP.
Outcomes: ok | regression (pass case failing -> fails the run) |
expected_gap | flip_candidate (known_fail now passing -> report, update status)."""
from __future__ import annotations

import tomllib
from dataclasses import dataclass
from pathlib import Path

from .model import FunctionDef, Location


MATRIX_LANGUAGES = ["rust", "go", "python", "javascript", "typescript"]


@dataclass
class Case:
    path: Path
    language: str
    capability: str
    status: str
    seed_symbol: str
    seed_file: str
    seed_line: int
    expect_callers: set
    exact: bool
    expected_resolution_kind: str | None
    forbid_resolution_kind: str | None


@dataclass
class CaseResult:
    capability: str
    language: str
    outcome: str
    got: set
    expected: set
    got_kinds: dict
    expected_resolution_kind: str | None
    forbid_resolution_kind: str | None


def _expect_callers(expect: dict) -> set:
    callers = expect.get("callers", [])
    return {(c["file"], c["line"]) for c in callers}


def load_case(toml_path: Path) -> Case:
    d = tomllib.loads(toml_path.read_text())
    expect_callers = _expect_callers(d["expect"])
    expected_resolution_kind = d["expect"].get("resolution_kind")
    if expected_resolution_kind is not None and not expect_callers:
        raise ValueError(
            f"{toml_path}: expect.resolution_kind requires at least one expect.callers entry"
        )
    return Case(
        path=toml_path.parent,
        language=d["case"]["language"],
        capability=d["case"]["capability"],
        status=d["case"]["status"],
        seed_symbol=d["seed"]["symbol"],
        seed_file=d["seed"]["file"],
        seed_line=d["seed"]["line"],
        expect_callers=expect_callers,
        exact=d["expect"].get("exact", True),
        expected_resolution_kind=expected_resolution_kind,
        forbid_resolution_kind=d["expect"].get("forbid_resolution_kind"),
    )


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
            if case.status == "pass":
                outcome = "ok" if matched else "regression"
            else:
                outcome = "flip_candidate" if matched else "expected_gap"
            results.append(CaseResult(case.capability, lang, outcome, got,
                                      case.expect_callers, got_kinds,
                                      case.expected_resolution_kind,
                                      case.forbid_resolution_kind))
    return results
