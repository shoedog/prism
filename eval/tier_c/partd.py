"""Part-D single-cell runner (design-of-record §7/§8, spec P5): steered
prism-ON `impact` arm vs unsteered-OFF, scored via `structural.py` set-math
against a FROZEN `gold/<task_id>/gold.json` — NOT judges/ensemble/leak-as-score.

Clones partc.py's ``run_partc_cell``/``PartCCell`` shape and
``_run_partc_live``'s persistence contract (raw streams, meta.json,
manifest.json, status.json, one-cell-per-run-id) so ``rescore.py`` works day
one, and reuses ``cli.py``'s ``_persist_one_arm``/``_persist_partc_cell``
directly rather than re-deriving the persistence shape.
"""
from __future__ import annotations

import os
from dataclasses import dataclass, field
from typing import Any

from .checkout import Checkout
from .model import Dose
from .prompts import stage_prompt
from .structural_corpus import load_structural_tasks


# ---------------------------------------------------------------------------
# Data model
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class PartDCell:
    """Result of one Part-D (task_id, model) cell — the impact-stage set-math
    A/B comparison (steered prism-on vs unsteered-off)."""
    task_id: str
    model: str
    report_off: dict          # StructuralReport.to_dict() — off-arm
    report_on: dict           # StructuralReport.to_dict() — on-arm
    d_recall_delta: float     # on.d_recall - off.d_recall (THE headline, design §5)
    file_f1_delta: float      # on.file_f1  - off.file_f1
    dose: Dose
    low_dose: bool
    administered: bool        # False when the on-arm made ZERO real prism calls
    leaked: bool               # on-arm text names prism/nav_* tools (blinding break)
    tokens_off: int = 0
    tokens_on: int = 0
    in_tokens_off: int = 0
    out_tokens_off: int = 0
    in_tokens_on: int = 0
    out_tokens_on: int = 0
    cost_off: float = 0.0
    cost_on: float = 0.0
    wall_off: float = 0.0
    wall_on: float = 0.0
    off_migration_order: list = field(default_factory=list)
    on_migration_order: list = field(default_factory=list)


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------

def run_partd_cell(cell: tuple, comps: Any) -> PartDCell:
    """Build ONE Part-D cell from a ``(task_id, model)`` pair + injected comps.

    ``comps`` must expose:
      - ``run_off_arm(cell) -> (ArmOutput, ImpactParseResult)``
      - ``run_on_arm(cell)  -> (ArmOutput, ImpactParseResult)``
      - ``score(parsed, *, cell, arm) -> StructuralReport``

    Both arms are fresh runs of the SAME ``impact`` stage prompt (identical
    contract, design §4); the retry-on-parse-failure handling lives inside
    each arm's run_* implementation (``impact.run_impact_with_retry``) so a
    parse failure that survives one retry surfaces as an arm failure
    (``ArmRunError``), never a silent zero score here.
    """
    from .leak import scan_leak

    task_id, model = cell

    off_out, off_parsed = comps.run_off_arm(cell)
    report_off = comps.score(off_parsed, cell=cell, arm="off")

    on_out, on_parsed = comps.run_on_arm(cell)
    administered = on_out.used_prism  # False when prism_calls == 0
    report_on = comps.score(on_parsed, cell=cell, arm="on")

    leak_result = scan_leak(on_out.text)

    d_recall_delta = report_on.d_recall - report_off.d_recall
    file_f1_delta = report_on.file_f1 - report_off.file_f1

    in_tokens_off, out_tokens_off = off_out.in_tokens, off_out.tokens
    in_tokens_on, out_tokens_on = on_out.in_tokens, on_out.tokens

    return PartDCell(
        task_id=task_id,
        model=model,
        report_off=report_off.to_dict(),
        report_on=report_on.to_dict(),
        d_recall_delta=d_recall_delta,
        file_f1_delta=file_f1_delta,
        dose=on_out.dose,
        low_dose=on_out.low_dose,
        administered=administered,
        leaked=leak_result.leaked,
        tokens_off=in_tokens_off + out_tokens_off,
        tokens_on=in_tokens_on + out_tokens_on,
        in_tokens_off=in_tokens_off,
        out_tokens_off=out_tokens_off,
        in_tokens_on=in_tokens_on,
        out_tokens_on=out_tokens_on,
        cost_off=off_out.cost_usd,
        cost_on=on_out.cost_usd,
        wall_off=off_out.wall_s,
        wall_on=on_out.wall_s,
        off_migration_order=list(off_parsed.migration_order),
        on_migration_order=list(on_parsed.migration_order),
    )


# ---------------------------------------------------------------------------
# Report renderer
# ---------------------------------------------------------------------------

_WIDTH = 84
_HEADER = (
    "Part-D structural blast-radius pilot signal (set-math vs frozen gold)\n"
    + "=" * _WIDTH + "\n"
    + f"{'cell':<32} {'d-recall off':>12} {'d-recall on':>12} {'Δd-recall':>10} "
    + f"{'Δfile-F1':>9}\n"
    + f"{'':32} {'dose':>5} {'low?':>4} {'adm?':>4} {'leak?':>5} {'phantom(on)':>12}\n"
    + "-" * _WIDTH
)


def render_partd(cells: list) -> str:
    lines = [_HEADER]
    for c in cells:
        cell_id = f"{c.task_id}/{c.model}"
        lines.append(
            f"{cell_id:<32} "
            f"{c.report_off['d_recall']:>12.3f} "
            f"{c.report_on['d_recall']:>12.3f} "
            f"{c.d_recall_delta:>+10.3f} "
            f"{c.file_f1_delta:>+9.3f}"
        )
        lines.append(
            f"{'':32} "
            f"{c.dose.count:>5} "
            f"{'yes' if c.low_dose else 'no':>4} "
            f"{'yes' if c.administered else 'NO':>4} "
            f"{'YES' if c.leaked else 'no':>5} "
            f"{c.report_on['phantom']:>12}"
            f"   file-dR {c.report_off['d_recall_file']:.2f}/{c.report_on['d_recall_file']:.2f}"
        )
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Prompt (identical contract both arms — design §4; only `steer` differs)
# ---------------------------------------------------------------------------

def _partd_prompt(task, *, steer: str) -> str:
    recv = f"{task.receiver}." if task.receiver else ""
    file, line = task.def_site
    scoped_slice = (
        f"Symbol: {recv}{task.symbol} defined at {file}:{line}. "
        f"Known dispatch path (context, not the answer): {task.dispatch}"
    )
    return stage_prompt("impact", issue_text=task.prompt_change,
                        scoped_slice=scoped_slice, steer=steer)


# ---------------------------------------------------------------------------
# Live components (mirrors cli.py's _LivePartCComps)
# ---------------------------------------------------------------------------

class _LivePartDComps:
    """Real components for a live Part-D cell run: real pinned checkout, real
    runner (ClaudeRunner/CodexRunner by model family), the impact stage with
    ONE auto re-prompt on parse failure, and structural.py's set-math scorer
    against the task's frozen gold.json.
    """

    def __init__(self, *, co, task, model: str, gold_root: str,
                cache_dir: str | None = None, skip_warm_gate: bool = True,
                warm_gate_timeout_s: float = 15.0):
        self._co = co
        self._task = task
        self._model = model
        self._gold_root = gold_root
        self._cache_dir = cache_dir
        self._skip_warm_gate = skip_warm_gate
        self._warm_gate_timeout_s = warm_gate_timeout_s
        self._gold: dict | None = None
        self._last_off = None
        self._last_on = None
        self._last_off_parsed = None
        self._last_on_parsed = None
        self._last_off_prompt: str | None = None
        self._last_on_prompt: str | None = None
        self._last_prewarm: dict | None = None

    def _runner_for(self):
        from .arm_runner import ClaudeRunner, CodexRunner
        return (ClaudeRunner(no_cache=False, cache_dir=self._cache_dir)
               if self._model.startswith("opus")
               else CodexRunner(no_cache=False, cache_dir=self._cache_dir))

    def run_off_arm(self, cell: tuple):
        """Run ONE fresh prism-OFF status-quo impact arm (no steer, no prism)."""
        from .arm_runner import run_arm_isolated
        from .model import Variant
        from .impact import run_impact_with_retry

        task_id, model = cell
        variant = Variant(model, prism=False)
        prompt = _partd_prompt(self._task, steer="")
        self._last_off_prompt = prompt
        runner = self._runner_for()

        def run_once(p: str):
            iso = run_arm_isolated(runner, checkout=self._co, variant=variant,
                                   stage="impact", prompt=p, no_cache=False,
                                   prewarm=False, cache_dir=self._cache_dir)
            return iso.out

        out, parsed = run_impact_with_retry(run_once, prompt)
        self._last_off, self._last_off_parsed = out, parsed
        return out, parsed

    def run_on_arm(self, cell: tuple):
        """Run ONE steered prism-ON impact arm."""
        from .arm_runner import run_arm_isolated
        from .model import Variant
        from .impact import run_impact_with_retry

        task_id, model = cell
        variant = Variant(model, prism=True)
        prompt = _partd_prompt(self._task, steer="prism_on")
        self._last_on_prompt = prompt
        runner = self._runner_for()

        def run_once(p: str):
            iso = run_arm_isolated(runner, checkout=self._co, variant=variant,
                                   stage="impact", prompt=p, no_cache=False,
                                   prewarm=True, cache_dir=self._cache_dir,
                                   skip_warm_gate=self._skip_warm_gate,
                                   warm_gate_timeout_s=self._warm_gate_timeout_s)
            self._last_prewarm = iso.prewarm
            return iso.out

        out, parsed = run_impact_with_retry(run_once, prompt)
        self._last_on, self._last_on_parsed = out, parsed
        return out, parsed

    def score(self, parsed, *, cell: tuple, arm: str):
        from .structural import load_gold, score_structural, verify_site_exists

        if self._gold is None:
            gold_path = os.path.join(self._gold_root, self._task.id, "gold.json")
            self._gold = load_gold(gold_path)
        return score_structural(
            parsed.sites, self._gold,
            verify_exists=lambda f, s: verify_site_exists(self._co, f, s),
        )


class _CachedPartDComps:
    """Wraps an already-run _LivePartDComps so run_partd_cell (called AFTER
    per-arm persistence) reuses the already-executed arms instead of
    re-invoking them (mirrors cli.py's _run_partc_live._CachedComps)."""

    def __init__(self, live):
        self._live = live

    def run_off_arm(self, cell):
        return self._live._last_off, self._live._last_off_parsed

    def run_on_arm(self, cell):
        return self._live._last_on, self._live._last_on_parsed

    def score(self, parsed, **kwargs):
        return self._live.score(parsed, **kwargs)


# ---------------------------------------------------------------------------
# Live run: manifest + persistence (clones _run_partc_live EXACTLY — spec P5)
# ---------------------------------------------------------------------------

def _default_partd_run_id() -> str:
    """partd-YYYYmmdd-HHMMSS-XXXX — same pid+monotonic scheme as
    cli._default_partc_run_id, distinct prefix."""
    import time as _time
    from datetime import datetime

    ts = datetime.now().strftime("partd-%Y%m%d-%H%M%S")
    raw = os.getpid() ^ (int(_time.monotonic() * 1e6) & 0xFFFF)
    suffix = format(raw & 0xFFFF, "04x")
    return f"{ts}-{suffix}"


def _write_partd_manifest(*, cell: tuple, task, bench_root: str,
                          structural_issues_path: str, gold_root: str,
                          run_id: str, runs_root: str,
                          binary_preflight: dict | None = None,
                          cache_dir: str | None = None) -> str:
    import json
    import subprocess
    from datetime import datetime, timezone

    from .arm_runner import _prism_bin, _prism_mcp_bin
    from .cli import _prism_build_id

    task_id, model = cell
    mcp_bin = _prism_mcp_bin()
    mcp_sha = _prism_build_id(mcp_bin)
    prism_bin_path = _prism_bin()
    prism_bin_sha = _prism_build_id(prism_bin_path)

    try:
        git_out = subprocess.run(
            ["git", "rev-parse", "HEAD"], capture_output=True, text=True, timeout=10,
            cwd=os.path.dirname(os.path.abspath(__file__)),
        )
        harness_git_sha = git_out.stdout.strip() or "unknown"
    except Exception as e:
        harness_git_sha = f"error:{e}"

    manifest = {
        "timestamp": datetime.now(tz=timezone.utc).isoformat(),
        "cell": {"task_id": task_id, "model": model, "stage": "impact"},
        "task": {
            "id": task.id, "repo": task.repo, "lang": task.lang, "sha": task.sha,
            "symbol": task.symbol, "receiver": task.receiver,
            "def_site": list(task.def_site),
        },
        "bench_root": bench_root,
        "structural_issues_path": structural_issues_path,
        "gold_root": gold_root,
        "prism_mcp_bin": mcp_bin,
        "prism_mcp_sha": mcp_sha,
        "prism_bin": prism_bin_path,
        "prism_bin_sha": prism_bin_sha,
        "binary_preflight": binary_preflight or {},
        "prism_cache_dir": cache_dir,
        "harness_git_sha": harness_git_sha,
        "steer": {"on": "prism_on", "off": ""},
    }
    run_dir = os.path.join(runs_root, run_id)
    os.makedirs(run_dir, exist_ok=True)
    path = os.path.join(run_dir, "manifest.json")
    with open(path, "w") as f:
        json.dump(manifest, f, indent=2)
    return path



def resolve_run_paths(runs_root: str | None, run_id: str, cache_dir: str | None,
                      default_runs_root: str) -> tuple[str, str, str]:
    """Return ABSOLUTE (runs_root, run_dir, cache_dir) for one live cell.

    2026-08-21 harness defect (Part-D slate, 3 cells voided): a RELATIVE ``--run-store-root`` made
    ``cache_dir`` relative.  The prewarm and the F4 warm gate ran from ``eval/`` and resolved it
    fine, but codex spawns ``prism-mcp`` with cwd = the session checkout, so the agent's server
    resolved the same string INSIDE the checkout -> cache miss -> cold CPG build at MCP startup ->
    prism never exposed to the model (0-dose, silently).  Every path handed to an agent MCP config
    must therefore be absolute; do it ONCE here so prewarm, gate, manifest and agent configs agree.
    """
    root = os.path.abspath(runs_root) if runs_root else default_runs_root
    run_dir = os.path.join(root, run_id)
    cache = os.path.abspath(cache_dir) if cache_dir else os.path.join(run_dir, "prism-cache")
    return root, run_dir, cache

def _run_partd_live(cell: tuple, *, bench_root: str, structural_issues_path: str,
                    gold_root: str, run_id: str | None = None,
                    runs_root: str | None = None, force_new: bool = False,
                    prism_build_dir: str | None = None,
                    skip_binary_preflight: bool = True,
                    skip_warm_gate: bool = True,
                    warm_gate_timeout_s: float = 15.0,
                    cache_dir: str | None = None) -> None:
    """Open a pinned throwaway worktree at the task's SHA and run ONE Part-D
    cell live. Clones ``cli._run_partc_live``'s persistence contract EXACTLY
    (manifest.json before any arm runs; per-arm meta/prompt/out/raw written
    immediately via ``cli._persist_one_arm``; status.json always written;
    ONE cell per run-id, ``force_new`` clobbers) so ``rescore.py`` works day
    one. Never runs both arms if the off-arm fails (fail fast, mirrors Part-C).
    """
    import json as _json
    import shutil

    from .arm_runner import ArmRunError, PreflightError, resolve_matched_binaries
    from .cli import _persist_one_arm, _persist_partc_cell

    task_id, model = cell
    if run_id is None:
        run_id = _default_partd_run_id()
    runs_root, run_dir, cache_dir = resolve_run_paths(
        runs_root, run_id, cache_dir,
        os.path.join(os.path.dirname(os.path.abspath(__file__)), "runs", "partd"))
    if os.path.exists(run_dir) and not force_new:
        raise FileExistsError(f"run-id dir exists: {run_dir} (use --force-new to override)")
    if os.path.exists(run_dir) and force_new:
        shutil.rmtree(run_dir)
    os.makedirs(run_dir, exist_ok=True)

    safe_model = model.replace("/", "_").replace(":", "_")
    base = os.path.join(run_dir, f"{task_id}-impact-{safe_model}")

    failed_stage: str | None = None
    error_msg: str | None = None
    binary_preflight: dict | None = None

    try:
        if not skip_binary_preflight:
            try:
                binary_preflight = resolve_matched_binaries(build_dir=prism_build_dir)
            except PreflightError as e:
                failed_stage = "preflight"
                error_msg = str(e)
                raise

        tasks = load_structural_tasks(structural_issues_path)
        task = next(t for t in tasks if t.id == task_id)

        _write_partd_manifest(
            cell=cell, task=task, bench_root=bench_root,
            structural_issues_path=structural_issues_path, gold_root=gold_root,
            run_id=run_id, runs_root=runs_root,
            binary_preflight=binary_preflight, cache_dir=cache_dir,
        )

        with Checkout(os.path.join(bench_root, task.repo), task.sha) as co:
            comps = _LivePartDComps(
                co=co, task=task, model=model, gold_root=gold_root,
                cache_dir=cache_dir, skip_warm_gate=skip_warm_gate,
                warm_gate_timeout_s=warm_gate_timeout_s,
            )

            # --- OFF arm ---
            off_exception_info: dict | None = None
            try:
                comps.run_off_arm(cell)
            except ArmRunError as e:
                failed_stage = "off"
                error_msg = str(e)
                off_exception_info = {
                    "argv": e.argv, "returncode": e.returncode,
                    "stderr": e.stderr[:20000], "wall_s": 0.0, "exception": str(e),
                    "cwd": "", "mcp_args": [], "prism": False,
                }
            finally:
                if off_exception_info is not None:
                    with open(f"{base}.off.meta.json", "w") as f:
                        _json.dump(off_exception_info, f, indent=2)
                    with open(f"{base}.off.prompt.txt", "w") as f:
                        f.write(comps._last_off_prompt or "")
                else:
                    _persist_one_arm(comps._last_off, comps._last_off_prompt, None,
                                     base=base, arm="off", mcp_args=[], prism=False)

            if failed_stage == "off":
                raise ArmRunError(argv=off_exception_info["argv"],
                                  returncode=off_exception_info["returncode"],
                                  stderr=off_exception_info["stderr"], stdout="")

            # --- ON arm ---
            on_exception_info: dict | None = None
            try:
                comps.run_on_arm(cell)
            except ArmRunError as e:
                failed_stage = "on"
                error_msg = str(e)
                on_exception_info = {
                    "argv": e.argv, "returncode": e.returncode,
                    "stderr": e.stderr[:20000], "wall_s": 0.0, "exception": str(e),
                    "cwd": "", "mcp_args": [], "prism": True,
                }
            finally:
                if on_exception_info is not None:
                    with open(f"{base}.on.meta.json", "w") as f:
                        _json.dump(on_exception_info, f, indent=2)
                    with open(f"{base}.on.prompt.txt", "w") as f:
                        f.write(comps._last_on_prompt or "")
                else:
                    _persist_one_arm(comps._last_on, comps._last_on_prompt,
                                     getattr(comps, "_last_prewarm", None),
                                     base=base, arm="on", mcp_args=[], prism=True)

            if failed_stage == "on":
                raise ArmRunError(argv=on_exception_info["argv"],
                                  returncode=on_exception_info["returncode"],
                                  stderr=on_exception_info["stderr"], stdout="")

            # --- Both arms succeeded: score + persist ---
            partd_cell = run_partd_cell(cell, _CachedPartDComps(comps))

    except Exception as exc:
        status_path = os.path.join(run_dir, "status.json")
        with open(status_path, "w") as f:
            _json.dump({
                "status": "failed", "failed_stage": failed_stage,
                "error": error_msg or str(exc),
                "cell": {"task_id": task_id, "model": model},
            }, f, indent=2)
        raise

    status_path = os.path.join(run_dir, "status.json")
    with open(status_path, "w") as f:
        _json.dump({"status": "success", "failed_stage": None, "error": None,
                   "cell": {"task_id": task_id, "model": model}}, f, indent=2)

    print(f"Run dir: {run_dir}")
    print(render_partd([partd_cell]))

    json_path = _persist_partc_cell(partd_cell, task_id, "impact", model,
                                    run_id=run_id, runs_root=runs_root)
    print(f"cell JSON: {json_path}")
    print(f"status:    {status_path}")
