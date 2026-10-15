#!/usr/bin/env python3
"""Weighted file size for the CLAUDE.md file-size rule (Key Design Decision 7).

A code line counts 1, a comment-only line 0.5, a blank line 0. Comment-only
means the line, stripped, starts with `//` or lies inside a `/* ... */` block
(Rust and the other C-family files in this repo). A line holding code and a
trailing comment is a code line. Strings that contain `//` or `/*` are not
special-cased; the count is a size guide, not a parser.

Usage:
  python3 scripts/file_size.py FILE...           # one row per file
  python3 scripts/file_size.py --over 2000 FILE...  # only files above a limit;
                                                 # exits 1 if any are listed
"""

import argparse
import sys

TARGET = 600
SPLIT_EXPECTED = 1000
CEILING = 2000


def weigh(text):
    """Return (code, comment, blank, weighted) for the given source text."""
    code = comment = blank = 0
    in_block = False
    for raw in text.splitlines():
        line = raw.strip()
        if in_block:
            comment += 1
            if '*/' in line:
                in_block = False
            continue
        if not line:
            blank += 1
        elif line.startswith('//'):
            comment += 1
        elif line.startswith('/*'):
            comment += 1
            in_block = '*/' not in line[2:]
        else:
            code += 1
    return code, comment, blank, code + comment / 2


def main(argv):
    parser = argparse.ArgumentParser(description=(__doc__ or '').splitlines()[0])
    parser.add_argument('files', nargs='+')
    parser.add_argument('--over', type=float, default=None,
                        help='list only files whose weighted size exceeds this')
    args = parser.parse_args(argv)
    listed = 0
    for path in args.files:
        with open(path, encoding='utf-8', errors='replace') as handle:
            code, comment, blank, weighted = weigh(handle.read())
        if args.over is not None and weighted <= args.over:
            continue
        listed += 1
        print(f'{weighted:9.1f}  code {code:6d}  comment {comment:6d}  blank {blank:6d}  {path}')
    return 1 if args.over is not None and listed else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
