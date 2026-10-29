"""#527 adjudication (design §3): rustc decides which callable each sampled site binds.

Runs on a disposable copy of one corpus. `cargo check` only (build scripts and proc
macros run; no corpus binary or test is executed). Per configuration:
  1. baseline: unmodified copy, must be green;
  2. marker:   `#[deprecated(note = "p527:<id>")]` on the target and on every in-repo
               alternative (trait-impl methods: on their in-repo trait declaration);
               the note on a site's `deprecated` warning names the callable rustc bound;
  3. e0308:    each site's call replaced by `{ let _p527: [(); 527] = CALL; unreachable!() }`;
               the E0308 `found T` proves the site compiles in this configuration and names
               the return type rustc bound. Batched per (package, src|tests) round.
Writes <out>/<corpus>-verdicts.jsonl and <out>/<corpus>-adjudication.json.
"""

import argparse
import collections
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import cfg527_census as census  # noqa: E402

E0308_PRE = "{ let _p527: [(); 527] = "
E0308_POST = "; unreachable!() }"
CAP = "--cap-lints=warn"


def sh(cmd, cwd, env, log, timeout):
    t0 = time.time()
    with open(log, "w") as f:
        try:
            p = subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=f, timeout=timeout)
            out, code = p.stdout.decode("utf-8", "replace"), p.returncode
        except subprocess.TimeoutExpired as e:
            out, code = (e.stdout or b"").decode("utf-8", "replace"), "timeout"
    return out, code, time.time() - t0


def messages(out):
    msgs, success = [], None
    for line in out.splitlines():
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if m.get("reason") == "compiler-message":
            msgs.append(m["message"])
        elif m.get("reason") == "build-finished":
            success = m.get("success")
    return msgs, success


class Edits:
    """Per-file byte insertions; maps probe offsets back to original offsets."""

    def __init__(self):
        self.ins = collections.defaultdict(list)  # rel -> [(orig_pos, text_bytes)]

    def add(self, rel, pos, txt):
        self.ins[rel].append((pos, txt.encode()))

    def apply(self, root, orig_root):
        for rel, items in self.ins.items():
            with open(os.path.join(orig_root, rel), "rb") as f:
                src = f.read()
            out, last = [], 0
            for pos, b in sorted(items, key=lambda x: x[0]):
                out.append(src[last:pos])
                out.append(b)
                last = pos
            out.append(src[last:])
            with open(os.path.join(root, rel), "wb") as f:
                f.write(b"".join(out))

    def restore(self, root, orig_root):
        for rel in self.ins:
            shutil.copy2(os.path.join(orig_root, rel), os.path.join(root, rel))

    def to_orig(self, rel, b):
        cum = 0
        for pos, txt in sorted(self.ins.get(rel, ()), key=lambda x: x[0]):
            if b >= pos + cum + len(txt):
                cum += len(txt)
            elif b >= pos + cum:
                return pos  # inside an insertion
            else:
                break
        return b - cum


def norm_file(fname, root):
    if os.path.isabs(fname):
        fname = os.path.relpath(fname, root)
    return os.path.normpath(fname).replace(os.sep, "/")


def fn_id_hash(fid):
    return hashlib.sha256(("%s|%s|%s" % (fid["file"], fid["name"], fid["start_line"])).encode()).hexdigest()[:12]


def norm_type(t):
    t = re.sub(r"\s+", "", t or "")
    t = t.strip("`")
    t = re.sub(r"'\w+", "", t)  # lifetimes
    t = t.replace("&,", "&").replace("<>", "")
    return t


def last_seg_type(t):
    """Compare return types by outer constructor's last path segment and arity-free args."""
    t = norm_type(t)
    t = re.sub(r"(\w+::)+", "", t)
    return t


class Adjudicator:
    def __init__(self, a, sites):
        self.a = a
        self.sites = sites
        self.orig = os.path.abspath(a.root)
        self.src = os.path.join(a.work, a.corpus + "-src")
        self.target_dir = os.path.join(a.work, "target-" + a.corpus)
        self.logs = os.path.join(a.work, "logs-" + a.corpus)
        os.makedirs(self.logs, exist_ok=True)

    def prepare(self):
        if os.path.exists(self.src):
            shutil.rmtree(self.src)
        subprocess.run(["cp", "-a", self.orig, self.src], check=True)
        # Byte offsets of every fn, including fns inside expanded cfg-wrapper macros.
        self.sbyte = {}
        orig_add = census.Corpus.add_fn
        sbyte = self.sbyte

        def add_fn(corpus, node, st, ctx, inline, impl, allow_dep):
            orig_add(corpus, node, st, ctx, inline, impl, allow_dep)
            f = corpus.fns[-1]
            sbyte[f.key()] = (node.start_byte, st["src"][max(0, node.start_byte - 400):node.start_byte])

        census.Corpus.add_fn = add_fn
        c = census.Corpus(self.orig)
        c.discover_packages()
        c.walk_crates()
        files = {s["target"]["file"] for s in self.sites} | {s["caller"]["file"] for s in self.sites}
        for s in self.sites:
            files |= {x["fid"]["file"] for x in s.get("alternatives") or ()}
        c.load_orphans(files)
        census.Corpus.add_fn = orig_add
        self.corpus = c
        self.fns = {f.key(): f for f in c.fns}

    def env(self, cfg):
        e = dict(os.environ)
        e["CARGO_TARGET_DIR"] = self.target_dir
        e["RUSTFLAGS"] = " ".join([CAP] + cfg.get("rustflags", []))
        e.pop("RUSTC_WRAPPER", None)
        return e

    def cargo(self, cfg, tag):
        cmd = ["cargo", "check", "--message-format=json"] + cfg["args"]
        for p in self.packages:
            cmd += ["-p", p]
        out, code, secs = sh(cmd, self.src, self.env(cfg), os.path.join(self.logs, tag + ".stderr"),
                             self.a.timeout)
        msgs, success = messages(out)
        with open(os.path.join(self.logs, tag + ".jsonl"), "w") as f:
            for m in msgs:
                f.write(json.dumps({k: m.get(k) for k in ("code", "level", "message", "spans")}) + "\n")
        return msgs, success, code, secs

    # ---- markers

    def mark_targets(self):
        """fid-hash -> (role description, fid) for every callable we mark."""
        edits, marks, unmarkable = Edits(), {}, collections.Counter()
        done = set()

        def mark_fn(fn, label):
            key = fn.key()
            if key in done:
                return True
            hit = self.sbyte.get(key)
            if hit is None:
                unmarkable["no_offset"] += 1
                return False
            pos, before = hit
            if b"deprecated" in before.split(b"\n}")[-1].split(b";")[-1]:
                unmarkable["already_deprecated"] += 1
                return False
            h = fn_id_hash(fn.fid())
            edits.add(fn.file, pos, '#[deprecated(note = "p527:%s")] ' % h)
            marks[h] = {"fid": fn.fid(), "label": label, "kind": fn.kind}
            done.add(key)
            return True

        def mark(fid, label):
            fn = self.fns.get((fid["file"], fid["name"], fid["start_line"]))
            if fn is None:
                unmarkable["unmapped"] += 1
                return []
            if fn.kind == "trait_impl":
                decls = [d for d in self.corpus.trait_decls.get(fn.trait or "", ()) if d.name == fn.name]
                if not decls:
                    unmarkable["extern_trait_impl"] += 1
                    return []
                return [fn_id_hash(d.fid()) for d in decls if mark_fn(d, label + "@trait")]
            return [fn_id_hash(fn.fid())] if mark_fn(fn, label) else []

        self.site_marks = {}
        for s in self.sites:
            t = mark(s["target"], "T")
            alts = []
            for k, alt in enumerate(s.get("alternatives") or ()):
                if alt["relation"] == "crate_name_free":
                    continue
                alts.append(mark(alt["fid"], "S%d" % k))
            self.site_marks[s["site_id"]] = {"T": t, "S": alts}
        return edits, marks, unmarkable

    # ---- e0308 rounds

    def e0308_rounds(self):
        rounds = collections.defaultdict(list)
        for s in self.sites:
            f = s["caller"]["file"]
            lane = "tests" if re.search(r"(^|/)(tests|benches|examples)/", f) else "src"
            rounds[(s.get("caller_package") or "", lane)].append(s)
        out = []
        for key in sorted(rounds, key=lambda k: (k[1] != "src", k[0])):
            batch, spans = [], collections.defaultdict(list)
            later = []
            for s in sorted(rounds[key], key=lambda s: s["site_id"]):
                sp = s["span"]
                if any(not (sp["end_byte"] <= a or sp["start_byte"] >= b) for a, b in spans[sp["file"]]):
                    later.append(s)
                    continue
                spans[sp["file"]].append((sp["start_byte"], sp["end_byte"]))
                batch.append(s)
            out.append((key, batch))
            if later:
                out.append(((key[0], key[1] + "-overlap"), later))
        return out

    # ---- run

    def run(self, configs):
        self.prepare()
        self.packages = sorted({s["caller_package"] for s in self.sites if s.get("caller_package")})
        medits, marks, unmarkable = self.mark_targets()
        results = collections.defaultdict(dict)
        meta = {"corpus": self.a.corpus, "packages": self.packages, "configs": {}, "marks": len(marks),
                "unmarkable": dict(unmarkable)}
        for cfg in configs:
            name = cfg["name"]
            cm = meta["configs"][name] = {"args": cfg["args"], "rustflags": [CAP] + cfg.get("rustflags", [])}
            base, ok, code, secs = self.cargo(cfg, name + "-baseline")
            cm["baseline"] = {"success": ok, "exit": code, "secs": round(secs, 1)}
            base_dep = {(norm_file(sp["file_name"], self.src), sp["byte_start"]) for m in base
                        if (m.get("code") or {}).get("code") == "deprecated" for sp in m["spans"] if sp["is_primary"]}
            # Marker build.
            medits.apply(self.src, self.orig)
            mm, mok, mcode, msecs = self.cargo(cfg, name + "-marker")
            medits.restore(self.src, self.orig)
            cm["marker"] = {"success": mok, "exit": mcode, "secs": round(msecs, 1)}
            hits = collections.defaultdict(set)  # (file, orig byte) -> {hash}
            for m in mm:
                if (m.get("code") or {}).get("code") != "deprecated":
                    continue
                note = re.search(r"p527:([0-9a-f]{12})", m.get("message") or "")
                if not note:
                    continue
                for sp in m["spans"]:
                    if not sp["is_primary"]:
                        continue
                    fr = norm_file(sp["file_name"], self.src)
                    if (fr, sp["byte_start"]) in base_dep:
                        continue
                    hits[fr].add((medits.to_orig(fr, sp["byte_start"]), note.group(1)))
            for s in self.sites:
                sp = s["span"]
                got = {h for b, h in hits.get(sp["file"], ()) if sp["start_byte"] <= b < sp["end_byte"]}
                results[s["site_id"]][name] = {"baseline_green": ok, "marker_hits": sorted(got),
                                               "marker_build_success": mok}
            # E0308 rounds.
            cm["e0308_rounds"] = []
            for key, batch in self.e0308_rounds():
                ed = Edits()
                for s in batch:
                    sp = s["span"]
                    ed.add(sp["file"], sp["start_byte"], E0308_PRE)
                    ed.add(sp["file"], sp["end_byte"], E0308_POST)
                ed.apply(self.src, self.orig)
                tag = "%s-e0308-%s-%s" % (name, key[0] or "nopkg", key[1])
                em, eok, ecode, esecs = self.cargo(cfg, tag)
                ed.restore(self.src, self.orig)
                cm["e0308_rounds"].append({"round": list(key), "sites": len(batch), "exit": ecode,
                                           "secs": round(esecs, 1)})
                found = {}
                for m in em:
                    if (m.get("code") or {}).get("code") != "E0308":
                        continue
                    for sp in m["spans"]:
                        if not sp["is_primary"]:
                            continue
                        fr = norm_file(sp["file_name"], self.src)
                        lab = sp.get("label") or ""
                        mt = re.search(r"found (.+)$", lab)
                        if "[(); 527]" not in lab and "[(); 527]" not in json.dumps(m.get("spans")):
                            continue
                        found[(fr, ed.to_orig(fr, sp["byte_start"]))] = mt.group(1) if mt else lab
                for s in batch:
                    sp = s["span"]
                    ft = [v for (fr, b), v in found.items() if fr == sp["file"] and sp["start_byte"] <= b <= sp["end_byte"]]
                    results[s["site_id"]][name]["e0308_found"] = ft[0] if ft else None
                    results[s["site_id"]][name]["e0308_round"] = tag
        return results, meta, marks


def verdict(site, cfgres, marks, site_marks):
    """Per-configuration verdict from the two probes (design §3.2)."""
    if not cfgres.get("baseline_green"):
        return "INADMISSIBLE", "baseline red"
    hits = set(cfgres.get("marker_hits") or ())
    tmarks = set(site_marks["T"])
    amarks = {h: k for k, hs in enumerate(site_marks["S"]) for h in hs}
    found = cfgres.get("e0308_found")
    t_hit = bool(hits & tmarks)
    a_hit = sorted(amarks[h] for h in hits if h in amarks and h not in tmarks)
    if t_hit and not a_hit:
        return "correct_in_config", "target marker at site"
    if a_hit and not t_hit:
        k = a_hit[0]
        alt_ret = (site.get("alternatives") or [])[k].get("return_type_text")
        if found and last_seg_type(found) == last_seg_type(alt_ret) and \
                last_seg_type(alt_ret) != last_seg_type(site.get("return_type_text")):
            return "WRONG", "alternative S%d marker + E0308 found %s" % (k, found)
        return "WRONG_candidate", "alternative S%d marker; E0308 found %s" % (k, found)
    if t_hit and a_hit:
        return "indistinct", "target and alternative markers both at site"
    if found is None:
        return "dead_in_config", "no marker, no E0308: site not compiled"
    if not tmarks:
        return "indistinct", "target unmarkable; E0308 found %s" % found
    return "unindexed_binding", "compiled (E0308 found %s) but no marker note" % found


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--sample", required=True)
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--root", required=True)
    ap.add_argument("--work", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--configs", required=True, help="JSON list of {name,args,rustflags?}")
    ap.add_argument("--timeout", type=int, default=1500)
    a = ap.parse_args()
    with open(a.sample) as f:
        sample = json.load(f)
    sites = [s for s in sample["sites"] if s["corpus"] == a.corpus]
    configs = json.loads(a.configs)
    adj = Adjudicator(a, sites)
    results, meta, marks = adj.run(configs)
    os.makedirs(a.out, exist_ok=True)
    with open(os.path.join(a.out, "%s-verdicts.jsonl" % a.corpus), "w") as f:
        for s in sorted(sites, key=lambda s: s["site_id"]):
            for cfg in configs:
                r = results[s["site_id"]].get(cfg["name"], {})
                v, why = verdict(s, r, marks, adj.site_marks[s["site_id"]])
                f.write(json.dumps({"site_id": s["site_id"], "corpus": a.corpus, "group": s["sample_group"],
                                    "class": s["class"], "implied": s["implied"], "rung": s["rung"],
                                    "callee_text": s["callee_text"], "span": s["span"], "target": s["target"],
                                    "config": cfg["name"], "probe": "marker+e0308", "outcome": r,
                                    "site_marks": adj.site_marks[s["site_id"]], "verdict": v, "why": why},
                                   sort_keys=True) + "\n")
    meta["marks_table"] = marks
    with open(os.path.join(a.out, "%s-adjudication.json" % a.corpus), "w") as f:
        json.dump(meta, f, indent=1, sort_keys=True)
    print(json.dumps({k: meta[k] for k in ("corpus", "packages", "marks", "unmarkable")}))
    for n, c in meta["configs"].items():
        print(n, c["baseline"], c["marker"], [(r["round"], r["exit"], r["secs"]) for r in c["e0308_rounds"]])


if __name__ == "__main__":
    main()
