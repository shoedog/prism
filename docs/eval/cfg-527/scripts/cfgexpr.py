"""cfg predicate parsing, normalization and three-valued evaluation (#527 census).

A formula is a tuple:
  ("atom", key, value_or_None) | ("all", (f, ...)) | ("any", (f, ...)) | ("not", f)
Normalization (design §2.2): `all` flattened and its members sorted, double `not`
removed; `any` members sorted (not flattened). `cfg_attr(q, cfg(p))` is rendered
`all(q,p)` but evaluated with its real meaning `any(not(q), p)` (see `CfgAttr`).
"""

import re

TOKEN = re.compile(r'\s*(?:(r#)?([A-Za-z_][A-Za-z0-9_]*)|("(?:[^"\\]|\\.)*")|(r#*"[^"]*"#*)|(.))', re.S)


def tokenize(text):
    out = []
    pos = 0
    while pos < len(text):
        m = TOKEN.match(text, pos)
        if not m or m.end() == pos:
            break
        pos = m.end()
        if m.group(2):
            out.append(("id", m.group(2)))
        elif m.group(3):
            out.append(("str", m.group(3)[1:-1]))
        elif m.group(4):
            s = m.group(4)
            out.append(("str", s[s.index('"') + 1:s.rindex('"')]))
        elif m.group(5) and not m.group(5).isspace():
            out.append(("p", m.group(5)))
    return out


class ParseError(Exception):
    pass


def parse_pred(tokens, i=0):
    """Parse one cfg predicate starting at tokens[i]; return (formula, next_i)."""
    if i >= len(tokens) or tokens[i][0] != "id":
        raise ParseError("expected identifier at %d" % i)
    name = tokens[i][1]
    i += 1
    if name in ("all", "any", "not") and i < len(tokens) and tokens[i] == ("p", "("):
        args, i = parse_list(tokens, i + 1)
        if name == "not":
            if len(args) != 1:
                raise ParseError("not() takes one predicate")
            return ("not", args[0]), i
        return (name, tuple(args)), i
    if i < len(tokens) and tokens[i] == ("p", "="):
        if i + 1 >= len(tokens) or tokens[i + 1][0] != "str":
            raise ParseError("expected string after =")
        return ("atom", name, tokens[i + 1][1]), i + 2
    return ("atom", name, None), i


def parse_list(tokens, i):
    args = []
    while i < len(tokens) and tokens[i] != ("p", ")"):
        f, i = parse_pred(tokens, i)
        args.append(f)
        if i < len(tokens) and tokens[i] == ("p", ","):
            i += 1
    if i >= len(tokens):
        raise ParseError("unterminated list")
    return args, i + 1


def split_top_args(tokens):
    """Split a token list on top-level commas; each element is a token list."""
    parts, cur, depth = [], [], 0
    for t in tokens:
        if t[0] == "p" and t[1] in "([{":
            depth += 1
        elif t[0] == "p" and t[1] in ")]}":
            depth -= 1
        if depth == 0 and t == ("p", ","):
            parts.append(cur)
            cur = []
        else:
            cur.append(t)
    if cur:
        parts.append(cur)
    return parts


def norm(f):
    k = f[0]
    if k == "atom":
        return f
    if k == "not":
        inner = norm(f[1])
        if inner[0] == "not":
            return inner[1]
        return ("not", inner)
    members = [norm(x) for x in f[1]]
    if k == "all":
        flat = []
        for m in members:
            if m[0] == "all":
                flat.extend(m[1])
            else:
                flat.append(m)
        members = flat
        uniq = sorted(set(members), key=render)
        if len(uniq) == 1:
            return uniq[0]
        return ("all", tuple(uniq))
    uniq = sorted(set(members), key=render)
    if len(uniq) == 1:
        return uniq[0]
    return ("any", tuple(uniq))


def render(f):
    k = f[0]
    if k == "atom":
        return f[1] if f[2] is None else '%s="%s"' % (f[1], f[2])
    if k == "not":
        return "not(%s)" % render(f[1])
    return "%s(%s)" % (k, ",".join(render(x) for x in f[1]))


def conjuncts(f):
    """Top-level conjuncts of a normalized formula."""
    if f[0] == "all":
        return list(f[1])
    return [f]


def atoms(f):
    if f[0] == "atom":
        yield f
    elif f[0] == "not":
        yield from atoms(f[1])
    else:
        for x in f[1]:
            yield from atoms(x)


def is_empty_any(f):
    return f == ("any", ())


def contains_false(f):
    """True when the formula is statically false (an `any()` conjunct)."""
    return any(is_empty_any(c) for c in conjuncts(f))


class Env:
    """Atom valuation for one configuration: True / False / None (unknown)."""

    def __init__(self, host, features, unknown_atoms=(), test=False, debug_assertions=True):
        self.host = set(host)
        self.features = set(features)
        self.unknown = set(unknown_atoms)
        self.test = test
        self.debug_assertions = debug_assertions

    def atom(self, key, val):
        if key == "feature":
            return val in self.features
        if key == "test":
            return self.test
        if key == "debug_assertions":
            return self.debug_assertions
        if key in self.unknown:
            return None
        return (key, val) in self.host


def ev(f, env):
    k = f[0]
    if k == "atom":
        return env.atom(f[1], f[2])
    if k == "not":
        v = ev(f[1], env)
        return None if v is None else (not v)
    vals = [ev(x, env) for x in f[1]]
    if k == "all":
        if any(v is False for v in vals):
            return False
        return None if any(v is None for v in vals) else True
    if any(v is True for v in vals):
        return True
    return None if any(v is None for v in vals) else False


def parse_host_cfg(text):
    out = set()
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        if "=" in line:
            k, v = line.split("=", 1)
            out.add((k, v.strip('"')))
        else:
            out.add((line, None))
    return out


def atom_family(a):
    key = a[1]
    if key == "test":
        return "test"
    if key == "feature":
        return "feature"
    if key in ("unix", "windows") or key.startswith("target_") or key == "panic":
        return "target"
    if key == "debug_assertions":
        return "debug_assertions"
    return "other"
