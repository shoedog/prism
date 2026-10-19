#!/usr/bin/env python3
"""Render PRISM_PROFILE schema v1 JSONL. Python standard library only."""
import argparse
import json
from collections import defaultdict, deque
from pathlib import Path


def milliseconds(value):
    return "-" if value is None else f"{value / 1_000_000:.3f}"


def mebibytes(value):
    return "-" if value is None else f"{value / 2**20:.2f}"


def table(headers, rows):
    rows = [[str(cell).replace("|", "\\|").replace("\n", " ") for cell in row] for row in rows]
    return "\n".join([
        "| " + " | ".join(headers) + " |",
        "| " + " | ".join("---" for _ in headers) + " |",
        *("| " + " | ".join(row) + " |" for row in rows),
    ])


def render(records):
    header = next(r for r in records if r["record"] == "header")
    if header["schema_version"] != 1:
        raise ValueError("unsupported profile schema")
    phases = [r for r in records if r["record"] == "phase"]
    owners = defaultdict(deque)
    for r in records:
        if r["record"] == "owner":
            owners[(r["file"], r["owner"], r["start_line"], r["start_byte"], r["total_ns"])].append(r)
    summary = next(r for r in records if r["record"] == "summary")
    result = ["Phases (inclusive parents overlap children; parallel work is not build wall time).", table(
        ["Phase / file", "Kind", "Status", "Wall/work ms", "CPU ms", "process peak RSS (high-water mark, sampled after) MiB"],
        [(r["name"] + (f' #{r["instance"]}' if "instance" in r else "") + (" / " + r["file"] if r.get("file") else ""), r["time_kind"],
          r["status"], milliseconds(r.get("wall_ns", r.get("work_ns"))), milliseconds(r["cpu_ns"]),
          mebibytes(r["peak_rss_bytes"])) for r in phases])]
    command = next((r for r in records if r["record"] == "command"), None)
    if command is not None:
        result.insert(0, f'Command: wall {milliseconds(command["wall_ns"])} ms; CPU {milliseconds(command["cpu_ns"])} ms; process peak RSS (high-water mark, sampled after) {mebibytes(command["peak_rss_bytes"])} MiB.\n')
    top = []
    for r in summary["top_owners"]:
        owner = owners[(r["file"], r["owner"], r["start_line"], r["start_byte"], r["total_ns"])].popleft()
        top.append((f'{r["file"]}:{r["start_line"]} {r["owner"]}', owner["language"],
                    milliseconds(r["total_ns"]), milliseconds(owner["flow_search_ns"]),
                    milliseconds(owner["reaching_definitions_ns"]), owner["byte_size"], owner["ast_nodes"],
                    mebibytes(owner.get("peak_rss_bytes"))))
    result += ["", "Top owners (summed owner work).", table(
        ["Owner", "Language", "Total ms", "Search ms", "RD ms", "Bytes", "AST nodes", "process peak RSS (high-water mark, sampled after) MiB"], top)]
    other = summary["other_owners"]
    result += ["", f'Owners: {summary["owner_count"]}; top-{header["top_n"]} share of owner DFG work: {summary["top_dfg_share"]:.2%}.',
               table(["Other count", "Total ms", "p50 ms", "p90 ms", "p99 ms", "Max ms"],
                     [(other["count"], *(milliseconds(other[k]) for k in ("total_ns", "p50_ns", "p90_ns", "p99_ns", "max_ns")))])]
    return "\n".join(result) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("profile", type=Path)
    args = parser.parse_args()
    records = [json.loads(line) for line in args.profile.read_text().splitlines() if line.strip()]
    print(render(records), end="")


if __name__ == "__main__":
    main()
