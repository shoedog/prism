# eval/adoption/goldens.py
from __future__ import annotations
import os, tomllib
from .model import Probe

_PATH = os.path.join(os.path.dirname(__file__), "goldens", "probes.toml")

def load_probes(path: str = _PATH) -> list[Probe]:
    with open(path, "rb") as f:
        data = tomllib.load(f)
    return [Probe(id=p["id"], kind=p["kind"], prompt=p["prompt"], repo=p["repo"],
                  expected_tools=list(p.get("expected_tools", [])),
                  expected_symbol=p.get("expected_symbol"))
            for p in data["probe"]]

_REALISTIC = os.path.join(os.path.dirname(__file__), "goldens", "realistic_prompts.toml")

def load_realistic_probes(path: str = _REALISTIC) -> list[Probe]:
    with open(path, "rb") as f:
        data = tomllib.load(f)
    return [Probe(id=p["id"], kind=p["kind"], prompt=p["prompt"], repo=p["repo"],
                  expected_tools=[], expected_symbol=None)
            for p in data["probe"]]
