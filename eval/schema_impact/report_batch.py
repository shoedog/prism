"""Render impact reports and their separate replay companions."""

import argparse
import gzip
from pathlib import Path
import subprocess

from ._replay_git import revision
from .report_analysis import build_document
from .report_inputs import ReportRequest, build_inputs, selected_schema
from .report_layout import label
from .report_markdown import render_json, render_markdown


def _write_json(out, name, document):
    encoded = render_json(document).encode("utf-8")
    (out / f"{name}.json.gz").write_bytes(gzip.compress(encoded, mtime=0))


def _live(out, name, document):
    (out / f"{name}.md").write_text(render_markdown(document), encoding="utf-8")
    _write_json(out, name, document)
    print(f"Rendered {name}.md", flush=True)


def _subject_index(path):
    for line in path.read_text(encoding="utf-8").splitlines():
        number, _, subject, _ = line.split("\t")
        yield number, subject


def _look(document):
    groups = document["impacts"]
    if groups:
        first = next((group for group in groups if group["change_kinds"]), groups[0])
        basis = first["change_kinds"][0] if first["change_kinds"] else "Coverage limit"
        return f"{basis} for `{first['operation_id'].removeprefix('op:')}`."
    edits = document["call_changes"]
    if edits:
        return f"{edits[0]['kind']} · {edits[0]['review_action']}"
    return "Versions checked and the limits on the observations."


def _index_report(index, name, doc):
    direction = {"both": "Service and client", "service": "Service change", "client": "Client change"}
    subject = doc["change"]["subject"]
    role = direction[doc["change"]["subject_role"]]
    description = f"{subject['committed_at'][:10]} · {label(subject['subject'])} · {role}"
    index.append(f"- [{name}.md]({name}.md) · {description} · {_look(doc)}")
    index.append(f"- [{name}.json.gz]({name}.json.gz) · {description} · Snapshot and match evidence.")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--pin", required=True, help="Replay end revision, in the counterpart repository when supplied")
    parser.add_argument("--subjects", type=Path, required=True)
    parser.add_argument("--client-subject", action="append", default=[],
                        help="An independently selected client commit (repeat for each client report)")
    parser.add_argument("--out-dir", type=Path, required=True)
    parser.add_argument("--configs", type=Path, default=Path(__file__).parent / "configs")
    parser.add_argument("--root", default="ui/src")
    parser.add_argument("--schema")
    parser.add_argument("--no-schema", action="store_true")
    parser.add_argument("--route-spec", type=Path)
    parser.add_argument("--service-root", default=".")
    parser.add_argument("--source-url")
    parser.add_argument("--counterpart-repo", type=Path)
    parser.add_argument("--counterpart-rev")
    parser.add_argument("--counterpart-source-url")
    parser.add_argument("--selection-time")
    args = parser.parse_args(argv)
    args.repo = args.repo.resolve()
    if args.counterpart_repo is None and (args.counterpart_rev is not None or args.counterpart_source_url is not None):
        parser.error("--counterpart-rev and --counterpart-source-url require --counterpart-repo")
    if args.counterpart_repo is not None:
        args.counterpart_repo = args.counterpart_repo.resolve()
    args.schema = selected_schema(args.schema, args.route_spec, omitted=args.no_schema)
    out = args.out_dir
    out.mkdir(parents=True, exist_ok=True)

    def make(commit, role):
        return build_document(build_inputs(ReportRequest(
            args.repo, commit, role, args.configs, args.root, args.schema, selection_time=args.selection_time,
            source_url=args.source_url, service_root=args.service_root, route_spec=args.route_spec,
            counterpart_repo=args.counterpart_repo, counterpart_rev=args.counterpart_rev,
            counterpart_source_url=args.counterpart_source_url, schema_omitted=args.no_schema,
        )))

    subjects = [(number, revision(args.repo, subject)) for number, subject in _subject_index(args.subjects)]
    clients = [revision(args.repo, commit) for commit in args.client_subject]
    live, index = {}, ["# Impact reports", "", "## Live reports", ""]
    for number, commit in subjects:
        name = f"{number}-{commit[:8]}"
        doc = make(commit, "producer")
        _live(out, name, doc)
        _index_report(index, name, doc)
        live[commit] = name

    for i, commit in enumerate(clients, 1):
        doc = make(commit, "consumer")
        name = f"client-{i}-{commit[:8]}"
        _live(out, name, doc)
        _index_report(index, name, doc)

    index_path = out / "index.md"
    index_path.write_text("\n".join(index) + "\n", encoding="utf-8")

    from .report_batch_evaluation import evaluate

    companions = ["# Evaluation companions (uses later history)", ""]
    companion_path = out / "evaluation-index.md"
    try:
        evaluate(args, subjects, live, companions)
    except subprocess.CalledProcessError as error:
        companions.append(f"Not checked — the evaluation command failed with status {error.returncode}.")
        companion_path.write_text("\n".join(companions) + "\n", encoding="utf-8")
        return 1
    companion_path.write_text("\n".join(companions) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
