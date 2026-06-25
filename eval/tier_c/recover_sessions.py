#!/usr/bin/env python3
"""Recover Tier-C arm sessions for a run from the CLIs' OWN logs.

The run-store writes stage artifacts to `stages/<stage>/` with no per-issue
namespace, so a multi-issue run overwrites all but the last issue on disk. But
every arm ran as a real subprocess whose transcript persists independently:

  - opus (claude) arms: ~/.claude/projects/*-T-tc-co-*/<session>.jsonl  (8 / issue dir)
  - gpt  (codex)  arms: ~/.codex/sessions/<Y>/<M>/<D>/rollout-*.jsonl, cwd in a tc-co-* checkout

This reorganizes all arm sessions in a run's mtime window into
  <run>/recovered/<model>/<repo>/<stage>/<variant>.md  + manifest.json + README.md

Mapping confidence:
  model  = certain (which CLI log)
  repo   = certain (file-path fingerprint of the issue's own files; cross-checked by window)
  stage  = certain (the SPEC vs PLAN instruction text)
  prism  = certain (actual prism/nav tool-call presence)
  lsp    = INFERRED (execution order within each model/repo/stage/prism pair; LSP was inert
           in spec/plan so on/off leave no behavioral trace) — flagged per entry.

Usage:  python3 recover_sessions.py            # defaults to run full-2026-06-24
        python3 recover_sessions.py <run-id> <YYYY-MM-DD HH:MM> <YYYY-MM-DD HH:MM>
"""
from __future__ import annotations
import json, glob, os, sys, re, datetime, collections

HERE = os.path.dirname(os.path.abspath(__file__))
RUN_ID = sys.argv[1] if len(sys.argv) > 1 else "full-2026-06-24"
WIN_LO = datetime.datetime.fromisoformat(sys.argv[2]) if len(sys.argv) > 2 else datetime.datetime(2026, 6, 24, 15, 0)
WIN_HI = datetime.datetime.fromisoformat(sys.argv[3]) if len(sys.argv) > 3 else datetime.datetime(2026, 6, 24, 23, 30)
RUN_DIR = os.path.join(HERE, "runs", RUN_ID)
OUT = os.path.join(RUN_DIR, "recovered")

# Fingerprints: substrings unique to each corpus repo's own source tree.
REPO_FINGERPRINTS = [
    ("ruff",       "rust",   ("crates/ruff", "ruff_linter", "ruff_python", "ruff_source_file", ".rs:")),
    ("prometheus", "go",     ("model/labels", "promql", "tsdb/", "FastRegexMatcher", ".go:")),
    ("pydantic",   "python", ("pydantic/", "_internal/", "_model_construction", "__pydantic", "test_main.py")),
    ("excalidraw", "ts",     ("packages/element", "packages/excalidraw", "textElement", "resizeElements", ".tsx:")),
]
CITE_RE = re.compile(r"[\w./\-]+\.(?:rs|go|py|ts|tsx|js):\d+")


def in_window(path: str) -> bool:
    dt = datetime.datetime.fromtimestamp(os.path.getmtime(path))
    return WIN_LO <= dt <= WIN_HI


def fingerprint_repo(blob: str):
    counts = collections.Counter()
    for repo, lang, pats in REPO_FINGERPRINTS:
        for p in pats:
            counts[(repo, lang)] += blob.count(p)
    (repo, lang), n = counts.most_common(1)[0]
    return (repo, lang) if n > 0 else (None, None)


def stage_of(users: list[str]) -> str:
    blob = "\n".join(users)
    if "Write a short implementation SPEC" in blob:
        return "spec"
    if "Write a step-by-step PLAN" in blob:
        return "plan"
    return "unknown"


_NAV = re.compile(r"prism|nav_(?:callers|callees|nodes_at|ego_graph|module_deps|repo_map)", re.I)


def prism_used(tools: list[str]) -> bool:
    # Actual prism invocation only — a generic `mcp:` prefix (e.g. `_fetch_pr`) is NOT prism.
    return any(_NAV.search(t or "") for t in tools)


def parse_claude(path: str) -> dict:
    users, last_text, tools = [], "", []
    for line in open(path, errors="replace"):
        try:
            o = json.loads(line)
        except Exception:
            continue
        m = o.get("message", {})
        typ = o.get("type")
        if typ == "user":
            c = m.get("content", "")
            if isinstance(c, list):
                c = " ".join(x.get("text", "") for x in c if isinstance(x, dict) and x.get("type") == "text")
            users.append(str(c))
        elif typ == "assistant":
            for x in (m.get("content") or []):
                if isinstance(x, dict):
                    if x.get("type") == "text":
                        last_text = x["text"]
                    elif x.get("type") == "tool_use":
                        tools.append(x.get("name", ""))
    return dict(model="opus-4.8", users=users, text=last_text, tools=tools,
                mtime=os.path.getmtime(path), path=path)


def parse_codex(path: str) -> dict:
    users, last_text, tools, cwd = [], "", [], None
    for line in open(path, errors="replace"):
        try:
            o = json.loads(line)
        except Exception:
            continue
        t = o.get("type")
        p = o.get("payload", {})
        if t == "session_meta":
            cwd = p.get("cwd", cwd)
        elif t == "response_item" and p.get("role") == "user":
            c = p.get("content", "")
            if isinstance(c, list):
                c = " ".join(x.get("text", "") for x in c if isinstance(x, dict))
            users.append(str(c))
        elif t == "response_item" and p.get("role") == "assistant":
            c = p.get("content", "")
            if isinstance(c, list):
                txt = " ".join(x.get("text", "") for x in c
                               if isinstance(x, dict) and x.get("type") in ("text", "output_text"))
                if txt.strip():
                    last_text = txt
        elif t == "response_item" and p.get("type") in ("function_call", "custom_tool_call"):
            tools.append(p.get("name", ""))
        elif t == "event_msg" and p.get("type") == "mcp_tool_call_end":
            inv = p.get("invocation", {})
            tools.append("mcp:" + str(inv.get("tool", inv.get("server", ""))))
        elif t == "event_msg" and p.get("type") == "agent_message":
            mt = p.get("message", "")
            if isinstance(mt, str) and mt.strip():
                last_text = mt
    return dict(model="gpt-5.5", users=users, text=last_text, tools=tools,
                mtime=os.path.getmtime(path), path=path, cwd=cwd)


def collect() -> list[dict]:
    arms = []
    # opus / claude: every session under a run-window tc-co checkout project dir
    for d in glob.glob(os.path.expanduser("~/.claude/projects/*-T-tc-co-*")):
        for f in glob.glob(os.path.join(d, "*.jsonl")):
            if in_window(f):
                arms.append(parse_claude(f))
    # gpt / codex: window sessions whose cwd is a tc-co checkout
    for f in glob.glob(os.path.expanduser("~/.codex/sessions/*/*/*/rollout-*.jsonl")):
        if not in_window(f):
            continue
        rec = parse_codex(f)
        if rec.get("cwd") and "tc-co" in rec["cwd"]:
            arms.append(rec)
    return arms


def main():
    arms = collect()
    for a in arms:
        a["stage"] = stage_of(a["users"])
        a["prism"] = prism_used(a["tools"])
        blob = a["text"] + "\n" + "\n".join(a["users"])
        a["repo"], a["lang"] = fingerprint_repo(blob)
        a["n_cites"] = len(set(CITE_RE.findall(a["text"])))
    # NB: every "prism-on" arm invoked ZERO prism tools (verified: 0 mcp__nav_* calls, 0 Bash/exec
    # references to prism/nav), so prism CANNOT be detected from tool calls. Label the variant by
    # EXECUTION ORDER within each (model, repo, stage) group of 4 — the run drives the variants list
    # `for p in (False,True) for l in (False,True)` => order [base, +lsp, +prism, +prism+lsp].
    # VALIDATED text-exact against the surviving excalidraw run-store arms (sim=1.00 on all 8).
    # `prism_called` is the ACTUAL evidence (expected False everywhere) — the treatment was never administered.
    ORDER = [(False, False), (False, True), (True, False), (True, True)]
    groups = collections.defaultdict(list)
    for a in arms:
        groups[(a["model"], a["repo"], a["stage"])].append(a)
    for key, grp in groups.items():
        grp.sort(key=lambda a: a["mtime"])
        for i, a in enumerate(grp):
            a["prism"], a["lsp"] = ORDER[i] if i < 4 else (None, None)
            a["prism_called"] = prism_used(a["tools"])   # actual prism/nav/mcp invocation evidence
            a["group_size"] = len(grp)                   # expect 4; anything else => labeling ambiguity

    def variant_label(a):
        return a["model"] + ("+prism" if a["prism"] else "") + ("+lsp" if a["lsp"] else "")

    os.makedirs(OUT, exist_ok=True)
    manifest = []
    for a in sorted(arms, key=lambda a: (a["model"], a["repo"] or "?", a["stage"], variant_label(a))):
        lbl = variant_label(a)
        repo = a["repo"] or "UNKNOWN"
        dst_dir = os.path.join(OUT, a["model"], repo, a["stage"])
        os.makedirs(dst_dir, exist_ok=True)
        dst = os.path.join(dst_dir, lbl + ".md")
        ts = datetime.datetime.fromtimestamp(a["mtime"]).isoformat(timespec="seconds")
        header = (
            f"# {repo} / {a['stage']} / {lbl}\n\n"
            f"- model: `{a['model']}`  |  repo: `{repo}` ({a['lang']})  |  stage: `{a['stage']}`\n"
            f"- variant (exec-order, validated vs run-store): prism=**{a['prism']}**  lsp=**{a['lsp']}**\n"
            f"- prism_called (actual tool evidence): **{a['prism_called']}**"
            f"{'  ⚠️ prism-intended but NEVER invoked prism' if (a['prism'] and not a['prism_called']) else ''}\n"
            f"- session: `{a['path']}`\n"
            f"- mtime: {ts}  |  tool_calls: {len(a['tools'])}  |  citations: {a['n_cites']}"
            f"  |  group_size: {a['group_size']}{'  ⚠️AMBIGUOUS' if a['group_size'] != 4 else ''}\n\n"
            f"---\n\n"
        )
        with open(dst, "w") as fh:
            fh.write(header + (a["text"] or "*(no final assistant text captured)*") + "\n")
        manifest.append(dict(model=a["model"], repo=repo, lang=a["lang"], stage=a["stage"],
                             prism=a["prism"], lsp=a["lsp"], variant=lbl,
                             label_method="exec-order(validated)", prism_called=a["prism_called"],
                             session=a["path"], mtime=ts, tool_calls=len(a["tools"]),
                             text_chars=len(a["text"]), citations=a["n_cites"],
                             group_size=a["group_size"]))

    with open(os.path.join(OUT, "manifest.json"), "w") as fh:
        json.dump(manifest, fh, indent=2)

    # coverage summary
    grid = collections.Counter((m["model"], m["repo"], m["stage"]) for m in manifest)
    by_repo = collections.Counter((m["repo"], m["lang"]) for m in manifest)
    print(f"recovered {len(manifest)} arm sessions -> {OUT}")
    print("per (model, repo, stage):")
    for k in sorted(grid):
        print(f"  {k[0]:9} {str(k[1]):11} {k[2]:5} : {grid[k]} arms")
    print("per repo:", dict(by_repo))
    prism_intended = [m for m in manifest if m["prism"]]
    prism_actually = [m for m in manifest if m["prism_called"]]
    print(f"prism-INTENDED arms: {len(prism_intended)}  |  arms that ACTUALLY invoked prism: {len(prism_actually)}")
    if not prism_actually:
        print("  ⚠️ NO arm invoked prism — the prism TREATMENT WAS NEVER ADMINISTERED (run measures no prism effect)")
    unknown = [m for m in manifest if m["repo"] == "UNKNOWN" or m["stage"] == "unknown"]
    amb = [m for m in manifest if m["group_size"] != 4]
    if unknown:
        print(f"⚠️ {len(unknown)} sessions with UNKNOWN repo/stage")
    if amb:
        print(f"⚠️ {len(amb)} sessions in non-4 groups (order labeling ambiguous)")

    with open(os.path.join(OUT, "README.md"), "w") as fh:
        fh.write(
            "# Recovered Tier-C arm sessions — run `" + RUN_ID + "`\n\n"
            "Rebuilt from the CLIs' own session logs after the run-store's per-issue overwrite "
            "(only excalidraw survived in `../stages/`). Regenerate with `python3 ../../recover_sessions.py`.\n\n"
            "Layout: `<model>/<repo>/<stage>/<variant>.md` (full spec/plan text + provenance header). "
            "`manifest.json` indexes all 64 sessions.\n\n"
            "## Mapping confidence\n"
            "- **model / repo / stage = certain** (which CLI log / file-path fingerprint of the issue's own "
            "files / the SPEC-vs-PLAN instruction text).\n"
            "- **prism + lsp variant = by EXECUTION ORDER** within each model·repo·stage group of 4 "
            "(`[base, +lsp, +prism, +prism+lsp]`, the run's variants-list order). **VALIDATED text-exact "
            "(sim=1.00) against the 8 surviving excalidraw run-store arms.**\n\n"
            "## ⚠️ Two reasons these arms carry NO prism signal\n"
            "1. **prism was never invoked.** Across all 64 arms: 0 `mcp__…nav_*` tool calls, and 0 of 195 "
            "claude Bash + 0 of 740 codex exec commands reference prism/nav. The 'prism-on' arms navigated "
            "with the same Read/rg/Bash as 'prism-off' → identical treatment. (`prism_called=False` on every "
            "arm; the run-store's `used_prism=True` was a heuristic `variant.prism AND tool_calls>0`, not "
            "evidence.) prism-on vs prism-off is a null contrast independent of scoring.\n"
            "2. The grading objective was degenerate (code-blind relevance oracle → precision≡0); see "
            "`docs/superpowers/specs/2026-06-24-tier-c-investigator-relevance-oracle-fix-design.md`.\n\n"
            "## Still useful for\n"
            "Auditing **spec/plan quality per repo/stage/model** (real grounded text + tool traces) and "
            "re-scoring with the fixed oracle. It canNOT answer 'does prism help' — that needs a rerun where "
            "prism-on arms actually call prism (force/verify prism usage + detect real prism tool calls).\n"
        )


if __name__ == "__main__":
    main()
