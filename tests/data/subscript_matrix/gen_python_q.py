"""Python Q seed programs and the independent DESIGN §7 expectations."""
from hashlib import sha256
import re
from textwrap import dedent, indent

from gen_python_q06 import completion_cases


WEAK = "nameonly_cfg_incomplete"
SRC = "«a:t» = src()"
STORE = '«w:d["a"]» = t'
SINK = "sink(«r/s:d»)"
MARK = re.compile(r"«([a-z0-9/]+):([^»]+)»")


def python_q():
    cases = []

    def add(name, body, *, group="recall-py", present=True, verdict=None, helper="",
            role="argument", loss=None, additions=None, pairs=None, source=SRC, limit=None,
            absent=(), ambiguous=()):
        case_marked = "def case():\n" + indent(
            "\n".join(filter(None, (source, dedent(body).strip("\n")))), "    "
        )
        if helper:
            case_marked += "\n\n" + dedent(helper).strip("\n")
        case, sites = strip_marks(case_marked + "\n")
        encoded = case.encode()

        def typed(key):
            site = sites[key]
            lo, hi = site["bytes"]
            prefix, bracket, _ = encoded[lo:hi].decode().partition("[")
            base, *fields = prefix.split(".")
            return dict(site, path=dict(base=base, fields=fields + (["[]"] if bracket else [])))

        for key in absent:
            sites[key]["node_count"] = 0
        for key in ambiguous:
            sites[key]["node_count"] = 2
        reads = {key: site for key, site in sites.items() if key.startswith("r")}
        mixed = "b" in sites
        if not isinstance(present, tuple):
            present = (present,)
        if verdict is None:
            verdict = "Reached" if group == "recall-py" else "NotReached"
        if isinstance(verdict, str):
            verdict = (verdict,) * len(present)
        if additions is None:
            additions = (group == "recall-py",) * len(present)
        if pairs is None:
            pairs = ([("w", read) for read in reads] if "w" in sites else [],)
        keys = ("a", "b") if mixed else ("a",)
        assert len(keys) == len(present) == len(verdict) == len(additions) == len(pairs), name
        markers = []
        for key, bit, public, add_edge, candidate_pairs in zip(
            keys, present, verdict, additions, pairs
        ):
            expected_pairs = []
            for write, read, *override in candidate_pairs:
                expected = override[0] if override else add_edge
                expected_pairs.append(dict(write=typed(write) if expected else sites[write],
                                           read=typed(read), label=WEAK if expected else None))
            markers.append(dict(
                marker=key.upper(), source=typed(key), present=bit, verdict=public,
                pairs=expected_pairs,
                selected_pair=public == "Reached" and add_edge,
            ))
        ident = f"{name}.py"
        first_read = reads["r"]
        first_write = next((site for key, site in sites.items() if key.startswith("w")), None)
        row = dict(id=ident, group=group, lang="py", file=ident,
                   **{"class": name.split(".")[0]}, census="C3",
                   src_line=sites["a"]["line"], src_bytes=sites["a"]["bytes"],
                   sink_line=sites["s"]["line"], sink_bytes=sites["s"]["bytes"],
                   r_marker_line=first_read["line"], r_bytes=first_read["bytes"],
                   case_sha256=sha256(encoded).hexdigest(), sink=typed("s"),
                   reads=[dict(role=role if isinstance(role, str) else role[key], **read)
                          for key, read in reads.items()],
                   markers=markers)
        if first_write:
            row.update(w_marker_line=first_write["line"], w_bytes=first_write["bytes"])
        if mixed:
            row["truth_protocol"] = "ab"
        if loss:
            row["accepted_loss"] = dict(reason=loss, issue=None)
        if limit:
            row["limit"] = limit
        cases.append((row, case))

    def simple(name, *lines, initial='d = {"a": "", "b": ""}', write=STORE, read=SINK, **options):
        add(name, "\n".join((initial, write, *lines, read)), **options)

    def boundary(name, *lines, loss, **options):
        simple(name, *lines, group="boundary", loss=loss, **options)

    simple("Q01.argument", initial='d = {}; k = "a"', write="«w:d[k]» = t")
    simple("Q01.integer", initial="d = [None]", write="«w:d[0]» = t")
    simple("Q01.keyword", read="take(value=«r:d»)\n", helper="def take(value):\n    sink(«s:value»)")
    simple("Q01.fstring", initial='d = {}; k = "a"', write='«w:d[f"slot-{k}"]» = t')
    simple("Q01.key-effect", write='«w:d[(key := "a")]» = t')
    simple("Q01.assignment", read="e = «r:d»\nsink(«s:e»)", role="assignment_rhs")
    simple("Q01.parenthesized", read="sink((«r/s:d»))")
    add("Q01.return", "x = make(t)\nsink(«s:x»)", role="return", helper="""
        def make(value):
            d = {}
            «w:d["a"]» = value
            return «r:d»
    """)
    boundary("Q01.rebind-index", write='«w:d[(d := {}) or "a"]» = t', present=False,
             verdict="Reached",
             loss="eager_effect_in_target")
    simple("Q01.rebind-rhs", write='«w:d["a"]» = ((d := {}) or t)')

    take = "def take(one, two):\n    sink(«s:two»)"
    simple("Q02.second-formal", "clean = {}", read="take(«r2:clean», «r:d»)", helper=take,
           pairs=([("w", "r"), ("w", "r2", False)],))
    boundary("Q02.first-formal", "clean = {}", read="take(«r:d», clean)",
             helper=take.replace("«s:", "«r2/s:"), present=False, pairs=([("w", "r2")],),
             loss="first formal does not flow to the sink")
    simple("Q02.clean-return", read="v = take(«r:d»)\nsink(«s:v»)", role="argument",
           helper="def take(value):\n    return {}",
           present=False, verdict="NotReached", loss="clean return suppresses caller shortcut")
    simple("Q02.alternate-exact",
           "a1 = t", "a2 = a1", "a3 = a2", "a4 = a3", "a5 = a4",
           read="copy = «r:d»\njoined = (copy, a5)\nsink(«s:joined»)", role="assignment_rhs",
           loss="longer parent route already reaches through independent assignments")

    source_ab = "«a:t» = src_a()\n«b:u» = src_b()"
    for name, left, right, bit_a, bit_b, extra in (
        ("only-a", "d", "e", True, False, ""),
        ("only-b", "e", "d", False, True, ""),
        ("both", "d", "d", True, True, ""),
        ("neither", "e", "e", False, False, ""),
        ("replace-a", "d", "d", False, True, 'd["a"] = "clean"\n'),
    ):
        live_a, live_b = left == "d", right == "d"
        add("Q02." + name,
            f'd = {{}}\ne = {{}}\n«w:{left}["a"]» = t\n«w2:{right}["b"]» = u\n{extra}{SINK}',
            group="recall-py" if live_a or live_b else "boundary", source=source_ab,
            present=(bit_a, bit_b), additions=(live_a, live_b),
            verdict=tuple("Reached" if live else "NotReached" for live in (live_a, live_b)),
            pairs=([("w", "r")], [("w2", "r")]),
            loss="same-slot clean overwrite may leave a weak edge" if name == "replace-a" else None)

    for name, mutation, helper, role in (
        ("O01-mixed", "mutate(«r2:d»)", 'def mutate(value):\n    value["a"] = "clean"', "argument"),
        ("N01-mixed", 'e = «r2:d»\ne["a"] = "clean"', "", "assignment_rhs"),
    ):
        add("Q02." + name, f'd = {{}}\n{STORE}\n«w2:d["b"]» = u\n{mutation}\n{SINK}',
            source=source_ab, present=(False, True), additions=(True, True), helper=helper,
            pairs=([("w", "r"), ("w", "r2")], [("w2", "r"), ("w2", "r2")]),
            role={"r": "argument", "r2": role}, loss="mutation can leave A's stale weak edge")

    for name, expr, presence, extra in (
        ("projected", '«r:d»["a"]', True, ""),
        ("length", "len(«r:d»)", False, ""),
        ("identity", "id(«r:d»)", False, ""),
        ("membership", '"a" in «r:d»', False, ""),
        ("receiver", '«r:d».get("a")', True, ""),
        ("literal-leaf", "[«r:d»]", True, ""),
        ("shadowed-len", "len(«r:d»)", True, "len = identity\n"),
    ):
        add(f"Q03.{name}", f'd = {{}}\n{STORE}\n{extra}x = {expr}\nsink(«s:x»)',
            group="boundary", present=presence, role=name,
            helper="def identity(value):\n    return value" if extra else "",
            loss="excluded consumer operand")

    simple("Q03.parenthesized", read="x = ( «r:d» )\nsink(«s:x»)", role="assignment_rhs")
    boundary("Q03.projected-with-whole", "copy = «r2:d»",
             read='x = «r:d»["a"]\nsink(«s:x»)', pairs=([("w", "r")],),
             role={"r": "projection", "r2": "assignment_rhs"}, loss="projected operand exclusion")
    helper = "def take(p, q):\n    sink(«s:q»)"
    for name, arguments, presence in (
        ("projected-first", '«r:d»["a"], «r2:d»', True),
        ("whole-first", '«r:d», «r2:d»["a"]', True),
        ("duplicate-read", "«r:d», «r2:d»", True),
    ):
        boundary(f"Q04.{name}", read=f"take({arguments})", helper=helper,
                 present=presence, loss="same-line Use key collision", absent=("r2",))
    boundary("Q04.duplicate-write", write=STORE + '; «w2:d["b"]» = t',
             pairs=([("w", "r"), ("w2", "r")],), loss="same-line Def key collision", absent=("w2",))
    boundary("Q04.same-line", write=STORE + "; " + SINK, read="", loss="same-line W and R")
    simple("Q04.different-line")

    box = "class Box:\n    pass"
    setup = "o = Box()\no.items = [[None]]\no.other = {}"
    nested = "«w:o.items[0][0]» = t"
    for name, operand, after, presence, loss in (
        ("prefix", "o.items", (), True, None),
        ("root", "o", (), True, None),
        ("indexed-read", "o.items[0]", (), True, "indexed subcontainer operand"),
        ("other-prefix", "o.other", (), False, "unrelated selector"),
        ("replace-prefix", "o", ("o.items = []",), False, "static prefix replacement"),
        ("write-sibling", "o", ("o.other = []",), True, None),
    ):
        if name == "indexed-read":
            boundary(f"Q05.{name}", initial=setup, write=nested, helper=box,
                     read="x = «r:o.items»[0]\nsink(«s:x»)", role="projection", loss=loss)
        else:
            simple(f"Q05.{name}", *after, initial=setup, write=nested, helper=box,
                   read=f"sink(«r/s:{operand}»)", present=presence, loss=loss,
                   group="boundary" if loss else "recall-py")
    boundary("Q05.field-after-index", initial="o = Box()\no.items = [Box()]",
             write="«w:o.items[0].name» = t", read="sink(«r/s:o»)", helper=box,
             loss="field following an indexed part of the target")

    completion_cases(add)

    for name, program, presence, loss, helper in (
        ("closure", f'd = {{}}\n{STORE}\ndef inner():\n    return «r:d»\nx = inner()\nsink(«s:x»)', True,
         "consumer in a nested callable", ""),
        ("shadowed", f'd = {{}}\n{STORE}\ndef inner():\n    d = {{}}\n    return «r:d»\nx = inner()\nsink(«s:x»)', False,
         "shadowed binding in a nested owner", ""),
        ("global", f'global d\n{STORE}\n{SINK}', True, "global root", "d = {}"),
        ("free", f'd = {{}}\ndef inner():\n    «w:d["a"]» = t\ninner()\n{SINK}', True,
         "captured root", ""),
        ("deferred", f'd = {{}}\n{STORE}\nx = («r:d» for _ in [0])\ny = next(x)\nsink(«s:y»)', True,
         "deferred generator operand", ""),
        ("exceptional", f'd = {{}}\ntry:\n    {STORE}\n    raise ValueError()\nexcept ValueError:\n    {SINK}',
         True, "completion_unproven", ""),
    ):
        colliding = {"free": ("w",), "closure": ("r",), "shadowed": ("r",)}
        add(f"Q07.{name}", program, present=presence, loss=loss, helper=helper, group="boundary",
            ambiguous=colliding.get(name, ()))

    simple("Q08.alias-after", read="e = «r:d»\nsink(«s:e»)", role="assignment_rhs")
    boundary("Q08.alias-before", initial='d = {}; e = d', read="sink(«r/s:e»)",
             loss="pre-established alias is outside the binding")
    simple("Q08.alias-overwrite", 'e["a"] = "clean"', initial='d = {}; e = d', present=False,
           loss="alias mutation can leave stale weak flow")
    simple("Q08.mutator", "mutate(«r2:d»)", helper='def mutate(value):\n    value["a"] = "clean"',
           present=False, loss="opaque mutation can leave stale weak flow")
    simple("Q08.item-override", initial="d = Discard()", present=False,
           helper="class Discard(dict):\n    def __setitem__(self, key, value):\n        pass",
           loss="custom item storage can discard the value")
    simple("Q08.unaffected-sibling", 'e["b"] = "clean"', initial='d = {}; e = d')
    boundary("Q08.name-as-key", read='x = table[«r:d»]\nsink(«s:x»)',
             initial='d = Hashable(); table = {d: "clean"}',
             helper="class Hashable(dict):\n    __hash__ = object.__hash__",
             present=False, loss="storage binding used as an index")

    limit_cases(add)

    boundary("Q12.call-root", write='«w:make()["a"]» = t', helper="def make():\n    return {}",
             present=False, loss="call-result root", absent=("w",))
    boundary("Q12.initializer", initial='d = {"a": t}', write="", loss="no indexed write", verdict="Reached")
    boundary("Q12.slice-read", read="x = «r:d»[:1]\nsink(«s:x»)", initial="d = [None]",
             write="«w:d[0]» = t", role="projection", loss="slice operand")
    boundary("Q12.slice-write", initial="d = [None]", write="«w:d[:1]» = [t]",
             loss="slice assignment target")
    boundary("Q12.extended-slice", initial="d = [None, None, None]", write="«w:d[::2]» = [t, None]",
             loss="extended slice assignment target")
    boundary("Q12.unpack", write='«w:d["a"]», other = t, 0', loss="unpacking target")
    boundary("Q12.loop-head", write='for «w:d["a"]» in [t]:\n    pass', loss="iteration target", absent=("w",))
    boundary("Q12.deferred", read="f = lambda: «r:d»\nx = f()\nsink(«s:x»)",
             loss="whole operand only evaluated inside a deferred callable")

    simple("Q13.same-slot", 'd["a"] = "clean"', present=False, loss="stale same-slot overwrite")
    simple("Q13.other-slot", 'd["b"] = "clean"')
    boundary("Q13.delete-same", 'del d["a"]', present=False, loss="key-blind delete")
    boundary("Q13.delete-sibling", 'del d["b"]', loss="key-blind delete")
    boundary("Q13.clear", "d.clear()", present=False, loss="direct clear")
    simple("Q13.replace-indexed-ancestor", "d[0] = [None]", initial="d = [[None]]",
           write="«w:d[0][0]» = t", present=False, loss="stale indexed ancestor replacement")
    boundary("Q13.detach-child", initial="d = {0: [None]}",
             write="«w:d[0][(d.clear(), 0)[1]]» = t", present=False, loss="eager_effect_in_target")
    simple("Q13.clear-before-commit", initial="d = {0: [None]}",
           write="«w:d[(d.clear(), 0)[1]]» = t")
    return cases


def strip_marks(source):
    sites = {}
    lines = []
    owners = []
    offset = 0
    for number, line in enumerate(source.splitlines(keepends=True), 1):
        if line.strip():
            width = len(line) - len(line.lstrip(" "))
            while owners and width <= owners[-1][0]:
                owners.pop()
            owner = re.match(r"\s*(?:async )?def (\w+)\(", line)
            if owner:
                owners.append((width, owner[1], number))
        plain = ""
        cursor = 0
        for match in MARK.finditer(line):
            plain += line[cursor:match.start()]
            lo = offset + len(plain.encode())
            text = match[2]
            for key in match[1].split("/"):
                assert key not in sites, (key, source)
                _, owner_name, owner_line = owners[-1]
                sites[key] = dict(line=number, bytes=[lo, lo + len(text.encode())],
                                  owner=owner_name, owner_line=owner_line)
            plain += text
            cursor = match.end()
        plain += line[cursor:]
        lines.append(plain)
        offset += len(plain.encode())
    return "".join(lines), sites


def limit_cases(add):
    neighbour = "def independent(value):\n    z = {}\n    «wn:z[0]» = value\n    return «rn:z»"
    for n in (8, 9):
        stores = [STORE] + [f'«w{i}:d[{i}]» = "clean"' for i in range(1, n)]
        keys = ["w", *(f"w{i}" for i in range(1, n))]
        pairs = [(w, "r", n == 8) for w in keys] + [("wn", "rn", True)]
        add(f"Q09.fanin-{n}", "\n".join((
            "d = {}; e = {}", *stores, '«wn:e[0]» = t', "copy = «rn:e»", SINK,
        )), group="limits", pairs=(pairs,),
            additions=(n == 8,), verdict="Reached" if n == 8 else "NotReached",
            limit=dict(kind="read_sources", candidates=n, limit=8,
                       admitted_pairs=n if n == 8 else 0, independent_pairs=1),
            role={"r": "argument", "rn": "assignment_rhs"})
    for n in (256, 257):
        stores = [STORE] + [f'«w{i}:d[{i}]» = "clean"' for i in range(1, 8)]
        reads = [f"copy{i} = «r{i}:d»" for i in range(1, 32)]
        pairs = [(w, r, n == 256) for w in ["w", *(f"w{i}" for i in range(1, 8))]
                 for r in ["r", *(f"r{i}" for i in range(1, 32))]]
        extra = []
        if n == 257:
            extra = ['e = {}', '«we:e[0]» = "clean"', 'extra = «re:e»']
            pairs.append(("we", "re", False))
        add(f"Q10.owner-{n}", "\n".join(("d = {}", "independent(t)", *stores, *reads, *extra, SINK)),
            group="limits", helper=neighbour, pairs=([*pairs, ("wn", "rn", True)],),
            additions=(n == 256,), verdict="Reached" if n == 256 else "NotReached",
            role=dict(r="argument", rn="return", re="assignment_rhs",
                      **{f"r{i}": "assignment_rhs" for i in range(1, 32)}),
            limit=dict(kind="owner_edges", candidates=n, limit=256,
                       admitted_pairs=n if n == 256 else 0, independent_pairs=1))
    for n in (1_000_000, 1_000_001):
        # This compact matrix shape represents 12 or 13 raw pairs at an
        # injected limit of 12. The production arithmetic uses 1M and 1M+1.
        over = n > 1_000_000
        stores = [STORE] + [f'«w{i}:d[{i}]» = "clean"' for i in range(1, 3)]
        reads = [f"copy{i} = «r{i}:d»" for i in range(1, 4)]
        pairs = [(w, r, not over) for w in ("w", "w1", "w2") for r in ("r", "r1", "r2", "r3")]
        extra = []
        if over:
            extra = ['e = {}', '«we:e[0]» = "clean"', 'extra = «re:e»']
            pairs.append(("we", "re", False))
        add(f"Q10.raw-{n}", "\n".join(("d = {}", "independent(t)", *stores, *reads, *extra, SINK)),
            group="limits", helper=neighbour, pairs=([*pairs, ("wn", "rn", True)],),
            additions=(not over,), verdict="NotReached" if over else "Reached",
            role=dict(r="argument", rn="return", re="assignment_rhs",
                      r1="assignment_rhs", r2="assignment_rhs", r3="assignment_rhs"),
            limit=dict(kind="raw_pairs", candidates=n, limit=1_000_000,
                       matrix_candidates=13 if over else 12, matrix_limit=12,
                       raw_refused=over, independent_pairs=1))
    for name, padding in (
        ("def-cap", ["local_a = None", "local_b = None"]),
        ("line-cap", ["noop()", "noop()", "noop()"]),
        ("without-cfg", []),
        ("fallback-cap", ["local_a, local_b = 0, 1"]),
    ):
        reason = "rd_" + name.replace("-", "_")
        add(f"Q11.{name}", "\n".join(("d = {}", STORE, *padding, SINK)), group="limits",
            helper="def noop():\n    pass" if name == "line-cap" else "", loss=reason,
            limit=dict(kind=reason, inject=True, admitted_pairs=0))
