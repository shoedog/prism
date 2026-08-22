"""tier-c entry (spec). Supports --list and (Phase-1b/1c) a `run` subcommand.
Mirrors tier_a/cli.py argument-parsing style."""
from __future__ import annotations
import argparse
import os
from .corpus import load_issues
from .arm_runner import _prism_mcp_bin, _prism_bin
from .partc import render_partc
from .prompts import stage_prompt
from .checkout import Checkout


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
    partc_p.add_argument(
        "--issues",
        default="tier_c/issues/issues.toml",
        help="issues TOML file (default: tier_c/issues/issues.toml)",
    )
    partc_p.add_argument(
        "--run-id", default=None,
        help="run identifier for artifact namespacing (default: auto timestamp partc-YYYYmmdd-HHMMSS)",
    )
    partc_p.add_argument(
        "--force-new", action="store_true",
        help="override run-id collision guard (overwrites existing run dir)",
    )
    partc_p.add_argument(
        "--run-store-root", default=None,
        help="root dir for run artifacts (default: eval/tier_c/runs next to this file)",
    )
    partc_p.add_argument(
        "--reuse-off-from", default=None, metavar="RUN_DIR",
        help="reconstruct the prism-OFF arm from this saved run dir instead of re-running it "
             "(holds the baseline constant when only the on-arm/steer changed; no off-arm spend)",
    )
    partc_p.add_argument(
        "--prism-build-dir", default=None, metavar="DIR",
        help="F1 matched-binary preflight: explicit build dir containing BOTH `prism` and "
             "`prism-mcp` (default: $PRISM_BUILD_DIR, else the parent dir of $PRISM_BIN, "
             "else <repo>/target/release). The run FAILS LOUD before any cell work if the "
             "two binaries cannot be matched to one build.",
    )
    partc_p.add_argument(
        "--skip-warm-gate", action="store_true",
        help="F4 escape hatch: skip the per-cell warm-initialize gate (spawns a throwaway "
             "prism-mcp + JSON-RPC initialize/tools-list handshake before launching the "
             "on-arm agent). Debugging only — the gate is ON by default so a cell can never "
             "silently run with a not-actually-warm prism-mcp.",
    )
    partc_p.add_argument(
        "--warm-gate-timeout-s", type=float, default=15.0, metavar="SECONDS",
        help="F4: seconds to wait for the prism-mcp initialize+tools/list handshake before "
             "failing the cell (default: 15.0)",
    )

    # Re-score a saved Part-C run WITHOUT re-running the arms (judge/scoring fixes are free).
    rescore_p = sub.add_parser(
        "rescore",
        help="re-score a saved Part-C run dir from its raw streams (no arm re-run)",
    )
    rescore_p.add_argument("--run-dir", required=True,
                           help="source Part-C run dir (contains manifest.json + *.raw.jsonl)")
    rescore_p.add_argument("--bench-root", default="~/code/bench-repos",
                           help="bench-repos root (default: ~/code/bench-repos; else manifest's)")
    rescore_p.add_argument("--issues", default=None,
                           help="issues TOML (default: the path recorded in the run's manifest)")
    rescore_p.add_argument("--out-run-id", default=None,
                           help="output run-id (default: <src>-rescore); NEVER clobbers the source")
    rescore_p.add_argument("--run-store-root", default=None,
                           help="root dir for the rescored artifacts (default: eval/tier_c/runs/partc)")
    rescore_p.add_argument("--judge", choices=["sonnet", "codex"], default="sonnet",
                           help="judge primary model: sonnet (sonnet-4.6) or codex (gpt-5.5-xhigh)")

    # Part-D: emit gold CANDIDATE sites for one/all structural tasks (spec P2).
    # Does NOT decide truth — writes candidates.json + adjudicate.md for the
    # controller's source-verified adjudication pass, which freezes gold.json.
    buildgold_p = sub.add_parser(
        "build-gold",
        help="emit Part-D gold CANDIDATE sites (LSP ∪ prism, provenance-tagged) for a structural task",
    )
    buildgold_p.add_argument("--task", default=None,
                             help="structural task id (default: all tasks; requires --all)")
    buildgold_p.add_argument("--all", action="store_true",
                             help="build candidates for every task in --structural-issues")
    buildgold_p.add_argument("--structural-issues", default="tier_c/issues/structural.toml",
                             help="structural task TOML (default: tier_c/issues/structural.toml)")
    buildgold_p.add_argument("--bench-root", default="~/code/bench-repos",
                             help="root directory containing cloned benchmark repos")
    buildgold_p.add_argument("--gold-root", default=None,
                             help="output root for gold/<task_id>/ (default: eval/tier_c/gold)")

    # Part-D: run ONE (task_id, model) cell — steered prism-on impact arm vs
    # unsteered-off, scored via structural.py against the frozen gold.json.
    partd_p = sub.add_parser(
        "run-partd",
        help="run ONE Part-D structural-impact cell (steered prism-on vs unsteered-off)",
    )
    partd_p.add_argument("--task", required=True, help="structural task id")
    partd_p.add_argument("--model", required=True, help="model id (e.g. opus-4.8, gpt-5.5)")
    partd_p.add_argument("--live", action="store_true",
                         help="execute a real live cell (requires a frozen gold.json + bench-repos)")
    partd_p.add_argument("--bench-root", default="~/code/bench-repos",
                         help="root directory containing cloned benchmark repos")
    partd_p.add_argument("--structural-issues", default="tier_c/issues/structural.toml",
                         help="structural task TOML (default: tier_c/issues/structural.toml)")
    partd_p.add_argument("--gold-root", default=None,
                         help="root containing gold/<task_id>/gold.json (default: eval/tier_c/gold)")
    partd_p.add_argument("--run-id", default=None,
                         help="run identifier; required for --live (use --force-new to override collision guard)")
    partd_p.add_argument("--force-new", action="store_true",
                         help="override run-id collision guard (overwrites existing run dir)")
    partd_p.add_argument("--run-store-root", default=None,
                         help="root dir for run artifacts (default: eval/tier_c/runs/partd)")
    partd_p.add_argument("--prism-build-dir", default=None,
                         help="F1 matched-binary preflight build dir (default resolution mirrors run-partc)")
    partd_p.add_argument("--skip-warm-gate", action="store_true",
                         help="F4 escape hatch: skip the per-cell warm-initialize gate")
    partd_p.add_argument("--warm-gate-timeout-s", type=float, default=15.0,
                         help="F4: seconds to wait for the prism-mcp handshake (default: 15.0)")

    args = ap.parse_args(argv)

    if args.cmd == "rescore":
        from .rescore import rescore_run_dir
        judge_primary = {"sonnet": "sonnet-4.6", "codex": "gpt-5.5-xhigh"}[args.judge]
        cell_result, out_dir = rescore_run_dir(
            os.path.expanduser(args.run_dir),
            bench_root=args.bench_root,
            issues_path=args.issues,
            out_run_id=args.out_run_id,
            runs_root=args.run_store_root,
            judge_primary=judge_primary,
        )
        print(render_partc([cell_result]))  # render_partc takes a list of cells
        print(f"\nrescored (arms NOT re-run) → {out_dir}")
        return 0

    if args.cmd == "build-gold":
        from .structural_corpus import load_structural_tasks
        from .buildgold import build_gold

        tasks = load_structural_tasks(args.structural_issues)
        if args.task:
            tasks = [t for t in tasks if t.id == args.task]
            if not tasks:
                ap.error(f"no such structural task: {args.task!r}")
        elif not args.all:
            ap.error("build-gold requires --task <id> or --all")

        bench_root = os.path.expanduser(args.bench_root)
        gold_root = args.gold_root or _default_gold_root()
        for task in tasks:
            with Checkout(os.path.join(bench_root, task.repo), task.sha) as co:
                result, cand_path, adjudicate_path = build_gold(task, co, out_root=gold_root)
            both = sum(1 for s in result.sites if s.provenance == "both")
            band = len(result.sites) - both
            print(f"{task.id}: {len(result.sites)} candidate sites "
                 f"({both} auto-accepted, {band} need adjudication)")
            print(f"  oracle_health: {result.oracle_health}")
            print(f"  candidates: {cand_path}")
            print(f"  adjudicate: {adjudicate_path}")
        return 0

    if args.cmd == "run-partd":
        if not args.live:
            print(
                "live run requires --live + a frozen gold/<task_id>/gold.json + bench-repos;\n"
                "pass --live --run-id <id> to execute one (task, model) cell."
            )
            return 0
        if not args.run_id:
            ap.error("--run-id is required for --live (use --force-new to override collision guard)")

        from .partd import _run_partd_live

        bench_root = os.path.expanduser(args.bench_root)
        gold_root = args.gold_root or _default_gold_root()
        runs_root = args.run_store_root or _default_partd_run_store_root()
        _run_partd_live(
            (args.task, args.model),
            bench_root=bench_root,
            structural_issues_path=args.structural_issues,
            gold_root=gold_root,
            run_id=args.run_id,
            runs_root=runs_root,
            force_new=args.force_new,
            prism_build_dir=args.prism_build_dir,
            skip_binary_preflight=False,
            skip_warm_gate=args.skip_warm_gate,
            warm_gate_timeout_s=args.warm_gate_timeout_s,
        )
        return 0

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
            issues_path = args.issues
            run_id = args.run_id or _default_partc_run_id()
            runs_root = args.run_store_root or _default_partc_run_store_root()
            force_new = args.force_new
            _run_partc_live(cell, bench_root=bench_root, base_root=base_root,
                            issues_path=issues_path, run_id=run_id,
                            runs_root=runs_root, force_new=force_new,
                            reuse_off_from=args.reuse_off_from,
                            prism_build_dir=args.prism_build_dir,
                            skip_binary_preflight=False,
                            skip_warm_gate=args.skip_warm_gate,
                            warm_gate_timeout_s=args.warm_gate_timeout_s)
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


def _default_partc_run_store_root() -> str:
    """Default Part-C run store: eval/tier_c/runs/partc/ (gitignored)."""
    return os.path.join(os.path.dirname(os.path.abspath(__file__)), "runs", "partc")


def _default_partd_run_store_root() -> str:
    """Default Part-D run store: eval/tier_c/runs/partd/ (gitignored)."""
    return os.path.join(os.path.dirname(os.path.abspath(__file__)), "runs", "partd")


def _default_gold_root() -> str:
    """Default Part-D gold root: eval/tier_c/gold/ (committed — candidates.json,
    adjudicate.md, and the frozen gold.json are the audit trail + scoring input)."""
    return os.path.join(os.path.dirname(os.path.abspath(__file__)), "gold")


def _default_partc_run_id() -> str:
    """Generate a unique timestamp-based Part-C run id: partc-YYYYmmdd-HHMMSS-XXXX.

    The 4-hex suffix is derived from os.getpid() + time.monotonic() so two calls
    within the same second produce distinct ids without random state.
    """
    import time as _time
    from datetime import datetime
    ts = datetime.now().strftime("partc-%Y%m%d-%H%M%S")
    # Combine pid and monotonic counter to build a deterministic short suffix.
    raw = os.getpid() ^ (int(_time.monotonic() * 1e6) & 0xFFFF)
    suffix = format(raw & 0xFFFF, "04x")
    return f"{ts}-{suffix}"


def _prism_build_id(path: str) -> str:
    """Build identity of the prism-mcp binary = full sha256 of its bytes.

    Full 64-hex digest so audit/replay gets unambiguous binary identity.
    (prism-mcp has no --version flag; content hash is the only reliable identity.)
    """
    import hashlib
    try:
        with open(path, "rb") as f:
            return "sha256:" + hashlib.sha256(f.read()).hexdigest()
    except Exception as e:
        return f"error:{e}"


def _write_partc_manifest(
    *,
    cell: tuple,
    issue,
    bench_root: str,
    base_root: str,
    issues_path: str,
    run_id: str,
    runs_root: str,
    binary_preflight: dict | None = None,
    cache_dir: str | None = None,
) -> str:
    """Write <runs_root>/<run_id>/manifest.json for the Part-C live run.

    Fields: timestamp, cell, issue {key,language,repo,sha,url}, bench_root,
    base_root, issues_path, prism_mcp_bin+sha, prism_bin+sha,
    harness_git_sha, config_summary per arm, steer per arm.
    binary_preflight (F1): the resolve_matched_binaries() dict (build_dir + mtimes/sizes
    for both binaries) when the preflight ran; {} when skipped.
    cache_dir (F2): the shared prism nav-cache directory used by this run's prewarm +
    both agent MCP configs; None when not threaded (e.g. --skip-warm-gate-era callers).
    Returns the path written.
    """
    import json
    import hashlib
    import subprocess
    from datetime import datetime, timezone

    repo, stage, model = cell

    # Binary paths + sha256
    mcp_bin = _prism_mcp_bin()
    mcp_sha = _prism_build_id(mcp_bin)
    prism_bin_path = _prism_bin()
    prism_bin_sha = _prism_build_id(prism_bin_path)

    # Harness git SHA (best-effort)
    try:
        git_out = subprocess.run(
            ["git", "rev-parse", "HEAD"], capture_output=True, text=True, timeout=10,
            cwd=os.path.dirname(os.path.abspath(__file__)),
        )
        harness_git_sha = git_out.stdout.strip() or "unknown"
    except Exception as e:
        harness_git_sha = f"error:{e}"

    # Dirty git status (best-effort; truncated to 4000 chars)
    try:
        git_status_out = subprocess.run(
            ["git", "status", "--porcelain"], capture_output=True, text=True, timeout=10,
            cwd=os.path.dirname(os.path.abspath(__file__)),
        )
        dirty_git = (git_status_out.stdout or "").strip()[:4000]
    except Exception as e:
        dirty_git = f"error:{e}"

    # Thinking: both arms' isolated settings.json set "showThinkingSummaries": true
    # (adoption.env.build_isolated_config), so claude emits thinking summaries into the
    # stream — preserved verbatim in raw_stdout (saved unstripped by _persist_one_arm).
    show_thinking_summaries = True
    thinking_enabled = True

    manifest = {
        "timestamp": datetime.now(tz=timezone.utc).isoformat(),
        "cell": {"repo": repo, "stage": stage, "model": model},
        "issue": {
            "key": getattr(issue, "key", ""),
            "language": getattr(issue, "language", ""),
            "repo": getattr(issue, "repo", ""),
            "sha": getattr(issue, "sha", ""),
            "url": getattr(issue, "url", ""),
        },
        "bench_root": bench_root,
        "base_root": base_root,
        "issues_path": issues_path,
        "prism_mcp_bin": mcp_bin,
        "prism_mcp_sha": mcp_sha,
        "prism_bin": prism_bin_path,
        "prism_bin_sha": prism_bin_sha,
        "binary_preflight": binary_preflight or {},
        "prism_cache_dir": cache_dir,
        "harness_git_sha": harness_git_sha,
        "dirty_git": dirty_git,
        "thinking_enabled": thinking_enabled,
        "show_thinking_summaries": show_thinking_summaries,
        "config_summary": {
            # Claude prism-ON arm: skill + mcp__prism allow, deny Write/Edit
            "on": {
                "skill_present": True,
                "allow_includes_mcp_prism": True,
                "allow": ["Read", "Grep", "Glob", "Bash", "mcp__prism"],
                "deny": ["Write", "Edit"],
            },
            # Claude prism-OFF arm: no skill, no mcp__prism, deny Write/Edit
            "off": {
                "skill_present": False,
                "allow_includes_mcp_prism": False,
                "allow": ["Read", "Grep", "Glob", "Bash"],
                "deny": ["Write", "Edit"],
            },
            # Codex arm: uses workspace-write sandbox (-s workspace-write in build_codex_cmd)
            # NOTE: codex does NOT use claude CLAUDE_CONFIG_DIR deny/allow; the sandbox flag
            # controls filesystem access at the OS level.  Write/Edit ARE permitted by codex.
            "codex": {
                "sandbox": "workspace-write",
                # Both codex arms now write config.toml (on-arm adds [mcp_servers.prism];
                # off-arm is reasoning-only) — see adoption.codex_env.build_isolated_codex_home.
                "prism_via": "CODEX_HOME/config.toml [mcp_servers.prism] (on-arm) / reasoning-only config.toml (off-arm)",
                # Reasoning exposure: config.toml sets these in BOTH arms (auditability).
                "hide_agent_reasoning": False,
                "show_raw_agent_reasoning": True,
            },
        },
        # Reasoning/thinking exposure across both CLIs, for after-the-fact audit:
        #   claude → showThinkingSummaries (settings.json); codex → raw agent reasoning (config.toml).
        "reasoning_exposure": {
            "claude": {"show_thinking_summaries": show_thinking_summaries},
            "codex": {"hide_agent_reasoning": False, "show_raw_agent_reasoning": True},
        },
        "steer": {"on": "prism_on", "off": ""},
    }

    run_dir = os.path.join(runs_root, run_id)
    os.makedirs(run_dir, exist_ok=True)
    path = os.path.join(run_dir, "manifest.json")
    with open(path, "w") as f:
        json.dump(manifest, f, indent=2)
    return path


def _build_manifest(*, issues, bench_root: str, run_id: str, run_store_root: str) -> dict:
    """Build the run manifest (models, prism bin/SHA, harness git SHA, corpus, env)."""
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
    from .llm import live_ask, JUDGE_MODEL
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
    relevance = LlmRelevanceJudge(live_ask, JUDGE_MODEL)  # sonnet oracle (opus went agentic on YES/NO)
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
    from .partc import run_partc_cell

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

        def run_off_arm(self, c):
            from .model import Dose, ArmOutput, Variant, Citation
            off_citations = [Citation(file="src/main.go", line=2, symbol=None)]
            return ArmOutput(
                variant=Variant(model, False),
                text="off-arm baseline text src/main.go:2",
                citations=off_citations,
                tokens=8,
                tool_calls=0,
                wall_s=0.0,
                used_prism=False,
                prism_calls=0,
                dose=Dose(count=0),
                low_dose=False,
            )

        def score(self, citations, **kwargs):
            from .investigator import InvestigatorReport, CitationVerdict, CitationVerdict
            from .model import Citation as _Cit
            self._call += 1
            # off-arm scored first → base precision (0.4) with 1 valid cite
            if self._call == 1:
                cite = _Cit(file="src/main.go", line=2, symbol=None)
                v = CitationVerdict(cite=cite, file_ok=True, line_ok=True,
                                    symbol_ok=True, relevant=True)
                return InvestigatorReport(precision=0.4, recall=0.4,
                                         hallucinations=0, verdicts=[v])
            # on-arm second → on precision (0.7) with 1 valid + 1 halluc + 1 irrelevant
            c1 = _Cit(file="src/main.go", line=1, symbol=None)
            c2 = _Cit(file="src/missing.go", line=99, symbol=None)
            c3 = _Cit(file="src/other.go", line=5, symbol=None)
            v1 = CitationVerdict(cite=c1, file_ok=True, line_ok=True,
                                 symbol_ok=True, relevant=True)
            v2 = CitationVerdict(cite=c2, file_ok=False, line_ok=False,
                                 symbol_ok=True, relevant=True)
            v3 = CitationVerdict(cite=c3, file_ok=True, line_ok=True,
                                 symbol_ok=True, relevant=False)
            return InvestigatorReport(precision=0.7, recall=0.7,
                                     hallucinations=1, verdicts=[v1, v2, v3])

        def run_on_arm(self, c):
            return fake_arm_out

    partc_cell = run_partc_cell(cell, _FakeComps())
    print(render_partc([partc_cell]))


class _LivePartCComps:
    """Real components for a live Part-C cell run.

    Mirrors LiveComponents used by _run_live_cmd: real pinned checkout, real
    runner (ClaudeRunner/CodexRunner by model family), and the real
    LlmRelevanceJudge oracle so both base and on scoring see issue.text + the
    code window at each cited location (defect #2 fix — NOT RelevanceAllTrue).

    ``ask`` is injectable for unit-testing (default: live_ask).  One
    _LivePartCComps is opened per cell inside a single Checkout context manager.
    """

    def __init__(self, *, co, issue, model: str, base_root: str, ask=None,
                 reuse_off_from: str | None = None,
                 judge_primary: str | None = None,
                 cache_dir: str | None = None,
                 skip_warm_gate: bool = True,
                 warm_gate_timeout_s: float = 15.0):
        if ask is None:
            from .llm import live_ask
            ask = live_ask
        self._co = co
        self._issue = issue
        self._model = model
        self._base_root = base_root
        self._ask = ask
        # When set, run_off_arm reconstructs the off-arm from this saved run dir instead of
        # running it live — holds the baseline constant when only the on-arm/steer changed.
        self._reuse_off_from = reuse_off_from
        # When set, overrides the default JUDGE_MODEL for relevance/quality judges.
        self._judge_primary = judge_primary
        # F2/F4 (harness hardening): shared cache dir + warm-gate config threaded down to
        # run_arm_isolated/the runners. skip_warm_gate defaults to True (gate skipped) so
        # direct/test construction of this class is unaffected; _run_partc_live passes the
        # CLI-resolved values explicitly for real --live runs.
        self._cache_dir = cache_dir
        self._skip_warm_gate = skip_warm_gate
        self._warm_gate_timeout_s = warm_gate_timeout_s
        self._last_off = None         # set after run_off_arm; carries ArmOutput for audit persistence
        self._last_on = None          # set after run_on_arm; carries ArmOutput for audit persistence
        self._last_off_prompt = None  # exact prompt sent to the off-arm
        self._last_on_prompt = None   # exact prompt sent to the on-arm
        self._last_prewarm = None     # prewarm telemetry dict from run_arm_isolated
        self._last_off_judge: list[dict] = []  # per-cite judge records for off-arm
        self._last_on_judge: list[dict] = []   # per-cite judge records for on-arm

    def _upstream_spec(self, cell: tuple) -> str:
        """Return the recovered prism-off SPEC body for a plan cell; '' for spec cells.

        The plan arm and plan oracle both plan from the SAME spec (the prism-off
        recovered baseline spec for this model+repo), so only the tool availability
        during planning differs.
        """
        repo, stage, model = cell
        if stage != "plan":
            return ""
        from .partc_baseline import load_base
        return load_base(model=model, repo=repo, stage="spec", root=self._base_root)

    def load_base(self, cell: tuple) -> str:
        repo, stage, model = cell
        from .partc_baseline import load_base
        return load_base(model=model, repo=repo, stage=stage, root=self._base_root)

    def extract_citations(self, text: str):
        from .citations import parse_citations
        return parse_citations(text)

    def score(self, citations, *, cell: tuple, arm: str):
        """Score citations using the pinned checkout + the real LlmRelevanceJudge.

        Both base and on calls see issue.text (and for plan cells, also the
        upstream spec) plus the code window at each cited location, mirroring
        how chain.py builds plan_issue_text (spec §6a).

        Per-cite judge artifacts (verdict, escalated, votes, relevant) are captured
        and stored on self._last_off_judge (arm="base") / self._last_on_judge (arm="on")
        for later persistence as <base>.<arm>.judge.jsonl.

        D0 REPAIR: the recall denominator is claims.count_claims(arm_text) — the
        substantive-code-claim count of the arm's OWN text (self._last_off/self._last_on,
        set by run_off_arm/run_on_arm before score() is called) — NOT
        max(len(citations), 1). The old code made claim_count identically equal to the
        citation count, which forced recall == precision whenever >=1 citation was made,
        silently disabling the under-citing penalty (spec §6a). When no arm text is available
        (e.g. a caller that constructs this class and calls score() directly, bypassing
        run_off_arm/run_on_arm), we fall back to the old max(len(citations), 1) behaviour
        so pre-existing direct callers/tests are unaffected.

        Batch judging (perf fix): relevance is judged in ONE model call per arm (no
        ensemble) instead of one ensemble call PER citation. Every citation is first
        pre-resolved through investigator.verify_citation (the SAME resolver
        score_citations itself uses) with a code-window-capturing stand-in relevance
        object — this costs zero model calls, it only tells us which citations will
        actually reach a relevance check (file_ok/line_ok/symbol_ok) and reads their
        code windows. Those are classified in ONE LlmRelevanceJudge.relevance_batch
        call, then looked up by (file, line, symbol) via a small precomputed oracle
        injected into the REAL score_citations call — score_citations's signature and
        the RelevanceJudge protocol are unchanged; the precomputed oracle just
        satisfies the protocol.

        Returns an InvestigatorReport (precision, recall, hallucinations, verdicts).
        """
        from .investigator import score_citations, verify_citation, InvestigatorReport
        from .judges_live import LlmRelevanceJudge
        from .llm import JUDGE_MODEL
        from .claims import count_claims
        if not citations:
            if arm == "on":
                self._last_on_judge = []
            else:
                self._last_off_judge = []
            return InvestigatorReport(precision=0.0, recall=0.0, hallucinations=0, verdicts=[])
        upstream = self._upstream_spec(cell)
        issue_text = (self._issue.text + "\n\n## Upstream spec\n" + upstream
                      if upstream else self._issue.text)
        last = self._last_on if arm == "on" else self._last_off
        arm_text = last.text if last is not None else ""
        claim_count = count_claims(arm_text) if arm_text else max(len(citations), 1)

        # --- Batch judging (perf fix): pre-resolve, then ONE relevance_batch call ---
        class _CaptureCode:
            """Stand-in RelevanceJudge: records (cite, code) for every citation that
            reaches the relevance check inside verify_citation, WITHOUT spending a
            model call (this is a local Python object, not self._ask)."""

            def __init__(self):
                self.items: list[tuple] = []

            def is_relevant(self, cite, issue_text, code: str = "") -> bool:
                self.items.append((cite, code))
                return True  # placeholder; overwritten by relevance_batch below

        capture = _CaptureCode()
        for c in citations:
            verify_citation(self._co, c, issue_text=issue_text, relevance=capture,
                            read_code=lambda f, l: self._co.read_window(f, l),
                            text=arm_text, ask=self._ask)

        # Sonnet, not opus — owner-tested to follow the YES/NO instruction (opus went agentic).
        # judge_primary overrides JUDGE_MODEL when a codex ensemble is requested.
        rel_judge = LlmRelevanceJudge(self._ask, self._judge_primary or JUDGE_MODEL)
        batch_verdicts = (rel_judge.relevance_batch(capture.items, issue_text)
                          if capture.items else [])
        lookup = {
            (c.file, c.line, c.symbol): v
            for (c, _code), v in zip(capture.items, batch_verdicts)
        }
        records: list[dict] = [
            {"file": c.file, "line": c.line, "symbol": c.symbol,
             "verdict": "YES" if v else "NO", "escalated": False, "votes": [],
             "relevant": v}
            for (c, _code), v in zip(capture.items, batch_verdicts)
        ]

        class _PrecomputedRelevance:
            """Satisfies the RelevanceJudge protocol by looking up the batch verdict
            computed above; default False (conservative) on a miss."""

            def is_relevant(self, cite, issue_text, code: str = "") -> bool:
                return lookup.get((cite.file, cite.line, cite.symbol), False)

        result = score_citations(
            self._co,
            citations,
            claim_count=claim_count,
            relevance=_PrecomputedRelevance(),
            issue_text=issue_text,
            read_code=lambda f, l: self._co.read_window(f, l),
            # resolver-fix-spec.md R1/R2: `text` feeds the layer-3 salient-token
            # tie-break + R2 Q3 disambiguator context (the citation's own enclosing
            # sentence); `ask` wires the SAME judge seam this class already uses
            # (self._ask) into the disambiguator, invoked only on a genuine
            # >=2-candidate tie (rare) — never a new live-call surface.
            text=arm_text,
            ask=self._ask,
        )
        # Store per-cite judge records on self for later persistence
        if arm == "on":
            self._last_on_judge = records
        else:
            self._last_off_judge = records
        return result

    def head_to_head(self, off_out, on_out, cell: tuple) -> dict:
        """Anonymized head-to-head spec-quality comparison (off vs on) via the ensemble."""
        from .judges_live import SpecQualityJudge
        judge = SpecQualityJudge(self._ask, self._judge_primary)
        return judge.compare(self._issue.text, off_out.text or "", on_out.text or "")

    def score_validity(self, text: str, *, arm: str) -> dict:
        """D1 — citation VALIDITY (judged): does the code AT each cited location support
        the sentence that cites it? Independent of the relevance judge (score()) and of
        prism (never the oracle here — circularity doctrine). Rescorable: operates only
        on the already-saved arm TEXT, exactly like D0's count_claims() denominator.

        Returns a JSON-safe dict: {support_rate, contradicted, total, verdicts:[...]}."""
        from .validity import CitationValidityJudge
        from .validity import score_validity as _score_validity
        if not text:
            return {"support_rate": 0.0, "contradicted": 0, "total": 0, "verdicts": []}
        judge = CitationValidityJudge(self._ask, self._judge_primary)
        # ask=self._ask (R5): a bare-but-tied citation gets one more shot at the R2 Q3
        # disambiguator before D1 gives up and reads no window (see validity.py).
        report = _score_validity(self._co, text, judge, ask=self._ask)
        return {
            "support_rate": report.support_rate,
            "contradicted": report.contradicted,
            "total": report.total,
            "verdicts": [
                {
                    "file": v.claim.cite.file, "line": v.claim.cite.line,
                    "symbol": v.claim.cite.symbol, "sentence": v.claim.sentence,
                    "verdict": v.verdict, "escalated": v.escalated,
                }
                for v in report.verdicts
            ],
        }

    def head_to_head_annotated(self, off_annotated: str, on_annotated: str, cell: tuple) -> dict:
        """D3 — fact-annotated head-to-head: the SAME SpecQualityJudge ensemble as
        head_to_head() above (kept byte-identical — spec cardinal constraint), fed the
        machine-annotated pair instead of the raw text. A fully separate call/verdict."""
        from .judges_live import SpecQualityJudge
        judge = SpecQualityJudge(self._ask, self._judge_primary)
        return judge.compare(self._issue.text, off_annotated or "", on_annotated or "")

    def score_relational(self, off_text: str, on_text: str, *, cell: tuple) -> dict:
        """D2 — relational-fact accuracy (mechanical; prism NEVER the oracle). ONE cheap
        extractor call runs IDENTICALLY on both arms (same prompt template + model,
        blind/symmetric — it never sees which arm is which). Verification is mechanical:
        calls()/called_by() -> NullCallOracle (fail-open UNKNOWN stub; full LSP
        call-hierarchy wiring is a follow-up, see relational.py module docstring);
        depends() -> a neutral per-language import-text parser (NOT prism module-deps).

        Returns {"off": {...}, "on": {...}, "delta": on.precision - off.precision}."""
        from .llm import JUDGE_MODEL
        from .relational import (
            NullCallOracle,
            extract_relational_claims,
            score_relational_claims,
        )
        extractor_model = self._judge_primary or JUDGE_MODEL
        language = getattr(self._issue, "language", "") or ""
        call_oracle = NullCallOracle()  # TODO(D2 follow-up): tier_a.oracles.LspOracle bridge
        off_claims = extract_relational_claims(self._ask, extractor_model, off_text)
        on_claims = extract_relational_claims(self._ask, extractor_model, on_text)
        off_report = score_relational_claims(off_claims, call_oracle=call_oracle,
                                             co=self._co, language=language)
        on_report = score_relational_claims(on_claims, call_oracle=call_oracle,
                                            co=self._co, language=language)

        def _to_dict(report) -> dict:
            return {
                "precision": report.precision,
                "unknown_rate": report.unknown_rate,
                "contradicted": report.contradicted,
                "total": report.total,
                "verdicts": [
                    {"kind": v.claim.kind, "a": v.claim.a, "b": v.claim.b, "status": v.status}
                    for v in report.verdicts
                ],
            }

        return {
            "off": _to_dict(off_report),
            "on": _to_dict(on_report),
            "delta": on_report.precision - off_report.precision,
        }

    def run_off_arm(self, cell: tuple):
        """Run ONE fresh prism-OFF status-quo arm inside the pinned checkout.

        No steer, no prism MCP, no prewarm — mirrors what the agent would do without
        any prism tooling (the status-quo baseline).  Uses the SAME de-specified prompt
        as run_on_arm so the only variable between the two arms is tool availability.
        """
        repo, stage, model = cell

        # Reuse path: reconstruct the off-arm from a saved run dir (no live run, no spend).
        if self._reuse_off_from:
            from .rescore import _reconstruct_arm_from_files
            safe_model = model.replace("/", "_").replace(":", "_")
            base = os.path.join(self._reuse_off_from, f"{repo}-{stage}-{safe_model}")
            out = _reconstruct_arm_from_files(base, "off", model)
            prompt_path = f"{base}.off.prompt.txt"
            self._last_off_prompt = (
                open(prompt_path).read() if os.path.exists(prompt_path)
                else f"(off-arm reused from {self._reuse_off_from}; not re-run)")
            self._last_off = out
            return out

        from .arm_runner import ClaudeRunner, CodexRunner, run_arm_isolated
        from .model import Variant

        variant = Variant(model, prism=False)
        upstream = self._upstream_spec(cell)
        prompt = stage_prompt(
            stage,
            issue_text=self._issue.text,
            scoped_slice=self._issue.scoped_slice,
            upstream=upstream,
            # NO steer — status quo, agent must discover the code itself
        )
        self._last_off_prompt = prompt
        runner = (ClaudeRunner(no_cache=False, cache_dir=self._cache_dir) if model.startswith("opus")
                 else CodexRunner(no_cache=False, cache_dir=self._cache_dir))
        iso = run_arm_isolated(
            runner,
            checkout=self._co,
            variant=variant,
            stage=stage,
            prompt=prompt,
            no_cache=False,
            prewarm=False,
            cache_dir=self._cache_dir,
        )
        self._last_off = iso.out
        return iso.out

    def run_on_arm(self, cell: tuple):
        """Run ONE steered prism-on arm inside the pinned checkout."""
        repo, stage, model = cell
        from .arm_runner import ClaudeRunner, CodexRunner, run_arm_isolated
        from .model import Variant

        variant = Variant(model, prism=True)
        upstream = self._upstream_spec(cell)
        prompt = stage_prompt(
            stage,
            issue_text=self._issue.text,
            scoped_slice=self._issue.scoped_slice,
            upstream=upstream,
            steer="prism_on",
        )
        self._last_on_prompt = prompt
        runner = (ClaudeRunner(no_cache=False, cache_dir=self._cache_dir) if model.startswith("opus")
                 else CodexRunner(no_cache=False, cache_dir=self._cache_dir))
        iso = run_arm_isolated(
            runner,
            checkout=self._co,
            variant=variant,
            stage=stage,
            prompt=prompt,
            no_cache=False,
            prewarm=True,
            cache_dir=self._cache_dir,
            skip_warm_gate=self._skip_warm_gate,
            warm_gate_timeout_s=self._warm_gate_timeout_s,
        )
        self._last_on = iso.out
        self._last_prewarm = iso.prewarm
        return iso.out


def _persist_partc_cell(partc_cell, repo: str, stage: str, model: str,
                        *, run_id: str | None = None, runs_root: str | None = None) -> str:
    """Serialize a PartCCell to JSON and write to <runs_root>/<run_id>/.

    Returns the path written.  The runs/ dir is gitignored.
    """
    import json
    import dataclasses

    if runs_root is None:
        runs_root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "runs")
    if run_id is not None:
        runs_dir = os.path.join(runs_root, run_id)
    else:
        runs_dir = os.path.join(runs_root, "partc")
    os.makedirs(runs_dir, exist_ok=True)

    safe_model = model.replace("/", "_").replace(":", "_")
    fname = f"{repo}-{stage}-{safe_model}.json"
    path = os.path.join(runs_dir, fname)

    # Dose is a frozen dataclass with a frozenset — convert for JSON
    def _to_jsonable(obj):
        if dataclasses.is_dataclass(obj) and not isinstance(obj, type):
            d = dataclasses.asdict(obj)
            # frozenset fields -> list
            for k, v in d.items():
                if isinstance(v, (set, frozenset)):
                    d[k] = sorted(v)
            return d
        return obj

    cell_dict = _to_jsonable(partc_cell)
    # frozenset in nested dose
    if "dose" in cell_dict and isinstance(cell_dict["dose"].get("distinct_tools"), (set, frozenset)):
        cell_dict["dose"]["distinct_tools"] = sorted(cell_dict["dose"]["distinct_tools"])

    with open(path, "w") as f:
        json.dump(cell_dict, f, indent=2, default=str)

    return path


def _persist_partc_arm_files(
    *,
    off_out,
    on_out,
    repo: str,
    stage: str,
    model: str,
    runs_dir: str | None = None,
    run_id: str | None = None,
    runs_root: str | None = None,
) -> list[str]:
    """Persist arm text + raw stream for each arm to the namespaced run dir.

    Writes up to four files (two per arm, skipped when the arm is None):
      <repo>-<stage>-<model>.off.out.md
      <repo>-<stage>-<model>.off.raw.jsonl
      <repo>-<stage>-<model>.on.out.md
      <repo>-<stage>-<model>.on.raw.jsonl

    When run_id and runs_root are given, writes to <runs_root>/<run_id>/.
    Legacy runs_dir override is still accepted for direct tests.
    Returns the list of paths written.  The runs/ dir is gitignored and is
    NEVER staged via git add.
    """
    if runs_dir is None:
        if run_id is not None and runs_root is not None:
            runs_dir = os.path.join(runs_root, run_id)
        else:
            _default_root = runs_root or os.path.join(
                os.path.dirname(os.path.abspath(__file__)), "runs")
            runs_dir = os.path.join(_default_root, run_id or "partc")
    os.makedirs(runs_dir, exist_ok=True)

    safe_model = model.replace("/", "_").replace(":", "_")
    base = f"{repo}-{stage}-{safe_model}"
    paths: list[str] = []

    def _write(filename: str, content: str) -> str:
        p = os.path.join(runs_dir, filename)
        with open(p, "w") as f:
            f.write(content)
        return p

    if off_out is not None:
        paths.append(_write(f"{base}.off.out.md", off_out.text))
        paths.append(_write(f"{base}.off.raw.jsonl", off_out.raw_stdout))
    if on_out is not None:
        paths.append(_write(f"{base}.on.out.md", on_out.text))
        paths.append(_write(f"{base}.on.raw.jsonl", on_out.raw_stdout))

    return paths


def _persist_one_arm(
    out,  # ArmOutput | None
    prompt: str | None,
    prewarm,  # dict | None
    *,
    base: str,
    arm: str,  # "off" | "on"
    mcp_args: list | None = None,
    prism: bool | None = None,
) -> list[str]:
    """Write all per-arm artifacts for a single arm, immediately after it runs.

    Always written (even on failure — call this from a finally block):
      <base>.<arm>.meta.json   — argv, returncode, stderr (capped 20000 chars), wall_s,
                                  exception, cwd, mcp_args, prism flag
      <base>.<arm>.prompt.txt — exact prompt sent to the arm

    Written only on success (out is not None and returncode == 0):
      <base>.<arm>.out.md     — arm response text
      <base>.<arm>.raw.jsonl  — full raw stdout stream

    Additionally for on-arm with prewarm:
      <base>.on.prewarm.json  — prewarm telemetry

    Returns list of paths written.
    """
    import json as _json
    from tier_c.arm_runner import ArmRunError

    paths: list[str] = []

    def _w(path: str, content: str) -> str:
        with open(path, "w") as _f:
            _f.write(content)
        paths.append(path)
        return path

    # --- meta.json (always) ---
    if out is not None:
        meta = {
            "argv": getattr(out, "argv", []),
            "returncode": getattr(out, "returncode", 0),
            "stderr": (getattr(out, "stderr", "") or "")[:20000],
            "wall_s": getattr(out, "wall_s", 0.0),
            "exception": None,
            "cwd": getattr(out, "cwd", ""),
            "mcp_args": mcp_args or [],
            "prism": prism if prism is not None else False,
        }
    else:
        # out is None → placeholder (failure path; caller will set exception separately)
        meta = {
            "argv": [],
            "returncode": -1,
            "stderr": "",
            "wall_s": 0.0,
            "exception": None,
            "cwd": "",
            "mcp_args": mcp_args or [],
            "prism": prism if prism is not None else False,
        }

    _w(f"{base}.{arm}.meta.json", _json.dumps(meta, indent=2))

    # --- prompt.txt (always) ---
    _w(f"{base}.{arm}.prompt.txt", prompt or "")

    # --- out.md + raw.jsonl (success only) ---
    if out is not None and getattr(out, "returncode", 0) == 0:
        _w(f"{base}.{arm}.out.md", out.text or "")
        _w(f"{base}.{arm}.raw.jsonl", out.raw_stdout or "")

    # --- prewarm.json (on-arm only) ---
    if arm == "on" and prewarm is not None:
        pw_path = f"{base}.on.prewarm.json"
        with open(pw_path, "w") as _f:
            _json.dump(prewarm, _f, indent=2)
        paths.append(pw_path)

    return paths


def _persist_judge_jsonl(records: list[dict], path: str) -> None:
    """Write per-cite judge records to a .judge.jsonl file (one JSON object per line).

    Each record has: {file, line, symbol, prompt, response, relevant}.
    Always written (empty list → empty file), so the audit trail exists even when
    no citations were scored.
    """
    import json as _json
    with open(path, "w") as _f:
        for rec in records:
            _f.write(_json.dumps(rec) + "\n")


def _run_partc_live(cell: tuple, *, bench_root: str, base_root: str,
                    issues_path: str, run_id: str | None = None,
                    runs_root: str | None = None, force_new: bool = False,
                    reuse_off_from: str | None = None,
                    prism_build_dir: str | None = None,
                    skip_binary_preflight: bool = True,
                    skip_warm_gate: bool = True,
                    warm_gate_timeout_s: float = 15.0,
                    cache_dir: str | None = None) -> None:
    """Open a pinned throwaway worktree at the issue SHA and run one Part-C cell live.

    All artifacts go under <runs_root>/<run_id>/ (collision-guarded unless force_new).
    Per-arm artifacts are written IMMEDIATELY after each arm finishes so a failure
    in one arm does not lose the other arm's context.  status.json is always written
    at the end (success or failure).

    Harness-hardening params (F1/F2/F4):
      prism_build_dir/skip_binary_preflight — F1 matched-binary preflight (resolve_matched_binaries).
      cache_dir                             — F2 shared nav-cache dir for prewarm + both agent
                                               MCP configs; auto-derived as <run_dir>/prism-cache
                                               when None.
      skip_warm_gate/warm_gate_timeout_s    — F4 per-cell warm-initialize gate before the
                                               on-arm agent is launched.
    skip_binary_preflight and skip_warm_gate default to True (skipped) so direct callers
    (unit tests) that predate this hardening are unaffected; the CLI's `--live` path
    always passes skip_binary_preflight=False and threads --skip-warm-gate explicitly.
    """
    import json as _json
    import shutil
    from .partc import run_partc_cell
    from .arm_runner import ArmRunError, resolve_matched_binaries, PreflightError

    repo, stage, model = cell
    if runs_root is None:
        runs_root = _default_partc_run_store_root()
    if run_id is None:
        run_id = _default_partc_run_id()

    # Collision guard + force-new clear. Paths are made ABSOLUTE first (see
    # partd.resolve_run_paths: a relative root silently voided a Part-D slate).
    from .partd import resolve_run_paths
    runs_root, run_dir, cache_dir = resolve_run_paths(runs_root, run_id, cache_dir, runs_root)
    if os.path.exists(run_dir) and not force_new:
        raise FileExistsError(
            f"run-id dir exists: {run_dir} (use --force-new to override)"
        )
    if os.path.exists(run_dir) and force_new:
        shutil.rmtree(run_dir)
    os.makedirs(run_dir, exist_ok=True)

    # F2: one shared cache base per run (absolute; resolved above), passed to prewarm AND both
    # agent MCP configs.

    safe_model = model.replace("/", "_").replace(":", "_")
    base = os.path.join(run_dir, f"{repo}-{stage}-{safe_model}")

    # Track failure state for status.json
    failed_stage: str | None = None
    error_msg: str | None = None
    binary_preflight: dict | None = None

    try:
        # F1: matched-binary preflight — FAIL LOUD before any cell work if prism +
        # prism-mcp cannot be matched to one build (the independent-resolution skew
        # vector that silently degraded prism-ON arms to a cold/never-warm MCP handshake).
        if not skip_binary_preflight:
            try:
                binary_preflight = resolve_matched_binaries(build_dir=prism_build_dir)
            except PreflightError as e:
                failed_stage = "preflight"
                error_msg = str(e)
                raise

        issues = load_issues(issues_path)
        issue = next(i for i in issues if i.repo == repo)

        # Write manifest before running arms (so we have an audit trail even if arms fail)
        _write_partc_manifest(
            cell=cell, issue=issue, bench_root=bench_root, base_root=base_root,
            issues_path=issues_path, run_id=run_id, runs_root=runs_root,
            binary_preflight=binary_preflight, cache_dir=cache_dir,
        )

        with Checkout(os.path.join(bench_root, repo), issue.sha) as co:
            comps = _LivePartCComps(co=co, issue=issue, model=model, base_root=base_root,
                                    reuse_off_from=reuse_off_from,
                                    cache_dir=cache_dir,
                                    skip_warm_gate=skip_warm_gate,
                                    warm_gate_timeout_s=warm_gate_timeout_s)

            # --- OFF arm (try/finally for immediate persistence) ---
            off_exception_info: dict | None = None
            try:
                comps.run_off_arm(cell)
            except ArmRunError as e:
                failed_stage = "off"
                error_msg = str(e)
                off_exception_info = {
                    "argv": e.argv,
                    "returncode": e.returncode,
                    "stderr": e.stderr[:20000],
                    "wall_s": 0.0,
                    "exception": str(e),
                    "cwd": "",
                    "mcp_args": [],
                    "prism": False,
                }
            finally:
                # Always write off-arm meta.json + prompt, even on failure
                if off_exception_info is not None:
                    import json as _j
                    with open(f"{base}.off.meta.json", "w") as _f:
                        _j.dump(off_exception_info, _f, indent=2)
                    with open(f"{base}.off.prompt.txt", "w") as _f:
                        _f.write(comps._last_off_prompt or "")
                else:
                    _persist_one_arm(
                        comps._last_off,
                        comps._last_off_prompt,
                        None,
                        base=base,
                        arm="off",
                        mcp_args=[],
                        prism=False,
                    )

            if failed_stage == "off":
                raise ArmRunError(
                    argv=off_exception_info["argv"],
                    returncode=off_exception_info["returncode"],
                    stderr=off_exception_info["stderr"],
                    stdout="",
                )

            # --- Score off arm ---
            # (score is called inside run_partc_cell; we mirror partial flow here for
            #  failure isolation, but we defer to run_partc_cell for the full cell)

            # --- ON arm (try/finally for immediate persistence) ---
            on_exception_info: dict | None = None
            try:
                comps.run_on_arm(cell)
            except ArmRunError as e:
                failed_stage = "on"
                error_msg = str(e)
                on_exception_info = {
                    "argv": e.argv,
                    "returncode": e.returncode,
                    "stderr": e.stderr[:20000],
                    "wall_s": 0.0,
                    "exception": str(e),
                    "cwd": "",
                    "mcp_args": [],
                    "prism": True,
                }
            finally:
                # Always write on-arm meta.json + prompt, even on failure
                if on_exception_info is not None:
                    import json as _j
                    with open(f"{base}.on.meta.json", "w") as _f:
                        _j.dump(on_exception_info, _f, indent=2)
                    with open(f"{base}.on.prompt.txt", "w") as _f:
                        _f.write(comps._last_on_prompt or "")
                else:
                    _persist_one_arm(
                        comps._last_on,
                        comps._last_on_prompt,
                        getattr(comps, "_last_prewarm", None),
                        base=base,
                        arm="on",
                        mcp_args=[],
                        prism=True,
                    )

            if failed_stage == "on":
                raise ArmRunError(
                    argv=on_exception_info["argv"],
                    returncode=on_exception_info["returncode"],
                    stderr=on_exception_info["stderr"],
                    stdout="",
                )

            # --- Both arms succeeded: run the full cell and persist results ---
            # Re-run via run_partc_cell using already-set _last_off / _last_on
            # to avoid a second subprocess call.  Wrap comps so run_*_arm just
            # returns what already ran.
            class _CachedComps:
                def run_off_arm(self_inner, c):
                    return comps._last_off
                def run_on_arm(self_inner, c):
                    return comps._last_on
                def score(self_inner, citations, **kwargs):
                    return comps.score(citations, **kwargs)
                def head_to_head(self_inner, off, on, c):
                    # Forward so the LIVE run-partc path produces the head-to-head verdict
                    # (run_partc_cell only computes it when the comps exposes this method).
                    # Guard for comps shapes (test fakes) that don't implement it; the real
                    # _LivePartCComps always does.
                    if hasattr(comps, "head_to_head"):
                        return comps.head_to_head(off, on, c)
                    return {}
                # Scorecard-v2 (D1/D2/D3) forwarding — run_partc_cell's hasattr gates
                # check THIS wrapper object (which always defines these methods), so
                # each one guards internally on `comps` — mirroring head_to_head above —
                # so a comps shape that predates the v2 wiring (e.g. a monkeypatched test
                # fake) degrades to an empty dict instead of raising AttributeError.
                def score_validity(self_inner, text, **kwargs):
                    if hasattr(comps, "score_validity"):
                        return comps.score_validity(text, **kwargs)
                    return {}
                def score_relational(self_inner, off_text, on_text, **kwargs):
                    if hasattr(comps, "score_relational"):
                        return comps.score_relational(off_text, on_text, **kwargs)
                    return {}
                def head_to_head_annotated(self_inner, off_annotated, on_annotated, c):
                    if hasattr(comps, "head_to_head_annotated"):
                        return comps.head_to_head_annotated(off_annotated, on_annotated, c)
                    return {}

            partc_cell = run_partc_cell(cell, _CachedComps())

            # --- Persist per-cite judge artifacts (<base>.<arm>.judge.jsonl) ---
            # judge records are populated by comps.score() (called inside _CachedComps.score)
            _persist_judge_jsonl(getattr(comps, "_last_off_judge", []), f"{base}.off.judge.jsonl")
            _persist_judge_jsonl(getattr(comps, "_last_on_judge", []), f"{base}.on.judge.jsonl")

    except (ArmRunError, Exception) as exc:
        # Write status.json on failure
        status_path = os.path.join(run_dir, "status.json")
        with open(status_path, "w") as _f:
            _json.dump({
                "status": "failed",
                "failed_stage": failed_stage,
                "error": error_msg or str(exc),
                "cell": {"repo": repo, "stage": stage, "model": model},
            }, _f, indent=2)
        raise

    # --- Success: write status.json + remaining artifacts ---
    status_path = os.path.join(run_dir, "status.json")
    with open(status_path, "w") as _f:
        _json.dump({
            "status": "success",
            "failed_stage": None,
            "error": None,
            "cell": {"repo": repo, "stage": stage, "model": model},
        }, _f, indent=2)

    print(f"Run dir: {run_dir}")
    print(render_partc([partc_cell]))

    # Persist cell JSON under run_id/
    json_path = _persist_partc_cell(partc_cell, repo, stage, model,
                                    run_id=run_id, runs_root=runs_root)
    print(f"cell JSON: {json_path}")
    print(f"status:    {status_path}")
