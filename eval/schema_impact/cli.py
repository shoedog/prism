"""Command line entry point for the schema impact feasibility prototype."""

import argparse

from .schema import load


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="schema-impact")
    commands = parser.add_subparsers(dest="command", required=True)
    ids = commands.add_parser("ids", help="Print sorted operation and field IDs")
    ids.add_argument("schema")
    args = parser.parse_args(argv)

    index = load(args.schema)
    for group in (index.operations, index.fields):
        for element in sorted(group):
            print(element)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
