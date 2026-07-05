"""D2 — relational-fact accuracy: blind LLM extraction + MECHANICAL verification.

calls()/called_by() are stubbed to UNKNOWN via NullCallOracle (spec's explicit D2
escalation allowance — full LSP call-hierarchy wiring is a follow-up). depends() is
fully implemented via neutral per-language import-text parsing (no LSP needed).

Fail-open doctrine under test throughout: an oracle that cannot confirm a claim must
report UNKNOWN, never CONTRADICTED — only a positive disproof yields CONTRADICTED.
"""
from __future__ import annotations

from tier_c.relational import (
    NullCallOracle,
    RelationalClaim,
    confirm_depends,
    extract_imports,
    extract_relational_claims,
    parse_relational_claims,
    score_relational_claims,
)


# ---------------------------------------------------------------------------
# parse_relational_claims / extract_relational_claims
# ---------------------------------------------------------------------------

def test_parses_all_three_claim_kinds():
    raw = (
        "calls: parse_config -> load_file\n"
        "called_by: load_file <- parse_config\n"
        "depends: pkg/a -> pkg/b\n"
    )
    claims = parse_relational_claims(raw)
    assert claims == [
        RelationalClaim("calls", "parse_config", "load_file"),
        RelationalClaim("called_by", "load_file", "parse_config"),
        RelationalClaim("depends", "pkg/a", "pkg/b"),
    ]


def test_parse_ignores_garbage_lines_and_none():
    raw = "NONE\nrandom prose that is not a claim\ncalls: a -> b\n"
    claims = parse_relational_claims(raw)
    assert claims == [RelationalClaim("calls", "a", "b")]


def test_parse_empty_raw_returns_empty():
    assert parse_relational_claims("") == []
    assert parse_relational_claims(None) == []


def test_extract_relational_claims_calls_ask_with_text_and_returns_parsed():
    seen = {}
    def ask(model, prompt):
        seen["model"] = model
        seen["prompt"] = prompt
        return "calls: foo -> bar\n"
    claims = extract_relational_claims(ask, "sonnet-4.6", "the text body")
    assert claims == [RelationalClaim("calls", "foo", "bar")]
    assert seen["model"] == "sonnet-4.6"
    assert "the text body" in seen["prompt"]


def test_extract_relational_claims_empty_text_skips_ask_call():
    calls = []
    def ask(model, prompt):
        calls.append(prompt)
        return "NONE"
    assert extract_relational_claims(ask, "sonnet-4.6", "") == []
    assert calls == []


def test_extract_relational_claims_symmetric_same_model_both_arms():
    """The SAME extractor model/prompt template must be usable identically on both
    off-arm and on-arm text — i.e. nothing in extract_relational_claims itself is
    arm-aware (the caller enforces symmetry by using one model for both calls)."""
    models_used = []
    def ask(model, prompt):
        models_used.append(model)
        return "depends: a -> b\n"
    extract_relational_claims(ask, "sonnet-4.6", "off text")
    extract_relational_claims(ask, "sonnet-4.6", "on text")
    assert models_used == ["sonnet-4.6", "sonnet-4.6"]


# ---------------------------------------------------------------------------
# NullCallOracle — fail-open stub for calls()/called_by()
# ---------------------------------------------------------------------------

def test_null_call_oracle_always_unknown():
    o = NullCallOracle()
    assert o.confirm_calls("a", "b") == "UNKNOWN"
    assert o.confirm_calls("x", "y") == "UNKNOWN"


# ---------------------------------------------------------------------------
# extract_imports — neutral per-language import-text parsing
# ---------------------------------------------------------------------------

def test_extract_imports_go():
    src = 'package main\n\nimport (\n\t"fmt"\n\t"example.com/pkg/util"\n)\n'
    imports = extract_imports("go", src)
    assert "fmt" in imports
    assert "example.com/pkg/util" in imports


def test_extract_imports_rust():
    src = "use std::io;\nuse crate::util::helper;\n\nfn main() {}\n"
    imports = extract_imports("rust", src)
    assert any("std::io" in i for i in imports)
    assert any("crate::util::helper" in i for i in imports)


def test_extract_imports_python():
    src = "from pkg.util import helper\nimport os\n\ndef f(): pass\n"
    imports = extract_imports("python", src)
    assert "pkg.util" in imports
    assert "os" in imports


def test_extract_imports_typescript():
    src = "import { helper } from './util';\nconst x = require('fs');\n"
    imports = extract_imports("typescript", src)
    assert "./util" in imports
    assert "fs" in imports


def test_extract_imports_unknown_language_returns_empty():
    assert extract_imports("cobol", "IDENTIFICATION DIVISION.") == []


def test_extract_imports_empty_source_returns_empty():
    assert extract_imports("go", "") == []


# ---------------------------------------------------------------------------
# confirm_depends — mechanical, fail-open depends() verification
# ---------------------------------------------------------------------------

class _Co:
    """Fake checkout: resolve_rel exact-matches by dict; read_file returns fixed source."""
    def __init__(self, files: dict[str, str], resolvable: set[str] | None = None):
        self.files = files
        self.resolvable = resolvable if resolvable is not None else set(files)

    def resolve_rel(self, rel):
        return rel if rel in self.resolvable else None

    def read_file(self, rel):
        return self.files.get(rel)


def test_confirm_depends_supported_when_modb_in_imports():
    co = _Co({"main.go": 'package main\nimport (\n\t"pkg/util"\n)\n'})
    assert confirm_depends(co, "main.go", "pkg/util", "go") == "SUPPORTED"


def test_confirm_depends_contradicted_when_modb_resolves_but_absent_from_imports():
    co = _Co({
        "main.go": 'package main\nimport (\n\t"pkg/util"\n)\n',
        "pkg/other.go": "package pkg",
    })
    assert confirm_depends(co, "main.go", "pkg/other.go", "go") == "CONTRADICTED"


def test_confirm_depends_unknown_when_moda_unresolvable():
    co = _Co({}, resolvable=set())
    assert confirm_depends(co, "ghost.go", "pkg/util", "go") == "UNKNOWN"


def test_confirm_depends_unknown_when_imports_unparseable():
    """modA resolves and reads, but the language has no import pattern (or the file has
    none) -> UNKNOWN, never CONTRADICTED, per the fail-open doctrine."""
    co = _Co({"main.cobol": "IDENTIFICATION DIVISION."})
    assert confirm_depends(co, "main.cobol", "pkg/util", "cobol") == "UNKNOWN"


def test_confirm_depends_unknown_when_modb_itself_unresolvable():
    """modA's imports genuinely omit modB's needle, but modB doesn't resolve to any real
    file either -> UNKNOWN (can't be sure the LLM's naming just doesn't match import
    spelling), NOT a positive disproof."""
    co = _Co({"main.go": 'package main\nimport (\n\t"pkg/util"\n)\n'}, resolvable={"main.go"})
    assert confirm_depends(co, "main.go", "totally/unresolvable/mod", "go") == "UNKNOWN"


# ---------------------------------------------------------------------------
# score_relational_claims — precision excludes UNKNOWN; UNKNOWN rate reported
# ---------------------------------------------------------------------------

class _FixedCallOracle:
    def __init__(self, verdict: str):
        self._v = verdict
    def confirm_calls(self, caller, callee):
        return self._v


def test_score_relational_claims_precision_excludes_unknown():
    """1 SUPPORTED + 1 CONTRADICTED + 2 UNKNOWN (calls, oracle stub) -> precision is
    computed over SUPPORTED+CONTRADICTED only (1/2 = 0.5), and unknown_rate = 2/4."""
    co = _Co({"main.go": 'package main\nimport (\n\t"pkg/util"\n)\n',
             "pkg/other.go": "package pkg"})
    claims = [
        RelationalClaim("depends", "main.go", "pkg/util"),      # SUPPORTED
        RelationalClaim("depends", "main.go", "pkg/other.go"),  # CONTRADICTED
        RelationalClaim("calls", "a", "b"),                     # UNKNOWN (stub)
        RelationalClaim("called_by", "b", "a"),                 # UNKNOWN (stub)
    ]
    report = score_relational_claims(claims, call_oracle=NullCallOracle(), co=co, language="go")
    assert report.total == 4
    assert report.contradicted == 1
    assert report.unknown_rate == 0.5
    assert report.precision == 0.5, f"expected 0.5 (1 supported / (1 supported + 1 contradicted)), got {report.precision}"


def test_score_relational_claims_all_unknown_gives_zero_precision_not_crash():
    claims = [RelationalClaim("calls", "a", "b")]
    report = score_relational_claims(claims, call_oracle=NullCallOracle(), co=_Co({}), language="go")
    assert report.precision == 0.0
    assert report.unknown_rate == 1.0
    assert report.contradicted == 0


def test_score_relational_claims_empty_claims_no_crash():
    report = score_relational_claims([], call_oracle=NullCallOracle(), co=_Co({}), language="go")
    assert report.total == 0
    assert report.precision == 0.0
    assert report.unknown_rate == 0.0


def test_score_relational_claims_never_auto_contradicts_calls():
    """Even a confidently-worded 'CONTRADICTED'-shaped call oracle stub result is only
    trusted if the oracle SAYS so; NullCallOracle (the shipped default) never does, so
    calls()/called_by() claims can never land as CONTRADICTED through the stub path."""
    claims = [RelationalClaim("calls", "a", "b"), RelationalClaim("called_by", "b", "a")]
    report = score_relational_claims(claims, call_oracle=NullCallOracle(), co=_Co({}), language="go")
    assert report.contradicted == 0
