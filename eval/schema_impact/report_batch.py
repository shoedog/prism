"""Render impact reports and their separate replay companions."""

import argparse
from datetime import datetime, timezone
import gzip
import json
import re
from pathlib import Path

from ._replay_git import History, changed_files, git, parent, revision
from .replay import DEFAULT_SCHEMA
from .report_analysis import build_document
from .report_evaluation import join_evaluation, render_evaluation, replay_counterpart
from .report_inputs import ReportRequest, build_inputs
from .report_layout import label
from .report_markdown import render_json, render_markdown


def _write_json(out, name, document):
    encoded = render_json(document).encode("utf-8")
    (out / f"{name}.json.gz").write_bytes(gzip.compress(encoded, mtime=0))


def _live(out, name, document):
    (out / f"{name}.md").write_text(render_markdown(document), encoding="utf-8")
    _write_json(out, name, document)
    print(f"Rendered {name}.md", flush=True)


def _read_live(out, name):
    with gzip.open(out / f"{name}.json.gz", "rt", encoding="utf-8") as stream:
        return json.load(stream)


def _companion(out, name, index, document, *, assessed=False):
    suffix = f"{name}.evaluation"
    (out / f"{suffix}.md").write_text(render_evaluation(document), encoding="utf-8")
    _write_json(out, suffix, document)
    subject = document["subject_metadata"]
    description = f"{subject['committed_at'][:10]} · {label(subject['subject'])} · Service replay"
    evidence = "Tracked source edits and the study’s recorded assessments." if assessed else "Tracked source edits."
    index.append(f"- [{suffix}.md]({suffix}.md) · {description} · {evidence}")
    index.append(f"- [{suffix}.json.gz]({suffix}.json.gz) · {description} · Source lines and comparisons used by the study.")
    print(f"Rendered {suffix}.md with {len(document['rows'])} tracked request and element pairs", flush=True)


def _subject_index(path):
    for line in path.read_text(encoding="utf-8").splitlines():
        number, _, subject, _ = line.split("\t")
        yield number, subject


def _legacy_locations(path):
    if not path.is_file():
        return []
    pattern = re.compile(r"^- `(.+):(\d+)` for `([^\x60]+)`")
    return [(m[1], int(m[2]), m[3]) for line in path.read_text(encoding="utf-8").splitlines()
            if (m := pattern.match(line))]


def _assessments(source):
    rows = {}
    if source.is_file():
        for line in source.read_text(encoding="utf-8").splitlines():
            key, category, reason = line.split("\t", 2)
            rows[key] = {"class": category, "reason": reason}
    return rows


def _replay_results(args, subjects, pin):
    from . import replay
    from ._replay_git import History

    history = History(args.repo, pin)
    operations = replay.operation_sets(args.repo, history.commits)
    raw = {commit: replay.changes(args.repo, commit, operations, args.schema) for commit in history.commits}
    remaining = replay.usable(raw)
    for number, subject in subjects:
        base = parent(args.repo, subject)
        config = replay.select_config(args.repo, base, args.configs)
        yield number, replay.replay_one(args.repo, subject, remaining.get(subject, []), config,
                                        args.root, pin, schema_path=args.schema, history=history)


def _client_documents(args, pin, make):
    candidates = git(args.repo, "log", "--first-parent", "--format=%H", pin, "--", "ui/src/services").decode().splitlines()
    for commit in candidates:
        base = parent(args.repo, commit)
        if not base:
            continue
        paths = changed_files(args.repo, base, commit)
        if not paths or not all(file.startswith("ui/") for file in paths):
            continue
        patch = git(args.repo, "diff", "--no-ext-diff", "--no-textconv", base, commit, "--", "ui/src/services")
        if not re.search(rb"(?m)^\+[^+\n].*(?:\.(?:get|post|put|delete|patch)\(|\bfetch\()", patch):
            continue
        document = make(commit, "consumer")
        comparison = document["comparisons"][1]
        if any(comparison["matches"].get(row["after_site_id"], {}).get("status") in {"match", "partial"}
               and comparison["matches"][row["after_site_id"]]["target_ids"]
               for row in document["call_changes"]):
            yield commit, document


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


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--pin", required=True, help="Replay end revision, in the counterpart repository when supplied")
    parser.add_argument("--subjects", type=Path, required=True)
    parser.add_argument("--out-dir", type=Path, required=True)
    parser.add_argument("--configs", type=Path, default=Path(__file__).parent / "configs")
    parser.add_argument("--root", default="ui/src")
    parser.add_argument("--schema")
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
    elif args.source_url is None:
        args.source_url = "https://github.com/apache/answer"
    args.schema = args.schema or (None if args.counterpart_repo and args.route_spec else DEFAULT_SCHEMA)
    out = args.out_dir
    out.mkdir(parents=True, exist_ok=True)
    pin = revision(args.repo, args.pin) if args.counterpart_repo is None else None
    before_time = args.selection_time
    if args.counterpart_repo is None and before_time is None:
        before_time = datetime.now(timezone.utc).isoformat(timespec="seconds")

    def make(commit, role):
        return build_document(build_inputs(ReportRequest(
            args.repo, commit, role, args.configs, args.root, args.schema, selection_time=before_time,
            source_url=args.source_url, service_root=args.service_root, route_spec=args.route_spec,
            counterpart_repo=args.counterpart_repo, counterpart_rev=args.counterpart_rev,
            counterpart_source_url=args.counterpart_source_url,
        )))

    subjects = [(number, revision(args.repo, subject)) for number, subject in _subject_index(args.subjects)]
    live, index = {}, ["# Impact reports", ""]
    if args.counterpart_repo is None:
        index = ["# Public impact reports", "",
                 "Client subjects are the two most recent first-parent commits with only UI paths changed, "
                 "an added request expression in the services diff, and a readable after request matching "
                 "a service declaration or listed among the closest declarations read.", ""]
    for number, commit in subjects:
        name = f"{number}-{commit[:8]}"
        doc = make(commit, "producer")
        _live(out, name, doc)
        direction = "Service and client" if doc["change"]["subject_role"] == "both" else "Service change"
        subject = doc["change"]["subject"]
        description = f"{subject['committed_at'][:10]} · {label(subject['subject'])} · {direction}"
        index.append(f"- [{name}.md]({name}.md) · {description} · {_look(doc)}")
        index.append(f"- [{name}.json.gz]({name}.json.gz) · {description} · Snapshot and match evidence.")
        live[commit] = name

    if args.counterpart_repo is not None:
        # Open the replay end and later history only after writing the live reports.
        history = History(args.counterpart_repo, args.pin)
        end = history.detail(history.pin)
        observation_end = {"rev": end.commit, "committed_at": end.date, "subject": end.subject,
                           "repo": args.counterpart_repo.name}
        for name in live.values():
            doc = _read_live(out, name)
            companion = join_evaluation(doc, replay_counterpart(doc, history), observation_end)
            _companion(out, name, index, companion)
        (out / "index.md").write_text("\n".join(index) + "\n", encoding="utf-8")
        return 0

    clients = _client_documents(args, pin, make)
    for i in (1, 2):
        try:
            commit, doc = next(clients)
        except StopIteration:
            raise ValueError("Could not find two client subjects meeting the selection rule") from None
        name = f"client-{i}-{commit[:8]}"
        _live(out, name, doc)
        subject = doc["change"]["subject"]
        description = f"{subject['committed_at'][:10]} · {label(subject['subject'])} · Client change"
        index.append(f"- [{name}.md]({name}.md) · {description} · {_look(doc)}")
        index.append(f"- [{name}.json.gz]({name}.json.gz) · {description} · Snapshot and match evidence.")

    # Open replay and adjudication inputs only after each live file has been written.
    directory = args.subjects.parent
    assessments = _assessments(directory.parent / "f4-r3-labels.tsv")
    date, pin_subject = git(args.repo, "show", "-s", "--format=%cI%x00%s", pin).decode().rstrip("\n").split("\0")
    observation_end = {"rev": pin, "committed_at": date, "subject": pin_subject}
    for number, result in _replay_results(args, subjects, pin):
        name = live[result.commit]
        doc = _read_live(out, name)
        original = _legacy_locations(directory / f"{number}.md")
        current = [(row.call.file, row.call.line, row.element) for row in result.predicted]
        labels = assessments if original == current else {}
        companion = join_evaluation(doc, result, observation_end, labels, model="gpt-6.1-terra")
        _companion(out, name, index, companion, assessed=True)
    (out / "index.md").write_text("\n".join(index) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
