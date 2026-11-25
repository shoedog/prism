"""Read later evidence after batch review files have been delivered."""

import gzip
import json
import re

from ._replay_git import History, git, parent, revision
from .report_evaluation import join_evaluation, render_evaluation, replay_counterpart
from .report_layout import label
from .report_markdown import render_json


def _read_live(out, name):
    with gzip.open(out / f"{name}.json.gz", "rt", encoding="utf-8") as stream:
        return json.load(stream)


def _companion(out, name, index, document, *, assessed=False):
    suffix = f"{name}.evaluation"
    (out / f"{suffix}.md").write_text(render_evaluation(document), encoding="utf-8")
    encoded = render_json(document).encode("utf-8")
    (out / f"{suffix}.json.gz").write_bytes(gzip.compress(encoded, mtime=0))
    subject = document["subject_metadata"]
    description = f"{subject['committed_at'][:10]} · {label(subject['subject'])} · Service replay"
    evidence = "Tracked source edits and the study’s recorded assessments." if assessed else "Tracked source edits."
    index.append(f"- [{suffix}.md]({suffix}.md) · {description} · {evidence}")
    index.append(f"- [{suffix}.json.gz]({suffix}.json.gz) · {description} · Source lines and comparisons used by the study.")
    print(f"Rendered {suffix}.md with {len(document['rows'])} tracked request and element pairs", flush=True)


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


def evaluate(args, subjects, live, index):
    out = args.out_dir
    if args.counterpart_repo is not None:
        history = History(args.counterpart_repo, args.pin)
        end = history.detail(history.pin)
        observation_end = {"rev": end.commit, "committed_at": end.date, "subject": end.subject,
                           "repo": args.counterpart_repo.name}
        for name in live.values():
            doc = _read_live(out, name)
            companion = join_evaluation(doc, replay_counterpart(doc, history), observation_end)
            _companion(out, name, index, companion)
        return
    pin = revision(args.repo, args.pin)
    if args.schema is None:
        index.append("Not checked — the single-repository replay requires a committed schema path.")
        return
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
