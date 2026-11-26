"""Turn independently labelled, already captured call sites into qualifications."""

from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory

from .edges import canonical
from .inputs import jsonl_objects
from .oracle import launch, recheck
from .oracle_inputs import FORMS, OracleError, digest, name, prepare_inputs, regular, safe_path, shape, source
from .oracle_selection import admit, number, read, require, site
from .run import claim_output


def labelled(value, run_count):
    require(shape(value, ("run_index", "record_ordinal", "target_index", "form",
                          "declaration", "contradiction_line"))
            and all(number(value[key]) for key in ("run_index", "record_ordinal", "target_index"))
            and value["run_index"] < run_count and value["form"] in FORMS,
            "qualification-label: invalid index or form")
    point = value["declaration"]
    require(shape(point, ("file", "name", "start_byte", "end_byte", "line"))
            and safe_path(point["file"]) and name(point["name"])
            and number(point["start_byte"]) and number(point["end_byte"], point["start_byte"] + 1)
            and number(point["line"], 1) and number(value["contradiction_line"], 1)
            and point["line"] != value["contradiction_line"], "qualification-label: invalid source label")
    return value


def build(runs, labels, output_dir):
    """Validate existing --sites receipts and write a new bundle plus indexed evidence.

    Labels may be dictionaries or the path to a JSONL file. The public function
    does not execute a producer. The checker controls run inside memguard.
    """
    runs = [admit(Path(run), None) for run in runs]
    if isinstance(labels, (str, Path)):
        labels = list(jsonl_objects(Path(labels)))
    labels = [labelled(record, len(runs)) for record in labels]
    require(labels, "qualification-label: no controls")
    sources, states = {}, {}
    for label in labels:
        index = label["run_index"]
        if index not in sources:
            sources[index], states[index] = source(runs[index])
            require(sources[index]["matches"], "qualification-source: snapshot differs")
    rows = {}
    for index in sources:
        wanted = {label["record_ordinal"] for label in labels if label["run_index"] == index}
        for ordinal, row in enumerate(read(runs[index], "sites")):
            if ordinal in wanted:
                require(row.get("record_kind") == "call_site", "qualification: expected a call_site")
                rows[index, ordinal] = site(row)
        require(wanted <= {ordinal for arm, ordinal in rows if arm == index},
                "qualification: record index unavailable")
    candidates = []
    evidence = []
    builds = {}
    for label in labels:
        index, ordinal, target = (label[key] for key in ("run_index", "record_ordinal", "target_index"))
        row = rows[index, ordinal]
        require(target < len(row["resolved_targets"]) and row["resolved_targets"][target]["confidence"] == "exact",
                "qualification: expected exact indexed target")
        binary = runs[index].receipt["binary"]["sha256"]
        form = label["form"]
        forms = builds.setdefault(binary, {})
        require(form not in forms, "qualification: duplicate build/form qualification")
        forms[form] = None
        src = sources[index]
        point = label["declaration"]
        require(point["file"] in src["files"], "qualification: label source missing")
        file = regular(Path(src["root"]) / point["file"])
        sha = digest(file)
        require(sha == src["files"][point["file"]], "qualification: label source drift")
        candidates.append(dict(source=src, label=label, raw=row))
        evidence.append(dict(binary_sha256=binary, form=form, label=label, raw=row,
                             source_tree_sha256=states[index][0], source_state_sha256=states[index][1],
                             source_file_sha256=sha, receipt_sha256=runs[index].receipt_sha256,
                             sites_sha256=runs[index].expected("sites", "output")["sha256"]))
    output_dir = Path(output_dir)
    with claim_output(output_dir), TemporaryDirectory(prefix=".stage-", dir=output_dir) as directory:
        stage = Path(directory)
        inputs = prepare_inputs()
        response = launch(dict(inputs, operation="qualify", candidates=candidates), stage)
        require(response.get("qualified") == len(evidence), "qualification: incomplete controls")
        for index in sources:
            if source(runs[index], recheck=True)[1] != states[index]:
                raise OracleError(1, "source-drift: qualification source changed")
        recheck(inputs, None, None)
        outputs = []
        for index, record in enumerate(evidence):
            record["controls"] = {"match": "CORRECT", "contradiction": "WRONG"}
            filename = f"evidence-{index + 1}.json"
            raw = canonical(record) + b"\n"
            (stage / filename).write_bytes(raw)
            builds[record["binary_sha256"]][record["form"]] = dict(path=filename, sha256=digest(raw))
            outputs.append(filename)
        bundle = canonical(dict(schema="prism.measure.qualification/1", builds=builds)) + b"\n"
        (stage / "bundle.json").write_bytes(bundle)
        for filename in (*outputs, "bundle.json"):
            (stage / filename).replace(output_dir / filename)
    return digest(bundle)
