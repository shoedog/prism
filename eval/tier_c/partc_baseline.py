"""Baseline loader for recovered prism-off files (Task 7, spec §7).

Recovered baseline files begin with a metadata header that leaks the
experimental condition (``prism=False``, ``prism_called``, ``session:``,
``group_size``, …) terminated by a line that is exactly ``---``.  Scoring
the raw text would break blinding.

``load_base_text`` returns ONLY the assistant text after that first separator,
stripped of surrounding whitespace.  The body may itself contain ``---``
Markdown horizontal rules — those are kept intact; only the FIRST separator
line is the header boundary.

``load_base`` is the file-level entry point: it reads
``<root>/<model>/<repo>/<stage>/<model>.md`` and delegates to
``load_base_text``.
"""
from __future__ import annotations

import pathlib


def load_base_text(raw: str) -> str:
    """Strip the condition header from *raw* and return the body.

    Splits on the **first** line whose ``.strip()`` is exactly ``"---"``.
    Everything before (and including) that line is discarded.  The body
    (everything after) is returned stripped of leading/trailing whitespace.

    If no such separator is found, returns ``""`` (safe default — prevents
    a malformed file from accidentally leaking header text into scoring).
    """
    lines = raw.split("\n")
    for i, line in enumerate(lines):
        if line.strip() == "---":
            body = "\n".join(lines[i + 1:])
            return body.strip()
    return ""


def load_base(model: str, repo: str, stage: str, root: str) -> str:
    """Read the prism-off baseline file and return its stripped body.

    The file is located at::

        <root>/<model>/<repo>/<stage>/<model>.md

    which is the ``<model>.md`` (no suffix) file — the prism-off variant.
    The ``+prism.md`` / ``+lsp.md`` / ``+prism+lsp.md`` siblings are ignored.

    Parameters
    ----------
    model:
        Model identifier, e.g. ``"opus-4.8"``.
    repo:
        Repository name, e.g. ``"prometheus"``.
    stage:
        Stage name, e.g. ``"spec"`` or ``"impl"``.
    root:
        Path to the ``recovered/`` directory that contains the per-model trees.

    Returns
    -------
    str
        The body text after the first ``---`` separator, stripped.
    """
    path = pathlib.Path(root) / model / repo / stage / f"{model}.md"
    raw = path.read_text(encoding="utf-8")
    return load_base_text(raw)
