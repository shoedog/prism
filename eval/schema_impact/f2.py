"""Write the F2 consumer census and reproducible reader sample."""

from collections import Counter
from dataclasses import asdict
from functools import cache
import json
from pathlib import Path
import random

from . import config, consumer_ts, match, schema


_SEED = 20261120
_SAMPLE_SIZE = 50


def _write_jsonl(path: Path, rows: list[dict]) -> None:
    with path.open("w", encoding="utf-8") as stream:
        for row in rows:
            stream.write(json.dumps(row, ensure_ascii=False) + "\n")


def run(repo: Path, root: str, schema_path: Path, config_path: Path, out: Path) -> None:
    schema_bytes = schema_path.read_bytes()
    idx = schema.load(schema_bytes)
    document = json.loads(schema_bytes)
    sites = consumer_ts.calls(repo, config.load(config_path), root)
    rows = []
    bound = []
    several = 0
    none_reasons: Counter[str] = Counter()
    for site in sites:
        grade, operation_ids, reason = match.bind(site, idx)
        row = asdict(site) | {"grade": grade, "operation_ids": operation_ids, "match_reason": reason}
        rows.append(row)
        if grade == "bound":
            bound.append((site, operation_ids[0]))
        elif len(operation_ids) > 1:
            several += 1
        else:
            none_reasons[reason] += 1

    all_unique_string_equal = all(
        site.url_class == "literal" and operation == f"op:{site.method} {site.pattern}"
        for site, operation in bound
    )
    counts = {
        "denominator": len(sites),
        "url_class": dict(Counter(site.url_class for site in sites)),
        "pre_match_reasons": dict(Counter(site.reason for site in sites if site.reason is not None)),
        "unique": len(bound),
        "several": several,
        "none": sum(none_reasons.values()),
        "none_reasons": dict(none_reasons),
        "all_unique_string_equal": all_unique_string_equal,
        "schema_parameter_operations": sum("{}" in operation for operation in idx.operations),
        "schema_sha256": idx.sha256,
    }
    if all_unique_string_equal:
        counts["unique_match_statement"] = "Every unique match uses string equality on the canonical path."

    @cache
    def source_lines(file: str) -> list[str]:
        return (repo / file).read_text(encoding="utf-8").splitlines()

    population = sorted(bound, key=lambda entry: (entry[0].file, entry[0].line))
    selected = random.Random(_SEED).sample(population, _SAMPLE_SIZE) if len(population) > _SAMPLE_SIZE else population
    sample = []
    for site, operation_id in selected:
        source_path = idx.operations[operation_id].source_path
        operation = document["paths"][source_path].get(site.method.lower(), {})
        summary = operation.get("summary")
        sample.append({
            "file": site.file, "line": site.line, "source_line": source_lines(site.file)[site.line - 1],
            "pattern": site.pattern, "method": site.method, "operation_id": operation_id,
            "schema_summary": summary,
        })

    out.mkdir(parents=True, exist_ok=True)
    _write_jsonl(out / "consumer.jsonl", rows)
    (out / "f2.json").write_text(json.dumps(counts, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    _write_jsonl(out / "f2-sample.jsonl", sample)
    _write_sample(out / "f2-sample.md", sample)


def _write_sample(path: Path, sample: list[dict]) -> None:
    with path.open("w", encoding="utf-8") as stream:
        stream.write("# F2 reader sample\n\n")
        stream.write("Does this call address this operation? Answer yes, no, or cannot tell, with one sentence.\n")
        for index, row in enumerate(sample, 1):
            summary = json.dumps(row["schema_summary"], ensure_ascii=False)
            stream.write(
                f"\n## {index}. `{row['file']}:{row['line']}`\n\n"
                f"```typescript\n{row['source_line']}\n```\n\n"
                f"- URL pattern and method: `{row['pattern']}` · `{row['method']}`\n"
                f"- Operation: `{row['operation_id']}`\n"
                f"- Schema summary: {summary}\n"
            )
