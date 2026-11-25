"""Regenerate the lane 10 seed programs and row index."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import tempfile
import textwrap

ROOT = Path(__file__).resolve().parent
ROWS = []
# Line-seeded CLI verdicts from prism-base. R01.py and R01.js returned
# SymbolNotFound for the sink seed and yielded no verdict to pin.
BASE_ROWS_BY_VERDICT = {
    "BoundaryExited": """
        A01.js M04.js
    """,
    "Reached": """
        A02.js C01.c C01.rs I03.py K22.js M01.py
        M02.py M03.py P08.js P17.go P17.js P17.py
        P20.go P20.js P20.py P23.go P24.js P24.py
        P26.py P27.go P27.js P27.py P30.py
    """,
    "NotReached": """
        C01.java K01.py K02.py K05.js K10.js K12.py
        K13.py K14.go K15.js K16.py O01.js O01.py
        O02.py O03.js O04.py P01.go P01.js P01.py
        P03.go P03.js P03.py P04.js P04.py P09.js
        P10.go P12.js P12.py P28.js P28.py P29.py
        R03.go R03.js R03.py X04.py Y02.go Y02.js
        Y02.py Y04.go Y04.js Y04.py Y07.py Y08.go
        Y08.js Y08.py Y09.go Y09.js Y09.py
    """,
}
BASE_ROUTES = {
    "C01.c": "exact",
    "C01.java": "missing_sink_use",
    "C01.rs": "exact",
    "M01.py": "exact",
    "M02.py": "exact",
    "M03.py": "exact",
    "M04.js": "missing_sink_use",
    "P28.js": "none",
    "P28.py": "none",
    "P29.py": "none",
    "Y09.go": "missing_sink_use",
    "Y09.js": "missing_sink_use",
    "Y09.py": "missing_sink_use",
}
BASE_VERDICTS = {
    row_id: verdict
    for verdict, ids in BASE_ROWS_BY_VERDICT.items()
    for row_id in ids.split()
}


def by_lang(options, lang):
    return options.get(lang, options.get("js") if lang == "ts" else None)


def local(lang, name, value):
    if lang == "py":
        return f"{name} = {value}"
    if lang == "go":
        return f"{name} := {value}"
    return f"const {name} = {value}"


def container(lang, *keys, name="d", entries=None, go_type="string"):
    entries = entries if entries is not None else ", ".join(f'"{key}": "s0"' for key in keys)
    value = "{" + entries + "}"
    if lang == "go":
        value = f"map[string]{go_type}" + value
    return local(lang, name, value)


def add(rid, langs, group, cls, census, body, *, tail=None, negative=None,
        runner=False, pin=None, loss=None, candidate=False, label=None):
    for lang in langs.split():
        file = f"{rid}.{lang}"
        row = dict(id=file, group=group, lang=lang, file=file, **{"class": cls}, census=census)
        if negative:
            row["expect_if_not" if runner else "expect"] = negative
        if label:
            row["label"] = label
        if candidate:
            row["accepted_loss_candidate"] = True
        if loss:
            row["accepted_loss"] = {"reason": loss, "issue": None}
        if group == "control" and file in BASE_VERDICTS:
            row["base_verdict"] = BASE_VERDICTS[file]
        if pin:
            row["pinned_main"] = {"verdict": BASE_VERDICTS[file], "reason": pin}
            if not runner and cls != "out_of_model":
                row["pinned_main"]["route"] = BASE_ROUTES[file]

        contents = body(lang) if callable(body) else by_lang(body, lang)
        extra = by_lang(tail, lang) if tail else None
        if lang == "py":
            source = "def case():\n    «t» = src()\n" + textwrap.indent(contents, "    ")
        elif lang in ("js", "ts"):
            source = "function caseFn() {\n    const «t» = src();\n"
            source += textwrap.indent(contents, "    ") + "\n}"
        elif lang == "go":
            source = "package main\nfunc caseFn() {\n    «t» := src()\n"
            source += textwrap.indent(contents, "    ") + "\n}"
        else:
            source = contents
        if extra:
            source += "\n\n" + textwrap.dedent(extra).strip()
        case, locations = annotate(source + "\n", lang)
        row.update(locations)
        ROWS.append((row, case))


def annotate(source, lang):
    """Strip span notation and emit the exact UTF-8 ranges and marker lines."""
    output = []
    offset = 0
    sites = {}
    for number, line in enumerate(source.splitlines(keepends=True), start=1):
        comment = "#" if lang == "py" else "//"
        marker = re.fullmatch(r"(.*) @([WR])\n", line)
        kind = marker[2] if marker else "src"
        if marker:
            line = f"{marker[1]} {comment} {kind}\n"
        match = re.search(r"«([^»]+)»", line)
        if match:
            assert kind not in sites, (kind, source)
            start = offset + len(line[:match.start()].encode())
            sites[kind] = (number, [start, start + len(match[1].encode())])
            line = line[:match.start()] + match[1] + line[match.end():]
        else:
            assert not marker, (kind, source)
        output.append(line)
        offset += len(line.encode())
    case = "".join(output)
    # The sole sink call occupies one line. Its last closing parenthesis is the
    # call delimiter, even when the argument itself contains another call.
    sink_lines = [(i, line) for i, line in enumerate(output, start=1) if "sink(" in line]
    assert len(sink_lines) == 1 and set(sites) == {"src", "W", "R"}, source
    sink_line, line = sink_lines[0]
    begin = sum(len(prefix.encode()) for prefix in output[:sink_line - 1])
    sink = [begin + len(line[:line.index("sink(") + 5].encode()),
            begin + len(line[:line.rindex(")")].encode())]
    return case, dict(src_line=sites["src"][0], src_bytes=sites["src"][1],
                      w_marker_line=sites["W"][0], w_bytes=sites["W"][1],
                      r_marker_line=sites["R"][0], r_bytes=sites["R"][1],
                      sink_line=sink_line, sink_bytes=sink)


def write(target, value):
    return f"«{target}» = {value} @W"


def read(value):
    return f"sink(«{value}») @R"


def lines(lang, *statements):
    formatted = []
    for statement in statements:
        code, separator, marker = statement.partition(" @")
        block_line = code.endswith(("{", "},")) or code.strip() == "}" or code.startswith("for ")
        if lang in ("js", "ts") and not block_line:
            code += ";"
        formatted.append(code + separator + marker)
    return "\n".join(formatted)


def pair(rid, langs, group, cls, target, observed, *,
         census="C1", setup=None, between=(), after=(), value="t", **metadata):
    add(rid, langs, group, cls, census, lambda lang: lines(
        lang, *((setup or (lambda lang: [container(lang, "a", "b")]))(lang)),
        write(target, value), *between, *after, read(observed)), **metadata)


def defs(*statements):
    return lambda lang: list(statements)


def object_with(*keys, name="o"):
    return lambda lang: [container(lang, *keys, name=name)]


def nested(lang):
    return [container(lang, entries='"u": {"n": "s0", "m": "s0"}',
                      go_type="map[string]string")]


def c_fields(lang):
    if lang == "py":
        return ['class C:', '    def __init__(self):', '        self.n = "s0"',
                '        self.m = "s0"', '        self.c = "s0"']
    return ['class C {', '    n = "s0"', '    m = "s0"', '    c = "s0"', '}']


def constructed(lang, *keys):
    fresh = "C()" if lang == "py" else "new C()"
    return c_fields(lang) + [container(lang, *keys)] + [f'd["{key}"] = {fresh}' for key in keys]


ANY = "no_route"
EXACT = "no_exact_route"
ALL = "py js go"

pair("P01", ALL, "control", "different_key", 'd["a"]', 'd["b"]',
     between=('d["b"] = "safe"',), negative=ANY)
pair("P02", ALL, "S3", "same_key", 'd["a"]', 'd["a"]', between=('d["b"] = "safe"',))
pair("P03", ALL, "control", "kill", 'd["a"]', 'd["a"]',
     between=('d["a"] = "clean"',), negative=EXACT)
for rid, group, key, negative in [("P04", "control", "m", ANY), ("P05", "S3", "n", None)]:
    pair(rid, "py js", group, "same_key" if key == "n" else "different_key",
         'd["u"]["n"]', f'd["u"]["{key}"]', setup=nested,
         between=('d["u"]["m"] = "safe"',), negative=negative)
pair("P06", "js ts", "S3", "same_key", 'o["x"]', "o.x", setup=object_with("x"))
pair("P07", "js ts", "S2", "same_key", "o.x", 'o["x"]', setup=object_with("x"))
pair("P08", "js", "control", "kill", "o.x", "o.x", setup=object_with("x"),
     between=('o["x"] = "clean"',), negative=EXACT)
pair("P09", "js", "control", "different_key", "o.x", "o.y", setup=object_with("x", "y"),
     between=('o.y = "safe"',), negative=ANY)
pair("P10", "go", "control", "different_key", 'm["k"]', 'm["j"]',
     setup=object_with("k", "j", name="m"), negative=ANY)
pair("P11", "go", "S3", "same_key", 'm["k"]', 'm["k"]', setup=object_with("k", name="m"))
pair("P12", "py js", "control", "dynamic_key", "d[k]", 'd["b"]',
     setup=lambda lang: [local(lang, "k", '"q"'), container(lang, "q", "b")],
     between=('d["b"] = "safe"',), negative=EXACT)
pair("P13", "js", "S5", "dynamic_key", "o[k]", "o.q",
     setup=lambda lang: [local(lang, "k", '"q"'), container(lang, "q", name="o")])
pair("P14", ALL, "S6", "whole_container", 'd["a"]', "d", census="C3")
add("P15", ALL, "S3", "same_key", "C1", lambda lang: lines(
    lang, container(lang, "a"), write('d["a"]', "t"), local(lang, "x", '«d["a"]»') + " @R", "sink(x)"))
pair("P16", "py", "S3", "alias", 'd["a"]', 'e["a"]', census="C6",
     after=("e = d",), candidate=True)
for rid, group, key, multiline, negative in [
    ("P17", "control", "a", False, None), ("P18", "S7", "b", True, EXACT),
    ("P19", "S7", "b", False, EXACT), ("I01", "S7", "a", True, None),
    ("I02", "S7", "b", True, EXACT),
]:
    def initializer(lang):
        if multiline:
            opening = container(lang, entries="")[:-1]
            closing = '}' + (';' if lang in ("js", "ts") else "")
            if rid == "P18" and lang != "go":
                return "\n".join((opening, '    «"a"»: t, @W', '    "b": "safe"' + closing,
                                  lines(lang, read(f'd["{key}"]'))))
            second = '    "b": "safe",' if rid == "P18" else '    "b": "s",'
            return "\n".join((opening, '    «"a"»: t, @W', second,
                              closing, lines(lang, read(f'd["{key}"]'))))
        entries = '"a": t' + (', "b": "safe"' if rid == "P19" else "")
        definition = container(lang, entries=entries).replace("d ", "«d» ", 1)
        return lines(lang, definition + " @W", read(f'd["{key}"]'))
    add(rid, ALL, group, "initialiser", "C4" if multiline else "none", initializer,
        negative=negative, candidate=rid == "P19")

add("P20", ALL, "control", "initialiser", "C2", lambda lang: lines(
    lang, local(lang, "«xs»", {"py": "[t]", "js": "[t]", "go": "[]string{t}"}[lang]) + " @W",
    read("xs[0]")))
for rid, first, augment in [("P21", "t", '"x"'), ("P31", '""', "t")]:
    add(rid, ALL, "S3", "augmented_write", "C2", lambda lang: lines(
        lang, container(lang, "a"),
        write('d["a"]', first) if rid == "P21" else f'd["a"] = {first}',
        f'd["a"] += {augment}' if rid == "P21" else f'«d["a"]» += {augment} @W',
        read('d["a"]')))
for rid, group, field, negative in [("P22", "S3", "m", ANY), ("P24", "control", "n", None)]:
    pair(rid, "py js", group, "different_tail" if negative else "same_key",
         'd["u"].n', f'd["u"].{field}', setup=lambda lang: constructed(lang, "u"),
         between=('d["u"].m = "safe"',) if negative else (), negative=negative)
pair("P23", "go", "control", "extension_read", 'm["k"]', 'm["k"].x',
     setup=lambda lang: ['s := S{x: "s0"}', 's.x = t', 'm := map[string]S{"k": {x: "s0"}}'],
     value="s", tail={"go": "type S struct {\n    x string\n}"})
pair("P25", "py", "S3", "kill", 'd["a"].c', 'd["a"].c',
     setup=lambda lang: constructed(lang, "a", "b"), between=('d["b"].c = "x"',), label="exact")
pair("P26", "py", "control", "kill", 'd["a"].c', 'd["a"].c',
     setup=lambda lang: constructed(lang, "a"), between=('d["a"].c = "clean"',), negative=EXACT)

def dot_base(lang):
    if lang == "go":
        return ['o := S{a: "s0"}']
    if lang == "py":
        return ['class C:', '    pass', 'o = C()', 'o.a = "s0"']
    return [container(lang, "a", name="o")]


pair("P27", ALL, "control", "same_field", "o.a", "o.a", census="none", setup=dot_base,
     tail={"go": "type S struct {\n    a string\n}"})
pair("P28", "py js", "control", "whole_container", "o.a", "o", census="none", setup=dot_base,
     pin="Dot-field contributions to whole containers are outside this lane.",
     loss="Dot-field write to whole-object read")
pair("P29", "py", "control", "alias", "o.a", "p.a", census="none", setup=dot_base,
     after=("p = o",), pin="Dot-field aliases are outside this lane.", loss="Alias read")
pair("P30", "py", "control", "initialiser", "o", "o.a", census="none",
     setup=defs(), value="C(t)", tail={"py": """
     class C:
         def __init__(self, v):
             self.a = v
     """})

for rid, target, observed in [("K01", 'd["a"]', 'd.a'), ("K02", 'd.a', 'd["a"]')]:
    pair(rid, "py", "control", "spelling", target, observed, census="C2", setup=defs(
         'd = D({"a": "s0"})', 'd.a = "s0"'), between=(f'{observed} = "safe"',),
         negative=EXACT, tail={"py": "class D(dict):\n    pass"})
pair("K04", "js", "S3", "spelling", 'o["a-b"]', 'o["a-b"]', census="C2",
     setup=object_with("a-b"))
pair("K05", "js", "control", "different_key", 'o["a-b"]', 'o["a b"]',
     setup=object_with("a-b", "a b"), between=('o["a b"] = "s"',), negative=ANY)
for rid, target, observed in [("K06", 'o["1"]', "o[1]"), ("K07", "o[1]", 'o["1"]'),
                              ("K08", r'o["\x78"]', "o.x")]:
    pair(rid, "js", "S5", "spelling", target, observed, census="C2",
         setup=object_with("x" if rid == "K08" else "1"))
pair("K09", "py", "S5", "spelling", r'd["\x61"]', 'd["a"]', census="C2")
pair("K09", "go", "S5", "spelling", r'm["\x6b"]', 'm["k"]', census="C2",
     setup=object_with("k", name="m"))
for rid, lang, group, target, observed, root, key in [
    ("K10", "js", "S5", "o[`x`]", "o.x", "o", "x"),
    ("K13", "py", "S3", 'd[r"a"]', 'd["a"]', "d", "a"),
    ("K14", "go", "S3", "m[`k`]", 'm["k"]', "m", "k"),
    ("K15", "js", "S5", 'o["__proto__"]', "o.__proto__", "o", None),
    ("K16", "py", "S3", 'd["a" "b"]', 'd["ab"]', "d", "ab"),
]:
    # Keep the inherited setter and give its getter a safe initial prototype.
    setup = object_with(key, name=root) if key else defs('const o = {}', 'o.__proto__ = {value: "s0"}')
    pair(rid, lang, group, "spelling", target, observed, census="C2", setup=setup,
         negative=EXACT, runner=True, pin="Pending the stage D key rule and executed truth.")
pair("K11", "py", "S5", "spelling", 'd[f"{p}"]', 'd["a"]', census="C2",
     setup=defs('p = "a"', 'd = {"a": "s0"}'))
pair("K11", "js", "S5", "spelling", 'o[`${p}`]', 'o["a"]', census="C2",
     setup=defs('const p = "a"', 'const o = {"a": "s0"}'))
pair("K12", "py", "control", "spelling", 'd[b"a"]', 'd["a"]', census="C2",
     between=('d["a"] = "s"',), negative=EXACT)
pair("K18", "go", "S5", "spelling", "m[k]", 'm["a"]', census="C2",
     setup=defs('const k = "a"', 'm := map[string]string{"a": "s0"}'))
for rid, target, observed, root, key in [
    ("K19", 'o["x"]', 'o?.["x"]', "o", "x"),
    ("K20", 'o["$x"]', "o.$x", "o", "$x"),
    ("K21", '$o["x"]', "$o.x", "$o", "x"),
]:
    pair(rid, "js ts", "S3", "spelling", target, observed, census="C2",
         setup=object_with(key, name=root), candidate=rid == "K21")

for langs, group in [("py go", "S3"), ("js", "S4")]:
    pair("L01", langs, group, "kill", 'd["a"]', 'd["a"]',
         between=('d["b"] = "x"',), label="exact")
pair("L02", "py js", "S4", "kill", 'd["u"]["n"]', 'd["u"]["n"]',
     setup=nested, between=('d["u"] = {"n": "s0"}',), negative=EXACT)
pair("L03", "js", "S4", "kill", 'd["u"]', 'd["u"]',
     setup=defs('const d = {"u": {v: "s0", w: "s0"}}'), value="{v: t}",
     between=('d["u"].w = "x"',), label="exact")
for rid, key, negative in [("L04", "a", EXACT), ("L05", "b", None)]:
    pair(rid, ALL, "S3", "kill", 'd["a"]', 'd["a"]',
         setup=lambda lang: [local(lang, "k", f'"{key}"'), container(lang, "a", "b")],
         between=('d[k] = "clean"',), negative=negative, label="nonexact")
pair("L06", "js", "S4", "kill", "o.x", "o.x", setup=object_with("x", "y"),
     between=('o["y"] = "c"',), label="exact")
pair("L07", "js", "S4", "kill", 'o["a-b"]', 'o["a-b"]', setup=object_with("a-b", "x"),
     between=('o.x = "c"',), label="exact")
pair("L08", "js", "S4", "kill", "o[k]", "o[k]", setup=defs(
     'const k = "z"', 'const o = {z: "s0", x: "s0"}'), between=('o.x = "c"',))

pair("N01", "py", "S3", "alias", 'd["a"]', 'd["a"]', census="C6",
     between=("e = d", 'e["a"] = "safe"'), negative=EXACT,
     label={"C": "nonexact", "A": "nonexact", "B": "exact"})
for lang, group, replacement in [("py", "S3", '{"a": "s0"}'), ("go", "S4", 'map[string]string{"a": "s0"}')]:
    pair("N02", lang, group, "identity", 'd["a"]', 'd["a"]', census="C6",
         between=(f"d = {replacement}",), negative=EXACT)
pair("N03", "js", "S3", "alias", 'o["x"]', 'o["x"]', census="C6",
     setup=defs('const o = {x: "s0"}', 'const u = o'), between=('u["x"] = "safe"',),
     negative=EXACT, label="nonexact")

for rid, key, target, observed in [
    ("Y01", "a", "d[k]", 'd["a"]'), ("Y02", "b", "d[k]", 'd["a"]'),
    ("Y03", "a", 'd["a"]', "d[k]"), ("Y04", "b", 'd["a"]', "d[k]"),
]:
    pair(rid, ALL, "S5" if key == "a" else "control", "dynamic_key", target, observed,
         setup=lambda lang: [local(lang, "k", f'"{key}"'), container(lang, "a", "b")],
         negative=EXACT if key == "b" else None, label="nonexact" if key == "a" else None)
pair("Y05", "py js", "S5", "dynamic_key", 'd[k]["a"]', 'd["x"]["a"]',
     setup=lambda lang: [local(lang, "k", '"x"'), container(lang, entries='"x": {"a": "s0"}')])
pair("Y06", "py js", "S5", "dynamic_key", 'd["u"][k]', 'd["u"]["n"]',
     setup=lambda lang: [local(lang, "k", '"n"'), *nested(lang)])
pair("Y07", "py", "control", "spelling", "d[0]", 'd["0"]', census="C2",
     setup=object_with("0", name="d"), negative=EXACT)
for rid, index in [("Y08", 1), ("Y09", 0)]:
    pair(rid, ALL, "control", "integer_index", "xs[0]", f"xs[{index}]", census="C2",
         setup=lambda lang: [local(lang, "xs", {
             "py": '["s0", "s0"]', "js": '["s0", "s0"]', "go": '[]string{"s0", "s0"}'}[lang])],
         negative=EXACT if index == 1 else None, pin="Integer indices are outside this lane." if index == 0 else None,
         loss="Integer-index write to read" if index == 0 else None)
for rid, key in [("Y10", "y"), ("Y11", "x")]:
    pair(rid, "py js", "S3", "dynamic_key", 'd[i]["a"]', 'd[j]["a"]',
         setup=lambda lang: [local(lang, "i", '"x"'), local(lang, "j", f'"{key}"'),
                             container(lang, entries='"x": {"a": "s0"}, "y": {"a": "s0"}')],
         negative=EXACT if key == "y" else None, label="nonexact" if key == "x" else None)
add("Y12", "py js", "S5", "dynamic_key", "C1", lambda lang: lines(
    lang, local(lang, "k", '"b"'), container(lang, "a", "b"), write('d["a"]', "t"),
    'f(d["b"], «d[k]») @R'), tail={"py": "def f(p, q):\n    sink(q)",
                                 "js": "function f(p, q) {\n    sink(q);\n}"})
pair("Y13", "js", "S5", "dynamic_key", "o.x", "o[k]",
     setup=defs('const k = "x"', 'const o = {x: "s0"}'))

pair("X01", "js", "S4", "kill", 'o["k"]', 'o["k"]', setup=object_with("k"),
     between=('delete o["k"]',), negative=EXACT)
pair("X02", "py", "S6", "whole_container", 'd["k"]', "d", census="C3",
     setup=object_with("k", name="d"), between=('del d["k"]',), negative=EXACT)
pair("X03", "go", "S4", "kill", 'm["k"]', 'm["k"]', setup=object_with("k", name="m"),
     between=('delete(m, "k")',), negative=EXACT, candidate=True)
pair("X04", "py", "control", "kill", 'd["k"]', 'd["k"]',
     setup=object_with("k", name="d"), between=('del d["k"]', 'd["k"] = "c"'), negative=EXACT)

for rid, target in [("T01", 'for («o["a"]» of [t]) {}'),
                    ("T02", '[«o["a"]»] = [t]'),
                    ("T03", '({v: «o["a"]»} = {v: t})')]:
    add(rid, "js", "S3", "loop_target" if rid == "T01" else "destructure_target", "C2",
        lambda lang: lines(lang, container(lang, "a", name="o"), target + " @W", read('o["a"]')))
for rid, target in [("T04", 'for «d["a"]» in [t]: @W\n    pass'),
                    ("T05", '«d["a"]», x = t, 1 @W')]:
    add(rid, "py", "S3", "loop_target" if rid == "T04" else "destructure_target", "C2",
        lambda lang: lines(lang, container(lang, "a"), target, read('d["a"]')))

SINK_ARG = {
    "py": "def f(p):\n    sink(p)",
    "js": "function f(p) {\n    sink(p);\n}",
    "go": "func f(p string) {\n    sink(p)\n}",
}
CONSUME = {"js": "function consume(p) {\n    sink(p.slot);\n}"}
STRUCT = {"go": "type S struct {\n    n string\n}"}
for rid, call, helper in [
    ("A01", 'consume({slot: «d["u"].n»})', CONSUME),
    ("A02", 'consume2(«d["u"].n»)', {"js": "function consume2(p) {\n    sink(p);\n}"}),
]:
    add(rid, "js", "control", "argument", "C1", lambda lang: lines(
        lang, container(lang, "u"), 'd["u"] = {n: "s0"}', write('d["u"].n', "t"), call + " @R"),
        tail=helper)


def extension(lang, key="u"):
    if lang == "py":
        return ['class C:', '    def __init__(self, v):', '        self.n = v',
                'd = {"u": C("s0")' + (', "w": C("s0")' if key == "w" else "") + '}'], "C(t)"
    if lang == "js":
        entries = '"u": {n: "s0"}' + (', "w": {n: "s0"}' if key == "w" else "")
        return [container(lang, entries=entries)], "{n: t}"
    entries = '"u": {n: "s0"}' + (', "w": {n: "s0"}' if key == "w" else "")
    return [container(lang, entries=entries, go_type="S")], "S{n: t}"


def ext_step(lang, action, key="u"):
    setup, value = extension(lang, key)
    return lines(lang, *setup, write('d["u"]', value), *action)


add("E01", ALL, "S3", "extension_read", "C1",
    lambda lang: ext_step(lang, ['f(«d["u"].n») @R']),
    tail={**SINK_ARG, "go": STRUCT["go"] + "\n\n" + SINK_ARG["go"]})
add("E02", ALL, "S3", "extension_read", "C1",
    lambda lang: ext_step(lang, [local(lang, "x", '«d["u"].n»') + " @R", "sink(x)"]), tail=STRUCT)
add("E03", ALL, "S3", "extension_read", "C1", lambda lang: lines(lang, "sink(h(t))"),
    tail={
        "py": "def h(t):\n" + textwrap.indent(ext_step("py", ['return «d["u"].n» @R']), "    "),
        "js": "function h(t) {\n" + textwrap.indent(ext_step("js", ['return «d["u"].n» @R']), "    ") + "\n}",
        "go": STRUCT["go"] + "\n\nfunc h(t string) string {\n"
        + textwrap.indent(ext_step("go", ['return «d["u"].n» @R']), "    ") + "\n}",
    })
for lang, declaration, value in [
    ("py", 'd = {"u": {"n": "s0"}}', '{"n": t}'),
    ("js", 'const d = {"u": new Map([["n", "s0"]])}', 'new Map([["n", t]])'),
    ("go", 'd := map[string]S{"u": {n: "s0"}}', 'S{n: t}'),
]:
    add("E04", lang, "S3", "extension_read", "C1", lambda lang: lines(
        lang, declaration, write('d["u"]', value), 'sink(«d["u"].get»("n")) @R'),
        tail={"go": STRUCT["go"] +
         '\n\nfunc (s S) get(key string) string {\n    if key == "n" {\n        return s.n\n    }\n    return "s0"\n}'})
add("E05", "js", "S3", "extension_read", "C1",
    lambda lang: ext_step(lang, ['consume({slot: «d["u"].n»}) @R']), tail=CONSUME)
for langs, value in [("py js", '{"n": t}'), ("go", 'map[string]string{"n": t}')]:
    pair("E06", langs, "S3", "extension_read", 'd["u"]', 'd["u"]["n"]', setup=nested, value=value)
add("E07", ALL, "S3", "extension_read", "C1",
    lambda lang: ext_step(lang, [read('d["w"].n')], "w"), tail=STRUCT, negative=ANY)

pair("K22", "js", "control", "spelling", "o.é", "o.é", census="C2", setup=object_with("é"),
     between=('o["é"] = "clean"',), negative=EXACT, label="killed")
pair("K23", "js", "S5", "spelling", 'o["é"]', "o.é", census="C2", setup=object_with("é"))

add("I03", "py", "control", "initialiser", "none", lambda lang: lines(
    lang, '«base» = {"a": t} @W', 'd = {**base, "b": "s"}', read('d["a"]')))


def split_initializer(lang):
    opening = container(lang, entries="")[:-1]
    closing = "}" + (";" if lang == "js" else "")
    if lang == "go":
        # Go inserts a semicolon after a bare identifier at end of line.
        entry = ['    «"a"»: ( @W', '        t),']
    else:
        entry = ['    «"a"»: ( @W', '        t', '    ),']
    return "\n".join([opening, *entry, '    "b": "s",', closing, lines(lang, read('d["a"]'))])


add("I04", ALL, "S7", "initialiser", "C2", split_initializer)

pair("R01", "py js", "control", "refused_root", 'd["a"]', 'make()["a"]', census="C2",
     setup=lambda lang: [local(lang, "d", "make()")], negative=EXACT,
     tail={"py": 'def make():\n    return {"a": "s0"}',
           "js": 'function make() {\n    return {a: "s0"};\n}'})
pair("R09", "py", "S3", "refused_root", 'make()["a"]', "a", census="C2",
     setup=defs('a = "s0"'), negative=EXACT, tail={"py": 'def make():\n    return {"a": "s0"}'})
for rid, group, key in [("R02", "S3", "a"), ("R03", "control", "b")]:
    add(rid, ALL, group, "argument", "C1", lambda lang: lines(
        lang, *(["_ = t"] if lang == "go" and key == "b" else []),
        container(lang, "a", "b"), write(f'd["{key}"]', "t" if key == "a" else '"s"'),
        f'f(«d["{key}"]») @R'), tail=SINK_ARG, negative=ANY if key == "b" else None)
for lang, root, setup in [
    ("py", "self", defs('self = C()', 'self.d = {"a": "s0"}')),
    ("js", "this", defs('this.d = {"a": "s0"}')),
    ("go", "self", defs('self := S{d: map[string]string{"a": "s0"}}')),
]:
    if lang == "js":
        add("R04", lang, "S3", "member_root", "C2", lambda lang: lines(
            lang, 'run.call({})',
        ) + '\nfunction run() {\n    this.d = {"a": "s0"};\n    «this.d["a"]» = t; @W\n'
            '    sink(«this.d["a"]»); @R\n}')
    else:
        pair("R04", lang, "S3", "member_root", f'{root}.d["a"]', f'{root}.d["a"]', census="C2",
             setup=setup, tail={"py": "class C:\n    pass", "go": "type S struct {\n    d map[string]string\n}"})
for lang, target in [("js", '(d)["a"]'), ("ts", '(d as any)["a"]')]:
    pair("R05", lang, "S3", "parenthesised_root", target, 'd["a"]', census="C2")
add("R06", "py js", "S3", "ordering", "C1", {
    "py": 'd = {"a": "s0"}\nfor i in range(2):\n    x = «d["a"]» @R\n    «d["a"]» = t @W\nsink(x)',
    "js": 'const d = {"a": "s0"};\nlet x = "s0";\nfor (let i = 0; i < 2; i++) {\n'
          '    x = «d["a"]»; @R\n    «d["a"]» = t; @W\n}\nsink(x);',
})
add("R07", "py js", "S3", "ordering", "C1", {
    "py": 'd = {"a": "s0"}\n«d["a"]» = t @W\ndef g():\n    sink(«d["a"]») @R\ng()',
    "js": 'const d = {"a": "s0"};\n«d["a"]» = t; @W\nfunction g() {\n    sink(«d["a"]»); @R\n}\ng();',
}, candidate=True)
add("R08", ALL, "S3", "ordering", "C1", {
    "py": """
d = {"a": "s0"}
c = True
if c:
    «d["a"]» = t @W
else:
    d["a"] = "s"
sink(«d["a"]») @R
""".strip(),
    "js": """
const d = {"a": "s0"};
const c = true;
if (c) {
    «d["a"]» = t; @W
} else {
    d["a"] = "s";
}
sink(«d["a"]»); @R
""".strip(),
    "go": """
d := map[string]string{"a": "s0"}
c := true
if c {
    «d["a"]» = t @W
} else {
    d["a"] = "s"
}
sink(«d["a"]») @R
""".strip(),
})

pair("M01", "py", "control", "known_wrong", "o[i].z", "o[j].z", census="none",
     setup=defs('o = {"x": C(), "y": C()}', 'i = "x"', 'j = "y"', 'o[j].z = "safe"'),
     tail={"py": 'class C:\n    z = "s0"'}, negative=EXACT,
     pin="Dynamic-only index paths collapse and can produce a wrong exact edge.")
for rid, setup, between in [
    ("M02", defs('o = C()'), ('o = C()',)),
    ("M03", defs('o = C()', 'p = o'), ('p.x = "safe"',)),
]:
    pair(rid, "py", "control", "known_wrong", "o.x", "o.x", census="none", setup=setup,
         between=between, tail={"py": 'class C:\n    x = "s0"'}, negative=EXACT,
         pin="Python dot-field edges can survive rebinding or writes through aliases.")
add("M04", "js", "control", "known_wrong", "none", {"js": 'put({a: «t», b: "safe"}); @W'},
    tail={"js": 'function put(q) {\n    sink(«q["b"]»); @R\n}'}, negative=EXACT,
    pin="The object-literal argument binder can propagate into the sibling formal read.")

pair("O01", "py js", "control", "out_of_model", 'd["a"]', 'd["a"]', census="none",
     between=("mutate(d)",), negative=EXACT, pin="Mutation in a callee is out of model.",
     tail={"py": 'def mutate(d):\n    d["a"] = "safe"',
           "js": 'function mutate(d) {\n    d["a"] = "safe";\n}'})
pair("O02", "py", "control", "out_of_model", 'd["a"]', 'd["b"]', census="none",
     setup=defs("d = R()"), negative=ANY, runner=True, pin="User-defined item access is out of model.",
     tail={"py": """
     class R(dict):
         def __init__(self):
             self.slot = "s0"

         def __setitem__(self, key, value):
             self.slot = value

         def __getitem__(self, key):
             return self.slot
     """})
pair("O03", "js", "control", "out_of_model", 'o["x"]', "o.x", census="none",
     setup=defs('const elements = ["s0"]', 'const target = {',
                '    get x() {', '        return elements[0] ?? "s0"', '    },',
                '    set x(value) {', '        elements[0] = value', '        elements.length = 0',
                '    },', '}', 'const o = new Proxy(target, {})'), negative=EXACT, runner=True,
     pin="Setters, Proxies and array length truncation are out of model.")
pair("O04", "py", "control", "out_of_model", 'd["a"]["x"]', 'd["b"]["x"]', census="none",
     setup=defs('v = {"x": "s0"}', 'd = {"a": v, "b": v}'),
     pin="Different static paths that alias one child are out of model.")
add("C01", "rs java c", "control", "other_language", "none", {
    "rs": """fn case_fn() {
    let «t» = src();
    let mut v = [S { x: "s0" }];
    «v[0].x» = t; @W
    let m = std::collections::HashMap::from([("k", "s0")]);
    let _value = m["k"];
    sink(«v[0].x»); @R
}

struct S {
    x: &'static str,
}""",
    "java": """class Case {
    static void caseFn() {
        String «t» = src();
        String[] a = {"s0"};
        «a[0]» = t; @W
        sink(«a[0]»); @R
    }
}""",
    "c": """typedef const char *Text;

struct S {
    Text x;
};

void case_fn(void) {
    Text «t» = src();
    struct S buf[1] = {{"s0"}};
    int i = 0;
    «buf[i].x» = t; @W
    sink(«buf[i].x»); @R
}""",
}, pin="This language is outside the lane. The runner records n/a.")


def probe(binary):
    receipts = {}
    for row, case in ROWS:
        if row["group"] != "control" and "pinned_main" not in row:
            continue
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / f'case.{row["lang"]}'
            path.write_text(case, encoding="utf-8")
            result = subprocess.run(
                [str(binary), "nav", "--no-cache", "taint-reaches", "--repo", directory,
                 "--source", f'{path.name}:{row["src_line"]}',
                 "--sink", f'{path.name}:{row["sink_line"]}', "--format", "json"],
                capture_output=True, text=True,
            )
            if result.returncode == 0:
                receipts[row["id"]] = json.loads(result.stdout)
            else:
                receipts[row["id"]] = {"exit": result.returncode, "stdout": result.stdout,
                                       "stderr": result.stderr}
    return receipts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe-base", type=Path, help="print the pinned base binary's raw responses")
    args = parser.parse_args()
    assert len({row["id"] for row, _ in ROWS}) == len(ROWS)
    if args.probe_base:
        print(json.dumps(probe(args.probe_base.resolve()), indent=2, ensure_ascii=False))
        return
    rows = sorted(ROWS, key=lambda entry: entry[0]["id"])
    (ROOT / "cases").mkdir(exist_ok=True)
    for row, case in rows:
        (ROOT / "cases" / row["file"]).write_text(case, encoding="utf-8")
    with (ROOT / "rows.jsonl").open("w", encoding="utf-8") as output:
        for row, _ in rows:
            output.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")


if __name__ == "__main__":
    main()
