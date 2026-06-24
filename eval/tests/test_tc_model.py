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
