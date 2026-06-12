"""Probe accounting + validity floors (spec §2.2/§2.5, review B2).

``inventory_miss`` is recorded by the runner when a sampled oracle symbol has no
Prism inventory match. An SUT adapter returning no callees is not itself an
outcome. Inventory-miss probes score all oracle edges as false negatives and do
not count as SUT errors; they are tracked as their own first-class outcome. The
runner counts them as floor-successful because they are fully scored probes; the
stratum floor guards unscored probes, not bad Prism results.
"""
from __future__ import annotations

from dataclasses import dataclass, field

OUTCOMES = ("ok", "oracle_error", "sut_error", "inventory_miss")


@dataclass
class CorpusAccounting:
    probes: dict[str, str] = field(default_factory=dict)
    inventory_misses: list[str] = field(default_factory=list)

    def record(self, probe_id: str, outcome: str) -> None:
        assert outcome in OUTCOMES, outcome
        self.probes[probe_id] = outcome
        if outcome == "inventory_miss":
            self.inventory_misses.append(probe_id)

    def _rate(self, outcome: str) -> float:
        if not self.probes:
            return 0.0
        return sum(1 for recorded in self.probes.values() if recorded == outcome) / len(
            self.probes
        )

    def oracle_error_rate(self) -> float:
        return self._rate("oracle_error")

    def sut_error_rate(self) -> float:
        return self._rate("sut_error")

    def successful(self) -> int:
        return sum(1 for outcome in self.probes.values() if outcome == "ok")

    def score_inventory_miss(self, oracle_sites: set) -> set:
        """§2.5: Prism couldn't be seeded; every oracle site is a recall miss."""
        return set(oracle_sites)


def evaluate_floors(
    strata: dict,
    oracle_error_rate: float,
    sut_error_rate: float,
    oracle_floor: float,
    sut_floor: float,
) -> tuple[bool, list[str]]:
    """§2.2 failure-based floors. Returns (baseline_valid, reasons)."""
    reasons = []
    for name, stratum in strata.items():
        need = min(6, stratum["eligible"])
        if stratum["successful"] < need:
            reasons.append(
                f"stratum {name}: {stratum['successful']}/{need} successful probes"
            )
    if oracle_error_rate > oracle_floor:
        reasons.append(f"oracle_error_rate {oracle_error_rate:.2f} > {oracle_floor:.2f}")
    if sut_error_rate > sut_floor:
        reasons.append(f"sut_error_rate {sut_error_rate:.2f} > {sut_floor:.2f}")
    return (not reasons, reasons)
