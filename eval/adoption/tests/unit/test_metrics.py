# eval/adoption/tests/unit/test_metrics.py
from deepeval.test_case import LLMTestCase, ToolCall
from adoption.metrics import SkillActivationMetric

def _tc(loaded):
    return LLMTestCase(input="i", actual_output="o", tools_called=[ToolCall(name="nav_callers")],
                       expected_tools=[ToolCall(name="nav_callers")],
                       additional_metadata={"prism_skill_loaded": loaded})

def test_skill_activation_passes_when_loaded():
    m = SkillActivationMetric()
    assert m.measure(_tc(True)) == 1.0 and m.is_successful()

def test_skill_activation_fails_when_not_loaded():
    m = SkillActivationMetric()
    assert m.measure(_tc(False)) == 0.0 and not m.is_successful()
    assert "not load" in m.reason.lower()
