"""Compare two signed edge populations and publish the evidence certificate."""

from __future__ import annotations

from contextlib import closing
import json
import os
from pathlib import Path
from tempfile import TemporaryDirectory

from . import classifier, edges
from .diff_stats import admit_pair, counter_tables
from .inputs import sha256_file
from .receipt import gate_binding
from .run import claim_output
from .trace import STATUSES, TraceStore


def classify_changes(db, lane, context, store):
    counts = dict(attributed=0, unattributed=0, ambiguous=0, occurrences=0,
                  base_residual=0, head_residual=0, by_kind=dict.fromkeys(edges.KINDS, 0),
                  by_class={name: {"keys": 0, "occurrences": 0} for name in lane.classes})
    previous = None
    key_classes = set()
    for ident, key, kind, base, head, count in db.execute(
            "SELECT id, key, kind, base, head, count FROM changes ORDER BY id"):
        change = dict(id=ident, key=json.loads(key), kind=kind, count=count,
                      base_bag=json.loads(base), head_bag=json.loads(head),
                      records=store.attached(ident), error=None, **{"class": None})
        name, error = classifier.classify(lane, change, context)
        change.update({"class": name, "error": error})
        db.execute("UPDATE changes SET final=? WHERE id=?", (edges.canonical(change), ident))
        if key != previous:
            key_classes.clear()
            previous = key
        if name is not None:
            counts["by_class"][name]["keys"] += name not in key_classes
            counts["by_class"][name]["occurrences"] += count
            key_classes.add(name)
        counts["attributed" if name is not None else "unattributed"] += 1
        counts["ambiguous"] += kind == "ambiguous"
        counts["by_kind"][kind] += count
        counts["occurrences"] += count
        counts["base_residual"] += len(change["base_bag"])
        counts["head_residual"] += len(change["head_bag"])
    return counts


def build_certificate(db, store, context, lane, counts, totals, discarded, reason,
                      conservation_failed, runs, out, gate_sha256, gate_step_id):
    analysis = classify_changes(db, lane, context, store)
    validation = classifier.validate(lane, classifier.Bound(context))
    context.recheck_sources()
    records, unjoined, by_tag, mechanisms = store.totals()
    failures = []
    if analysis["unattributed"]:
        failures.append(f"unattributed change rows: {analysis['unattributed']}")
    if records["ambiguous"]:
        failures.append(f"ambiguous edge records: {records['ambiguous']}")
    if records["stray"] and lane.strays == "fail":
        failures.append(f"forbidden stray edge records: {records['stray']}")
    failures.extend(validation)
    rows, occurrences = db.execute("SELECT count(*), coalesce(sum(count),0) FROM changes").fetchone()
    if (conservation_failed or not edges.conserved(db, totals)
            or rows != analysis["attributed"] + analysis["unattributed"]
            or occurrences != analysis["occurrences"]):
        failures.append("conservation failure in edge partition")
    return dict(schema="prism.measure.certificate/1", key_mode=context.key_mode,
                base_edges=totals[0], head_edges=totals[1], changed_keys=counts["changed_keys"],
                collisions=counts["collisions"], line_projection_loss=discarded,
                line_reason=reason, **analysis, records=records, unjoined=unjoined,
                trace_by_tag=by_tag, mechanisms=mechanisms, validation=validation, failures=failures,
                result="fail" if failures else "pass", gate_sha256=gate_sha256, gate_step_id=gate_step_id,
                inputs={arm: dict(run=os.path.relpath(run.directory, out),
                                  receipt_sha256=run.receipt_sha256)
                        for arm, run in zip(("base", "head"), runs)},
                classifier_sha256=lane.sha256)


def write_evidence(db, stage, out, certificate):
    changes = stage / "changes.jsonl"
    with changes.open("wb") as stream:
        for (payload,) in db.execute("SELECT final FROM changes ORDER BY id"):
            stream.write(payload + b"\n")
    certificate["changes_sha256"] = sha256_file(changes)
    classes = ["class\tkeys\toccurrences"]
    for name, row in certificate["by_class"].items():
        classes.append(f"{name}\t{row['keys']}\t{row['occurrences']}")
    (stage / "classes.tsv").write_bytes(("\n".join(classes) + "\n").encode("utf-8"))
    (stage / "certificate.json").write_bytes(edges.canonical(certificate) + b"\n")
    for name in ("changes.jsonl", "classes.tsv", "certificate.json"):
        (stage / name).replace(out / name)
    return classes


def display(certificate, counts, classes, out, listing, order):
    print(f"edges base={certificate['base_edges']} head={certificate['head_edges']} "
          f"added={counts['added']} removed={counts['removed']} label-moved keys={counts['moved']}")
    kinds = certificate["by_kind"]
    print("occurrences: " + " ".join(f"{key}={kinds[key]}" for key in
                                    ("added", "removed", "label_moved", "ambiguous")))
    if certificate["key_mode"] == "line":
        print(f"collisions keys={certificate['collisions']}")
    for row in classes:
        print(row)
    print("trace records:")
    for tag, values in sorted(certificate["trace_by_tag"].items()):
        print("\t".join([tag, *(str(values[key]) for key in (*STATUSES, "unjoined"))]))
    mechanisms = certificate["mechanisms"]
    ordered = dict.fromkeys((*order, *sorted(mechanisms)))
    print("trace mechanism lines:", {tag: mechanisms[tag] for tag in ordered if tag in mechanisms})
    if listing:
        with (out / "changes.jsonl").open(encoding="utf-8", newline="") as stream:
            for row in stream:
                print(row, end="")
    for failure in certificate["failures"]:
        print(failure)
    print(certificate["result"].upper())


def compare_edges(base, head, classifier_path, *, entries=None, key=None, line_reason=None,
                  out=Path("diff-edges"), listing=False, gate_sha256=None, gate_step_id=None):
    gate_binding(gate_sha256, gate_step_id)
    runs = admit_pair(base, head, entries)
    tables = counter_tables(runs)
    with claim_output(out), TemporaryDirectory(prefix=".stage-", dir=out) as temporary:
        stage = Path(temporary)
        with closing(edges.make_database(stage / "work.sqlite3")) as db:
            sources = [(run.artifact("edges", "output"), run.expected("edges", "output"))
                       for run in runs]
            mode, discarded, totals = edges.ingest(db, sources, key, line_reason,
                                                   runs[0].receipt["options"]["bytes"])
            with classifier.load(classifier_path) as lane:
                store = TraceStore(db)
                has_edges = store.read(runs, lane.parse_trace)
                counts, conservation_failed = edges.partition(db, mode, track_assignments=has_edges)
                store.join(mode, has_edges)
                context = classifier.Context(runs, mode, store, tables)
                certificate = build_certificate(db, store, context, lane, counts, totals, discarded,
                                                line_reason, conservation_failed, runs, out,
                                                gate_sha256, gate_step_id)
                classes = write_evidence(db, stage, out, certificate)
    order = lane.legacy_order.get(runs[1].receipt["corpus"]["key"], ())
    display(certificate, counts, classes, out, listing, order)
    return certificate
