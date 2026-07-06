"""P1: structural.toml loader (spec docs/superpowers/specs/2026-07-05-tier-c-part-d...).
Mirrors tier_c/corpus.py's loader style but for the Part-D StructuralTask schema."""
from __future__ import annotations
import pytest
from tier_c.structural_corpus import load_structural_tasks, StructuralTask, StructuralCorpusError

REAL_TOML = "tier_c/issues/structural.toml"


def test_load_real_structural_toml_core_tasks_present():
    tasks = load_structural_tasks(REAL_TOML)
    ids = {t.id for t in tasks}
    assert len(ids) == len(tasks)  # ids unique
    # The corpus is the frozen 12-task, 5-language Part-D slate; spot-check the anchors.
    assert {
        "prometheus-matchstring", "ruff-typechecker-match-annotation",
        "hugo-converter-convert", "prometheus-promql-walk",
        "typescript-resolve-signature", "mypy-meet-types",
        "guava-equivalence-doequivalent",
    } <= ids
    langs = {t.lang for t in tasks}
    assert {"go", "rust", "typescript", "python", "java"} <= langs


def test_def_site_parses_to_file_and_line():
    tasks = load_structural_tasks(REAL_TOML)
    by_id = {t.id: t for t in tasks}
    pt = by_id["prometheus-matchstring"]
    assert pt.def_site == ("model/labels/regexp.go", 328)
    rt = by_id["ruff-typechecker-match-annotation"]
    assert rt.def_site == ("crates/ruff_python_semantic/src/analyze/typing.rs", 615)


def test_task_fields_populated():
    tasks = load_structural_tasks(REAL_TOML)
    pt = next(t for t in tasks if t.id == "prometheus-matchstring")
    assert pt.repo == "prometheus"
    assert pt.lang == "go"
    assert pt.sha == "505095b"
    assert pt.symbol == "MatchString"
    assert pt.receiver == "(*FastRegexMatcher)"
    assert "Matches" in pt.dispatch
    assert pt.prompt_change.startswith("We are changing")
    assert "MatchString" in pt.grep_name_stats
    assert pt.notes  # notes present (documentation)


def test_missing_required_field_raises(tmp_path):
    bad = tmp_path / "bad.toml"
    bad.write_text(
        '[[task]]\nid = "x"\nrepo = "r"\nlang = "go"\nsha = "abc"\n'
        'symbol = "S"\ndef_site = "a.go:1"\ndispatch = "d"\n'
        'prompt_change = "p"\ngrep_name_stats = "g"\n'
    )
    with pytest.raises(StructuralCorpusError, match="receiver"):
        load_structural_tasks(bad)


def test_bad_def_site_shape_raises(tmp_path):
    bad = tmp_path / "bad.toml"
    bad.write_text(
        '[[task]]\nid = "x"\nrepo = "r"\nlang = "go"\nsha = "abc"\n'
        'symbol = "S"\nreceiver = "R"\ndef_site = "no-colon-here"\ndispatch = "d"\n'
        'prompt_change = "p"\ngrep_name_stats = "g"\n'
    )
    with pytest.raises(StructuralCorpusError, match="def_site"):
        load_structural_tasks(bad)


def test_no_tasks_raises(tmp_path):
    empty = tmp_path / "empty.toml"
    empty.write_text("# nothing here\n")
    with pytest.raises(StructuralCorpusError, match="no \\[\\[task\\]\\]"):
        load_structural_tasks(empty)


def test_notes_optional(tmp_path):
    """notes is documentation-only; a task without it should still load."""
    p = tmp_path / "i.toml"
    p.write_text(
        '[[task]]\nid = "x"\nrepo = "r"\nlang = "go"\nsha = "abc"\n'
        'symbol = "S"\nreceiver = "R"\ndef_site = "a.go:1"\ndispatch = "d"\n'
        'prompt_change = "p"\ngrep_name_stats = "g"\n'
    )
    [t] = load_structural_tasks(p)
    assert t.notes == ""
