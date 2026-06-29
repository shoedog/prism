"""Tests for partc_baseline loader — Task 7.

Verifies that the header-stripping loader correctly removes the prism=False
metadata header from recovered baseline files, keeping only the body after
the first ``---`` separator line.
"""
from __future__ import annotations

import pathlib

import pytest

from tier_c.partc_baseline import load_base_text, load_base


# ---------------------------------------------------------------------------
# Unit tests for load_base_text
# ---------------------------------------------------------------------------

def test_loader_strips_condition_header_through_first_separator():
    raw = "# x\n- prism=**False**\n- session: /p\n\n---\n\nSpec body here\n## Summary\n"
    result = load_base_text(raw)
    assert result == "Spec body here\n## Summary"
    assert "prism" not in result.lower()  # no condition leak


def test_loader_keeps_markdown_rules_in_body():
    # Body itself has its own --- horizontal rule — only the FIRST --- is the header boundary
    raw = "- prism=False\n---\nbody line\n\n---\n\nmore body"
    result = load_base_text(raw)
    assert result == "body line\n\n---\n\nmore body"


def test_loader_separator_with_trailing_whitespace():
    """A separator line with trailing spaces should still be recognised."""
    raw = "header stuff\n---   \nbody text"
    result = load_base_text(raw)
    assert result == "body text"


def test_loader_separator_with_surrounding_blank_lines():
    """Blank lines around the separator (the real file format) work correctly."""
    raw = "- model: opus\n- prism_called: False\n\n---\n\nActual spec text.\n"
    result = load_base_text(raw)
    assert result == "Actual spec text."


def test_loader_strips_outer_whitespace():
    """Returned body is stripped (no leading/trailing blank lines)."""
    raw = "header\n---\n\n\nbody\n\n"
    result = load_base_text(raw)
    assert result == "body"


def test_loader_no_separator_returns_empty_string():
    """If there is no --- line, return '' (safe default for blinding)."""
    raw = "no separator at all\njust a header"
    result = load_base_text(raw)
    assert result == ""


def test_loader_first_separator_only_used():
    """Only the first --- splits header from body; later ones stay in body."""
    raw = "header\n---\nfirst body\n---\nsecond body"
    result = load_base_text(raw)
    # Body is everything after the first ---, stripped
    assert result == "first body\n---\nsecond body"


# ---------------------------------------------------------------------------
# Real-file test via load_base
# ---------------------------------------------------------------------------

_REAL_FILE = pathlib.Path(
    "/Users/wesleyjinks/code/slicing/eval/tier_c/runs/full-2026-06-24"
    "/recovered/opus-4.8/prometheus/spec/opus-4.8.md"
)


@pytest.mark.skipif(not _REAL_FILE.exists(), reason="recovered run data not present")
def test_load_base_real_file_no_condition_leak():
    """load_base on the real prometheus/spec file drops the header entirely."""
    result = load_base(
        model="opus-4.8",
        repo="prometheus",
        stage="spec",
        root=str(_REAL_FILE.parents[3]),  # .../runs/full-2026-06-24/recovered
    )
    assert "prism" not in result.lower(), "condition header leaked into body"
    # The real spec starts with "Spec written to"
    assert result.startswith("Spec written to"), (
        f"Expected body to start with 'Spec written to', got: {result[:80]!r}"
    )
