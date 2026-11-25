#!/usr/bin/env python3
"""Inventory source-only W→R contributions for one owner or a frozen C3 frame."""

import argparse
import json
from pathlib import Path

import flow

GRAPH_EVIDENCE = {
    "raw_def_use_join": "unverified",
    "assembled_endpoint_bytes": "unverified",
    "occupied_pair_keys": "unverified",
    "rd_availability": "unverified",
    "public_cfg_stop": "unverified",
}


def parse(source, lang):
    source.decode("utf-8")
    if lang == "python":
        from python_frontend import PythonFile
        return PythonFile(source)
    if lang in ("javascript", "typescript", "tsx", "go"):
        from tree_frontend import TreeFile
        return TreeFile(source, lang)
    raise ValueError("unsupported language " + lang)


def unavailable(reason, detailed):
    row = dict(status=reason, counts={}, flags={reason: 1}, endpoint_flags={},
               pair_flags={}, pair_refusals={}, would_add_by_role={})
    if detailed:
        row.update(writes=[], reads=[], pairs=[], would_add=[])
    return row


def analyze_parsed(parsed, lang, owner, *, detailed=True, c3=()):
    from audit import c3_report

    choices = parsed.owners.get(owner, ())
    if len(choices) != 1:
        result = unavailable("owner_missing" if not choices else "owner_ambiguous", detailed)
    elif lang != "python" and choices[0].has_error:
        result = unavailable("owner_parse_recovery", detailed)
    else:
        graph, entry = parsed.compile(choices[0])
        recorded = {(r["w_start"], (r["r_start"], r["r_end"])) for r in c3}
        c3_spans = {span for _, span in recorded}
        result, surviving, read_costs, recorded_costs = flow.measure(graph, entry, detailed, recorded)
        result["status"] = "ok"
        result["counts"]["would_add_at_c3"] = sum(
            pair["r"] in c3_spans for pair in surviving
        )
        result.update(c3_report(graph, c3, surviving, detailed, read_costs, recorded_costs,
                                bool(result["counts"]["raw_pair_cap"])))
    if result["status"] != "ok":
        result["c3_counts"] = {"recorded_reads": len(c3), "unverified_reads": len(c3)}
    result.update(lang=lang, owner=owner, evidence="syntax_only",
                  graph_evidence=GRAPH_EVIDENCE,
                  limits=dict(raw_pairs=flow.RAW_LIMIT, sources_per_read=flow.READ_LIMIT,
                              edges_per_owner=flow.OWNER_LIMIT))
    return result


def analyze(source, lang, owner, *, detailed=True):
    try:
        parsed = parse(source, lang)
    except (SyntaxError, UnicodeError):
        result = unavailable("source_parse_error", detailed)
        result.update(lang=lang, owner=owner, evidence="syntax_only", graph_evidence=GRAPH_EVIDENCE)
        return result
    return analyze_parsed(parsed, lang, owner, detailed=detailed)


def main(argv=None):
    from audit import (
        FAMILIES, ROOTS, LANGUAGES, batch, merged, write_outputs,
    )

    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--corpus", choices=sorted(ROOTS))
    cli.add_argument("--file")
    cli.add_argument("--owner")
    cli.add_argument("--lang", choices=sorted(set(LANGUAGES.values())))
    cli.add_argument("--root", type=Path, help="override source root for a single owner")
    cli.add_argument("--draw", type=Path, help="use frozen sample owners, or frames with --counts-only")
    cli.add_argument("--census-dir", type=Path)
    cli.add_argument("--counts-only", action="store_true", help="whole frozen frames in batch mode")
    cli.add_argument("--family", choices=sorted(FAMILIES), action="append")
    cli.add_argument("--output", type=Path)
    cli.add_argument("--provenance", type=Path, help="record draw, census and analyzed file hashes")
    cli.add_argument("--merge", type=Path, nargs="+", help="combine disjoint family shards")
    args = cli.parse_args(argv)
    if args.merge:
        if not args.output:
            cli.error("--merge requires --output")
        merged(args.merge, args.output, args.counts_only)
    elif args.draw:
        if not args.census_dir or not args.output:
            cli.error("--draw requires --census-dir and --output")
        rows, provenance = batch(args.draw, args.census_dir, args.family or sorted(FAMILIES),
                                 args.counts_only)
        write_outputs(rows, args.output, args.counts_only)
        if args.provenance:
            args.provenance.write_text(json.dumps(provenance, sort_keys=True, indent=2) + "\n")
    elif args.corpus and args.file and args.owner:
        path = (args.root or ROOTS[args.corpus]) / args.file
        lang = args.lang or LANGUAGES.get(path.suffix.lower())
        if lang is None:
            cli.error("cannot infer the language of " + args.file)
        row = analyze(path.read_bytes(), lang, args.owner, detailed=not args.counts_only)
        row.update(corpus=args.corpus, file=args.file)
        print(json.dumps(row, sort_keys=True))
    else:
        cli.error("supply --draw or --corpus, --file and --owner")


if __name__ == "__main__":
    main()
