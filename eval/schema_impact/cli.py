"""Command line entry point for the schema impact feasibility prototype."""

import argparse
from pathlib import Path

from . import producer_go, producer_route_table
from .schema import load


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="schema-impact")
    commands = parser.add_subparsers(dest="command", required=True)
    ids = commands.add_parser("ids", help="Print sorted operation and field IDs")
    ids.add_argument("schema")
    producer = commands.add_parser("producer", help="Measure Go producer bindings and route drift")
    producer_go.add_arguments(producer)
    routes = commands.add_parser("routes", help="Extract literal JavaScript route tables")
    producer_route_table.add_arguments(routes)
    consumer = commands.add_parser("consumer", help="Measure TypeScript and JavaScript consumer bindings")
    consumer.add_argument("--repo", type=Path, required=True)
    consumer.add_argument("--root", required=True)
    consumer.add_argument("--schema", type=Path, required=True)
    consumer.add_argument("--config", type=Path, required=True)
    consumer.add_argument("--out", type=Path, required=True)
    mismatch = commands.add_parser("mismatch", help="List standing producer and consumer mismatches")
    mismatch.add_argument("--producer", type=Path, required=True)
    mismatch.add_argument("--consumer", type=Path, required=True)
    mismatch.add_argument("--schema", type=Path, required=True)
    mismatch.add_argument("--out", type=Path, required=True)
    replay = commands.add_parser("replay", help="Replay producer changes against historical client calls")
    replay.add_argument("--repo", type=Path, required=True)
    replay.add_argument("--schema", type=Path, required=True)
    replay.add_argument("--configs", type=Path, required=True)
    replay.add_argument("--root", required=True)
    replay.add_argument("--pin", required=True)
    replay.add_argument("--out", type=Path, required=True)
    report = commands.add_parser("report", help="Compare service and client snapshots for one commit")
    report.add_argument("--repo", type=Path, required=True)
    report.add_argument("--subject", required=True)
    report.add_argument("--role", choices=["producer", "consumer"], required=True)
    report.add_argument("--out-dir", type=Path, required=True)
    report.add_argument("--configs", type=Path, default=Path(__file__).parent / "configs")
    report.add_argument("--root", default="ui/src")
    report.add_argument("--schema")
    report.add_argument("--no-schema", action="store_true")
    report.add_argument("--service-root", default=".")
    report.add_argument("--route-spec", type=Path)
    report.add_argument("--selection-time")
    report.add_argument("--source-url", help="HTTPS repository URL for revision-specific source links")
    report.add_argument("--counterpart-repo", type=Path, help="Repository of the held-unchanged side")
    report.add_argument("--counterpart-rev", help="Use this revision instead of selecting by commit date")
    report.add_argument("--counterpart-source-url", help="HTTPS repository URL for counterpart source links")
    report.add_argument("--name")
    args = parser.parse_args(argv)

    if args.command == "report":
        from .report_analysis import build_document
        from .report_inputs import DEFAULT_SCHEMA, ReportRequest, build_inputs
        from .report_markdown import render_json, render_markdown

        if args.name is not None and (Path(args.name).name != args.name or args.name in {"", ".", ".."}):
            report.error("--name must be a filename stem")
        if args.counterpart_repo is None and (args.counterpart_rev is not None or args.counterpart_source_url is not None):
            report.error("--counterpart-rev and --counterpart-source-url require --counterpart-repo")
        schema_path = None if args.no_schema else args.schema or (None if args.route_spec else DEFAULT_SCHEMA)
        request = ReportRequest(args.repo.resolve(), args.subject, args.role, args.configs, args.root, schema_path,
                                args.service_root, args.route_spec, args.selection_time, args.source_url,
                                counterpart_repo=args.counterpart_repo.resolve() if args.counterpart_repo else None,
                                counterpart_rev=args.counterpart_rev, counterpart_source_url=args.counterpart_source_url)
        document = build_document(build_inputs(request))
        name = args.name or document["change"]["after"][:8]
        args.out_dir.mkdir(parents=True, exist_ok=True)
        (args.out_dir / f"{name}.md").write_text(render_markdown(document), encoding="utf-8")
        (args.out_dir / f"{name}.json").write_text(render_json(document), encoding="utf-8")
        return 0
    if args.command == "producer":
        return producer_go.run(args)
    if args.command == "routes":
        return producer_route_table.run(args)
    if args.command == "replay":
        from ._replay_measure import run

        run(args.repo, args.schema, args.configs, args.root, args.pin, args.out)
        return 0
    if args.command == "consumer":
        from .f2 import run

        run(args.repo, args.root, args.schema, args.config, args.out)
        return 0

    index = load(args.schema)
    if args.command == "mismatch":
        from .mismatch import run

        run(args.producer, args.consumer, index, args.out)
        return 0

    for group in (index.operations, index.fields):
        for element in sorted(group):
            print(element)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
