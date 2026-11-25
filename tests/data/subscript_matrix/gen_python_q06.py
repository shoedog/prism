"""Python Q06 phase and compound-completion cases."""
from textwrap import indent


SOURCE = "«a:t» = src()"
WRITE = '«w:d["a"]» = t'
SINK = "sink(«r/s:d»)"

LOOPS = {
    "range": ("for i in range(1):", ()),
    "iterator": ("for i in iter([0]):", ()),
    "while": ("while i < 1:", ("i += 1",)),
}


def block(*lines):
    return indent("\n".join(lines), "    ")


def suite(head, body):
    return "\n".join((head, block(*body)))


def completion_cases(add):
    def simple(name, between, *, before="d = {}", store=WRITE, read=SINK, **options):
        add("Q06." + name, "\n".join((before, store, between, read)), **options)

    def refusal(name, between, *, loss, **options):
        simple(name, between, group="boundary", loss=loss, **options)

    add("Q06.branch-join", f'd = {{}}\nif True:\n    {WRITE}\nelse:\n    d = {{}}\n{SINK}')
    add("Q06.exclusive", f'd = {{}}\nif False:\n    {WRITE}\nelse:\n    {SINK}',
        group="boundary", present=False, loss="exclusive arms")
    add("Q06.early-return", f'x = inner(t)\nsink(«s:x»)', group="boundary", present=False,
        helper=f'''def inner(t):
    d = {{}}
    if True:
        return {{}}
        {WRITE}
    return «r:d»''', role="return", loss="store after early return")
    refusal("root-replace", "d = {}", loss="root rebinding", present=False)
    simple("correlated", f'if flag:\n    d = {{}}', before="d = {}; flag = True",
           present=False, loss="weak path can ignore predicate correlation",
           store=suite("if flag:", (WRITE,)))
    simple("read-before-rebind", "", read="d = take(«r:d»)\nsink(«s:d»)",
           helper="def take(arg):\n    return arg")
    refusal("read-after-rebind", "d = {}", read="d = take(«r:d»)\nsink(«s:d»)",
            helper="def take(arg):\n    return arg", loss="root replaced before argument capture",
            present=False)
    refusal("list-iadd", "d += [None]", before="d = [None]", store="«w:d[0]» = t",
            loss="root compound assignment")
    refusal("dict-ior", 'd |= {"b": None}', loss="root compound assignment")
    refusal("prefix-iadd", "o.items += [None]", before="o = Box()\no.items = [None]",
            store="«w:o.items[0]» = t", read="sink(«r/s:o»)",
            helper="class Box:\n    pass", loss="static prefix compound assignment")
    simple("unrelated-compound", "other = []\nother += [None]")

    loop_cases(add)
    compound_cases(add)

    simple("unsupported-finally", "try:\n    flag = True\nfinally:\n    flag = False",
           group="boundary", loss="completion_unproven")
    simple("unsupported-alternate", "if flag:\n    pass\nelse:\n    try:\n        pass\n    finally:\n        pass",
           before="d = {}; flag = True")
    for preserves in (True, False):
        arm = "spare = 0" if preserves else "d = {}"
        simple(
            "elif-" + ("preserve" if preserves else "replace"),
            f"if False:\n    d = {{}}\nelif True:\n    {arm}\nelse:\n    d = {{}}",
            group="recall-py" if preserves else "boundary", present=preserves,
            loss=None if preserves else "every elif exit rebinds",
        )
    for thrown in (False, True):
        for final in (False, True):
            try_body = "raise ValueError()" if thrown else "flag = True"
            code = f"try:\n    {try_body}\nexcept ValueError:\n    spare = 1\nelse:\n    spare = 2"
            if final:
                code += "\nfinally:\n    spare = 3"
            suffix = ("handled" if thrown else "else") + ("-finally" if final else "")
            refusal("try-" + suffix, code, loss="completion_unproven")
    refusal("with", "with Manager():\n    spare = 0", loss="completion_unproven", helper="""
        class Manager:
            def __enter__(self):
                return self

            def __exit__(self, exc, value, tb):
                return False
    """)
    add("Q06.async-with", "try:\n    run(t).send(None)\nexcept StopIteration:\n    pass", group="boundary",
        loss="completion_unproven", helper=f'''
        class Manager:
            async def __aenter__(self):
                return self

            async def __aexit__(self, exc, value, tb):
                return False

        async def run(t):
            d = {{}}
            {WRITE}
            async with Manager():
                spare = 0
            {SINK}
        ''')
    for name, subject in (("guarded", "[1]"), ("unmatched", "[]")):
        refusal("match-" + name, f"match {subject}:\n    case [item] if item > 0:\n        spare = item",
                loss="completion_unproven")


def loop_cases(add):
    setup = ("d = {}", "i = 0")
    for kind, (head, progress) in LOOPS.items():
        add(f"Q06.loop-{kind}-same-iteration", "\n".join((
            *setup, suite(head, (*progress, WRITE, SINK)),
        )))
        add(f"Q06.loop-{kind}-exhaustion", "\n".join((
            *setup, suite(head, (*progress, WRITE)), SINK,
        )))
        for jump in ("break", "continue"):
            for with_else in (False, True):
                suffix = "-else" if with_else else ""
                code = [suite(head, (*progress, WRITE, jump))]
                if with_else:
                    code.append(suite("else:", ("spare = None",)))
                for position in ("body", "pre-loop"):
                    body = code if position == "pre-loop" else [
                        suite(head, (*progress, SOURCE, WRITE, jump)), *code[1:],
                    ]
                    add(f"Q06.cfg-{kind}-{jump}{suffix}-{position}",
                        "\n".join((*setup, *body, SINK)),
                        source=SOURCE if position == "pre-loop" else "",
                        verdict="Reached" if position == "pre-loop" else "NotReached",
                        loss="public_cfg_stop" if position == "body" else None)
            exhausts = jump == "continue"
            add(f"Q06.loop-{kind}-{jump}-else-replace", "\n".join((
                *setup, suite(head, (*progress, SOURCE, WRITE, jump)),
                suite("else:", ("d = {}",)), SINK,
            )), group="boundary" if exhausts else "recall-py", present=not exhausts, source="",
                verdict="NotReached",
                loss="loop else replaces on exhaustion" if exhausts else "public_cfg_stop")
        add(f"Q06.loop-{kind}-skip-suffix", "\n".join((
            *setup, suite(head, (*progress, WRITE, "continue", "d = {}")), SINK,
        )))
    add("Q06.loop-target-replace", f'd = {{}}\n{WRITE}\nfor d in [{{}}]:\n    pass\n{SINK}',
        group="boundary", present=False, loss="executed loop target rebinds")
    add("Q06.loop-target-skipped", f'd = {{}}\n{WRITE}\nfor d in []:\n    pass\n{SINK}')
    add("Q06.loop-iterable-replace", f'd = {{}}\n{WRITE}\nfor _ in (d := {{}}):\n    pass\n{SINK}',
        group="boundary", present=False, loss="iterable rebinds even at zero iterations")
    add("Q06.loop-test-replace", f'd = {{}}\n{WRITE}\nwhile (d := {{}}):\n    pass\n{SINK}',
        group="boundary", present=False, loss="false while test still replaces")
    add("Q06.loop-nested-jump", "\n".join((
        "d = {}", suite("for i in range(1):", (WRITE, suite("for j in range(1):", ("break",)),
                                               "continue", "d = {}")), SINK,
    )))
    add("Q06.loop-no-exit", "x = inner(t)\nsink(«s:x»)", group="boundary",
        present=False, loss="literal-true loop has no normal exit", role="return",
        helper=f'''def inner(t):
    d = {{}}
    while True:
        {WRITE}
        return {{}}
    return «r:d»''')
    add("Q06.reentry-checked-break", "\n".join((
        *setup, suite("while True:", ("if i:", "    break", WRITE, "i += 1")), SINK,
    )), group="boundary", loss="reentry_required")
    add("Q06.reentry-exclusive", "\n".join((
        *setup, suite("while True:", ("if i:", "    x = «r:d»", "    break",
                                     "else:", "    " + WRITE, "i += 1")), "sink(«s:x»)",
    )), group="boundary", loss="reentry_required", role="assignment_rhs")
    add("Q06.reentry-immediate-break", "\n".join((
        *setup, suite("while True:", (WRITE, "break")), SINK,
    )))


def compound_cases(add):
    for depth in (1, 2):
        for preserve in (False, True):
            leaf = "spare = 1" if preserve else "d = {}"
            compound = "if True:\n    " + leaf + "\nelse:\n    d = {}"
            if depth == 2:
                compound = "if True:\n" + block(compound) + "\nelse:\n    d = {}"
            suffix = f"-{depth}-" + ("preserve" if preserve else "replace")
            options = dict(group="recall-py" if preserve else "boundary", present=preserve,
                           loss=None if preserve else "every normal compound exit rebinds")
            add("Q06.compound-branch" + suffix, "\n".join((
                "d = {}", suite("if True:", (WRITE, compound)), suite("else:", ("d = {}",)), SINK,
            )), **options)
            for form in ("range", "while"):
                head, progress = LOOPS[form]
                setup = ("d = {}", "i = 0")
                add(f"Q06.compound-{form}-body" + suffix, "\n".join((
                    *setup, suite(head, (*progress, WRITE, compound)), SINK,
                )), **options)
                add(f"Q06.compound-{form}-else" + suffix, "\n".join((
                    *setup, suite(head, (*progress, WRITE)), suite("else:", (compound,)), SINK,
                )), **options)
