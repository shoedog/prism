"""Command line for the corpus registry and guarded measurements."""

from __future__ import annotations

import argparse
import sqlite3
import sys
from pathlib import Path

from .corpora import DEFAULT_MANIFEST, list_corpora, load_manifest, verify_corpora
from .diff_edges import compare_edges
from .diff_stats import compare_stats, display_stats
from .receipt import trace_settings
from .run import DEFAULT_GUARD, run


def positive_integer(value):
    if not value.isascii() or not value.isdecimal() or int(value) < 1:
        raise argparse.ArgumentTypeError("expected a positive integer")
    return int(value)


def corpus_roots(parser):
    parser.add_argument("--corpora-root", action="append", type=Path, default=[])


def main(argv=None, *, gate_sha256=None, gate_step_id=None, prediction_sha256=None):
    parser = argparse.ArgumentParser(prog="measure")
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    commands = parser.add_subparsers(dest="command", required=True)
    corpora = commands.add_parser("corpora").add_subparsers(dest="operation", required=True)
    corpora.add_parser("list")
    verification = corpora.add_parser("verify")
    corpus_roots(verification)
    verification.add_argument("keys", nargs="*")

    measurement = commands.add_parser("run")
    measurement.add_argument("--binary", type=Path, required=True)
    measurement.add_argument("--corpus", required=True)
    measurement.add_argument("--out", type=Path, required=True)
    corpus_roots(measurement)
    measurement.add_argument("--trace-env", action="append", default=[])
    measurement.add_argument("--no-bytes", action="store_true")
    measurement.add_argument("--sites", action="store_true")
    measurement.add_argument("--prediction", type=Path)
    measurement.add_argument("--memguard", type=Path, default=DEFAULT_GUARD)
    measurement.add_argument("--limit-gb", type=positive_integer, default=8)
    measurement.add_argument("--expect-version")
    edge_diff = commands.add_parser("diff-edges")
    stats_diff = commands.add_parser("diff-stats")
    for comparison in (edge_diff, stats_diff):
        comparison.add_argument("base", type=Path)
        comparison.add_argument("head", type=Path)
    edge_diff.add_argument("--classifier", type=Path, required=True)
    edge_diff.add_argument("--key", choices=("bytes", "line"))
    edge_diff.add_argument("--line-reason")
    edge_diff.add_argument("--out", type=Path, default=Path("diff-edges"))
    edge_diff.add_argument("--list", action="store_true")
    stats_diff.add_argument("--allow", type=Path)
    stats_diff.add_argument("--which", choices=("dfg", "call", "both"), default="both")
    args = parser.parse_args(argv)

    try:
        entries = load_manifest(args.manifest)
        if args.command == "corpora":
            if args.operation == "list":
                return list_corpora(entries)
            keys = list(dict.fromkeys(args.keys)) or sorted(entries)
            if any(key not in entries for key in keys):
                raise ValueError("unknown corpus key(s): " + ", ".join(k for k in keys if k not in entries))
            return verify_corpora(entries, keys, args.corpora_root)
        if args.command == "diff-stats":
            return display_stats(compare_stats(args.base, args.head, entries=entries,
                                               which=args.which, allow=args.allow))
        if args.command == "diff-edges":
            certificate = compare_edges(args.base, args.head, args.classifier, entries=entries,
                                        key=args.key, line_reason=args.line_reason, out=args.out,
                                        listing=args.list, gate_sha256=gate_sha256,
                                        gate_step_id=gate_step_id)
            return int(certificate["result"] == "fail")
        if args.corpus not in entries:
            raise ValueError(f"unknown corpus key: {args.corpus}")
        overrides = trace_settings(args.trace_env)
    except (ValueError, OSError, sqlite3.Error) as error:
        if args.command in ("diff-edges", "diff-stats"):
            print(f"measure {args.command}: {error}", file=sys.stderr)
            return 2
        parser.error(str(error))
    return run(args, entries[args.corpus], overrides, gate_sha256=gate_sha256,
               gate_step_id=gate_step_id, prediction_sha256=prediction_sha256)
