from tier_c.model import Issue, Variant, Citation, ArmOutput

def test_variant_id_is_stable_and_family_derived():
    v = Variant(model="opus-4.8", prism=True)
    assert v.id == "opus-4.8+prism"
    assert v.family == "anthropic"
    assert Variant(model="gpt-5.5", prism=False).family == "openai"

def test_armoutput_carries_text_and_citations():
    out = ArmOutput(
        variant=Variant("gpt-5.5", False),
        text="see src/a.py:10",
        citations=[Citation("src/a.py", 10, None)],
        tokens=123, tool_calls=4, wall_s=1.5, used_prism=False,
    )
    assert out.citations[0].file == "src/a.py"
    assert out.tokens == 123

def test_variant_lsp_dimension_id():
    assert Variant("opus-4.8", True, True).id == "opus-4.8+prism+lsp"
    assert Variant("opus-4.8", False, True).id == "opus-4.8+lsp"
    assert Variant("opus-4.8", True).id == "opus-4.8+prism"   # lsp defaults False, back-compat
    assert Variant("gpt-5.5", False, False).family == "openai"


def test_armoutput_raw_stdout_defaults_empty():
    """ArmOutput.raw_stdout defaults to '' so existing constructions keep working."""
    out = ArmOutput(
        variant=Variant("opus-4.8", True),
        text="spec text",
        citations=[],
        tokens=5,
        tool_calls=0,
        wall_s=0.0,
        used_prism=False,
    )
    assert out.raw_stdout == "", f"expected '' default, got {out.raw_stdout!r}"


def test_armoutput_raw_stdout_round_trips():
    """ArmOutput.raw_stdout carries whatever string is passed."""
    raw = '{"type":"result","subtype":"success"}\n{"type":"assistant"}\n'
    out = ArmOutput(
        variant=Variant("opus-4.8", True),
        text="spec",
        citations=[],
        tokens=3,
        tool_calls=0,
        wall_s=0.0,
        used_prism=False,
        raw_stdout=raw,
    )
    assert out.raw_stdout == raw
