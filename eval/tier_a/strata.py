"""Strata (spec §2.5 precedence), M1 inventory diff (§2.4), seeded sampling."""
from __future__ import annotations

import os
import random
from dataclasses import dataclass, field

from .model import FunctionDef, Location, match_by_selection

STRATA = ("C-method", "C-name", "Q-scoped", "U-method", "U-free")


def is_nested(fd: FunctionDef, lang: str, package_dirs: set[str] | None = None) -> bool:
    f = fd.location.file
    if lang == "rust":
        # spec §2.5 + review m10: path-based; crate roots are the only non-nested files
        return f not in ("src/lib.rs", "src/main.rs")
    if lang == "go":
        return "/" in f
    if lang == "python":
        parts = f.split("/")[:-1]
        prefixes = {"/".join(parts[: i + 1]) for i in range(len(parts))}
        return bool(prefixes & (package_dirs or set()))
    raise ValueError(lang)


def classify(fd: FunctionDef, defs_per_name: dict, lang: str,
             package_dirs: set[str] | None = None) -> str:
    is_m = fd.kind in ("method", "constructor")
    if fd.name and defs_per_name.get(fd.name, 0) >= 2:
        return "C-method" if is_m else "C-name"
    if not is_m and is_nested(fd, lang, package_dirs):
        return "Q-scoped"
    return "U-method" if is_m else "U-free"


@dataclass
class InventoryDiff:
    matched: list = field(default_factory=list)        # (oracle_fd, prism_fd)
    prism_missing: list = field(default_factory=list)  # oracle-only
    prism_extra: list = field(default_factory=list)    # prism-only
    anon_oracle: int = 0
    anon_prism: int = 0


def inventory_diff(oracle: list[FunctionDef], prism: list[FunctionDef]) -> InventoryDiff:
    d = InventoryDiff()
    d.anon_oracle = sum(1 for f in oracle if f.name is None)
    d.anon_prism = sum(1 for f in prism if f.name is None)
    named_prism = [f for f in prism if f.name is not None]
    used: set[FunctionDef] = set()
    for ofd in oracle:
        if ofd.name is None:
            continue
        m = match_by_selection(ofd, [p for p in named_prism if p not in used])
        if m is None:
            d.prism_missing.append(ofd)
        else:
            used.add(m)
            d.matched.append((ofd, m))
    d.prism_extra = [p for p in named_prism if p not in used]
    return d


def _canon_path(path: str) -> str:
    out = path.replace(os.sep, "/")
    while out.startswith("./"):
        out = out[2:]
    return out


def _canon_record(r: FunctionDef) -> FunctionDef:
    file = _canon_path(r.location.file)
    if file == r.location.file:
        return r
    return FunctionDef(
        r.name,
        r.kind,
        r.container,
        Location(file, r.location.start_line, r.location.end_line),
        r.selection_line,
        r.selection_char,
    )


def filter_to_universe(records: list[FunctionDef],
                       universe_files: set[str]) -> list[FunctionDef]:
    """§2.4: apply the corpus universe filter to BOTH inventories. The runner MUST
    pass prism's `nav functions` output through this before inventory_diff."""
    files = {_canon_path(f) for f in universe_files}
    kept = [_canon_record(r) for r in records if _canon_path(r.location.file) in files]
    if records and universe_files and not kept:
        raise ValueError("filter_to_universe: empty intersection — path-form mismatch?")
    return kept


def sample_strata(oracle: list[FunctionDef], defs_per_name: dict, lang: str,
                  seed: int, per_stratum: int = 8,
                  package_dirs: set[str] | None = None) -> dict[str, list[FunctionDef]]:
    byst: dict[str, list[FunctionDef]] = {s: [] for s in STRATA}
    for f in sorted((f for f in oracle if f.name),
                    key=lambda f: (f.location.file, f.location.start_line, f.name)):
        byst[classify(f, defs_per_name, lang, package_dirs)].append(f)
    rng = random.Random(seed)
    return {s: (v if len(v) <= per_stratum else rng.sample(v, per_stratum))
            for s, v in byst.items()}
