"""Planted-error sensitivity probe (spec §6a). DIAGNOSTIC metric (codex new-4): catch-rate
is reported alongside real task correctness, not as standalone value. Sanitation gate
(codex new-5) guarantees zero planted residue before a frame is carried forward."""
from __future__ import annotations
from dataclasses import dataclass

@dataclass(frozen=True)
class PlantedError:
    kind: str    # file|function|variable|claim
    token: str   # the invalid reference text

@dataclass(frozen=True)
class PlantedReport:
    caught: int
    total: int
    @property
    def recall(self) -> float:
        return self.caught / self.total if self.total else 0.0

_FLAG_WORDS = ("does not exist", "doesn't exist", "no such", "invalid", "nonexistent",
               "not found", "incorrect", "wrong", "ignore", "remove", "typo")

def inject(frame: str, plants: list[PlantedError]) -> tuple[str, list[PlantedError]]:
    salt = "\n\n[references to verify] " + ", ".join(p.token for p in plants)
    return frame + salt, list(plants)

def score_catch(out_text: str, plants: list[PlantedError]) -> PlantedReport:
    low = out_text.lower()
    caught = 0
    for p in plants:
        i = low.find(p.token.lower())
        if i == -1:
            continue
        window = low[max(0, i - 80): i + len(p.token) + 80]
        if any(w in window for w in _FLAG_WORDS):
            caught += 1
    return PlantedReport(caught=caught, total=len(plants))

def sanitation_ok(carried_frame: str, plants: list[PlantedError]) -> bool:
    low = carried_frame.lower()
    return not any(p.token.lower() in low for p in plants)
