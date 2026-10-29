"""Aggregation and RESULTS.md rendering for measure.py. Standard library only."""
from collections import defaultdict
from itertools import combinations
from statistics import median

KINDS = ("serial_wall", "parallel_work")
CAVEAT = ("Cloud VM wall times are not comparable with local history. "
          "Use phase shares, rankings and paired deltas only.")
TOP = 15


def phase_totals(records):
    """Sum measured phase instances per (name, time_kind); kinds are never merged."""
    totals = defaultdict(int)
    for record in records:
        if record.get("record") != "phase" or record.get("status") != "measured":
            continue
        kind = record.get("time_kind")
        if kind not in KINDS:
            raise ValueError(f"unknown time_kind: {kind!r}")
        value = record.get("wall_ns" if kind == "serial_wall" else "work_ns")
        if value is None:
            value = record.get("wall_ns", record.get("work_ns"))
        if not isinstance(value, int) or isinstance(value, bool) or value < 0:
            raise ValueError(f"invalid phase duration for {record.get('name')}: {value!r}")
        totals[(record["name"], kind)] += value
    return dict(totals)


def paired_delta(before, after):
    """(after - before per pair, percent of before or None, verdict)."""
    if not before or len(before) != len(after) or any(
            v is None for v in [*before, *after]):
        raise ValueError("paired_delta needs equal-length nonempty lists without gaps")
    values = [b - a for a, b in zip(before, after)]
    percents = [None if a == 0 else 100.0 * (b - a) / a for a, b in zip(before, after)]
    same = all(v > 0 for v in values) or all(v < 0 for v in values)
    n = len(values)
    verdict = (f"insufficient pairs ({n})" if n < 3 else
               f"{'holds in every pair' if same else 'mixed'} ({n} pairs)")
    return values, percents, verdict


def table(headers, rows):
    rows = [[str(c).replace("|", "\\|") for c in row] for row in rows]
    return "\n".join(["| " + " | ".join(headers) + " |",
                      "| " + " | ".join("---" for _ in headers) + " |",
                      *("| " + " | ".join(r) + " |" for r in rows)])


def fmt(value, digits=3):
    return "unavailable" if value is None else f"{value:.{digits}f}"


def ms(ns):
    return "unavailable" if ns is None else f"{ns / 1e6:.1f}"


def pct(value):
    return "n/a" if value is None else f"{value:+.1f}%"


def delta_cell(before, after, scale=1.0, unit="", base_wall_spread="unavailable"):
    if len(before) != len(after):
        raise ValueError("delta_cell needs equal-length lists")
    pairs = [(a, b) for a, b in zip(before, after) if a is not None and b is not None]
    if not pairs:
        return ("unavailable", "unavailable", "unavailable", base_wall_spread,
                "insufficient pairs (0)")
    before, after = zip(*pairs)
    values, percents, verdict = paired_delta(before, after)
    base = median(before)
    median_percent = None if base == 0 else 100.0 * median(values) / base
    return (", ".join(f"{v / scale:+.1f}{unit}" for v in values),
            ", ".join(pct(p) for p in percents), pct(median_percent), base_wall_spread, verdict)


def render_results(environment, binaries, runs, failure=None, warnings=()):
    if (not runs or not binaries) and failure is None:
        raise ValueError("no runs or binaries to render")
    refs = [b["ref"] for b in binaries]
    corpora = sorted({r["corpus"] for r in runs})
    group = defaultdict(dict)
    for r in runs:
        group[(r["corpus"], r["ref"])][r["round"]] = r
    out = ["# Cloud measurement results", "", CAVEAT, ""]
    if failure is not None:
        out += ["## PARTIAL", "", "Failure reason / incomplete state:", "", failure, ""]
    if warnings:
        out += ["## Warnings", "", *[f"- {w}" for w in warnings], ""]
    out += ["## Environment", ""]
    out += [f"- {k}: {v}" for k, v in sorted(environment.items())]
    out += ["", "## Binaries", "", table(["ref", "commit", "sha256"], [
        (b["ref"], b.get("commit", "unavailable"), b.get("sha256", "unavailable"))
        for b in binaries]), "",
            "## Runs", "", table(
                ["round", "corpus", "ref", "wall s", "peak RSS MiB", "cpu user s",
                 "cpu sys s", "load", "steal ticks", "gate_failed", "gate failure reason"],
                [(r["round"], r["corpus"], r["ref"], fmt(r["wall_s"]),
                  fmt(r["peak_rss_bytes"] / 2**20, 1), fmt(r["cpu_user_s"]),
                  fmt(r["cpu_system_s"]), fmt(r["quiet_load"], 2), steal(r),
                  r.get("gate_failed", False), r.get("gate_failure_reason") or "")
                 for r in sorted(runs, key=lambda r: (r["round"], r["corpus"], r["ref"]))])]
    out += ["", "Peak RSS from wait4 is a floor that may include the parent's size."]
    out += ["", f"## Top {TOP} phases by median", "",
            "Serial wall and parallel work are different units of time and are never summed."]
    for c in corpora:
        for ref in refs:
            rows = phase_rows(list(group[(c, ref)].values()))
            out += ["", f"### {c} / {ref}", "", table(
                ["phase", "time_kind", "median ms", "min ms", "max ms", "runs"], rows)]
    if len(refs) > 1:
        out += ["", "## Paired deltas", "",
                "Delta is second ref minus first ref, paired by round. "
                "A verdict is `holds in every pair` only with at least 3 pairs, all agreeing in sign. "
                "Missing metrics are omitted from that metric's pairs; every verdict gives its count. "
                "Median delta / base median is median(paired deltas) / median(base values). "
                "Base wall spread is (max-min)/median over this ref pair's shared rounds. "
                "Gate-failed runs are included and flagged above."]
        for c in corpora:
            for a, b in combinations(refs, 2):
                rounds = sorted(set(group[(c, a)]) & set(group[(c, b)]))
                out += ["", f"### {c}: {a} → {b}", "", table(
                    ["metric", "deltas", "percent", "median delta / base median",
                     "base wall spread", "verdict"], pair_rows(group, c, a, b, rounds))]
    out += ["", "## Noise", "", table(
        ["corpus", "ref", "wall spread (max-min)/median", "steal range ticks"],
        [(c, ref, spread(group[(c, ref)]), steal_range(group[(c, ref)]))
         for c in corpora for ref in refs]), "", CAVEAT, ""]
    return "\n".join(out)


def steal(run):
    a, b = run.get("steal_before"), run.get("steal_after")
    return "unavailable" if a is None or b is None else b - a


def steal_range(by_round):
    values = [steal(r) for r in by_round.values()]
    values = [v for v in values if v != "unavailable"]
    return "unavailable" if not values else f"{min(values)}..{max(values)}"


def spread(by_round):
    walls = [r["wall_s"] for r in by_round.values()]
    if not walls:
        return "unavailable"
    mid = median(walls)
    return "n/a" if mid == 0 else f"{100 * (max(walls) - min(walls)) / mid:.2f}%"


def per_run_phases(run):
    return {(p["name"], p["time_kind"]): p["ns"] for p in run.get("phases", [])}


def phase_rows(runs):
    series = defaultdict(list)
    for run in runs:
        for key, ns in per_run_phases(run).items():
            series[key].append(ns)
    ranked = sorted(series.items(), key=lambda kv: (-median(kv[1]), kv[0]))[:TOP]
    return [(name, kind, ms(median(v)), ms(min(v)), ms(max(v)), len(v))
            for (name, kind), v in ranked]


def pair_rows(group, corpus, a, b, round_ids):
    first = [group[(corpus, a)][i] for i in round_ids]
    second = [group[(corpus, b)][i] for i in round_ids]
    base_spread = spread({i: r for i, r in enumerate(first)})
    rows = [("command wall ms", *delta_cell([r["wall_s"] for r in first],
                                            [r["wall_s"] for r in second], 1e-3,
                                            base_wall_spread=base_spread))]
    top = phase_rows(list(group[(corpus, a)].values()))
    for name, kind, *_ in top:
        before = [per_run_phases(r).get((name, kind)) for r in first]
        after = [per_run_phases(r).get((name, kind)) for r in second]
        rows.append((f"{name} ({kind}) ms", *delta_cell(before, after, 1e6,
                                                     base_wall_spread=base_spread)))
    return rows
