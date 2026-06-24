from tier_c.model import Variant, ArmOutput
from tier_c.detect import run_detectability

def _out(model, prism, text):
    return ArmOutput(Variant(model, prism), text, [], 0, 0, 0.0, prism)

class Guesser:  # guesses prism-on iff text contains "navfact"
    def guess_used_prism(self, text): return "navfact" in text

def _pool(separated: bool, n_pairs: int):
    # n_pairs prism-on (navfact) + n_pairs prism-off (plain). If separated, guesser is perfect.
    outs = []
    for _ in range(n_pairs):
        outs.append(_out("opus-4.8", True, "navfact" if separated else "plain"))
        outs.append(_out("gpt-5.5", False, "plain"))
    return outs

def test_detectability_detects_when_pooled_and_separable():
    outs = _pool(separated=True, n_pairs=4)   # n=8, all 8 guessed correctly
    r = run_detectability(outs, Guesser())
    assert r.correct == 8 and r.n == 8
    assert r.pvalue < 0.05 and r.detectable is True   # 0.5^8 ~= 0.0039

def test_detectability_not_flagged_at_chance():
    # guesser never sees "navfact" -> guesses all prism-off -> only the off arms are correct (4/8)
    outs = _pool(separated=False, n_pairs=4)
    r = run_detectability(outs, Guesser())
    assert r.correct == 4 and r.n == 8
    assert r.detectable is False

def test_detectability_underpowered_single_stage_cannot_fire():
    # n=4 (one stage), even perfect separation -> p=0.0625 > 0.05 -> NOT detectable (documents the pooling need)
    outs = _pool(separated=True, n_pairs=2)  # n=4
    r = run_detectability(outs, Guesser())
    assert r.correct == 4 and r.n == 4
    assert r.detectable is False
