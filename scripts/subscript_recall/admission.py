#!/usr/bin/env python3
"""Admission check before I2 (lane 10, Python): the count A of owners whose recorded C3 read gains a pair.

`prepare` binds the frozen frame, the product trace (the `on` arm of `compare.py run`) and the independent
full-loader stream of `loader_inventory.py`, reconciles their predictions per owner and freezes the
qualifying pairs. `report` joins the witness judgments (`witnesses.jsonl`, bound to frozen pair keys) and
prints A against the gate. An owner counts once when a pair that BOTH sides predict, with no parent edge and
no equivalent parent value route, reaches a recorded C3 read (owner, exact r_start:r_end, role) in an owner
with an empty reconciliation residual and a feasible witness. Conservative readings are marked CHOICE.
"""

import argparse
import json
import math
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

import compare
from audit import digest, key as frame_key

GATE = 100
FROZEN = dict(sha="6c1c0998f4f1afa06b95b2dad84f220e71c784d3d1b8ec82e6606029bb05b818", owners=787,
              corpora=dict(P1=517, P2=270), rows=1137)
ROLE = dict(Argument="arg", Keyword="arg", Return="return", AssignmentRhs="assign_rhs")
STAGES = ("admitted", "joined", "bound", "routed", "unoccupied", "distinct", "after_fanin", "published")
RECON = ("rd", "raw_pairs", "raw_overflow", "raw_capped", "reads_over_fanin", "omitted_fanin_pairs",
         "over_edge_cap", "omitted_cap_pairs")
FLOOD_CAPS = (256, 4)


def jsonl(path):
    return [json.loads(line) for line in Path(path).read_text(encoding="utf-8").splitlines() if line.strip()]


def dump(row):
    return json.dumps(row, sort_keys=True, separators=(",", ":"))


def pair_key(corpus, file, owner, w, r, role):
    return f"{corpus}|{file}|{owner}|{w[0]}:{w[1]}|{r[0]}:{r[1]}|{role}"


def wilson_lower(k, n, z=1.96):
    if not n:
        return 0.0
    p = k / n
    return (p + z * z / (2 * n) - z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))) / (1 + z * z / n)


def load_frame(draw_path, census_dir):
    """Frozen frame keys with their C3 `whole` rows; the draw's hashes and sizes must match."""
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
    if spec["frame_sha256"] == FROZEN["sha"] and (
            len(frame) != FROZEN["owners"] or dict(by_corpus) != FROZEN["corpora"]
            or sum(map(len, frame.values())) != FROZEN["rows"]):
        raise ValueError("frozen frame sizes disagree with the recorded 787/517/270/1,137")
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
    return {(tuple(p["w"]), tuple(p["r"]), role_of(p)): p for p in preds if p["outcome"] == "predicted"}


def match_owners(product, independent):
    """Pair product and independent owners by (file, name, line) then span start."""
    by_id = defaultdict(list)
    for o in product:
        by_id[o.ident].append(o)
    matched, alone = [], []
    for ind in independent:
        cands = by_id.get((ind["file"], ind["name"], ind["line"]), [])
        if len(cands) > 1:
            cands = [o for o in cands if o.start == ind["span"][0]]
        if cands:
            matched.append((cands[0], ind))
        else:
            alone.append(ind)
    return matched, alone


def residual(prod, ind, ind_preds):
    """Differences between the product's and the independent owner's predictions and totals."""
    p = survivors(prod.preds, lambda x: ROLE.get(x["role"], x["role"]))
    i = survivors(ind_preds, lambda x: x["role"])
    out = [dict(field="pair", side="product_only", w=w, r=r, role=role) for (w, r, role) in sorted(set(p) - set(i))]
    out += [dict(field="pair", side="independent_only", w=w, r=r, role=role)
            for (w, r, role) in sorted(set(i) - set(p))]
    for field in RECON:
        a, b = prod.rec.get(field), ind.get(field)
        if a != b:
            out.append(dict(field=field, product=a, independent=b))
    return out, p, i


def flood(agreed):
    """predicted_edges_c <= min(P_c, 256 x O_c, 4 x R_c) per corpus (P_c, O_c, R_c over agreed survivors)."""
    rows = {}
    for corpus, items in defaultdict(list, agreed).items():
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
        agreed = {k: ind_set[k] for k in prod_set if k in ind_set}
        fk = (corpus, i["file"], i["owner"], "python")
        c3 = defaultdict(list)
        for r in fr["frame"].get(fk, ()):
            c3[(r["r_start"], r["r_end"], r["consumer"])].append(r["w_start"])
        for (w, r, role), pred in agreed.items():
            if fk in fr["frame"]:
                where = "c3" if (r[0], r[1], role) in c3 else "frame_other_read"
            else:
                where = "other_in_filter" if i["file"] in fr["files"][corpus] else "outside_filter"
            cat = acc["categories"][(corpus, role, where)]
            cat["pairs"] += 1
            cat["by_owner"][(i["file"], i["owner"])] += 1
            acc["agreed"][corpus].append(dict(file=i["file"], owner=i["owner"], r=r, raw=i["raw_pairs"]))
            # CHOICE: an owner with any residual earns nothing, even for a pair both sides predict.
            if where == "c3" and not res and not pred["occupied"] and not pred["parent_route"]:
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
    for p in product:
        if id(p) not in paired and any(x["outcome"] == "predicted" for x in p.preds):
            acc["residuals"].append(dict(corpus=corpus, file=p.rec["file"], owner=f"{p.ident[1]}@{p.ident[2]}",
                                         residual=[dict(field="owner", side="product_only")]))


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


def prepare(a):
    fr = load_frame(a.draw, a.census_dir)
    m = load_m_audit(a.m_audit)
    if m["owners"] != set(fr["frame"]):
        raise ValueError("M's frame owners disagree with the census frame")
    problems = []
    independent = load_independent(a.independent, problems)
    acc = dict(categories=defaultdict(lambda: dict(pairs=0, by_owner=Counter())), owners={}, candidates=[],
               residuals=[], stats=defaultdict(Counter), agreed=defaultdict(list))
    runs, dups = {}, set()
    for corpus in sorted(fr["census_sha"]):
        ind = independent.get(corpus)
        if ind is None:
            problems.append(f"{corpus}: no independent records")
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
                flood=flood(acc["agreed"]), residual_owners=len(acc["residuals"]),
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
    """`key -> verdict` over the frozen candidates; stale, unknown, conflicting and absent earn nothing."""
    by_key = defaultdict(list)
    for w in witnesses:
        by_key[w.get("key")].append(w)
    known = {c["key"] for c in candidates}
    verdicts, counts = {}, Counter()
    for key in known:
        rows = by_key.get(key)
        if not rows:
            verdicts[key] = "unverified"
        elif any(r.get("prepared_sha256", prepared_sha) != prepared_sha or r.get("verdict") == "stale" for r in rows):
            verdicts[key] = "stale"
        elif all(r.get("verdict") == "feasible" for r in rows):
            verdicts[key] = "feasible"
        else:
            verdicts[key] = "unknown" if any(r.get("verdict") == "unknown" for r in rows) else "infeasible"
    counts.update(verdicts.values())
    counts["stale"] += sum(1 for k in by_key if k not in known)
    return verdicts, counts


def report(a):
    prep = Path(a.prepared)
    meta = json.loads((prep / "prepared.json").read_text())
    candidates = jsonl(prep / "candidates.jsonl")
    if digest((prep / "candidates.jsonl").read_bytes()) != meta["candidates_sha256"]:
        raise ValueError("candidates.jsonl does not match prepared.json")
    witnesses = jsonl(a.witnesses) if a.witnesses and Path(a.witnesses).exists() else []
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
    blocked = verdict == "PASS" and not clean
    print(f"I2: {'BLOCKED by unreconciled predictions' if blocked else 'ADMITTED' if verdict == 'PASS' else 'NOT ADMITTED'}")
    return 4 if blocked else {"PASS": 0, "STOP": 1, "INVALID": 2}[verdict]


def main(argv=None):
    cli = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = cli.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("prepare")
    for flag in ("draw", "census-dir", "m-audit", "runs", "independent", "out"):
        p.add_argument("--" + flag, required=True)
    p.set_defaults(fn=prepare)
    r = sub.add_parser("report")
    r.add_argument("--prepared", required=True)
    r.add_argument("--witnesses")
    r.set_defaults(fn=report)
    args = cli.parse_args(argv)
    try:
        return args.fn(args)
    except (ValueError, KeyError, OSError, compare.Refused) as e:
        print(f"admission {args.cmd} refused: {e}", file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())
