#!/usr/bin/env python3
"""Exhaustive gopls audit for new promoted-selector resolver targets.

Consumes control and candidate ``prism nav call-stats --dump-sites`` JSONL,
selects every newly resolved Exact ``embedded_promotion`` target, and checks
``textDocument/definition`` against each prism FunctionId span. A non-empty
delta passes only when every site was processed and every target matches.

Run from ``eval/``::

    uv run python tools/promoted_delta_audit.py \
      --control-sites control.jsonl --candidate-sites candidate.jsonl \
      --repo ~/code/bench-repos/caddy --corpus caddy --out audit.json
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Iterable


EVAL = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(EVAL))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import dispatch_oracle as do  # noqa: E402


PROMOTED_KIND = "embedded_promotion"
SiteKey = tuple[str, int, int, str | None]
DeltaSite = tuple[SiteKey, dict, list[dict]]


def load_sites(path: str | Path) -> dict[SiteKey, dict]:
    sites: dict[SiteKey, dict] = {}
    with Path(path).open(encoding="utf-8") as source:
        for line in source:
            line = line.strip()
            if not line:
                continue
            record = json.loads(line)
            span = record["source_span"]
            key = (
                span["file"],
                span["start_byte"],
                span["end_byte"],
                record.get("callee_text"),
            )
            sites[key] = record
    return sites


def embedded_exact_targets(record: dict) -> list[dict]:
    return [
        target
        for target in record.get("resolved_targets", [])
        if target.get("confidence") == "exact" and target.get("kind") == PROMOTED_KIND
    ]


def _function_id_key(target: dict) -> tuple:
    function_id = target.get("function_id", {})
    return (
        function_id.get("file"),
        function_id.get("name"),
        function_id.get("start_line"),
        function_id.get("end_line"),
    )


def _resolved_function_ids(record: dict | None) -> set[tuple]:
    if record is None:
        return set()
    return {
        _function_id_key(target)
        for target in record.get("resolved_targets", [])
        if isinstance(target, dict) and isinstance(target.get("function_id"), dict)
    }


def select_changed_sites(
    control: dict[SiteKey, dict], candidate: dict[SiteKey, dict]
) -> list[DeltaSite]:
    """Return the entire newly resolved Exact promoted-target delta."""
    changed: list[DeltaSite] = []
    for key, record in candidate.items():
        control_targets = _resolved_function_ids(control.get(key))
        new_targets = [
            target
            for target in embedded_exact_targets(record)
            if _function_id_key(target) not in control_targets
        ]
        if new_targets:
            changed.append((key, record, new_targets))
    changed.sort(key=lambda site: (site[0][0], site[0][1], site[0][2], site[0][3] or ""))
    return changed


def token_position(
    root: str | Path,
    relative_file: str,
    start_byte: int,
    end_byte: int,
    name: str,
) -> tuple[int, int] | None:
    """Return line0 and UTF-16 char0 for the last method name in a call span."""
    if not name:
        return None
    try:
        data = (Path(root) / relative_file).read_bytes()
    except OSError:
        return None
    segment = data[start_byte:end_byte]
    index = segment.rfind(name.encode())
    if index < 0:
        return None
    offset = start_byte + index
    line0 = data.count(b"\n", 0, offset)
    line_start = data.rfind(b"\n", 0, offset) + 1
    prefix = data[line_start:offset].decode("utf-8", "replace")
    char0 = len(prefix.encode("utf-16-le")) // 2
    return line0, char0


def definition_in_target(target: dict, definition: dict) -> bool:
    function_id = target.get("function_id", {})
    definition_file = definition.get("file")
    definition_line = definition.get("line")
    start_line = function_id.get("start_line")
    end_line = function_id.get("end_line")
    if not all(
        isinstance(value, int) for value in (definition_line, start_line, end_line)
    ):
        return False
    if not isinstance(definition_file, str) or not isinstance(function_id.get("file"), str):
        return False
    return (
        os.path.normpath(definition_file) == os.path.normpath(function_id["file"])
        and start_line - 1 <= definition_line <= end_line - 1
    )


def summarize(
    *, new_sites: int, sampled: int, target_verdicts: Iterable[str]
) -> dict:
    verdicts = list(target_verdicts)
    hits = sum(verdict == "hit" for verdict in verdicts)
    misses = sum(verdict == "MISS" for verdict in verdicts)
    unknown = len(verdicts) - hits - misses
    if new_sites == 0 and sampled == 0 and not verdicts:
        verdict = "NO-DATA"
    elif (
        new_sites > 0
        and sampled == new_sites
        and hits == len(verdicts)
        and misses == 0
        and unknown == 0
        and hits > 0
    ):
        verdict = "PASS"
    else:
        verdict = "FAIL"
    return {
        "kinds": [PROMOTED_KIND],
        "new_sites": new_sites,
        "sampled": sampled,
        "targets": len(verdicts),
        "hits": hits,
        "misses": misses,
        "unknown": unknown,
        "verdict": verdict,
    }


def exit_code(summary: dict) -> int:
    return 0 if summary.get("verdict") in {"PASS", "NO-DATA"} else 1


def _callee_name(callee: str | None) -> str:
    return (callee or "").split(".")[-1].split("(")[0].strip()


def _unknown_records(delta: list[DeltaSite], reason: str) -> tuple[list[dict], list[str]]:
    records = []
    verdicts = []
    for key, _record, targets in delta:
        target_results = []
        for target in targets:
            verdict = f"unknown:{reason}"
            verdicts.append(verdict)
            target_results.append({"target": target, "gopls": None, "verdict": verdict})
        records.append(
            {
                "file": key[0],
                "start_byte": key[1],
                "end_byte": key[2],
                "callee": key[3],
                "target_results": target_results,
            }
        )
    return records, verdicts


def audit_sites(
    delta: list[DeltaSite], root: str | Path, oracle: do.GoplsSatisfiers
) -> tuple[list[dict], list[str]]:
    records = []
    verdicts = []
    for key, _record, targets in delta:
        relative_file, start_byte, end_byte, callee = key
        position = token_position(
            root,
            relative_file,
            start_byte,
            end_byte,
            _callee_name(callee),
        )
        if position is None:
            target_results, site_verdicts = _unknown_records(
                [(key, _record, targets)], "no_token"
            )
            records.extend(target_results)
            verdicts.extend(site_verdicts)
            continue
        try:
            definition = oracle.method_decl(relative_file, position[0], position[1])
        except Exception:
            definition = {
                "kind": "unknown",
                "failure_stage": "definition",
                "oracle_status": "exception",
            }
        target_results = []
        for target in targets:
            if definition.get("kind") == "unknown":
                verdict = f"unknown:{definition.get('oracle_status', 'unresolved')}"
            else:
                verdict = "hit" if definition_in_target(target, definition) else "MISS"
            verdicts.append(verdict)
            target_results.append(
                {"target": target, "gopls": definition, "verdict": verdict}
            )
        records.append(
            {
                "file": relative_file,
                "start_byte": start_byte,
                "end_byte": end_byte,
                "callee": callee,
                "target_results": target_results,
            }
        )
    return records, verdicts


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--control-sites", required=True)
    parser.add_argument("--candidate-sites", required=True)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--corpus", default=None)
    parser.add_argument("--timeout", type=float, default=120.0)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    control = load_sites(args.control_sites)
    candidate = load_sites(args.candidate_sites)
    delta = select_changed_sites(control, candidate)
    records: list[dict]
    target_verdicts: list[str]
    if not delta:
        records, target_verdicts = [], []
    else:
        root = Path(args.repo).resolve()
        oracle = do.GoplsSatisfiers(str(root), do.make_cmd(args.corpus), args.timeout)
        try:
            oracle.start()
        except Exception:
            records, target_verdicts = _unknown_records(delta, "oracle_start")
            try:
                oracle.stop()
            except Exception:
                pass
        else:
            try:
                records, target_verdicts = audit_sites(delta, root, oracle)
            finally:
                try:
                    oracle.stop()
                except Exception:
                    pass

    summary = summarize(
        new_sites=len(delta), sampled=len(delta), target_verdicts=target_verdicts
    )
    summary["corpus"] = args.corpus
    output = {"summary": summary, "records": records}
    Path(args.out).write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, sort_keys=True))
    for record in records:
        for result in record["target_results"]:
            if result["verdict"] != "hit":
                print(
                    result["verdict"],
                    record["file"],
                    record["start_byte"],
                    record["callee"],
                    "->",
                    result["target"]["function_id"],
                    "gopls",
                    result["gopls"],
                )
    return exit_code(summary)


if __name__ == "__main__":
    sys.exit(main())
