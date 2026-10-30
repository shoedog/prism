"""Authenticated launch provenance consumed by measurement and validation."""
from __future__ import annotations


PROTOCOL = "tier-a-isolated-launch-v2"
_PROVENANCE = None


def authorize(provenance: dict) -> None:
    global _PROVENANCE
    _PROVENANCE = dict(provenance)


def launch_provenance() -> dict:
    if _PROVENANCE is None:
        return {"isolated": False, "prefix_fresh": False, "protocol": "missing"}
    return dict(_PROVENANCE)


def launch_refusal_reason(provenance) -> str | None:
    if not isinstance(provenance, dict):
        return "launch_invalid: missing"
    if provenance.get("isolated") is not True:
        return "launch_invalid: isolated"
    if provenance.get("prefix_fresh") is not True:
        return "launch_invalid: prefix_fresh"
    if provenance.get("protocol") != PROTOCOL:
        return "launch_invalid: protocol"
    return None
