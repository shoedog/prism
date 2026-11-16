#!/usr/bin/env python3
"""#829 review-repair regressions for the scoring model (stdlib unittest).

Default: current candidate 12. To establish RED against the exact old scorer, export
xscore_lex.py and xscore_candidates.py with git show and pass --scorer-dir DIR
--candidate 10x-v2+list31. Inputs/assertions stay identical; no compiler is needed.
These are scoring-model tests, not evidence of an implemented Rust gate.
"""
import argparse
import hashlib
from pathlib import Path
import sys
import unittest

sys.dont_write_bytecode = True
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--scorer-dir", default=str(Path(__file__).resolve().parent))
parser.add_argument("--candidate", default="12-all-roots")
args, test_args = parser.parse_known_args()
sys.path.insert(0, str(Path(args.scorer_dir).resolve()))
import xscore_candidates as candidates
import xscore_lex as lex

# Each refusal has a tight/outside/literal/ordinary-vararg negative or edge twin.
CASES = [
    ("unknown_held", "", "ext({ int a,b; });", "arg_split"),
    ("unknown_tight", "", "ext((a,b));", None),
    ("loose_unknown", "#define PAIR 1,2\n", "ext(PAIR);", "loose_in_owner_group:PAIR"),
    ("loose_outside", "#define PAIR 1,2\n", "int a[] = { PAIR };", None),
    ("body_unknown_held", "#define B ext({ int a,b; })\n", "B;", "body_split:B"),
    ("body_unknown_tight", "#define B ext((a,b))\n", "B;", None),
    ("unknown_chain", "", "ext()({ int a,b; });", "arg_split"),
    ("unknown_chain_tight", "", "ext()((a,b));", None),
    ("nested_no_shield", "#define ID(x) x\n#define PAIR 1,2\n",
     "ID(ext(PAIR));", "loose_in_owner_group:PAIR"),
    ("vaopt_chain", "#define B(...) __VA_OPT__(K)({ int a,b; })\n", "B(1);", "body_split:B"),
    ("vaopt_tight", "#define B(...) __VA_OPT__(K)((a,b))\n", "B(1);", None),
    ("vaopt_held", "#define B(...) ext((struct o){ __VA_ARGS__ })\n", "B(1,2);", "body_split:B"),
    ("named_varargs", "#define B(args...) ext(args)\n", "B(1,2);", "body_split:B"),
    ("ordinary_varargs", "#define B(...) ext(__VA_ARGS__)\n", "B(1,2);", None),
    ("named_varargs_union", "#define B(args...) ext(args)\n#undef B\n#define B(x) (x)\n",
     "B(1);", "body_split:B"),
    ("named_varargs_reach", "#define A B\n#define B(args...) ext(args)\n", "A(1,2);", "body_split:A"),
    ("body_unknown_forward", "#define B ext(PAIR)\n#define PAIR 1,2\n", "B;", "body_split:B"),
    ("body_unknown_forward_tight", "#define B ext(PAIR)\n#define PAIR (1,2)\n", "B;", None),
    ("body_literal", '#define B ext("a,b}")\n', "B;", None),
    ("list31_preserved", "", "PG_TRY;", "block_list:PG_TRY"),
]


class ReviewRepairTests(unittest.TestCase):
    pass


def make_test(definitions, body, expected):
    def test(self):
        source = definitions + "int probe(void) { " + body + " return 0; }\n"
        db, fx = lex.MacroDb([("source.c", source)]), lex.FileIndex(source)
        functions = [f for f in fx.fns if fx.code[f.name_idx].text == "probe"]
        self.assertEqual(len(functions), 1, "fixture must locate exactly one owner")
        owner = lex.Owner(fx, functions[0], "source.c")
        actual = candidates.CANDIDATES[args.candidate][0](owner, db)
        self.assertEqual(actual, expected)
    return test


for name, definitions, body, expected in CASES:
    setattr(ReviewRepairTests, "test_" + name, make_test(definitions, body, expected))

if __name__ == "__main__":
    for module in (lex, candidates):
        path = Path(module.__file__).resolve()
        print("BOUND", path, hashlib.sha256(path.read_bytes()).hexdigest(), flush=True)
    print("CANDIDATE", args.candidate, flush=True)
    unittest.main(argv=[sys.argv[0]] + test_args)
