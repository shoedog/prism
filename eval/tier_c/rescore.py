"""Re-derive Part-C scores from SAVED arm artifacts — WITHOUT re-running the arms.

A judge/scoring/oracle fix must not force expensive live arm re-runs. The per-arm
raw model streams (``<base>.<arm>.raw.jsonl``, written verbatim by
``_persist_one_arm``) plus ``manifest.json`` contain everything scoring needs:
reconstruct each :class:`ArmOutput` from its raw stream and re-score with the
current judge.

Cardinal invariant (feedback_rescore_not_rerun): rescore writes to a NEW run-id
and NEVER mutates the source run dir. If we have the logs/telemetry, we rescore;
we don't re-run the cells.

reconstruct_arm_output mirrors arm_runner.ClaudeRunner.run's post-subprocess
mapping (parse → citations → classify) so a reconstructed ArmOutput is identical
to a freshly-run one for scoring/reporting purposes. rescore_cell reuses the real
``_LivePartCComps.score`` (the now-MCP-isolated judge) via a cached-arm wrapper —
the same pattern _run_partc_live uses — so scoring logic is never duplicated.
"""
from __future__ import annotations
import json
import os
from typing import Any

from .model import ArmOutput, Variant
from .llm import MODEL_CLI
from .parse import parse_claude_stream_json, parse_codex_jsonl
from .citations import parse_citations
from .classify import classify_tools


def reconstruct_arm_output(
    raw_stdout: str,
    *,
    variant: Variant,
    model: str,
    wall_s: float = 0.0,
    argv: list | None = None,
    cwd: str = "",
    stderr: str = "",
) -> ArmOutput:
    """Rebuild an ArmOutput from a saved raw model stream.

    Mirrors ClaudeRunner.run (and CodexRunner.run) exactly: parse the stream,
    extract citations from the text, classify tools. The parser is chosen by the
    model's CLI family (claude → stream-json, codex → JSONL). wall_s/argv/cwd/
    stderr come from the saved meta.json (the raw stream does not carry them).
    """
    cli = MODEL_CLI[model][0]
    parse = parse_claude_stream_json if cli == "claude" else parse_codex_jsonl
    r = parse(raw_stdout)
    flags = classify_tools(r.commands)
    prism_calls = r.prism_calls
    return ArmOutput(
        variant=variant,
        text=r.text,
        citations=parse_citations(r.text),
        tokens=r.output_tokens,
        tool_calls=r.tool_calls,
        wall_s=wall_s,
        used_prism=prism_calls > 0,
        prism_calls=prism_calls,
        dose=r.dose,
        low_dose=prism_calls > 0 and prism_calls <= 1,
        commands=r.commands,
        in_tokens=r.input_tokens,
        cost_usd=r.cost_usd,
        raw_stdout=raw_stdout,
        argv=argv or [],
        returncode=0,
        stderr=stderr,
        cwd=cwd,
        **flags,
    )


def rescore_cell(
    cell: tuple,
    *,
    off_out: ArmOutput,
    on_out: ArmOutput,
    co: Any,
    issue: Any,
    base_root: str = "",
    ask=None,
    judge_primary=None,
):
    """Score pre-built off/on ArmOutputs through the real oracle, no arm re-run.

    Reuses ``_LivePartCComps.score`` (deferred import to avoid a module-load cycle
    with cli.py) so the judge path — including the --strict-mcp-config isolation —
    is identical to a live run. A cached-arm wrapper supplies the saved arms to
    ``run_partc_cell`` (same shape as _run_partc_live's _CachedComps).

    Returns ``(PartCCell, off_judge_records, on_judge_records)``.
    """
    from .cli import _LivePartCComps  # deferred: cli.py imports rescore lazily for the subcommand
    from .partc import run_partc_cell

    repo, stage, model = cell
    live = _LivePartCComps(co=co, issue=issue, model=model, base_root=base_root, ask=ask,
                           judge_primary=judge_primary)

    class _CachedComps:
        def run_off_arm(self_inner, c):
            return off_out

        def run_on_arm(self_inner, c):
            return on_out

        def score(self_inner, citations, **kwargs):
            return live.score(citations, **kwargs)

        def head_to_head(self_inner, off, on, c):
            # Forward to the live comps so rescore produces the head-to-head verdict too
            # (run_partc_cell only computes it when the comps exposes this method).
            return live.head_to_head(off, on, c)

    result = run_partc_cell(cell, _CachedComps())
    return result, list(live._last_off_judge), list(live._last_on_judge)


def _reconstruct_arm_from_files(base: str, arm: str, model: str) -> ArmOutput:
    """Load ``<base>.<arm>.raw.jsonl`` (+ meta.json) and reconstruct the ArmOutput."""
    raw = _read(f"{base}.{arm}.raw.jsonl")
    meta = {}
    meta_path = f"{base}.{arm}.meta.json"
    if os.path.exists(meta_path):
        with open(meta_path) as f:
            meta = json.load(f)
    variant = Variant(model, prism=(arm == "on"))
    return reconstruct_arm_output(
        raw,
        variant=variant,
        model=model,
        wall_s=float(meta.get("wall_s", 0.0)),
        argv=meta.get("argv", []),
        cwd=meta.get("cwd", ""),
        stderr=meta.get("stderr", ""),
    )


def _read(path: str) -> str:
    with open(path) as f:
        return f.read()


def rescore_run_dir(
    src_dir: str,
    *,
    bench_root: str | None = None,
    issues_path: str | None = None,
    out_run_id: str | None = None,
    runs_root: str | None = None,
    ask=None,
    co: Any = None,
    issue: Any = None,
    judge_primary=None,
):
    """Re-score a saved Part-C run dir, writing results to a NEW run dir.

    Reads ``manifest.json`` for the cell + issue sha + paths, reconstructs both
    arms from their saved raw streams, re-scores via the current judge, and
    persists the PartCCell + per-cite judge artifacts under
    ``<runs_root>/<out_run_id>/``. The SOURCE dir is never modified.

    Test seams: ``ask`` (judge), ``co`` (Checkout), ``issue`` may be injected to
    run fully offline; in production they are derived from the manifest + bench
    repo. Returns ``(PartCCell, out_dir)``.
    """
    from .cli import (
        _persist_partc_cell,
        _persist_judge_jsonl,
        _default_partc_run_store_root,
    )

    with open(os.path.join(src_dir, "manifest.json")) as f:
        man = json.load(f)
    repo = man["cell"]["repo"]
    stage = man["cell"]["stage"]
    model = man["cell"]["model"]
    cell = (repo, stage, model)
    sha = man.get("issue", {}).get("sha", "")
    base_root = man.get("base_root", "") or ""
    issues_path = issues_path or man.get("issues_path")
    bench_root = bench_root or man.get("bench_root")

    safe_model = model.replace("/", "_").replace(":", "_")
    base = os.path.join(src_dir, f"{repo}-{stage}-{safe_model}")
    off_out = _reconstruct_arm_from_files(base, "off", model)
    on_out = _reconstruct_arm_from_files(base, "on", model)

    if issue is None:
        from .corpus import load_issues
        issues = load_issues(issues_path)
        issue = next(i for i in issues if i.repo == repo)

    if runs_root is None:
        runs_root = _default_partc_run_store_root()
    if out_run_id is None:
        out_run_id = os.path.basename(src_dir.rstrip("/")) + "-rescore"
    out_dir = os.path.join(runs_root, out_run_id)
    os.makedirs(out_dir, exist_ok=True)

    def _finish(checkout):
        cell_result, off_judge, on_judge = rescore_cell(
            cell, off_out=off_out, on_out=on_out, co=checkout,
            issue=issue, base_root=base_root, ask=ask,
            judge_primary=judge_primary,
        )
        _persist_partc_cell(cell_result, repo, stage, model,
                            run_id=out_run_id, runs_root=runs_root)
        out_base = os.path.join(out_dir, f"{repo}-{stage}-{safe_model}")
        _persist_judge_jsonl(off_judge, f"{out_base}.off.judge.jsonl")
        _persist_judge_jsonl(on_judge, f"{out_base}.on.judge.jsonl")
        # Provenance: record what was rescored (source dir + that arms were NOT re-run).
        with open(os.path.join(out_dir, "rescore.json"), "w") as f:
            json.dump({"source_run_dir": os.path.abspath(src_dir), "cell": list(cell),
                       "issue_sha": sha, "arms_rerun": False}, f, indent=2)
        return cell_result

    if co is not None:
        return _finish(co), out_dir

    from .checkout import Checkout
    with Checkout(os.path.join(os.path.expanduser(bench_root), repo), sha) as real_co:
        return _finish(real_co), out_dir
