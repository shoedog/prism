from tier_c.model import Variant, ArmOutput, Citation
from tier_c.detect import run_detectability

def _out(model, prism, text):
    return ArmOutput(Variant(model, prism), text, [], 0, 0, 0.0, prism)

class Guesser:  # guesses prism-on iff text contains "navfact"
    def guess_used_prism(self, text): return "navfact" in text

def test_detectability_counts_correct_guesses_and_pvalue():
    outs = [_out("opus-4.8", True, "navfact here"), _out("gpt-5.5", False, "plain"),
            _out("opus-4.8", False, "plain"), _out("gpt-5.5", True, "navfact")]
    r = run_detectability(outs, Guesser())
    assert r.correct == 4 and r.n == 4          # guesser perfectly separates -> detectable
    assert r.detectable is True
    assert r.pvalue < 0.1
