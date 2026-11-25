"""Load key spellings and exercise all ordered pairs in each runtime."""

import ast
from dataclasses import dataclass
import json
import re
import sys
from textwrap import indent
import unicodedata

from containment import BACKTICK, check_source


LANGS = ("py", "js", "go")


@dataclass
class Spelling:
    expression: str
    setup: str
    dot: str


def js_identifier(text):
    def start(char):
        return unicodedata.category(char) in {"Lu", "Ll", "Lt", "Lm", "Lo", "Nl"} or (
            char in "$_\u1885\u1886\u2118\u212e\u309b\u309c"
        )

    def part(char):
        return start(char) or unicodedata.category(char) in {"Mn", "Mc", "Nd", "Pc"} or (
            char in "\u00b7\u0387\u1369\u136a\u136b\u136c\u136d\u136e\u136f\u1370\u1371"
            "\u19da\u200c\u200d"
        )

    return bool(text) and start(text[0]) and all(part(char) for char in text[1:])


def plain_dot(expression):
    if len(expression) < 2 or expression[0] not in "'\"" + BACKTICK:
        return ""
    quote = expression[0]
    content = expression[1:-1]
    if expression[-1] == quote and js_identifier(content):
        return content
    return ""


def key_function(lang, index, spelling):
    expression, setup = spelling.expression, spelling.setup
    if lang == "py":
        body = f"{setup}\nreturn ({expression})" if setup else f"return ({expression})"
        return f"def key_{index}():\n{indent(body, '    ')}\n"
    if lang == "go":
        return f"func key_{index}() any {{\n{setup}\nreturn ({expression})\n}}\n"
    return f"function key_{index}() {{\n{setup}\nreturn ({expression});\n}}\n"


def load_spellings(directory):
    tables = {}
    for lang in LANGS:
        path = directory / f"{lang}.txt"
        spellings = []
        seen = set()
        directives = {}
        for number, line in enumerate(path.read_text(encoding="utf-8").split("\n"), 1):
            line = line.strip()
            if not line:
                continue
            label = f"{path}:{number}"
            if line.startswith("#"):
                directive, _, value = line[1:].partition(":")
                directive, value = directive.strip(), value.strip()
                if directive not in {"setup", "dot"}:
                    continue
                if not value or directive in directives:
                    raise ValueError(f"{label}: empty or repeated {directive} directive")
                if directive == "dot" and (lang != "js" or not js_identifier(value)):
                    raise ValueError(f"{label}: dot needs a JavaScript identifier")
                directives[directive] = value
                continue
            if line in seen:
                raise ValueError(f"{label}: duplicate spelling {line!r}")
            seen.add(line)
            dot = directives.get("dot", plain_dot(line)) if lang == "js" else ""
            spelling = Spelling(line, directives.get("setup", ""), dot)
            check_source(label, lang, key_function(lang, len(spellings), spelling))
            spellings.append(spelling)
            directives.clear()
        if directives or not spellings:
            raise ValueError(f"{path}: dangling directives or empty spelling list")
        tables[lang] = spellings
    return tables


def pair_keys(tables):
    for lang in LANGS:
        for left in tables[lang]:
            for right in tables[lang]:
                yield lang, left.expression, right.expression, "bracket"
                if right.dot:
                    yield lang, left.expression, right.expression, "dot"


PYTHON_PAIRS = """
for i, lhs in enumerate(keys):
    for j, rhs in enumerate(keys):
        try:
            c = {}
            c[lhs()] = 1
            same = "true" if c.get(rhs()) == 1 else "false"
        except Exception:
            same = "null"
        print("PAIR", i * len(keys) + j, same)
"""


JS_PAIRS = """
let pairIndex = 0;
for (const lhs of keys) {
    for (let i = 0; i < keys.length; ++i) {
        const rhs = keys[i];
        const dot = dots[i];
        function probe(read) {
            try {
                const c = {};
                c[lhs()] = 1;
                return read(c) === 1;
            } catch {
                return null;
            }
        }
        console.log("PAIR", pairIndex++, probe(c => c[rhs()]));
        if (dot) console.log("PAIR", pairIndex++, probe(dot));
    }
}
"""


GO_PAIRS = """
func sameSpelling(lhs, rhs func() any) (result string) {
    result = "null"
    defer func() { recover() }()
    c := make(map[any]int)
    c[lhs()] = 1
    if _, found := c[rhs()]; found {
        return "true"
    }
    return "false"
}

func main() {
    for i, lhs := range keys {
        for j, rhs := range keys {
            truthPrintPair(i * len(keys) + j, sameSpelling(lhs, rhs))
        }
    }
}
"""


def program(lang, spellings):
    functions = []
    names = []
    for index, spelling in enumerate(spellings):
        definition = key_function(lang, index, spelling)
        if lang == "py":
            try:
                ast.parse(definition)
            except SyntaxError:
                # Keep the syntax failure local to pairs using this spelling.
                definition = f"def key_{index}():\n    raise SyntaxError('invalid key expression')\n"
        functions.append(definition)
        names.append(f"key_{index}")
    keys = ", ".join(names)
    definitions = "\n".join(functions)
    if lang == "py":
        return definitions + f"\nkeys = [{keys}]\n" + PYTHON_PAIRS
    if lang == "go":
        return "package main\n\n" + definitions + f"\nvar keys = []func() any{{{keys}}}\n" + GO_PAIRS
    dots = []
    for index, spelling in enumerate(spellings):
        name = spelling.dot
        if name:
            dots.append(f"c => {{ if (String(key_{index}()) !== {json.dumps(name)})"
                        f" throw new Error('dot spelling mismatch'); return c.{name}; }}")
        else:
            dots.append("null")
    return definitions + f"\nconst keys = [{keys}];\nconst dots = [{', '.join(dots)}];\n" + JS_PAIRS


def parse_observations(output, count):
    observations = {}
    values = {"true": True, "false": False, "null": None}
    for line in output.splitlines():
        if not re.match(r"^PAIR(?:\s|$)", line):
            continue
        match = re.fullmatch(r"PAIR ([0-9]+) (true|false|null)", line)
        if not match:
            raise ValueError(f"invalid pair observation {line!r}")
        index = int(match[1])
        if index >= count or index in observations:
            raise ValueError(f"out-of-range or repeated pair observation {index}")
        observations[index] = values[match[2]]
    return observations


def describe_pair(key):
    lang, left, right, form = key
    return f"{lang} {form} {left!r} -> {right!r}"


def run_spellings(runner, preludes, tables):
    expected = list(pair_keys(tables))
    for lang, spellings in tables.items():
        keys = [key for key in expected if key[0] == lang]
        output, error = runner.execute(lang, program(lang, spellings), preludes[lang])
        observations = {}
        if error is None:
            try:
                observations = parse_observations(output, len(keys))
            except ValueError as invalid:
                error = str(invalid)
        null_count = 0
        for index, key in enumerate(keys):
            _, left, right, form = key
            same = observations.get(index)
            if same is None:
                null_count += 1
                reason = error or ("pair raised an error" if index in observations
                                   else "missing pair observation")
                print(f"inadmissible spelling pair {describe_pair(key)} ({reason})", file=sys.stderr)
            yield {"lang": lang, "s1": left, "s2": right, "form": form, "same": same}
        print(f"{lang}: {null_count}/{len(keys)} inadmissible spelling pairs", file=sys.stderr)
