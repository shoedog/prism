"""Frozen census/draw input binding, source hashes, and count summaries."""

from collections import Counter, defaultdict
from hashlib import sha256
from importlib.metadata import version
from itertools import groupby
import json
from pathlib import Path
import platform

FAMILIES = {"python": ("P1", "P2"), "jsts": ("X", "T"), "go": ("G1", "G2")}
FAMILY_OF = {c: f for f, corpora in FAMILIES.items() for c in corpora}
BENCH = Path.home() / "code" / "bench-repos"
ROOTS = {"P1": BENCH / "apache-superset", "P2": BENCH / "saleor-saleor",
         "X": Path.home() / "prism-evidence" / "inputs" / "excalidraw-0642e72c" / "source",
         "T": BENCH / "TypeScript" / "src", "G1": BENCH / "caddy", "G2": BENCH / "etcd"}
LANGUAGES = {".py": "python", ".js": "javascript", ".jsx": "javascript",
             ".mjs": "javascript", ".cjs": "javascript", ".ts": "typescript",
             ".tsx": "tsx", ".go": "go"}


def digest(data):
    return sha256(data).hexdigest()


def key(row):
    return tuple(row[k] for k in ("corpus", "file", "owner", "lang"))


def c3_report(graph, rows, surviving, detailed, read_costs, recorded_costs, raw_capped):
    by_read = Counter(row["r"] for row in surviving)
    published = {(row["w"][0], row["r"]) for row in surviving}
    by_write = {w.span[0]: w for w in graph.writes.values()}
    counts, by_role = Counter(), defaultdict(Counter)
    details = []
    for row in rows:
        read_span = (row["r_start"], row["r_end"])
        read = graph.reads.get(read_span)
        write = by_write.get(row["w_start"])
        pair_key = (row["w_start"], read_span)
        flags = set(read.flags if read else ["recorded_read_not_in_inventory"])
        flags.update(write.flags if write else ["recorded_write_not_in_inventory"])
        pair_flags = flags | recorded_costs.get(pair_key, set())
        flags.update(read_costs.get(read_span, ()))
        if raw_capped:
            flags.add("raw_pair_cap")
        counts["recorded_reads"] += 1
        counts["would_add_reads"] += bool(by_read[read_span])
        counts["recorded_pair_would_add"] += pair_key in published
        counts.update(flags)
        by_role[row["consumer"]].update(flags)
        if detailed:
            details.append(dict(w_start=row["w_start"], r=read_span, role=row["consumer"],
                                flags=sorted(flags), would_add_sources=by_read[read_span],
                                recorded_pair_flags=sorted(pair_flags),
                                recorded_pair_would_add=pair_key in published))
    result = dict(c3_counts=dict(counts), c3_flags_by_role=dict(by_role))
    if detailed:
        result["c3_reads"] = details
    return result


def load_inputs(draw_path, census_dir, families):
    source = draw_path.read_bytes()
    draw = json.loads(source)
    census, manifests, frames = {}, {}, {}
    module_counts = Counter()
    for family in families:
        frame = defaultdict(list)
        for corpus in FAMILIES[family]:
            census_file = (census_dir / corpus).with_suffix(".jsonl")
            data = census_file.read_bytes()
            census[corpus] = digest(data)
            for line in data.splitlines():
                row = json.loads(line)
                if row["kind"] != "whole":
                    continue
                if row["corpus"] != corpus:
                    raise ValueError("unexpected census corpus")
                if row["owner"] == "<module>":
                    module_counts[corpus] += 1
                    continue
                frame[key(row)].append(row)
        supplied = draw["families"][family]
        selections = [tuple(entry) for entry in supplied["sample"]]
        frame_hash = digest(json.dumps(sorted(frame)).encode())
        sample_hash = digest(json.dumps(selections).encode())
        if (frame_hash, sample_hash) != (supplied["frame_sha256"], supplied["sample_sha256"]):
            raise ValueError("frame or sample hash disagrees for " + family)
        if (len(frame) != supplied["frame_size"] or len(selections) != 60
                or len(set(selections)) != 60 or any(k not in frame for k in selections)):
            raise ValueError("invalid frozen draw for " + family)
        manifests[family] = dict(frame_sha256=frame_hash, sample_sha256=sample_hash,
                                 frame_size=len(frame), sample_size=len(selections))
        frames[family] = (frame, selections)
    provenance = dict(draw_sha256=digest(source), census_sha256=census,
                      frozen_draw_python=draw["python_version"], frozen_manifests=manifests,
                      module_reads_excluded=dict(module_counts), python=platform.python_version())
    if any(f != "python" for f in families):
        provenance["grammars"] = {p: version(p) for p in (
            "tree-sitter", "tree-sitter-javascript", "tree-sitter-typescript", "tree-sitter-go"
        )}
    return frames, provenance


def batch(draw_path, census_dir, families, counts_only):
    from inventory import GRAPH_EVIDENCE, analyze_parsed, parse, unavailable

    if len(families) != len(set(families)):
        raise ValueError("repeated family in batch")
    frames, provenance = load_inputs(draw_path, census_dir, families)
    file_hashes = defaultdict(dict)
    output = []
    for family in sorted(frames):
        frame, selections = frames[family]
        owner_keys = sorted(frame if counts_only else selections)
        for (corpus, relative), owners in groupby(owner_keys, key=lambda k: k[:2]):
            root = ROOTS[corpus]
            path = root / relative
            if not path.resolve().is_relative_to(root.resolve()):
                raise ValueError("census source path escapes corpus")
            data = path.read_bytes()
            file_hashes[corpus][relative] = sha = digest(data)
            lang = LANGUAGES[path.suffix.lower()]
            try:
                parsed = parse(data, lang)
            except (UnicodeError, SyntaxError):
                parsed = None
            for owner_key in owners:
                _, _, owner, census_lang = owner_key
                if lang != census_lang:
                    raise ValueError("census parser mode disagrees for " + relative)
                c3 = sorted(frame[owner_key], key=lambda r: (r["r_start"], r["w_start"]))
                if parsed is None:
                    row = unavailable("source_parse_error", not counts_only)
                    row.update(lang=lang, owner=owner, evidence="syntax_only",
                               graph_evidence=GRAPH_EVIDENCE,
                               c3_counts=dict(recorded_reads=len(c3), unverified_reads=len(c3)))
                else:
                    row = analyze_parsed(parsed, lang, owner, detailed=not counts_only, c3=c3)
                row.update(family=family, corpus=corpus, file=relative, source_sha256=sha)
                output.append(row)
    provenance["source_files_sha256"] = dict(file_hashes)
    provenance["source_manifest_sha256"] = digest(json.dumps(file_hashes, sort_keys=True).encode())
    provenance["analysis"] = "whole_frame" if counts_only else "sample"
    return output, provenance


def summarize(rows):
    result = dict(owners=len(rows), owners_with_would_add=0, owners_with_would_add_at_c3=0)
    stages = defaultdict(Counter)
    c3_roles, pair_roles = defaultdict(Counter), defaultdict(Counter)
    counters = {name: Counter() for name in (
        "status", "counts", "flags", "owners_with_flag", "would_add_by_role", "c3_counts",
        "pair_refusals", "endpoint_pair_refusals", "owners_without_c3_add_by_flag",
    )}
    for row in rows:
        counters["status"][row["status"]] += 1
        for name in ("counts", "flags", "would_add_by_role", "c3_counts", "pair_refusals",
                     "endpoint_pair_refusals"):
            counters[name].update(row.get(name, {}))
        counters["owners_with_flag"].update(row["flags"].keys())
        for stage, (pairs, reads) in row.get("attrition", {}).items():
            stages[stage].update(pairs=pairs, reads=reads, owners=int(bool(reads)))
        for role, flags in row.get("c3_flags_by_role", {}).items():
            c3_roles[role].update(flags)
        for role, flags in row.get("pair_flags", {}).items():
            pair_roles[role].update(flags)
        result["owners_with_would_add"] += bool(row["counts"].get("would_add"))
        result["owners_with_would_add_at_c3"] += bool(row["counts"].get("would_add_at_c3"))
        if not row["counts"].get("would_add_at_c3"):
            counters["owners_without_c3_add_by_flag"].update(
                flag for flag in row["c3_counts"]
                if flag not in ("recorded_reads", "would_add_reads", "recorded_pair_would_add")
            )
    result.update(counters)
    result.update(attrition=stages, c3_flags_by_role=c3_roles, pair_flags_by_role=pair_roles,
                  max_raw_pairs=max((row["counts"].get("raw_pairs", 0) for row in rows), default=0))
    return result


def write_outputs(rows, dest, counts_only):
    rows = sorted(rows, key=key)
    if len(rows) != len({key(row) for row in rows}):
        raise ValueError("duplicate owner output")
    if counts_only:
        partitions = {}
        for col in ("family", "corpus", "lang"):
            groups = defaultdict(list)
            for row in rows:
                groups[row[col]].append(row)
            partitions["by_" + col] = {k: summarize(v) for k, v in groups.items()}
        dest.write_text(json.dumps(dict(**partitions, owners=rows), sort_keys=True, indent=2) + "\n")
    else:
        with dest.open("w") as output:
            for row in rows:
                output.write(json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n")


def merged(paths, dest, counts_only):
    rows = []
    for path in paths:
        if counts_only:
            rows.extend(json.loads(path.read_bytes())["owners"])
        else:
            rows.extend(json.loads(line) for line in path.read_bytes().splitlines())
    write_outputs(rows, dest, counts_only)
