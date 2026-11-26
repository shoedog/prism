"""Regenerate the lane 10 seed programs and row index."""
import argparse
from hashlib import sha256
import json
from pathlib import Path
import re
import subprocess
import tempfile
import textwrap

from gen_python_q import python_q

ROOT = Path(__file__).resolve().parent
ROWS = []
# Line-seeded CLI verdicts for the retained draft controls.
BASE_ROWS_BY_VERDICT = {
    "BoundaryExited": "A01.js M04.js",
    "Reached": """
        A02.js C01.c C01.rs K22.js M01.py M02.py M03.py P08.js P23.go
        P24.js P24.py P26.py P27.go P27.js P27.py P30.py
    """,
    "NotReached": "C01.java K01.py O03.js P09.js P28.js P28.py P29.py R03.go R03.js R03.py",
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
}
# These retain draft metadata where the disposition changes only the group.
REGROUPED = {
    "baseline-gap": "A01.js C01.java M01.py M02.py M03.py M04.js P26.py P30.py".split(),
    "boundary": "P28.js P28.py P29.py".split(),
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
        runner=False, pin=None, loss=None, label=None, roles=None):
    for lang in langs.split():
        file = f"{rid}.{lang}"
        row = dict(id=file, group=group, lang=lang, file=file, **{"class": cls}, census=census)
        if negative:
            row["expect_if_not" if runner else "expect"] = negative
        if label:
            row["label"] = label
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
        case, locations, reads = annotate(source + "\n", lang)
        row.update(locations)
        if roles:
            assert len(reads) == len(roles), file
            row["case_sha256"] = sha256(case.encode()).hexdigest()
            row["reads"] = [dict(role=role, line=line, bytes=span)
                            for role, (line, span) in zip(roles, reads)]
        else:
            assert len(reads) == 1, file
        if group == "recall":
            row["group"] = f"recall-{lang}"
        for destination, ids in REGROUPED.items():
            if file in ids:
                row["group"] = destination
        ROWS.append((row, case))


def annotate(source, lang):
    """Strip span notation and emit the exact UTF-8 ranges and marker lines."""
    output = []
    offset = 0
    sites = {}
    for number, line in enumerate(source.splitlines(keepends=True), start=1):
        comment = "#" if lang == "py" else "//"
        marker = re.fullmatch(r"(.*) @(W|R[0-9]*)\n", line)
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
    reads = [sites[site] for site in sorted(sites) if site.startswith("R")]
    assert len(sink_lines) == 1 and {"src", "W", "R"} <= sites.keys(), source
    sink_line, line = sink_lines[0]
    begin = sum(len(prefix.encode()) for prefix in output[:sink_line - 1])
    sink = [begin + len(line[:line.index("sink(") + 5].encode()),
            begin + len(line[:line.rindex(")")].encode())]
    return case, dict(src_line=sites["src"][0], src_bytes=sites["src"][1],
                      w_marker_line=sites["W"][0], w_bytes=sites["W"][1],
                      r_marker_line=sites["R"][0], r_bytes=sites["R"][1],
                      sink_line=sink_line, sink_bytes=sink), reads


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

pair("P08", "js", "control", "kill", "o.x", "o.x", setup=object_with("x"),
     between=('o["x"] = "clean"',), negative=EXACT)
pair("P09", "js", "control", "different_key", "o.x", "o.y", setup=object_with("x", "y"),
     between=('o.y = "safe"',), negative=ANY)

pair("P14", ALL, "recall", "whole_container", 'd["a"]', "d", census="C3")
add("P15", ALL, "boundary", "same_key", "C1", lambda lang: lines(
    lang, container(lang, "a"), write('d["a"]', "t"), local(lang, "x", '«d["a"]»') + " @R", "sink(x)"))

pair("P23", "go", "control", "extension_read", 'm["k"]', 'm["k"].x',
     setup=lambda lang: ['s := S{x: "s0"}', 's.x = t', 'm := map[string]S{"k": {x: "s0"}}'],
     value="s", tail={"go": "type S struct {\n    x string\n}"})
pair("P24", "py js", "control", "same_key", 'd["u"].n', 'd["u"].n',
     setup=lambda lang: constructed(lang, "u"))

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


pair("K01", "py", "control", "spelling", 'd["a"]', 'd.a', census="C2", setup=defs(
     'd = D({"a": "s0"})', 'd.a = "s0"'), between=('d.a = "safe"',),
     negative=EXACT, tail={"py": "class D(dict):\n    pass"})


add("Y12", "py js", "boundary", "dynamic_key", "C1", lambda lang: lines(
    lang, local(lang, "k", '"b"'), container(lang, "a", "b"), write('d["a"]', "t"),
    'f(d["b"], «d[k]») @R'), tail={"py": "def f(p, q):\n    sink(q)",
                                 "js": "function f(p, q) {\n    sink(q);\n}"})

pair("X02", "py", "boundary", "whole_container", 'd["k"]', "d", census="C3",
     setup=object_with("k", name="d"), between=('del d["k"]',), negative=EXACT)

SINK_ARG = {
    "py": "def f(p):\n    sink(p)",
    "js": "function f(p) {\n    sink(p);\n}",
    "go": "func f(p string) {\n    sink(p)\n}",
}

CONSUME = {"js": "function consume(p) {\n    sink(p.slot);\n}"}

for rid, call, helper in [
    ("A01", 'consume({slot: «d["u"].n»})', CONSUME),
    ("A02", 'consume2(«d["u"].n»)', {"js": "function consume2(p) {\n    sink(p);\n}"}),
]:
    add(rid, "js", "control", "argument", "C1", lambda lang: lines(
        lang, container(lang, "u"), 'd["u"] = {n: "s0"}', write('d["u"].n', "t"), call + " @R"),
        tail=helper)

pair("K22", "js", "control", "spelling", "o.é", "o.é", census="C2", setup=object_with("é"),
     between=('o["é"] = "clean"',), negative=EXACT, label="killed")

add("R03", ALL, "control", "argument", "C1", lambda lang: lines(
    lang, *(["_ = t"] if lang == "go" else []), container(lang, "a", "b"),
    write('d["b"]', '"s"'), 'f(«d["b"]») @R'), tail=SINK_ARG, negative=ANY)


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


pair("O03", "js", "control", "out_of_model", 'o["x"]', "o.x", census="none",
     setup=defs('const elements = ["s0"]', 'const target = {',
                '    get x() {', '        return elements[0] ?? "s0"', '    },',
                '    set x(value) {', '        elements[0] = value', '        elements.length = 0',
                '    },', '}', 'const o = new Proxy(target, {})'), negative=EXACT, runner=True,
     pin="Setters, Proxies and array length truncation are out of model.")

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


# Programs rebuilt for whole-value recall or for an observable boundary.
def projected_boundary(lang, target, receiver, *, initial, between=()):
    if lang == "js":
        projection = ["let x", f"x = «{receiver}» @R"]
    else:
        projection = [local(lang, "x", f"«{receiver}»") + " @R"]
    return lines(lang, *initial, target, *between, *projection, "sink(x)")


add("P01", ALL, "boundary", "different_key", "C1", lambda lang: projected_boundary(
    lang, write('d["a"]', "t"), 'd["b"]', initial=[container(lang, "a", "b")],
    between=('d["b"] = "safe"',)), roles=("projection",), negative=ANY)
add("P03", ALL, "recall", "overwrite", "C1", lambda lang: lines(
    lang, container(lang, "a", "b"), write('d["a"]', "t"),
    'd["a"] = "clean"', read("d")), roles=("argument",), negative=EXACT)

add("O01", "py js", "recall", "opaque_mutation", "none", lambda lang: lines(
    lang, container(lang, "a", "b"), write('d["a"]', "t"), 'mutate(«d») @R1', read("d")),
    roles=("argument", "argument"), negative=EXACT,
    tail={"py": 'def mutate(d):\n    d["a"] = "safe"',
          "js": 'function mutate(d) {\n    d["a"] = "safe";\n}'})

add("Y09", ALL, "recall", "integer_index", "C2", lambda lang: lines(
    lang, local(lang, "xs", {
        "py": '["s0", "s0"]', "js": '["s0", "s0"]', "go": '[]string{"s0", "s0"}',
    }[lang]), write("xs[0]", "t"), read("xs")), roles=("argument",))

for rid, lang, root, delete in [
    ("X01", "js", "o", 'delete o["k"]'),
    ("X03", "go", "m", 'delete(m, "k")'),
]:
    add(rid, lang, "boundary", "delete", "C1", lambda lang: lines(
        lang, container(lang, "k", name=root), write(f'{root}["k"]', "t"), delete, read(root)),
        roles=("argument",), negative=EXACT)

add("P16", "py", "recall", "alias", "C6", lambda lang: lines(
    lang, container(lang, "a", "b"), write('d["a"]', "t"), "e = «d» @R", "sink(e)"),
    roles=("assignment_rhs",))
add("L01", ALL, "recall", "sibling_write", "C1", lambda lang: lines(
    lang, container(lang, "a", "b"), write('d["a"]', "t"), 'd["b"] = "x"', read("d")),
    roles=("argument",))
for rid, initial, augment in [
    ("P21", write('d["a"]', "t"), 'd["a"] += "x"'),
    ("P31", 'd["a"] = ""', '«d["a"]» += t @W'),
]:
    add(rid, ALL, "recall", "augmented_write", "C2", lambda lang: lines(
        lang, container(lang, "a"), initial, augment, read("d")), roles=("argument",))

add("R04", ALL, "recall", "member_root", "C2", lambda lang: lines(
    lang, {
        "py": 'o = C()\no.items = {"a": "s0"}',
        "js": 'const o = {items: {a: "s0"}}',
        "go": 'o := S{items: map[string]string{"a": "s0"}}',
    }[lang], write('o.items["a"]', "t"), read("o.items")), roles=("argument",),
    tail={"py": "class C:\n    pass", "go": "type S struct {\n    items map[string]string\n}"})

add("N02", "py go", "boundary", "replacement", "C6", lambda lang: lines(
    lang, container(lang, "a"), write('d["a"]', "t"), {
        "py": 'd = {"a": "s0"}', "go": 'd = map[string]string{"a": "s0"}',
    }[lang], read("d")), roles=("argument",), negative=EXACT)

add("R06", "py js", "boundary", "backward_iteration", "C1", {
    "py": """d = {"a": "s0"}
x = {}
for i in range(2):
    if i == 1:
        x = «d» @R
        break
    «d["a"]» = t @W
sink(x)""",
    "js": """const d = {a: "s0"};
let x = {};
for (let i = 0; i < 2; i++) {
    if (i === 1) {
        x = «d»; @R
        break;
    }
    «d["a"]» = t; @W
}
sink(x);""",
}, roles=("assignment_rhs",))

add("R07", "py js", "boundary", "nested_owner", "C1", {
    "py": 'd = {"a": "s0"}\n«d["a"]» = t @W\ndef g():\n    sink(«d») @R\ng()',
    "js": 'const d = {a: "s0"};\n«d["a"]» = t; @W\nfunction g() {\n    sink(«d»); @R\n}\ng();',
}, roles=("argument",))

add("R08", ALL, "recall", "branch_join", "C1", {
    "py": """d = {"a": "s0"}
c = True
if c:
    «d["a"]» = t @W
else:
    d["a"] = "s"
sink(«d») @R""",
    "js": """const d = {a: "s0"};
const c = true;
if (c) {
    «d["a"]» = t; @W
} else {
    d["a"] = "s";
}
sink(«d»); @R""",
    "go": """d := map[string]string{"a": "s0"}
c := true
if c {
    «d["a"]» = t @W
} else {
    d["a"] = "s"
}
sink(«d») @R""",
}, roles=("argument",))

add("R09", "py", "boundary", "refused_root", "C2", {"py": """«make()["a"]» = t @W
clean = {"a": "s0"}
x = «clean» @R
sink(x)"""}, roles=("assignment_rhs",), negative=EXACT,
    tail={"py": 'def make():\n    return {"a": "s0"}'})

for rid, target, kind in [
    ("T01", 'for («o["a"]» of [t]) {}', "loop_target"),
    ("T02", '[«o["a"]»] = [t]', "destructure_target"),
    ("T03", '({v: «o["a"]»} = {v: t})', "destructure_target"),
]:
    add(rid, "js", "boundary", kind, "C2", lambda lang: projected_boundary(
        lang, target + " @W", 'o["a"]', initial=[container(lang, "a", name="o")]),
        roles=("projection",))
for rid, target, kind in [
    ("T04", 'for «d["a"]» in [t]: @W\n    pass', "loop_target"),
    ("T05", '«d["a"]», other = t, 1 @W', "destructure_target"),
]:
    add(rid, "py", "boundary", kind, "C2", lambda lang: projected_boundary(
        lang, target, 'd["a"]', initial=[container(lang, "a")]), roles=("projection",))

add("N01", "py", "recall", "alias_mutation", "C6", lambda lang: lines(
    lang, container(lang, "a", "b"), write('d["a"]', "t"),
    "e = «d» @R1", 'e["a"] = "safe"', read("d")),
    roles=("argument", "assignment_rhs"), negative=EXACT)
add("N03", "js", "recall", "alias_mutation", "C6", lambda lang: lines(
    lang, container(lang, "x", name="o"), write('o["x"]', "t"),
    "let u", "u = «o» @R1", 'u["x"] = "safe"', read("o")),
    roles=("argument", "assignment_rhs"), negative=EXACT)


def endpoint(access, span):
    return {"access": access, "bytes": span}


def edge(start, end, *labels):
    return {"from": start, "to": end, "labels": labels}


def projected_receiver(case, argument):
    start, end = argument
    return [start, start + case[start:end].index(b"[")]


# A forbidden edge may refer to a deliberately absent or colliding draft node.
# Pin these exceptions explicitly so every other missing endpoint fails closed.
BOUNDARY_ENDPOINT_COUNTS = {
    "P01.go": {(142, 148): 0}, "P01.js": {(140, 146): 0}, "P01.py": {(104, 110): 0},
    "P15.go": {(111, 117): 0}, "P15.js": {(103, 104): 0, (103, 109): 0},
    "P15.py": {(73, 79): 0}, "R07.py": {(91, 92): 2}, "R09.py": {(30, 41): 0},
    "T01.js": {(77, 83): 0, (119, 125): 0}, "T02.js": {(73, 79): 0, (112, 118): 0},
    "T03.js": {(120, 126): 0}, "T04.py": {(54, 60): 0, (94, 100): 0},
    "T05.py": {(83, 89): 0}, "Y12.js": {(125, 131): 0, (133, 137): 0, (159, 160): 0},
    "Y12.py": {(94, 100): 0, (102, 103): 0, (102, 106): 0, (119, 120): 0},
}


def observations(row, case):
    """Keep new assertions separate from unchanged and group-only draft rows."""
    group = row["group"]
    if group == "control" or "markers" in row:
        return None
    ident = row["id"]
    write_def = endpoint("Def", row["w_bytes"])
    marked_read = endpoint("Use", row["r_bytes"])
    sink_use = endpoint("Use", row["sink_bytes"])
    if group.startswith("recall-"):
        # Publication and public-query assertions are activated in I2.
        reads = row.get("reads", [{"role": "argument", "line": row["r_marker_line"],
                                  "bytes": row["r_bytes"]}])
        return dict(verdict="NotReached", reads=reads, edges=[
            edge(write_def, endpoint("Use", read["bytes"]), "nameonly_cfg_incomplete")
            for read in reads
        ] + ([edge(write_def, sink_use)] if ident == "P16.py" else []))

    pin = dict(verdict=row.get("base_verdict", "NotReached"), edges=[])
    edges = pin["edges"]
    existing = row.get("pinned_main", {}).get("route")
    if existing:
        pin["route"] = existing
    if group == "baseline-gap":
        if ident == "A01.js":
            edges.extend((edge(write_def, marked_read, "exact"),
                          edge(marked_read, sink_use, "exact")))
        elif ident == "P26.py":
            pin.update(route="exact", truth="not")
            edges.append(edge(write_def, marked_read, "killed"))
        elif ident == "P30.py":
            pin["through_write"] = False
        elif ident in {"C01.java", "M04.js"}:
            nearby = {
                "C01.java": ("caseFn", "a", [126, 130], [131, 132]),
                "M04.js": ("put", "q", [99, 103], [104, 105]),
            }
            owner, receiver, callee_span, receiver_span = nearby[ident]
            pin["sink_uses"] = [
                dict(owner=owner, path="sink", bytes=callee_span),
                dict(owner=owner, path=receiver, bytes=receiver_span),
            ]
            if ident == "M04.js":
                edges.append(edge(endpoint("Use", row["w_bytes"]),
                                  endpoint("Use", receiver_span), "exact"))
        else:
            edges.append(edge(write_def, marked_read, "exact"))
        return pin

    source = case.encode("utf-8")
    reaches = {"P15", "P28", "P29", "R06", "R07", "T01", "T02", "T03", "T04", "T05"}
    pin["truth"] = "reaches" if ident.split(".")[0] in reaches else "not"
    edges.append(edge(write_def, marked_read))
    if ident.startswith(("P01.", "P15.", "T0")):
        edges.append(edge(write_def, endpoint("Use", projected_receiver(source, row["r_bytes"]))))
    if ident.startswith("Y12."):
        # Store both complete argument spans and their receivers even when the
        # public graph materializes only an inner operand or first receiver.
        start, end = row["r_bytes"]
        first = b'd["b"]'
        first_start = source.rindex(first, 0, start)
        arguments = [[first_start, first_start + len(first)], [start, end]]
        receivers = [projected_receiver(source, arg) for arg in arguments]
        formal_start = source.index(b"f(p, q)") + len(b"f(")
        p_def, q_def = [endpoint("Def", [offset, offset + 1])
                        for offset in (formal_start, formal_start + 3)]
        pin.update(arguments=arguments, receivers=receivers)
        targets = [endpoint("Use", span) for span in [arguments[0], *receivers]]
        edges.extend(edge(write_def, target) for target in [*targets, p_def, q_def, sink_use])
        for receiver in receivers:
            labels = ("exact",) if row["lang"] == "js" and receiver == receivers[1] else ()
            edges.append(edge(endpoint("Use", receiver), q_def, *labels))
            edges.append(edge(endpoint("Use", receiver), p_def))
        formal_label = "exact" if row["lang"] == "js" else "nameonly_cfg_incomplete"
        edges.append(edge(q_def, sink_use, formal_label))
    counts = BOUNDARY_ENDPOINT_COUNTS.get(ident, {})
    for pair in edges:
        if not pair["labels"]:
            for site in (pair["from"], pair["to"]):
                count = counts.get(tuple(site["bytes"]))
                if count is not None:
                    site["node_count"] = count
    return pin


def probe(binary):
    receipts = {}
    for row, case in ROWS:
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
    ROWS.extend(python_q())
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
    with (ROOT / "observations.jsonl").open("w", encoding="utf-8") as output:
        for row, case in rows:
            pin = observations(row, case)
            if pin is not None:
                record = dict(id=row["id"], **pin)
                output.write(json.dumps(record, ensure_ascii=False, separators=(",", ":")) + "\n")


if __name__ == "__main__":
    main()
