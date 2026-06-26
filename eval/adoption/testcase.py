# eval/adoption/testcase.py
"""Map a parsed Trajectory + Probe into a deepeval LLMTestCase (no-tracing path).
tools_called = prism nav calls only (the signal); skill-load goes in additional_metadata
for the custom SkillActivationMetric."""
from __future__ import annotations
from deepeval.test_case import LLMTestCase, ToolCall
from .model import Probe, Trajectory

def build_test_case(traj: Trajectory, probe: Probe) -> LLMTestCase:
    tools_called = [ToolCall(name=n) for n in traj.prism_nav_calls()]
    expected = [ToolCall(name=n) for n in probe.expected_tools]
    return LLMTestCase(
        input=probe.prompt,
        actual_output=traj.final_text or "(no answer)",
        tools_called=tools_called,
        expected_tools=expected,
        additional_metadata={
            "prism_skill_loaded": traj.loaded_prism_skill(),
            "probe_id": probe.id, "kind": probe.kind,
        },
    )
