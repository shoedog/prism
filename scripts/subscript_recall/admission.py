#!/usr/bin/env python3
"""Admission check before I2 (lane 10, Python): the count A of owners whose recorded C3 read gains a pair.

`prepare` binds the frozen frame, the product trace (the `on` arm of `compare.py run`) and the independent
full-loader stream of `loader_inventory.py`, reconciles their predictions per owner and freezes the
qualifying pairs. `report` joins the witness judgments (`witnesses.jsonl`, bound to frozen pair keys) and
prints A against the gate. An owner counts once when a pair that BOTH sides predict, with no parent edge and
no equivalent parent value route, reaches a recorded C3 read (owner, exact r_start:r_end, role) in an owner
with an empty reconciliation residual and a feasible witness bound to the prepared digest. Conservative
readings are marked CHOICE.

I2 is admitted only when A passes, every owner reconciles, flood evidence exists and holds for EVERY corpus of
the declared full-loader set (`prepare --full-loader-corpora P1,P2,X,G1,G2,R`, the default; the production
entry point refuses a set that omits one of them; a corpus with no Python owner is an explicit zero row; a
declared corpus the independent stream lacks blocks I2 and leaves the A decision as it was) and a prospective
precision receipt passes. The receipt is derived (`precision.py`): `draw --prepared DIR --out DRAW.json`
derives the DESIGN section 7 pair draw from `candidates.jsonl`; `precision --prepared DIR --draw DRAW.json
--judgments J.jsonl --out precision.json` checks the judgment keys are exactly the draw's and writes
`admission_precision/3`, bound by `prepared_sha256`, `draw_sha256` and `judgments_sha256`; `report --precision`
re-reads the draw and judgments files the receipt names and recomputes everything, so the receipt alone is
never trusted.
"""

import argparse
import json
import math
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

import compare
import precision
from audit import digest, key as frame_key
from loader_inventory import DEFAULT_CORPORA as FULL_LOADER_CORPORA

GATE = 100
FROZEN = dict(sha="6c1c0998f4f1afa06b95b2dad84f220e71c784d3d1b8ec82e6606029bb05b818", owners=787,
              corpora=dict(P1=517, P2=270), rows=1137,
              census=dict(P1="bbff78baaf6161cc6b7038d1315c18f45fef1b3d9c2efca6b1ac84572e8512bd",
                          P2="1f3c4c01d0c2828b247fab6970e85f852bb2acde316be48cbfa65785717d6599"))
ROLE = dict(Argument="arg", Keyword="arg", Return="return", AssignmentRhs="assign_rhs")
STAGES = ("admitted", "joined", "bound", "routed", "unoccupied", "distinct", "after_fanin", "published")
RECON = ("rd", "raw_pairs", "raw_overflow", "raw_capped", "reads_over_fanin", "omitted_fanin_pairs",
         "over_edge_cap", "omitted_cap_pairs")
FLOOD_CAPS = (256, 4)
PRECISION_SCHEMA = precision.PRECISION_SCHEMA


def jsonl(path):
    return [json.loads(line) for line in Path(path).read_text(encoding="utf-8").splitlines() if line.strip()]


def dump(row):
    return json.dumps(row, sort_keys=True, separators=(",", ":"))


def pair_key(corpus, file, owner, w, r, role):
    return f"{corpus}|{file}|{owner}|{w[0]}:{w[1]}|{r[0]}:{r[1]}|{role}"


wilson_lower = precision.wilson_lower


def load_frame(draw_path, census_dir, frozen=FROZEN):
    """Frozen frame keys with their C3 `whole` rows; the draw's hashes and sizes must match.

    `frozen` is the recorded frame (digest, 787 owners, 517/270, 1,137 rows, census file digests); the
    production entry point always passes it, and a substitute frame or census input is refused. A test
    injects another frame, or `None` for only the draw's own consistency checks (never from the CLI)."""
    draw = json.loads(Path(draw_path).read_text())
    spec = draw["families"]["python"]
    frame, census_files = defaultdict(list), defaultdict(set)
    census_sha = {}
    for corpus in sorted(spec["by_corpus"]):
        data = (Path(census_dir) / f"{corpus}.jsonl").read_bytes()
        census_sha[corpus] = digest(data)
        for line in data.splitlines():
            row = json.loads(line)
            census_files[corpus].add(row["file"])
            if row["kind"] == "whole" and row["owner"] != "<module>":
                frame[frame_key(row)].append(row)
    by_corpus = Counter(k[0] for k in frame)
    sample = [tuple(k) for k in spec["sample"]]
    if (digest(json.dumps(sorted(frame)).encode()) != spec["frame_sha256"] or len(frame) != spec["frame_size"]
            or dict(by_corpus) != spec["by_corpus"] or any(k not in frame for k in sample)
            or digest(json.dumps(sample).encode()) != spec["sample_sha256"]):
        raise ValueError("census frame disagrees with the frozen draw")
    if frozen is not None and (
            spec["frame_sha256"] != frozen["sha"] or len(frame) != frozen["owners"]
            or dict(by_corpus) != frozen["corpora"] or sum(map(len, frame.values())) != frozen["rows"]):
        raise ValueError("not the frozen frame (digest, owners, per-corpus owners and C3 rows are pinned)")
    if frozen is not None and census_sha != frozen["census"]:
        raise ValueError("census inputs are not the frozen ones (per-corpus digests are pinned)")
    return dict(frame=dict(frame), files=dict(census_files), sample=sample, sha=spec["frame_sha256"],
                census_sha=census_sha, draw_sha=digest(Path(draw_path).read_bytes()))


def load_m_audit(directory):
    """M's python rows (sample inventory), its frame owner keys, and the raters beside it when present."""
    d = Path(directory)
    plain, packed = d / "frame-counts.json", d / "frame-counts.json.zst"
    raw = plain.read_bytes() if plain.exists() else subprocess.run(
        ["zstd", "-dc", str(packed)], capture_output=True, check=True).stdout
    owners = {frame_key(r) for r in json.loads(raw)["owners"] if r["family"] == "python"}
    sample = {frame_key(r): r for r in jsonl(d / "sample-inventory.jsonl") if r["family"] == "python"}
    raters = {}
    for name in ("rate-sol", "rate-terra"):
        path = d.parent / name / "python.jsonl"
        if path.exists():
            raters[name] = jsonl(path)
    return dict(owners=owners, sample=sample, raters=raters, sha=digest(raw))


class Owner:
    """One product owner record with the records assigned to it."""

    def __init__(self, rec):
        self.rec, self.writes, self.nodes, self.preds = rec, [], [], []
        self.ident = (rec["file"], rec["owner"], rec["owner_line"])
        self.start = rec["span"][0]


def load_trace(run_dir, problems, corpus):
    d = Path(run_dir)
    manifest = json.loads((d / "manifest.json").read_text())
    path = d / "trace.txt"
    if manifest.get("complete") is not True or manifest.get("trace") != 1 or not path.is_file() or (
            compare.sha256_file(path) != manifest["artifacts"].get("trace.txt")):
        raise ValueError(f"{corpus}: the on arm is not a complete, bound trace run")
    records = []
    for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if line.startswith(compare.TRACE_PREFIX):
            records.append(json.loads(line[len(compare.TRACE_PREFIX):]))
        else:
            problems.append(f"{corpus}: trace line {n} lacks the record prefix")
    owners = [Owner(r) for r in records if r["kind"] == "owner"]
    groups = defaultdict(list)
    for o in owners:
        groups[o.ident].append(o)
    seen = Counter((o.ident, o.start) for o in owners)
    dups = {i for (i, _), n in seen.items() if n > 1}
    problems += [f"{corpus}: duplicate trace owner {i}" for i in sorted(dups)]
    for rec in (r for r in records if r["kind"] != "owner"):
        cands = groups.get((rec["file"], rec["owner"], rec["owner_line"]), [])
        at = (rec.get("bytes") or rec.get("w") or [0])[0]
        if len(cands) > 1:
            cands = [o for o in cands if o.start <= at < o.rec["span"][1]]
        if len(cands) != 1:
            problems.append(f"{corpus}: {rec['kind']} record without one owner ({rec['file']} {rec['owner']})")
            continue
        {"write": cands[0].writes, "node": cands[0].nodes, "prediction": cands[0].preds}[rec["kind"]].append(rec)
    for o in owners:
        pairs = Counter((tuple(p["w"]), tuple(p["r"])) for p in o.rec["pairs"])
        preds = Counter((tuple(p["w"]), tuple(p["r"])) for p in o.preds)
        if (len(o.writes), len(o.nodes)) != (o.rec["writes"], o.rec["nodes"]) or pairs != preds:
            problems.append(f"{corpus}: incomplete trace stream for {o.ident}")
    return owners, manifest, dups


def load_independent(path, problems):
    """`corpus -> (run record, file statuses, owner rows by identity)`; the `end` totals must match."""
    out = {}
    for row in jsonl(path):
        c = out.setdefault(row["corpus"], dict(run=None, files={}, owners=[], preds=defaultdict(list),
                                               seen=Counter(), end=None))
        kind = row["kind"]
        if kind == "run":
            c["run"] = row
        elif kind == "file":
            c["files"][row["file"]] = row
        elif kind == "end":
            c["end"] = row
        elif kind == "owner":
            c["owners"].append(row)
        elif kind == "prediction":
            c["preds"][(row["file"], row["owner"])].append(row)
        c["seen"][kind] += 1
    for corpus, c in out.items():
        end = c["end"]
        if c["run"] is None or end is None or (end["owners"], end["predictions"], end["files"]) != (
                c["seen"]["owner"], c["seen"]["prediction"], c["seen"]["file"]):
            problems.append(f"{corpus}: independent stream is incomplete")
    return out


def survivors(preds, role_of):
    """`(W bytes, R bytes, role) -> [prediction rows]`: a multiset, so a repeated pair stays visible."""
    out = defaultdict(list)
    for p in preds:
        if p["outcome"] == "predicted":
            out[(tuple(p["w"]), tuple(p["r"]), role_of(p))].append(p)
    return out


def identity_matches(rec, span, end_max):
    """The product's owner span against the independent one: the same start, and an end that is the node's
    own or reaches only over the comment-only text that follows it (tree-sitter keeps a trailing comment)."""
    return rec[0] == span[0] and span[1] <= rec[1] <= end_max


def match_owners(product, independent):
    """Pair product and independent owners on (file, name, product start line) and the span.

    The product starts a decorated function at its first decorator and keys it by that line, the census by
    the `def` line; the independent row carries both (`owner_line`/`owner_span`, `line`/`span`). The span
    start must be equal and the end equal up to trailing comments. CHOICE: a collision (two product owners
    for one key), a span disagreement or an owner whose product start the independent side could not
    establish pairs with nothing (refused, never guessed); it comes back in `alone`. Rows of the older shape
    (no `owner_line`) are read as undecorated."""
    by_id = defaultdict(list)
    for o in product:
        by_id[o.ident].append(o)
    matched, alone, taken = [], [], Counter()
    for ind in independent:
        line = ind.get("owner_line", ind["line"])
        span = ind.get("owner_span") or ind["span"]
        cands = [o for o in by_id.get((ind["file"], ind["name"], line), [])
                 if identity_matches(o.rec["span"], span, ind.get("end_max", span[1]))] if line and span else []
        if len(cands) == 1:
            taken[id(cands[0])] += 1
        matched.append((cands[0], ind)) if len(cands) == 1 else alone.append(ind)
    # a product owner claimed by two independent owners joins neither
    clash = {k for k, n in taken.items() if n > 1}
    alone += [i for p, i in matched if id(p) in clash]
    return [(p, i) for p, i in matched if id(p) not in clash], alone


def function_index(files):
    """Every function owner the independent side listed per file: `(file, name, product line) -> rows`; an
    identity listed twice, or one whose product start was not established, joins nothing."""
    out = defaultdict(list)
    for file, row in files.items():
        for key, name, _def_line, line, span, _def_span, ambiguous, end_max in row.get("functions", ()):
            if line is not None and span and not ambiguous:
                out[(file, name, line)].append((key, span, end_max))
    return out


def residual(prod, ind, ind_preds):
    """Differences between the product's and the independent owner's predictions and totals."""
    p = survivors(prod.preds, lambda x: ROLE.get(x["role"], x["role"]))
    i = survivors(ind_preds, lambda x: x["role"])
    out = []
    for key in sorted(set(p) | set(i)):
        # multiplicities, not sets: two product copies against one independent copy leave one residual
        for side, more, less in (("product_only", p, i), ("independent_only", i, p)):
            extra = len(more.get(key, ())) - len(less.get(key, ()))
            if extra > 0:
                w, r, role = key
                out.append(dict(field="pair", side=side, w=w, r=r, role=role, count=extra))
    for field in RECON:
        a, b = prod.rec.get(field), ind.get(field)
        if field == "raw_pairs" and a is not None and b is not None:
            # The independent side sees the graph after materialization, the product raw rows before it:
            # the product's P lies between the clean count and the count including join-refused endpoints.
            if not b <= a <= max(b, ind.get("raw_pairs_max") or 0):
                out.append(dict(field=field, product=a, independent=b, independent_max=ind.get("raw_pairs_max")))
        elif a != b:
            out.append(dict(field=field, product=a, independent=b))
    return out, p, i


def duplicate_pair_keys(survivor_sets):
    """Pair keys that one side predicts more than once (a problem before reconciliation)."""
    return [(side, key) for side, items in survivor_sets for key, rows in sorted(items.items()) if len(rows) > 1]


def flood(agreed, corpora=()):
    """predicted_edges_c <= min(P_c, 256 x O_c, 4 x R_c) per corpus (P_c, O_c, R_c over agreed survivors).

    Every corpus in `corpora` has a row, a corpus with no agreed survivor an explicit zero row."""
    rows = {}
    pools = defaultdict(list, agreed)
    for corpus in corpora:
        pools[corpus]
    for corpus, items in pools.items():
        reads = {(i["file"], i["owner"], tuple(i["r"])) for i in items}
        owners = {(i["file"], i["owner"]) for i in items}
        bound = min(sum(i["raw"] or 0 for i in {(x["file"], x["owner"]): x for x in items}.values()),
                    FLOOD_CAPS[0] * len(owners), FLOOD_CAPS[1] * len(reads))
        rows[corpus] = dict(predicted_edges=len(items), bound=bound, ok=len(items) <= bound,
                            reads=len(reads), owners=len(owners))
    return rows


def corpus_pass(corpus, fr, ind, product, problems, acc):
    """Reconcile one corpus; fill `acc` with owner rows, candidates, residuals and survivor categories."""
    matched, alone = match_owners(product, ind["owners"])
    problems += [f"{corpus}: independent owner {i['file']} {i['owner']} has no trace record"
                 for i in alone if ind["files"][i["file"]]["status"] == "parsed"]
    paired = {id(p) for p, _ in matched}
    for p, i in matched:
        res, prod_set, ind_set = residual(p, i, ind["preds"][(i["file"], i["owner"])])
        problems += [f"{corpus}: duplicate pair key on the {side} side of {i['file']} {i['owner']}: {key[0]} {key[1]}"
                     f" {key[2]}" for side, key in duplicate_pair_keys((("product", prod_set), ("independent", ind_set)))]
        agreed = {k: ind_set[k][0] for k in prod_set if k in ind_set}
        fk = (corpus, i["file"], i["owner"], "python")
        c3 = defaultdict(list)
        for r in fr["frame"].get(fk, ()):
            c3[(r["r_start"], r["r_end"], r["consumer"])].append(r["w_start"])
        for (w, r, role), pred in agreed.items():
            if fk in fr["frame"]:
                where = "c3" if (r[0], r[1], role) in c3 else "frame_other_read"
            else:
                where = "other_in_filter" if i["file"] in fr["files"].get(corpus, ()) else "outside_filter"
            cat = acc["categories"][(corpus, role, where)]
            cat["pairs"] += 1
            cat["by_owner"][(i["file"], i["owner"])] += 1
            acc["agreed"][corpus].append(dict(file=i["file"], owner=i["owner"], r=r, raw=i["raw_pairs"]))
            # CHOICE: an owner with any residual earns nothing, even for a pair both sides predict.
            # An unknown parent route (`None`: the search ran out of budget) is no proof of absence: no credit.
            if where == "c3" and not res and not pred["occupied"] and pred.get("parent_route") is False:
                acc["candidates"].append(dict(
                    key=pair_key(corpus, i["file"], i["owner"], w, r, role), corpus=corpus, file=i["file"],
                    owner=i["owner"], w=list(w), r=list(r), role=role, recorded_w=sorted(c3[(r[0], r[1], role)]),
                    route=pred["route"]))
        if res:
            acc["residuals"].append(dict(corpus=corpus, file=i["file"], owner=i["owner"], residual=res))
        acc["stats"][corpus].update(product_predicted=len(prod_set), independent_predicted=len(ind_set),
                                    agreed=len(agreed), residual_owners=int(bool(res)))
        acc["owners"][fk] = dict(product=p.rec, residual=res, agreed=len(agreed), product_survivors=len(prod_set),
                                 independent_survivors=len(ind_set), ident=p.ident)
    listed = function_index(ind["files"])
    for p in product:
        if id(p) in paired:
            continue
        keys = [k for k, span, end_max in listed.get(p.ident, []) if identity_matches(p.rec["span"], span, end_max)]
        predicted = any(x["outcome"] == "predicted" for x in p.preds)
        if len(keys) == 1 and not predicted:
            # The independent side parsed this exact function and found no candidate store in it, and the
            # product predicts no pair for it: both sides agree that the owner holds nothing to credit.
            acc["owners"][(corpus, p.ident[0], keys[0], "python")] = dict(
                product=p.rec, residual=[], agreed=0, product_survivors=0, independent_survivors=0,
                ident=p.ident, independent="no_candidate")
        elif predicted:
            acc["residuals"].append(dict(corpus=corpus, file=p.rec["file"], owner=f"{p.ident[1]}@{p.ident[2]}",
                                         residual=[dict(field="owner", side="product_only")]))
            if len(keys) == 1:
                acc["owners"][(corpus, p.ident[0], keys[0], "python")] = dict(
                    product=p.rec, residual=[dict(field="owner", side="product_only")], agreed=0,
                    product_survivors=sum(x["outcome"] == "predicted" for x in p.preds), independent_survivors=0,
                    ident=p.ident)


def status_rows(fr, m, independent, acc, dups, problems):
    cand_owners = {(c["corpus"], c["file"], c["owner"]) for c in acc["candidates"]}
    rows = []
    for fk, c3 in sorted(fr["frame"].items()):
        corpus, file, owner, lang = fk
        row = dict(corpus=corpus, file=file, owner=owner, lang=lang, c3_rows=len(c3),
                   in_sample=fk in set(fr["sample"]), m=m["sample"].get(fk, {}).get("counts"))
        entry, ind = acc["owners"].get(fk), independent.get(corpus)
        listed = ind["files"].get(file) if ind else None
        if entry:
            dup = (corpus, entry["ident"]) in dups
            row.update(status="duplicate_trace" if dup else "unreconciled" if entry["residual"] else "ok",
                       product_survivors=entry["product_survivors"], agreed=entry["agreed"],
                       independent_survivors=entry["independent_survivors"], rd=entry["product"]["rd"],
                       attrition=entry["product"]["attrition"], residual=entry["residual"],
                       candidates=int((corpus, file, owner) in cand_owners))
        elif listed and listed["status"] == "skipped":
            row["status"] = "loader_skip"
        else:
            row["status"] = "missing_trace"
            problems.append(f"{corpus}: frame owner {file} {owner} has no trace and no loader skip")
        rows.append(row)
    return rows


def declared_corpora(text, frozen):
    """The `--full-loader-corpora` set (sorted, distinct). A frozen (production) run must name at least the
    six corpora of the I2 measurement; a narrower set is the way to skip flood evidence, so it is refused."""
    names = [n.strip() for n in str(text).split(",")]
    if not names or any(not n for n in names):
        raise ValueError(f"--full-loader-corpora {text!r}: empty corpus name")
    declared = sorted(set(names))
    if frozen is not None and not set(FULL_LOADER_CORPORA) <= set(declared):
        raise ValueError("--full-loader-corpora must include every full-loader corpus "
                         f"({','.join(FULL_LOADER_CORPORA)}), missing {sorted(set(FULL_LOADER_CORPORA) - set(declared))}")
    return declared


def prepare(a, frozen=FROZEN):
    fr = load_frame(a.draw, a.census_dir, frozen)
    declared = declared_corpora(a.full_loader_corpora, frozen)
    m = load_m_audit(a.m_audit)
    if m["owners"] != set(fr["frame"]):
        raise ValueError("M's frame owners disagree with the census frame")
    problems = []
    independent = load_independent(a.independent, problems)
    acc = dict(categories=defaultdict(lambda: dict(pairs=0, by_owner=Counter())), owners={}, candidates=[],
               residuals=[], stats=defaultdict(Counter), agreed=defaultdict(list))
    runs, dups = {}, set()
    # A is the frame's (P1, P2); flood accounting covers every declared full-loader corpus, evidence or not.
    missing = []
    for corpus in sorted(set(fr["census_sha"]) | set(declared) | set(independent)):
        ind = independent.get(corpus)
        if ind is None:
            if corpus in fr["census_sha"]:
                problems.append(f"{corpus}: no independent records")
            else:
                missing.append(corpus)  # no A is lost, but I2 has no flood evidence for it
            continue
        product, manifest, dup = load_trace(Path(a.runs) / corpus / "on", problems, corpus)
        dups |= {(corpus, d) for d in dup}
        base = json.loads((Path(a.runs) / corpus / "base" / "manifest.json").read_text())
        if (base["source"]["digest"], base["run_id"]) != (ind["run"]["source_digest"], ind["run"]["run_id"]) or (
                manifest["source"]["digest"] != base["source"]["digest"]):
            problems.append(f"{corpus}: independent, parent and trace runs are not bound to one source tree")
        runs[corpus] = dict(trace_run=manifest["run_id"], parent_run=base["run_id"],
                            source_digest=base["source"]["digest"], trace_sha=manifest["artifacts"]["trace.txt"])
        corpus_pass(corpus, fr, ind, product, problems, acc)
    rows = status_rows(fr, m, independent, acc, dups, problems)
    problems = sorted(set(problems))
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    for name, items in (("owners", rows), ("candidates", sorted(acc["candidates"], key=lambda c: c["key"])),
                        ("residuals", acc["residuals"])):
        (out / f"{name}.jsonl").write_text("".join(dump(r) + "\n" for r in items))
    cats = [dict(corpus=k[0], role=k[1], where=k[2], pairs=v["pairs"], owners=len(v["by_owner"]),
                 max_per_owner=max(v["by_owner"].values())) for k, v in sorted(acc["categories"].items())]
    meta = dict(schema="admission_prepared/1", valid=not problems, problems=problems, frame_sha256=fr["sha"],
                draw_sha256=fr["draw_sha"], census_sha256=fr["census_sha"], m_audit_sha256=m["sha"],
                independent_sha256=digest(Path(a.independent).read_bytes()), runs=runs,
                frame_owners=len(fr["frame"]), frame_by_corpus=dict(Counter(k[0] for k in fr["frame"])),
                candidates_sha256=digest((out / "candidates.jsonl").read_bytes()),
                stats={c: dict(v) for c, v in acc["stats"].items()}, categories=cats,
                flood=flood(acc["agreed"], [c for c in declared if c in independent]),
                full_loader_corpora=declared, full_loader_missing=sorted(missing),
                residual_owners=len(acc["residuals"]),
                raters={n: dict(valid=sum(bool(r["a"]) for r in rs), addressable=sum(bool(r["b"]) for r in rs),
                                n=len(rs)) for n, rs in m["raters"].items()})
    meta["prepared_sha256"] = digest(dump(meta).encode())
    (out / "prepared.json").write_text(json.dumps(meta, indent=1, sort_keys=True) + "\n")
    print(f"prepare: {'VALID' if meta['valid'] else 'INVALID'}; {len(acc['candidates'])} candidate pairs, "
          f"{len(acc['residuals'])} residual owners, {len(problems)} problems")
    for p in problems[:20]:
        print("  " + p, file=sys.stderr)
    return 0 if meta["valid"] else 1


def witnessed(candidates, witnesses, prepared_sha):
    """`key -> verdict` over the frozen candidates. Every witness row must carry the exact prepared digest:
    a missing digest is `unbound` and a different one `stale`; stale, unbound, unknown, conflicting and absent
    earn nothing."""
    by_key = defaultdict(list)
    for w in witnesses:
        by_key[w.get("key")].append(w)
    known = {c["key"] for c in candidates}
    verdicts, counts = {}, Counter()
    for key in known:
        rows = by_key.get(key)
        if not rows:
            verdicts[key] = "unverified"
        elif any(r.get("prepared_sha256") not in (None, prepared_sha) or r.get("verdict") == "stale" for r in rows):
            verdicts[key] = "stale"
        elif any(r.get("prepared_sha256") is None for r in rows):
            verdicts[key] = "unbound"
        elif all(r.get("verdict") == "feasible" for r in rows):
            verdicts[key] = "feasible"
        else:
            verdicts[key] = "unknown" if any(r.get("verdict") == "unknown" for r in rows) else "infeasible"
    counts.update(verdicts.values())
    counts["stale"] += sum(1 for k in by_key if k not in known)
    return verdicts, counts


def make_draw(a):
    meta, candidates = precision.load_prepared(a.prepared)
    draw = precision.derive_draw(candidates, meta)
    Path(a.out).write_text(precision.dump(draw) + "\n")
    print(f"draw: {len(draw['keys'])} of {draw['population']} pairs (core {len(draw['core'])}, "
          f"top-up {len(draw['topup'])}, exhaustive {draw['exhaustive']}); roles {draw['role_population']}")
    return 0


def make_precision(a):
    meta, candidates = precision.load_prepared(a.prepared)
    receipt, problems = precision.score(meta, candidates, a.draw, a.judgments)
    Path(a.out).write_text(json.dumps(receipt, indent=1, sort_keys=True) + "\n")
    print(f"precision: {'PASS' if not problems else 'FAIL'}; n={receipt['n']} valid={receipt['valid']} "
          f"stale={receipt['stale']} unknown={receipt['unknown']} wilson_core={receipt['wilson_core']}")
    for p in problems:
        print("  " + p)
    return 1 if problems else 0


def report(a):
    prep = Path(a.prepared)
    meta = json.loads((prep / "prepared.json").read_text())
    candidates = jsonl(prep / "candidates.jsonl")
    if digest((prep / "candidates.jsonl").read_bytes()) != meta["candidates_sha256"]:
        raise ValueError("candidates.jsonl does not match prepared.json")
    witnesses = jsonl(a.witnesses) if a.witnesses and Path(a.witnesses).exists() else []
    receipt, receipt_problem = None, "no precision receipt (pass --precision precision.json)"
    if a.precision:
        try:
            receipt, receipt_problem = json.loads(Path(a.precision).read_text(encoding="utf-8")), None
        except (OSError, ValueError) as e:
            receipt_problem = f"precision receipt unreadable: {e}"
    verdicts, counts = witnessed(candidates, witnesses, meta["prepared_sha256"])
    owners = jsonl(prep / "owners.jsonl")
    total = len(owners)
    credited, ceiling, stops = set(), set(), set()
    qualifying = []
    stop_keys = {w["key"] for w in witnesses if w.get("public_stop")}
    for c in candidates:
        owner = (c["corpus"], c["file"], c["owner"])
        ceiling.add(owner)
        if verdicts[c["key"]] == "feasible":
            credited.add(owner)
            qualifying.append(c)
            if c["key"] in stop_keys:
                stops.add(owner)
    (prep / "qualifying.jsonl").write_text("".join(dump(c) + "\n" for c in qualifying))
    status = Counter(r["status"] for r in owners)
    by = lambda owners_, corpus: sum(1 for o in owners_ if o[0] == corpus)
    physical = sum(r.get("product_survivors", 0) for r in owners)
    both = sum(r.get("agreed", 0) for r in owners)
    A = len(credited)
    verdict = "INVALID" if not meta["valid"] else ("PASS" if A >= GATE else "STOP")
    print(f"frame: {total} owners ({', '.join(f'{c} {n}' for c, n in sorted(meta['frame_by_corpus'].items()))}); "
          f"status rows kept: {dict(status)}")
    for corpus, n in sorted(meta["frame_by_corpus"].items()):
        print(f"  {corpus}: A={by(credited, corpus)} of {n} (qualifying before witness: {by(ceiling, corpus)})")
    print(f"A={A} of {total}; raw physical survivors in frame owners: product {physical}, both sides {both} "
          f"(qualifying pairs {len(qualifying)}; graph credit with a public CFG stop, zero public recovery: {len(stops)})")
    print(f"witnesses: {dict(counts)}; unwitnessed ceiling {len(ceiling)} owners (not A)")
    clean = meta["residual_owners"] == 0
    print(f"reconciliation: {meta['residual_owners']} residual owners; run valid: {meta['valid']}")
    for row in meta["categories"]:
        print(f"  survivors {row['corpus']} {row['role']} {row['where']}: pairs {row['pairs']} "
              f"owners {row['owners']} max/owner {row['max_per_owner']}")
    for corpus, f in sorted(meta["flood"].items()):
        print(f"  flood {corpus}: predicted_edges {f['predicted_edges']} <= {f['bound']}: {'ok' if f['ok'] else 'FAIL'}")
    sample = [r for r in owners if r["in_sample"]]
    stages = {s: sum((r.get("attrition") or {}).get(s, 0) for r in sample) for s in STAGES}
    print(f"frozen sample ({len(sample)} owners) observed attrition: {stages}; "
          f"syntax-only M would_add_at_c3 > 0: {sum(bool((r.get('m') or {}).get('would_add_at_c3')) for r in sample)}")
    for name, r in sorted(meta["raters"].items()):
        lo = wilson_lower(r["addressable"], r["n"])
        print(f"  M {name}: valid {r['valid']}/{r['n']} L(a)={wilson_lower(r['valid'], r['n']):.4f}, "
              f"addressable {r['addressable']}/{r['n']} floor({total}*L(b))={math.floor(total * lo)}")
    print(f"GATE A >= {GATE} of {total}: {verdict} (A={A})")
    flood_failed = sorted(c for c, f in meta["flood"].items() if not f["ok"])
    reasons = [f"flood bound failed in {', '.join(flood_failed)}"] if flood_failed else []
    declared = meta.get("full_loader_corpora")
    if not declared:
        reasons.append("prepared receipt declares no full-loader corpus set")
    elif lacking := sorted(set(declared) - set(meta["flood"])):
        reasons.append(f"no flood evidence for full-loader corpora {', '.join(lacking)}")
    if verdict == "PASS":
        reasons += [] if clean else ["unreconciled predictions"]
        reasons += [receipt_problem] if receipt_problem else precision.verify(receipt, meta, candidates, Path(a.precision).resolve().parent)
    blocked = bool(reasons)
    print(f"I2: {'BLOCKED by ' + '; '.join(reasons) if blocked else 'ADMITTED' if verdict == 'PASS' else 'NOT ADMITTED'}")
    return 2 if verdict == "INVALID" else 4 if blocked else {"PASS": 0, "STOP": 1}[verdict]


def main(argv=None, frozen=FROZEN):
    """`frozen` is the production frame; only a test passes another (or None) to `prepare`."""
    cli = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = cli.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("prepare")
    for flag in ("draw", "census-dir", "m-audit", "runs", "independent", "out"):
        p.add_argument("--" + flag, required=True)
    p.add_argument("--full-loader-corpora", default=",".join(FULL_LOADER_CORPORA),
                   help="corpora whose independent stream and flood row I2 requires (default: all six)")
    p.set_defaults(fn=prepare)
    r = sub.add_parser("report")
    r.add_argument("--prepared", required=True)
    r.add_argument("--witnesses")
    r.add_argument("--precision")
    r.set_defaults(fn=report)
    d = sub.add_parser("draw")
    d.add_argument("--prepared", required=True)
    d.add_argument("--out", required=True)
    d.set_defaults(fn=make_draw)
    q = sub.add_parser("precision")
    for flag in ("prepared", "draw", "judgments", "out"):
        q.add_argument("--" + flag, required=True)
    q.set_defaults(fn=make_precision)
    args = cli.parse_args(argv)
    try:
        return args.fn(args, frozen) if args.cmd == "prepare" else args.fn(args)
    except (ValueError, KeyError, OSError, compare.Refused) as e:
        print(f"admission {args.cmd} refused: {e}", file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())
