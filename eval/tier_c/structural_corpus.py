"""Part-D structural-task registry loader (design-of-record §2 admission gate,
§7 first slice). Mirrors `corpus.py`'s loader style: parse `issues/structural.toml`
into frozen `StructuralTask` records. The admission gate itself (gold size,
D-share, recency, scale) is applied by the controller when authoring the corpus,
not re-derived here — this loader only enforces the STRUCTURAL shape so a
malformed entry fails loudly at load time."""
from __future__ import annotations
import tomllib
from dataclasses import dataclass
from pathlib import Path


class StructuralCorpusError(Exception): ...


@dataclass(frozen=True)
class StructuralTask:
    id: str
    repo: str
    lang: str
    sha: str
    symbol: str
    receiver: str
    def_site: tuple[str, int]   # (repo-relative file, 1-indexed line)
    dispatch: str
    prompt_change: str
    grep_name_stats: str
    notes: str = ""


_REQUIRED = ("id", "repo", "lang", "sha", "symbol", "receiver", "def_site",
             "dispatch", "prompt_change", "grep_name_stats")


def _parse_def_site(raw: str, task_id: str) -> tuple[str, int]:
    if ":" not in raw:
        raise StructuralCorpusError(
            f"{task_id}: def_site {raw!r} must be 'path/to/file.ext:LINE'")
    file, _, line_s = raw.rpartition(":")
    if not file or not line_s.isdigit():
        raise StructuralCorpusError(
            f"{task_id}: def_site {raw!r} must be 'path/to/file.ext:LINE'")
    return file, int(line_s)


def load_structural_tasks(path: str | Path) -> list[StructuralTask]:
    raw = tomllib.loads(Path(path).read_text())
    out: list[StructuralTask] = []
    for d in raw.get("task", []):
        task_id = d.get("id", "<no id>")
        for req in _REQUIRED:
            if not d.get(req):
                raise StructuralCorpusError(f"{task_id}: missing required field {req!r}")
        def_site = _parse_def_site(d["def_site"], task_id)
        out.append(StructuralTask(
            id=d["id"], repo=d["repo"], lang=d["lang"], sha=d["sha"],
            symbol=d["symbol"], receiver=d["receiver"], def_site=def_site,
            dispatch=d["dispatch"], prompt_change=d["prompt_change"],
            grep_name_stats=d["grep_name_stats"], notes=d.get("notes", ""),
        ))
    if not out:
        raise StructuralCorpusError("no [[task]] entries found")
    return out
