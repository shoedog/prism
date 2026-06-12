"""Capability matrix runner (spec §2.7): by-construction ground truth, no LSP.
Outcomes: ok | regression (pass case failing -> fails the run) |
expected_gap | flip_candidate (known_fail now passing -> report, update status)."""
from __future__ import annotations

import tomllib
from dataclasses import dataclass
from pathlib import Path

from .model import FunctionDef, Location


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


@dataclass
class CaseResult:
    capability: str
    language: str
    outcome: str
    got: set
    expected: set


def load_case(toml_path: Path) -> Case:
    d = tomllib.loads(toml_path.read_text())
    return Case(
        path=toml_path.parent,
        language=d["case"]["language"],
        capability=d["case"]["capability"],
        status=d["case"]["status"],
        seed_symbol=d["seed"]["symbol"],
        seed_file=d["seed"]["file"],
        seed_line=d["seed"]["line"],
        expect_callers={(c["file"], c["line"]) for c in d["expect"]["callers"]},
        exact=d["expect"].get("exact", True),
    )


def run_matrix(fixtures_root: Path, sut, languages: list[str]) -> list[CaseResult]:
    results = []
    for lang in languages:
        for toml_path in sorted((fixtures_root / lang).glob("*/expected.toml")):
            case = load_case(toml_path)
            seed = FunctionDef(case.seed_symbol, "function", None,
                               Location(case.seed_file, case.seed_line, case.seed_line),
                               case.seed_line)
            edges = sut.callers(str(case.path), seed)
            got = {(e.call_site.file, e.call_site.start_line) for e in edges}
            matched = got == case.expect_callers if case.exact \
                else case.expect_callers <= got
            if case.status == "pass":
                outcome = "ok" if matched else "regression"
            else:
                outcome = "flip_candidate" if matched else "expected_gap"
            results.append(CaseResult(case.capability, lang, outcome, got,
                                      case.expect_callers))
    return results
