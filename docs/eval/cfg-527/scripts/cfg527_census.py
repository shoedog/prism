"""#527 cfg census: classify every Rust Exact (site, target) pair by cfg context.

Usage (design §2.1):
  uv run --no-project --with 'tree-sitter==0.25.*' --with tree-sitter-rust==0.24.2 \
    python -I cfg527_census.py --corpus NAME --root DIR --dump FILE.jsonl --out DIR \
    --host-cfg host-cfg.txt [--identity identities.json]
"""

import argparse
import collections
import hashlib
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import cfgexpr as cx  # noqa: E402

try:
    import tomllib
except ImportError:  # Python < 3.11
    tomllib = None

import tree_sitter  # noqa: E402
import tree_sitter_rust  # noqa: E402

PARSER = tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language()))

# Fixed list (design §1): prelude/std trait method names an extern trait could bind.
STD_TRAIT_NAMES = frozenset("""
clone clone_from default from into try_from try_into eq ne cmp partial_cmp lt le gt ge max min clamp
fmt hash next next_back size_hint as_ref as_mut borrow borrow_mut deref deref_mut drop to_string
to_owned extend from_iter into_iter index index_mut add sub mul div rem neg not bitand bitor bitxor
shl shr add_assign sub_assign mul_assign div_assign rem_assign from_str poll call call_mut call_once
write_str write_fmt write flush read len is_empty iter iter_mut get get_mut map collect
""".split())

SKIP_DIRS = {".git", "target", "node_modules", ".venv", "__pycache__"}
SCRIPT_VERSION = "cfg527-census-1"


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def text(node, src):
    return src[node.start_byte:node.end_byte].decode("utf-8", "replace")


# ---------------------------------------------------------------- attributes


class Entry:
    """One availability conjunct: rendered key, origin, evaluation formula."""

    __slots__ = ("key", "origin", "formula", "display")

    def __init__(self, key, origin, formula, display):
        self.key, self.origin, self.formula, self.display = key, origin, formula, display

    def as_json(self):
        return {"cond": self.display, "origin": self.origin}


class AttrInfo:
    def __init__(self):
        self.cfgs = []  # list of (q_formula or None, p_formula)
        self.paths = []  # list of (q_formula or None, path string)
        self.allow_deprecated = False
        self.errors = 0


def _attr_tokens(attr_node, src):
    """Tokens of the attribute body (`cfg(...)`, `path = "..."`)."""
    return cx.tokenize(text(attr_node, src))


def _apply_attr(tokens, q, info):
    if not tokens or tokens[0][0] != "id":
        return
    name = tokens[0][1]
    rest = tokens[1:]
    if name == "cfg" and rest[:1] == [("p", "(")]:
        try:
            f, i = cx.parse_pred(rest, 1)
            info.cfgs.append((q, cx.norm(f)))
        except cx.ParseError:
            info.errors += 1
    elif name == "cfg_attr" and rest[:1] == [("p", "(")] and rest[-1:] == [("p", ")")]:
        parts = cx.split_top_args(rest[1:-1])
        if not parts:
            info.errors += 1
            return
        try:
            cond, _ = cx.parse_pred(parts[0], 0)
        except cx.ParseError:
            info.errors += 1
            return
        cond = cx.norm(cond)
        q2 = cond if q is None else cx.norm(("all", (q, cond)))
        for p in parts[1:]:
            _apply_attr(p, q2, info)
    elif name == "path" and rest[:1] == [("p", "=")] and len(rest) >= 2 and rest[1][0] == "str":
        info.paths.append((q, rest[1][1]))
    elif name == "allow" and any(t == ("id", "deprecated") for t in rest):
        info.allow_deprecated = True


def attrs_of(node, src):
    """Outer attributes: the attribute_item siblings directly preceding `node`."""
    info = AttrInfo()
    sib = node.prev_sibling
    items = []
    while sib is not None and sib.type in ("attribute_item", "line_comment", "block_comment"):
        if sib.type == "attribute_item":
            items.append(sib)
        sib = sib.prev_sibling
    for it in reversed(items):
        for ch in it.named_children:
            if ch.type == "attribute":
                _apply_attr(_attr_tokens(ch, src), None, info)
    return info


def inner_attrs(container, src):
    info = AttrInfo()
    for ch in container.named_children:
        if ch.type == "inner_attribute_item":
            for a in ch.named_children:
                if a.type == "attribute":
                    _apply_attr(_attr_tokens(a, src), None, info)
    return info


def entries_from(info, origin):
    out = []
    for q, p in info.cfgs:
        if q is None:
            for c in cx.conjuncts(p):
                out.append(Entry(cx.render(c), origin, c, cx.render(c)))
        else:
            shown = cx.norm(("all", (q, p)))
            # Real meaning of cfg_attr(q, cfg(p)): available unless q holds and p fails.
            real = ("any", (("not", q), p))
            out.append(Entry("cfg_attr:" + cx.render(shown), origin, real, cx.render(shown)))
    return out


# ---------------------------------------------------------------- type names


def last_type_name(node, src):
    """Last path segment of a type, generics and references stripped."""
    if node is None:
        return None
    t = node.type
    if t in ("type_identifier", "identifier", "primitive_type"):
        return text(node, src)
    if t == "generic_type":
        return last_type_name(node.child_by_field_name("type"), src)
    if t in ("scoped_type_identifier", "scoped_identifier"):
        return last_type_name(node.child_by_field_name("name"), src)
    if t in ("reference_type", "pointer_type"):
        return last_type_name(node.child_by_field_name("type"), src)
    return text(node, src)


def impl_type_params(impl_node, src):
    tp = impl_node.child_by_field_name("type_parameters")
    names = set()
    if tp is not None:
        for ch in tp.named_children:
            if ch.type == "type_identifier":
                names.add(text(ch, src))
            elif ch.type in ("constrained_type_parameter", "type_parameter", "optional_type_parameter"):
                left = ch.child_by_field_name("left") or ch.child_by_field_name("name")
                if left is None and ch.named_children:
                    left = ch.named_children[0]
                if left is not None:
                    names.add(text(left, src))
    return names


# ---------------------------------------------------------------- corpus model


class Fn:
    __slots__ = ("file", "name", "start", "end", "ctx", "kind", "owner", "trait", "blanket",
                 "module", "package", "ret", "allow_dep", "impl_ctx", "impl_id")

    def fid(self):
        return {"file": self.file, "name": self.name, "start_line": self.start, "end_line": self.end}

    def key(self):
        return (self.file, self.name, self.start)


class Impl:
    __slots__ = ("file", "line", "trait", "self_name", "blanket", "ctx", "fn_names")


class Corpus:
    def __init__(self, root):
        self.root = os.path.abspath(root)
        self.packages = {}  # manifest dir (rel) -> dict
        self.fns = []
        self.impls = []
        self.trait_decls = collections.defaultdict(list)  # trait name -> [Fn]
        self.file_ctx = {}  # rel file -> list of contexts (first is used)
        self.file_parent_unknown = set()
        self.parsed = {}  # rel -> (tree, src)
        self.error_files = set()
        self.cfg_if_files = set()
        self.attr_errors = 0
        self.modpath = {}  # module key -> (crate root rel, tuple path)
        self.globs = collections.defaultdict(list)  # module key -> [segments]
        self.visited = set()

    # ---- files

    def rel(self, path):
        return os.path.relpath(os.path.abspath(path), self.root).replace(os.sep, "/")

    def parse(self, rel):
        if rel in self.parsed:
            return self.parsed[rel]
        path = os.path.join(self.root, rel)
        try:
            with open(path, "rb") as f:
                src = f.read()
        except OSError:
            self.parsed[rel] = None
            return None
        tree = PARSER.parse(src)
        if tree.root_node.has_error:
            self.error_files.add(rel)
        if b"cfg_if!" in src or b"cfg_if::cfg_if!" in src:
            self.cfg_if_files.add(rel)
        self.parsed[rel] = (tree, src)
        return self.parsed[rel]

    def package_of(self, rel):
        d = os.path.dirname(rel)
        while True:
            if d in self.packages and self.packages[d].get("name"):
                return d
            if not d:
                return None
            d = os.path.dirname(d)

    # ---- packages

    def discover_packages(self):
        for dirpath, dirnames, filenames in os.walk(self.root):
            dirnames[:] = sorted(d for d in dirnames if d not in SKIP_DIRS)
            if "Cargo.toml" not in filenames:
                continue
            rel = self.rel(dirpath)
            rel = "" if rel == "." else rel
            man = os.path.join(dirpath, "Cargo.toml")
            data = load_toml(man)
            pkg = data.get("package") or {}
            entry = {"name": pkg.get("name"), "features": data.get("features") or {},
                     "optional": optional_deps(data), "roots": [], "build_cfgs": set(),
                     "build_dynamic": False}
            if pkg.get("name"):
                entry["roots"] = crate_roots(dirpath, data, rel)
                scan_build_rs(dirpath, pkg, entry)
            self.packages[rel] = entry

    # ---- module tree

    def walk_crates(self):
        for prel in sorted(self.packages):
            for root in self.packages[prel]["roots"]:
                self.load_file(root, [], (root, ()), is_modrs=True)

    def load_file(self, rel, ctx, modkey, is_modrs, parent_unknown=False):
        tag = (rel, modkey[0])
        if tag in self.visited:
            return
        self.visited.add(tag)
        parsed = self.parse(rel)
        if parsed is None:
            return
        tree, src = parsed
        info = inner_attrs(tree.root_node, src)
        self.attr_errors += info.errors
        fctx = ctx + entries_from(info, "file_inner")
        self.file_ctx.setdefault(rel, []).append(fctx)
        if parent_unknown:
            self.file_parent_unknown.add(rel)
        crate_root, mpath = modkey
        mkey = (rel, ())
        self.modpath.setdefault(mkey, (crate_root, mpath))
        pkg = self.package_of(rel)
        base = os.path.dirname(rel)
        if not is_modrs:
            stem = os.path.splitext(os.path.basename(rel))[0]
            base = os.path.join(base, stem) if base else stem
        state = {"file": rel, "src": src, "pkg": pkg, "crate_root": crate_root, "is_modrs": is_modrs,
                 "file_dir": os.path.dirname(rel), "allow_dep": info.allow_deprecated}
        self.walk(tree.root_node, state, fctx, (), mpath, base, None, info.allow_deprecated)

    def walk(self, node, st, ctx, inline, mpath, base, impl, allow_dep):
        src = st["src"]
        for ch in node.named_children:
            t = ch.type
            if t in ("function_item", "function_signature_item"):
                info = attrs_of(ch, src)
                self.attr_errors += info.errors
                fctx = ctx + entries_from(info, "item")
                self.add_fn(ch, st, fctx, inline, impl, allow_dep or info.allow_deprecated)
                body = ch.child_by_field_name("body")
                if body is not None:
                    nested = ctx + entries_from(info, "mod_inline")
                    self.walk(body, st, nested, inline, mpath, base, None,
                              allow_dep or info.allow_deprecated)
            elif t == "impl_item":
                info = attrs_of(ch, src)
                self.attr_errors += info.errors
                ictx = ctx + entries_from(info, "impl")
                im = Impl()
                im.file = st["file"]
                im.line = ch.start_point[0] + 1
                tr = ch.child_by_field_name("trait")
                im.trait = last_type_name(tr, src) if tr is not None else None
                ty = ch.child_by_field_name("type")
                im.self_name = last_type_name(ty, src)
                params = impl_type_params(ch, src)
                inner_ty = ty
                while inner_ty is not None and inner_ty.type in ("reference_type", "pointer_type"):
                    inner_ty = inner_ty.child_by_field_name("type")
                im.blanket = inner_ty is not None and inner_ty.type == "type_identifier" and \
                    text(inner_ty, src) in params
                im.ctx = ictx
                im.fn_names = {}
                self.impls.append(im)
                body = ch.child_by_field_name("body")
                if body is not None:
                    self.walk(body, st, ictx, inline, mpath, base, ("impl", im),
                              allow_dep or info.allow_deprecated)
            elif t == "trait_item":
                info = attrs_of(ch, src)
                self.attr_errors += info.errors
                tctx = ctx + entries_from(info, "trait")
                name = ch.child_by_field_name("name")
                body = ch.child_by_field_name("body")
                if body is not None:
                    self.walk(body, st, tctx, inline, mpath, base, ("trait", text(name, src)),
                              allow_dep or info.allow_deprecated)
            elif t == "mod_item":
                info = attrs_of(ch, src)
                self.attr_errors += info.errors
                name = text(ch.child_by_field_name("name"), src)
                body = ch.child_by_field_name("body")
                if body is not None:
                    inner = inner_attrs(body, src)
                    self.attr_errors += inner.errors
                    mctx = ctx + entries_from(info, "mod_inline") + entries_from(inner, "mod_inline")
                    ninline = inline + (name,)
                    nmpath = mpath + (name,)
                    self.modpath.setdefault((st["file"], ninline), (st["crate_root"], nmpath))
                    self.walk(body, st, mctx, ninline, nmpath, os.path.join(base, name) if base else name,
                              None, allow_dep or info.allow_deprecated or inner.allow_deprecated)
                else:
                    self.load_mod_decl(ch, name, info, st, ctx, inline, mpath, base)
            elif t == "use_declaration":
                self.record_glob(ch, st, inline)
            elif t in ("declaration_list", "block", "expression_statement", "const_item", "static_item",
                       "foreign_mod_item"):
                self.walk(ch, st, ctx, inline, mpath, base, impl, allow_dep)

    def record_glob(self, node, st, inline):
        src = st["src"]
        arg = node.child_by_field_name("argument")
        if arg is None or arg.type != "use_wildcard":
            return
        segs = [s for s in re.split(r"::", text(arg, src).replace(" ", "")) if s and s != "*"]
        self.globs[(st["file"], inline)].append(segs)

    def load_mod_decl(self, node, name, info, st, ctx, inline, mpath, base):
        mctx = ctx + entries_from(info, "mod_decl")
        nmpath = mpath + (name,)
        plain_paths = [p for q, p in info.paths if q is None]
        cond_paths = [(q, p) for q, p in info.paths if q is not None]
        if inline and st["is_modrs"]:
            path_base = os.path.join(st["file_dir"], *inline)
        elif inline:
            path_base = base
        else:
            path_base = st["file_dir"]
        if plain_paths:
            child = os.path.normpath(os.path.join(path_base, plain_paths[-1])).replace(os.sep, "/")
            self.load_file(child, mctx, (st["crate_root"], nmpath), is_modrs=True)
            return
        default = None
        for cand in (os.path.join(base, name + ".rs"), os.path.join(base, name, "mod.rs")):
            cand = os.path.normpath(cand).replace(os.sep, "/")
            if os.path.isfile(os.path.join(self.root, cand)):
                default = cand
                break
        neg = []
        for q, p in cond_paths:
            child = os.path.normpath(os.path.join(path_base, p)).replace(os.sep, "/")
            e = Entry(cx.render(q), "path_alt", q, cx.render(q))
            self.load_file(child, mctx + [e], (st["crate_root"], nmpath), is_modrs=True)
            nq = cx.norm(("not", q))
            neg.append(Entry(cx.render(nq), "path_alt", nq, cx.render(nq)))
        if default is not None:
            self.load_file(default, mctx + neg, (st["crate_root"], nmpath),
                           is_modrs=default.endswith("/mod.rs") or default == "mod.rs")

    def add_fn(self, node, st, ctx, inline, impl, allow_dep):
        src = st["src"]
        f = Fn()
        f.file = st["file"]
        f.name = text(node.child_by_field_name("name"), src)
        f.start = node.start_point[0] + 1
        f.end = node.end_point[0] + 1
        f.ctx = ctx
        f.module = (st["file"], inline)
        f.package = st["pkg"]
        rt = node.child_by_field_name("return_type")
        f.ret = re.sub(r"\s+", "", text(rt, src)) if rt is not None else "()"
        f.allow_dep = allow_dep
        f.owner = f.trait = None
        f.blanket = False
        f.impl_ctx = None
        f.impl_id = None
        if impl is None:
            f.kind = "free"
        elif impl[0] == "trait":
            f.kind = "trait_decl"
            f.owner = f.trait = impl[1]
            self.trait_decls[impl[1]].append(f)
        else:
            im = impl[1]
            f.kind = "trait_impl" if im.trait else "inherent"
            f.owner = im.self_name
            f.trait = im.trait
            f.blanket = im.blanket
            f.impl_ctx = im.ctx
            f.impl_id = (im.file, im.line)
            im.fn_names[f.name] = f
        self.fns.append(f)

    def load_orphans(self, files):
        for rel in sorted(files):
            if rel in self.file_ctx or not rel.endswith(".rs"):
                continue
            self.load_file(rel, [], (rel, ()), is_modrs=True, parent_unknown=True)


def load_toml(path):
    with open(path, "rb") as f:
        raw = f.read()
    if tomllib is not None:
        try:
            return tomllib.loads(raw.decode("utf-8", "replace"))
        except Exception:
            pass
    # Minimal fallback: [package] name only.
    m = re.search(rb'\[package\][^\[]*?\bname\s*=\s*"([^"]+)"', raw, re.S)
    return {"package": {"name": m.group(1).decode()}} if m else {}


def optional_deps(data):
    out = set()
    for sect in ("dependencies", "build-dependencies"):
        for k, v in (data.get(sect) or {}).items():
            if isinstance(v, dict) and v.get("optional"):
                out.add(k)
    for tgt in (data.get("target") or {}).values():
        for k, v in (tgt.get("dependencies") or {}).items():
            if isinstance(v, dict) and v.get("optional"):
                out.add(k)
    return out


def crate_roots(dirpath, data, rel):
    def r(p):
        p = os.path.normpath(os.path.join(rel, p)) if rel else os.path.normpath(p)
        return p.replace(os.sep, "/")

    roots = []
    lib = data.get("lib") or {}
    roots.append(lib.get("path", "src/lib.rs"))
    roots.append("src/main.rs")
    roots.append(data.get("package", {}).get("build", "build.rs") or "build.rs")
    for b in data.get("bin") or []:
        if b.get("path"):
            roots.append(b["path"])
    for sect in ("test", "bench", "example"):
        for b in data.get(sect) or []:
            if b.get("path"):
                roots.append(b["path"])
    for sub in ("src/bin", "tests", "benches", "examples"):
        d = os.path.join(dirpath, sub)
        if os.path.isdir(d):
            for name in sorted(os.listdir(d)):
                full = os.path.join(d, name)
                if name.endswith(".rs") and os.path.isfile(full):
                    roots.append(os.path.join(sub, name))
                elif os.path.isdir(full) and os.path.isfile(os.path.join(full, "main.rs")):
                    roots.append(os.path.join(sub, name, "main.rs"))
    out = []
    for p in roots:
        if isinstance(p, str) and os.path.isfile(os.path.join(dirpath, p)):
            q = r(p)
            if q not in out:
                out.append(q)
    return out


def scan_build_rs(dirpath, pkg, entry):
    b = pkg.get("build", "build.rs")
    if not isinstance(b, str):
        return
    path = os.path.join(dirpath, b)
    if not os.path.isfile(path):
        return
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        s = f.read()
    for m in re.finditer(r"rustc-cfg=([A-Za-z_][A-Za-z0-9_]*)", s):
        entry["build_cfgs"].add(m.group(1))
    if re.search(r"rustc-cfg=\{|autocfg|emit_rustc_cfg|\.emit\(", s):
        entry["build_dynamic"] = True


def default_features(pkg):
    feats = pkg["features"]
    implicit = set(pkg["optional"])
    for vals in feats.values():
        for v in vals:
            if isinstance(v, str) and v.startswith("dep:"):
                implicit.discard(v[4:])
    on = set()
    stack = list(feats.get("default", []))
    while stack:
        v = stack.pop()
        if not isinstance(v, str) or v.startswith("dep:"):
            continue
        if "/" in v:
            dep, _ = v.split("/", 1)
            if dep.endswith("?"):
                continue
            v = dep
        if v in on:
            continue
        if v in feats or v in implicit:
            on.add(v)
            stack.extend(feats.get(v, []))
    return on


# ---------------------------------------------------------------- classification


def ctx_keys(ctx):
    return frozenset(e.key for e in ctx)


def ctx_json(ctx):
    return [e.as_json() for e in ctx]


def ctx_eval(ctx, env):
    return cx.ev(("all", tuple(e.formula for e in ctx)), env) if ctx else True


def statically_false(ctx):
    """'always', 'under:<q>' (cfg_attr(q, cfg(any()))), or None."""
    under = []
    for e in ctx:
        f = e.formula
        if e.key.startswith("cfg_attr:"):
            q, p = f[1][0][1], f[1][1]
            if cx.contains_false(p):
                under.append(cx.render(q))
        elif cx.is_empty_any(f) or cx.contains_false(f):
            return "always"
    return ("under:" + "&".join(sorted(under))) if under else None


def find_fn(index, by_file, fid):
    if fid is None:
        return None, "none"
    hit = index.get((fid["file"], fid["name"], fid["start_line"]))
    if hit is not None:
        return hit, "exact"
    best = None
    for f in by_file.get(fid["file"], ()):
        if f.name == fid["name"] and f.start <= fid["start_line"] <= f.end:
            if best is None or (f.end - f.start) < (best.end - best.start):
                best = f
    return best, ("containment" if best else "unmapped")


def enclosing_fn(by_file, file, line):
    best = None
    for f in by_file.get(file, ()):
        if f.start <= line <= f.end and (best is None or (f.end - f.start) < (best.end - best.start)):
            best = f
    return best


def glob_modules(corpus, modkey):
    """Module keys glob-imported (`use p::*`) into module `modkey`."""
    out = set()
    crate_root, mpath = corpus.modpath.get(modkey, (None, None))
    if mpath is None:
        return out
    for segs in corpus.globs.get(modkey, ()):
        if not segs:
            continue
        cur = list(mpath)
        s = list(segs)
        anchored = False
        if s[0] == "crate":
            cur, s, anchored = [], s[1:], True
        while s and s[0] in ("super", "self"):
            if s[0] == "super" and cur:
                cur.pop()
            s = s[1:]
            anchored = True
        want = tuple(cur + s) if anchored else None
        for k, (cr, mp) in corpus.modpath.items():
            if cr != crate_root:
                continue
            if anchored and mp == want:
                out.add(k)
            elif not anchored and s and len(mp) >= len(s) and mp[-len(s):] == tuple(s):
                out.add(k)
    return out


def alternatives(corpus, tgt, caller_mod, idx):
    """Same-named fns whose availability could compete with `tgt` (design §1 relations)."""
    tkeys = ctx_keys(tgt.ctx)
    alts = []

    def add(f, rel, ctx):
        if f is tgt:
            return
        if ctx_keys(ctx) == tkeys:
            return
        alts.append((f, rel, ctx))

    same_name = idx["by_name"].get(tgt.name, ())
    if tgt.kind in ("inherent", "trait_impl") and tgt.owner:
        for f in same_name:
            if f.kind == "inherent" and f.owner == tgt.owner:
                add(f, "same_owner_inherent", f.ctx)
        for trait, decls in corpus.trait_decls.items():
            decl = next((d for d in decls if d.name == tgt.name), None)
            if decl is None:
                continue
            for im in idx["impls_by_trait"].get(trait, ()):
                if not (im.self_name == tgt.owner or im.blanket):
                    continue
                f = im.fn_names.get(tgt.name, decl)
                if f is tgt:
                    continue
                ctx = list(im.ctx) + [e for e in f.ctx if e.key not in ctx_keys(im.ctx)]
                add(f, "trait_reachable", ctx)
    if tgt.kind == "free":
        mods = {tgt.module}
        if caller_mod is not None:
            mods.add(caller_mod)
            mods |= glob_modules(corpus, caller_mod)
        mods |= glob_modules(corpus, tgt.module)
        for f in same_name:
            if f.kind != "free" or f is tgt:
                continue
            if f.module in mods:
                add(f, "same_module_free", f.ctx)
            elif f.package == tgt.package:
                add(f, "crate_name_free", f.ctx)
    # De-duplicate (an fn can be reached by several impls with the same context).
    seen, out = set(), []
    for f, rel, ctx in alts:
        k = (f.key(), rel, ctx_keys(ctx))
        if k not in seen:
            seen.add(k)
            out.append((f, rel, ctx))
    return out


def classify(tgt, tctx, cctx, alts, caller_mod):
    t_nonempty = bool(tctx)
    implied = t_nonempty and ctx_keys(tctx) <= ctx_keys(cctx)
    danger = [a for a in alts if a[1] in ("same_owner_inherent", "trait_reachable", "same_module_free")]
    if t_nonempty:
        if danger:
            if any(a[1] in ("same_owner_inherent", "same_module_free") for a in danger):
                return "a1", implied
            return "a2", implied
        return ("b-std" if tgt.name in STD_TRAIT_NAMES else "b-other"), implied
    # Empty T: a cfg'd alternative that would outrank the target when enabled.
    for f, rel, ctx in alts:
        if not ctx:
            continue
        if tgt.kind in ("trait_impl", "trait_decl") and rel == "same_owner_inherent":
            return "c", False
        if tgt.kind == "free" and rel == "same_module_free" and caller_mod is not None \
                and f.module == caller_mod and tgt.module != caller_mod:
            return "c", False
    return "none", False


def is_fixture(path):
    return "/fixtures/" in "/" + path


def site_id(site, fid):
    sp = site["source_span"]
    raw = "%s|%s|%s|%s|%s|%s|%s" % (sp["file"], sp["start_byte"], sp["end_byte"], site["callee_text"],
                                   fid["file"], fid["name"], fid["start_line"])
    return hashlib.sha256(raw.encode()).hexdigest()


def run(args):
    with open(args.host_cfg) as f:
        host = cx.parse_host_cfg(f.read())
    corpus = Corpus(args.root)
    corpus.discover_packages()
    corpus.walk_crates()

    rows = []
    dump_files = set()
    with open(args.dump) as f:
        for line in f:
            r = json.loads(line)
            if r.get("record_kind") != "call_site":
                continue
            rows.append(r)
            dump_files.add(r["caller"]["file"])
            for t in r.get("resolved_targets") or ():
                dump_files.add(t["function_id"]["file"])
    corpus.load_orphans(dump_files)

    index, by_file, by_name = {}, collections.defaultdict(list), collections.defaultdict(list)
    for fn in corpus.fns:
        index.setdefault(fn.key(), fn)
        by_file[fn.file].append(fn)
        by_name[fn.name].append(fn)
    impls_by_trait = collections.defaultdict(list)
    for im in corpus.impls:
        if im.trait:
            impls_by_trait[im.trait].append(im)
    idx = {"by_name": by_name, "impls_by_trait": impls_by_trait}

    envs = {}

    def env_for(pkg):
        if pkg not in envs:
            p = corpus.packages.get(pkg)
            if p is None:
                envs[pkg] = cx.Env(host, ())
            else:
                unknown = set(p["build_cfgs"])
                envs[pkg] = cx.Env(host, default_features(p), unknown)
                envs[pkg].dynamic = p["build_dynamic"]
        return envs[pkg]

    def fam(atom, pkg):
        p = corpus.packages.get(pkg) or {}
        if atom[1] in p.get("build_cfgs", ()):
            return "build_rs"
        return cx.atom_family(atom)

    out_rows = []
    map_kinds = collections.Counter()
    for r in rows:
        for t in r.get("resolved_targets") or ():
            fid = t["function_id"]
            if t.get("confidence") != "exact" or not fid["file"].endswith(".rs"):
                continue
            tgt, how = find_fn(index, by_file, fid)
            map_kinds[how] += 1
            caller, chow = find_fn(index, by_file, r["caller"])
            if caller is None:
                caller = enclosing_fn(by_file, r["caller"]["file"], r["source_span"]["line"])
            caller_file = r["caller"]["file"]
            if caller is not None:
                cctx, caller_mod = caller.ctx, caller.module
            else:
                cctx = (corpus.file_ctx.get(caller_file) or [[]])[0]
                caller_mod = (caller_file, ())
            row = {"site_id": site_id(r, fid), "corpus": args.corpus, "rung": t["kind"],
                   "caller": r["caller"], "target": fid, "span": r["source_span"],
                   "callee_text": r["callee_text"], "map": how, "caller_map": chow,
                   "fixture": is_fixture(fid["file"]) or is_fixture(caller_file)}
            if tgt is None:
                row["class"] = "unmapped"
                out_rows.append(row)
                continue
            alts = alternatives(corpus, tgt, caller_mod, idx)
            cls, implied = classify(tgt, tgt.ctx, cctx, alts, caller_mod)
            env = env_for(tgt.package)
            sf = statically_false(tgt.ctx)
            row.update({
                "T": ctx_json(tgt.ctx), "C": ctx_json(cctx), "implied": implied, "class": cls,
                "target_kind": tgt.kind, "target_owner": tgt.owner, "target_trait": tgt.trait,
                "alternatives": [{"fid": f.fid(), "relation": rel, "ctx": ctx_json(ctx),
                                  "c0_eval": ctx_eval(ctx, env_for(f.package)), "kind": f.kind,
                                  "return_type_text": f.ret, "allow_deprecated": f.allow_dep}
                                 for f, rel, ctx in alts],
                "statically_false": sf, "t_c0_eval": ctx_eval(tgt.ctx, env),
                "return_type_text": tgt.ret, "allow_deprecated": tgt.allow_dep,
                "package": (corpus.packages.get(tgt.package) or {}).get("name"),
                "package_dir": tgt.package,
                "caller_package": (corpus.packages.get(caller.package) or {}).get("name") if caller else None,
                "parent_unknown": tgt.file in corpus.file_parent_unknown,
                "atom_families": sorted({fam(a, tgt.package) for e in tgt.ctx for a in cx.atoms(e.formula)}),
            })
            out_rows.append(row)

    out_rows.sort(key=lambda x: x["site_id"])
    os.makedirs(args.out, exist_ok=True)
    pairs_path = os.path.join(args.out, "%s-pairs.jsonl" % args.corpus)
    with open(pairs_path, "w") as f:
        for row in out_rows:
            f.write(json.dumps(row, sort_keys=True) + "\n")

    summary = summarize(args, corpus, out_rows, map_kinds)
    summary["inputs"] = {"dump_sha256": sha256_file(args.dump), "host_cfg_sha256": sha256_file(args.host_cfg),
                         "script_sha256": {n: sha256_file(os.path.join(os.path.dirname(os.path.abspath(__file__)), n))
                                           for n in ("cfg527_census.py", "cfgexpr.py")},
                         "pairs_sha256": sha256_file(pairs_path)}
    if args.identity:
        with open(args.identity) as f:
            summary["identity"] = json.load(f).get(args.corpus)
    with open(os.path.join(args.out, "%s-summary.json" % args.corpus), "w") as f:
        json.dump(summary, f, indent=1, sort_keys=True)
    return summary


CLASSES = ("none", "a1", "a2", "b-std", "b-other", "c", "unmapped")


def summarize(args, corpus, rows, map_kinds):
    E = len(rows)
    mapped = sum(1 for r in rows if r["class"] != "unmapped")

    def block(rs):
        b = {"E": len(rs), "G": sum(1 for r in rs if r.get("T")), "classes": {},
             "statically_false": sum(1 for r in rs if r.get("statically_false")),
             "statically_false_always": sum(1 for r in rs if r.get("statically_false") == "always")}
        for c in CLASSES:
            sel = [r for r in rs if r["class"] == c]
            b["classes"][c] = {"implied": sum(1 for r in sel if r.get("implied")),
                               "not_implied": sum(1 for r in sel if not r.get("implied"))}
        a_ni = [r for r in rs if r["class"] in ("a1", "a2") and not r.get("implied")]
        b["A"] = len(a_ni)
        b["F1_cost"] = sum(1 for r in rs if r["class"] in ("a1", "a2", "b-std", "b-other") and not r.get("implied"))
        b["F2_cost"] = b["A"]
        b["F2_cost_with_c"] = b["A"] + sum(1 for r in rs if r["class"] == "c")
        b["F3_cost"] = sum(1 for r in rs if r.get("T") and r.get("t_c0_eval") is not True)
        b["F3_false"] = sum(1 for r in rs if r.get("T") and r.get("t_c0_eval") is False)
        b["F3_unknown"] = sum(1 for r in rs if r.get("T") and r.get("t_c0_eval") is None)
        origins, families = collections.Counter(), collections.Counter()
        for r in rs:
            for o in sorted({e["origin"] for e in r.get("T") or ()}):
                origins[o] += 1
            for fm in r.get("atom_families") or ():
                families[fm] += 1
        b["origin_hist"] = dict(sorted(origins.items()))
        b["atom_family_hist"] = dict(sorted(families.items()))
        return b

    nonfix = [r for r in rows if not r["fixture"]]
    fix = [r for r in rows if r["fixture"]]
    by_rung = collections.defaultdict(list)
    for r in nonfix:
        by_rung[r["rung"]].append(r)
    s = {"corpus": args.corpus, "script_version": SCRIPT_VERSION,
         "rust_exact_pairs": E, "mapped": mapped,
         "map_rate": (mapped / E) if E else 1.0,
         "map_kinds": dict(map_kinds),
         "admissible": (mapped / E if E else 1.0) >= 0.995,
         "parent_unknown_files": len(corpus.file_parent_unknown),
         "parent_unknown_pairs": sum(1 for r in rows if r.get("parent_unknown")),
         "unparsed_files": len(corpus.error_files),
         "cfg_if_files": len(corpus.cfg_if_files),
         "attr_parse_errors": corpus.attr_errors,
         "files_parsed": sum(1 for v in corpus.parsed.values() if v),
         "packages": sum(1 for p in corpus.packages.values() if p.get("name")),
         "nonfixture": block(nonfix), "fixture": block(fix),
         "by_rung_nonfixture": {k: block(v) for k, v in sorted(by_rung.items())}}
    return s


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--root", required=True)
    ap.add_argument("--dump", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--host-cfg", required=True)
    ap.add_argument("--identity")
    s = run(ap.parse_args(argv))
    print(json.dumps({k: s[k] for k in ("corpus", "rust_exact_pairs", "map_rate", "admissible")}
                     | {"A": s["nonfixture"]["A"], "G": s["nonfixture"]["G"]}))


if __name__ == "__main__":
    main()
