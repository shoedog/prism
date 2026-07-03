"""Report rendering (spec §2.10): per-corpus md+json under docs/eval/tier-a/."""
from __future__ import annotations

import json
from pathlib import Path


def fmt_wilson(t: list | tuple) -> str:
    p, lo, hi = t
    return f"{p:.2f} [{lo:.2f}–{hi:.2f}]"


def render_markdown(run: dict) -> str:
    m = run["meta"]
    lines = [
        f"# Tier-A run — {m['corpus']} ({m['date']})",
        "",
        f"- corpus: `{m['corpus']}` @ `{m['corpus_sha']}`"
        + (" **dirty**" if m.get("corpus_dirty") else ""),
        f"- prism: `{m['prism_sha']}` · oracle: {m['oracle']} · seed: {m['seed']}"
        f" · harness: `{m['harness_sha']}`",
        f"- oracle_error_rate: {m['oracle_error_rate']:.3f} ·"
        f" sut_error_rate: {m['sut_error_rate']:.3f} ·"
        f" baseline_invalid: {m['baseline_invalid']} ·"
        f" oracle_not_quiescent: {m['oracle_not_quiescent']}",
        f"- wall (s): {m['wall_s']}",
        "",
    ]
    for direction, strata in run.get("m2", {}).items():
        lines += [
            f"## M2 {direction}",
            "",
            "| stratum | site raw P | site raw R | site corr P | site corr R | fn raw P | fn raw R | tp/fp/fn | pending | shortfall |",
            "|---|---|---|---|---|---|---|---|---|---|",
        ]
        for s, d in strata.items():
            raw, cor = d["raw"], d["corrected"]
            fn_raw = d.get("function", {}).get("raw")
            lines.append(
                f"| {s} | {fmt_wilson(raw['precision'])} | {fmt_wilson(raw['recall'])} "
                f"| {fmt_wilson(cor['precision'])} | {fmt_wilson(cor['recall'])} "
                f"| {fmt_wilson(fn_raw['precision']) if fn_raw else ''} "
                f"| {fmt_wilson(fn_raw['recall']) if fn_raw else ''} "
                f"| {cor['tp']}/{cor['fp']}/{cor['fn']} | {d['pending']} "
                f"| {d['shortfall']} |"
            )
        lines.append("")
        # P6a confidence-stratified block: exact_tier is the P3 gate input (compare
        # vs the pre-change run); candidate_tier is informational + adjudication-fed
        # only (see eval/README.md). `.get` defaults so a --report-only replay of an
        # old (pre-P6a) run JSON that lacks these keys still renders, not crashes.
        if any("exact_tier" in d or "candidate_tier" in d for d in strata.values()):
            lines += [
                "_exact/candidate tier (P3 gate reads exact_tier only; "
                "candidate_tier is informational)_",
                "",
                "| stratum | exact P | exact R | exact tp/fp/fn | candidate count"
                " | oracle-confirmed | oracle-unconfirmed |",
                "|---|---|---|---|---|---|---|",
            ]
            for s, d in strata.items():
                et = d.get("exact_tier", {}).get("raw")
                ct = d.get("candidate_tier", {})
                et_counts = f"{et['tp']}/{et['fp']}/{et['fn']}" if et else ""
                lines.append(
                    f"| {s} | {fmt_wilson(et['precision']) if et else ''} "
                    f"| {fmt_wilson(et['recall']) if et else ''} "
                    f"| {et_counts} "
                    f"| {ct.get('count', 0)} | {ct.get('oracle_confirmed', 0)} "
                    f"| {ct.get('oracle_unconfirmed', 0)} |"
                )
            lines.append("")
    for key, title in (
        ("m1", "M1 inventory diff"),
        ("m3", "M3 spot-check"),
        ("matrix", "Capability matrix"),
        ("pending", "Pending triage"),
        ("pinned", "Pinned probes"),
    ):
        if key in run:
            lines += [
                f"## {title}",
                "",
                "```json",
                json.dumps(run[key], indent=1, sort_keys=True, default=str,
                           allow_nan=False),
                "```",
                "",
            ]
    return "\n".join(lines)


def write_reports(run: dict, out_dir: Path) -> None:
    m = run["meta"]
    out_dir.mkdir(parents=True, exist_ok=True)
    base = out_dir / f"{m['date']}-{m['corpus']}"
    base.with_suffix(".json").write_text(
        json.dumps(run, indent=1, sort_keys=True, default=str, allow_nan=False)
    )
    base.with_suffix(".md").write_text(render_markdown(run))
