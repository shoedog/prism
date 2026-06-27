"""tier-c entry (spec). Supports --list and (Phase-1b/1c) a `run` subcommand.
Mirrors tier_a/cli.py argument-parsing style."""
from __future__ import annotations
import argparse
import os
from .corpus import load_issues


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="tier-c")
    sub = ap.add_subparsers(dest="cmd")

    # Legacy flat interface (Phase-1): tier-c --issues <f> [--list]
    ap.add_argument("--issues")
    ap.add_argument("--list", action="store_true")

    # Phase-1b/1c/1d: tier-c run --issues <f> [--live] [--bench-root <dir>]
    run_p = sub.add_parser("run", help="run the A/B harness over an issues file")
    run_p.add_argument("--issues", required=True)
    run_p.add_argument("--live", action="store_true",
                       help="execute live model calls (requires corpus + API keys)")
    run_p.add_argument("--bench-root", default="~/code/bench-repos",
                       help="root directory containing cloned benchmark repos (default: ~/code/bench-repos)")
    run_p.add_argument("--run-id", default=None,
                       help="run identifier; required for --live (collision-guarded, use --force-new to override)")
    run_p.add_argument("--force-new", action="store_true",
                       help="override run-id collision guard (overwrites existing run dir)")
    run_p.add_argument("--run-store-root", default=None,
                       help="root dir for run artifacts (default: eval/tier_c/runs next to this file)")

    # Phase-1 Part-C: tier-c run-partc --cell <repo>:<stage>:<model> [--live]
    partc_p = sub.add_parser(
        "run-partc",
        help="run ONE steered-prism-on Part-C cell (fake comps by default; --live for real run)",
    )
    partc_p.add_argument(
        "--cell", required=True,
        help="cell descriptor as repo:stage:model (e.g. ruff:spec:opus-4.8)",
        metavar="REPO:STAGE:MODEL",
    )
    partc_p.add_argument(
        "--live", action="store_true",
        help="build real components (live model call + real checkout; default = fake comps)",
    )
    partc_p.add_argument(
        "--bench-root", default="~/code/bench-repos",
        help="root directory containing cloned benchmark repos (default: ~/code/bench-repos)",
    )
    partc_p.add_argument(
        "--base-root",
        default="tier_c/runs/full-2026-06-24/recovered",
        help="root of the recovered prism-off baseline tree (default: tier_c/runs/full-2026-06-24/recovered)",
    )

    args = ap.parse_args(argv)

    if args.cmd == "run-partc":
        parts = (args.cell or "").split(":", 2)
        if len(parts) != 3:
            ap.error("--cell must be repo:stage:model (e.g. ruff:spec:opus-4.8)")
        repo, stage, model = parts
        cell = (repo, stage, model)
        if not args.live:
            _run_partc_fake(cell)
        else:
            bench_root = os.path.expanduser(args.bench_root)
            base_root = args.base_root
            _run_partc_live(cell, bench_root=bench_root, base_root=base_root)
        return 0

    if args.cmd == "run":
        issues = load_issues(args.issues)
        if not args.live:
            print(
                "live run requires --live + corpus + API keys;\n"
                "pass --live --bench-root <dir> --run-id <id> to execute the full 8-variant spec->plan chain."
            )
            return 0

        if not args.run_id:
            ap.error("--run-id is required for --live (use --force-new to override collision guard)")

        bench_root = os.path.expanduser(args.bench_root)
        run_store_root = args.run_store_root or _default_run_store_root()
        _run_live_cmd(issues, bench_root=bench_root, run_id=args.run_id,
                      run_store_root=run_store_root, force_new=args.force_new)
        return 0

    # Legacy flat interface (original cli)
    if args.issues is None:
        ap.error("--issues is required")
    issues = load_issues(args.issues)
    if args.list:
        for i in issues:
            print(f"{i.key}\t{i.language}\t{i.scoped_slice}")
        return 0
    print(f"loaded {len(issues)} issues (run driver lands in Task 13 / Phase-1 live run)")
    return 0


def _default_run_store_root() -> str:
    """Default run store: eval/tier_c/runs/ (gitignored)."""
    return os.path.join(os.path.dirname(os.path.abspath(__file__)), "runs")


def _prism_build_id(path: str) -> str:
    """Build identity of the prism-mcp binary = sha256 of its bytes (prism-mcp has no --version).
    Different build => different hash, so audit/replay can detect a prism change."""
    import hashlib
    try:
        with open(path, "rb") as f:
            return "sha256:" + hashlib.sha256(f.read()).hexdigest()[:16]
    except Exception as e:
        return f"error:{e}"


def _build_manifest(*, issues, bench_root: str, run_id: str, run_store_root: str) -> dict:
    """Build the run manifest (models, prism bin/SHA, harness git SHA, corpus, env)."""
    import shutil
    import subprocess
    from .arm_runner import _prism_mcp_bin
    from .lspshim import DENIED

    models = ["opus-4.8", "gpt-5.5"]

    # Prism build identity: prism-mcp has NO --version flag (it needs --repo), so identify the
    # build by a content hash of the binary — unique per build, all replay/audit needs.
    prism_bin = _prism_mcp_bin()
    prism_sha = _prism_build_id(prism_bin)

    # Harness git SHA
    try:
        git_out = subprocess.run(
            ["git", "rev-parse", "HEAD"], capture_output=True, text=True, timeout=10,
            cwd=os.path.dirname(os.path.abspath(__file__)),
        )
        harness_git_sha = git_out.stdout.strip() or "unknown"
    except Exception as e:
        harness_git_sha = f"error:{e}"

    # CLI versions
    claude_version = _cli_version("claude", ["claude", "--version"])
    codex_version = _cli_version("codex", ["codex", "--version"])

    corpus = [{"key": i.key, "sha": i.sha, "language": i.language} for i in issues]

    return {
        "run_id": run_id,
        "models": models,
        "prism_bin": prism_bin,
        "prism_sha": prism_sha,
        "harness_git_sha": harness_git_sha,
        "cli_versions": {"claude": claude_version, "codex": codex_version},
        "corpus": corpus,
        "bench_root": bench_root,
        "run_store_root": run_store_root,
        "denied_list": DENIED,
        "env_path": os.environ.get("PATH", ""),
    }


def _cli_version(name: str, cmd: list[str]) -> str:
    import subprocess
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=5)
        return (r.stdout.strip() or r.stderr.strip() or "unknown")[:200]
    except Exception as e:
        return f"error:{e}"


def _run_live_cmd(issues, *, bench_root: str, run_id: str, run_store_root: str,
                  force_new: bool = False) -> None:
    """Build LiveComponents (8-variant 2×2) and run the full live loop."""
    from .model import Variant
    from .arm_runner import ClaudeRunner, CodexRunner, RoutingArmRunner
    from .judges_live import LlmRankJudge, LlmRelevanceJudge, LlmConditionGuesser
    from .llm import live_ask
    from .checkout import Checkout
    from .run import LiveComponents, run_live
    from .store import RunStore
    from .lspshim import make_lsp_deny_shim

    # Build full manifest (used as store payload)
    manifest = _build_manifest(issues=issues, bench_root=bench_root,
                               run_id=run_id, run_store_root=run_store_root)

    # Build store; collision-guard; create shim under the run dir
    store = RunStore(run_store_root, run_id, manifest)
    store.ensure_new(force=force_new)
    shim_log = os.path.join(store.dir, "shim-log.jsonl")

    # Build 8 variants: 2 models × prism on/off × lsp on/off
    variants = [
        Variant(m, p, l)
        for m in ("opus-4.8", "gpt-5.5")
        for p in (False, True)
        for l in (False, True)
    ]

    # Create the deny shim; wire it into both concrete runners
    lsp_deny_dir = make_lsp_deny_shim(shim_log)
    runner = RoutingArmRunner(
        claude=ClaudeRunner(lsp_deny_dir=lsp_deny_dir),
        codex=CodexRunner(lsp_deny_dir=lsp_deny_dir),
    )

    judges = {
        "anthropic": LlmRankJudge(live_ask, "opus-4.8"),
        "openai": LlmRankJudge(live_ask, "gpt-5.5"),
    }
    relevance = LlmRelevanceJudge(live_ask, "opus-4.8")
    guesser = LlmConditionGuesser(live_ask, "opus-4.8")

    def open_checkout(repo, sha):
        path = os.path.join(bench_root, repo)
        return Checkout(path, sha)

    comps = LiveComponents(
        variants=variants,
        runner=runner,
        judges=judges,
        relevance=relevance,
        guesser=guesser,
        plants=[],
        open_checkout=open_checkout,
        store=store,
        lsp_shim_dir=lsp_deny_dir,
    )

    print(f"Run ID: {run_id}")
    print(f"Run dir: {store.dir}")
    print(f"Issues: {len(issues)}  Variants: {len(variants)}  Stages: 2")
    print()
    report = run_live(issues, comps)

    # Print per-(stage x language) cells — 5 contrasts
    print("\n=== Per-(stage x language) 2x2 Report ===")
    for (stage, language), cell in sorted(report.cells.items()):
        print(f"\n[{stage} / {language}]")
        print(f"  gate: {cell.gate.decision}  — {cell.gate.reason}")
        print(f"  itt_available_rate:     {cell.itt_available_rate:.2f}")
        print(f"  per_protocol_used_rate: {cell.per_protocol_used_rate:.2f}")
        print(f"  prism_at_lsp_off:  {_fmt_delta(cell.prism_at_lsp_off)}")
        print(f"  prism_at_lsp_on:   {_fmt_delta(cell.prism_at_lsp_on)}  [primary gate]")
        print(f"  lsp_at_prism_off:  {_fmt_delta(cell.lsp_at_prism_off)}")
        print(f"  lsp_at_prism_on:   {_fmt_delta(cell.lsp_at_prism_on)}")
        print(f"  interaction:       {_fmt_delta(cell.interaction)}")

    # Print pooled detectability
    det = report.detectability
    print(f"\n=== Pooled Detectability ===")
    print(f"  correct/n: {det.correct}/{det.n}  pvalue: {det.pvalue:.4f}  detectable: {det.detectable}")
    if det.detectable:
        print("  WARNING: judge prism-delta is INVALID (condition detectable above chance, spec §6b)")
    else:
        print("  OK: judge prism-delta valid (condition not detectable above chance)")

    print(f"\nRun artifacts: {store.dir}")


def _fmt_delta(d: dict) -> str:
    return "  ".join(f"{m}: {v:+.3f}" for m, v in sorted(d.items()))


# ---------------------------------------------------------------------------
# Part-C cell helpers (Task 11)
# ---------------------------------------------------------------------------

def _run_partc_fake(cell: tuple) -> None:
    """Run run_partc_cell with fake comps (no live LLM calls) and print the report."""
    from .model import Dose, ArmOutput, Variant, Citation
    from .partc import run_partc_cell, render_partc

    repo, stage, model = cell

    # Build a minimal fake ArmOutput with 2 prism calls and clean text
    fake_citations = [Citation(file="src/main.go", line=1, symbol=None)]
    fake_arm_out = ArmOutput(
        variant=Variant(model, True),
        text="clean spec, no tool names; src/main.go:1",
        citations=fake_citations,
        tokens=10,
        tool_calls=2,
        wall_s=0.0,
        used_prism=True,
        prism_calls=2,
        dose=Dose(count=2),
        low_dose=False,
    )

    class _FakeComps:
        _call = 0

        def load_base(self, c):
            return "base spec text src/main.go:1"

        def extract_citations(self, text):
            from .citations import parse_citations
            return parse_citations(text)

        def score(self, citations, **kwargs):
            self._call += 1
            # base first (0.4), on second (0.7) — illustrative fake values
            return 0.4 if self._call == 1 else 0.7

        def run_on_arm(self, c):
            return fake_arm_out

    partc_cell = run_partc_cell(cell, _FakeComps())
    print(render_partc([partc_cell]))


class _LivePartCComps:
    """Real components for a live Part-C cell run.

    Mirrors LiveComponents used by _run_live_cmd: real checkout, real runner,
    real oracle.  The base is loaded from the recovered prism-off tree via
    Task 7 partc_baseline.load_base; citations are extracted via citations.py;
    scoring uses investigator.score_citations with a RelevanceAllTrue oracle
    (full judge is a follow-up).  The on-arm is run via arm_runner.run_arm_isolated
    with the Task 10 steer="prism_on" prompt from prompts.stage_prompt.
    """

    def __init__(self, *, bench_root: str, base_root: str):
        self._bench_root = bench_root
        self._base_root = base_root

    def load_base(self, cell: tuple) -> str:
        repo, stage, model = cell
        from .partc_baseline import load_base
        return load_base(model=model, repo=repo, stage=stage, root=self._base_root)

    def extract_citations(self, text: str):
        from .citations import parse_citations
        return parse_citations(text)

    def score(self, citations, *, cell: tuple, arm: str) -> float:
        """Score citations against the repo at HEAD (existence + RelevanceAllTrue oracle).

        Full judge scoring (LlmRelevanceJudge) is a follow-up; this uses
        RelevanceAllTrue so the live path runs without API keys for the oracle.
        """
        if not citations:
            return 0.0
        repo, stage, model = cell
        repo_path = os.path.join(self._bench_root, repo)

        class _HeadCo:
            """Minimal checkout-like object pointing at a repo on-disk (no git ops)."""
            def __init__(self, root: str):
                self._root = root
            def file_exists(self, rel: str) -> bool:
                return os.path.exists(os.path.join(self._root, rel))
            def read_line(self, rel: str, line: int):
                try:
                    with open(os.path.join(self._root, rel)) as f:
                        lines = f.readlines()
                    return lines[line - 1].rstrip() if 0 < line <= len(lines) else None
                except OSError:
                    return None

        from .investigator import score_citations, RelevanceAllTrue
        co = _HeadCo(repo_path)
        report = score_citations(co, citations, claim_count=max(len(citations), 1),
                                 relevance=RelevanceAllTrue(), issue_text="")
        return report.precision

    def run_on_arm(self, cell: tuple):
        """Run ONE steered prism-on arm via ClaudeRunner in an isolated checkout."""
        repo, stage, model = cell
        import types
        from .arm_runner import ClaudeRunner, run_arm_isolated
        from .model import Variant
        from .prompts import stage_prompt

        repo_path = os.path.join(self._bench_root, repo)
        # Minimal checkout-like object exposing .root for run_arm_isolated
        checkout = types.SimpleNamespace(root=repo_path)

        variant = Variant(model, prism=True)
        prompt = stage_prompt(stage, issue_text="", scoped_slice="", steer="prism_on")

        runner = ClaudeRunner(no_cache=True)
        iso = run_arm_isolated(runner, checkout=checkout, variant=variant,
                               stage=stage, prompt=prompt, no_cache=True)
        return iso.out


def _run_partc_live(cell: tuple, *, bench_root: str, base_root: str) -> None:
    """Build real components and run one Part-C cell live."""
    from .partc import run_partc_cell, render_partc

    comps = _LivePartCComps(bench_root=bench_root, base_root=base_root)
    partc_cell = run_partc_cell(cell, comps)
    print(render_partc([partc_cell]))
