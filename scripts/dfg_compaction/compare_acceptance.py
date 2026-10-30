"""Pre-registered numeric kernel; artifact admissibility is checked separately."""
from statistics import median
import math

def classify(base_heap, cand_heap, base_ms, cand_ms, base_peak, cand_peak, minimum):
    numbers = base_heap + cand_heap + base_ms + cand_ms + [base_peak, cand_peak, minimum]
    invalid = ((len(base_heap),len(cand_heap),len(base_ms),len(cand_ms)) != (2,2,2,2)
        or any(type(v) not in (int,float) or not math.isfinite(v) or v < 0 for v in numbers))
    if invalid or min(base_ms+cand_ms) <= 0:
        return dict.fromkeys(("heap", "load", "peak", "overall"), "INVALID")
    heap = "PASS" if min(base_heap)-max(cand_heap) >= minimum else "FAIL"
    peak = "PASS" if cand_peak <= base_peak else "FAIL"
    allowance = max(.05*median(base_ms), 100)
    if max(cand_ms) <= min(base_ms)+allowance:
        load = "PASS"
    elif any((max(v)-min(v))/median(v) > .10 for v in (base_ms,cand_ms)):
        load = "INCONCLUSIVE"
    else:
        load = "PASS" if median(cand_ms) <= median(base_ms)+allowance else "FAIL"
    verdicts = {"heap": heap, "load": load, "peak": peak}
    verdicts["overall"] = ("FAIL" if "FAIL" in verdicts.values() else
        "INCONCLUSIVE" if "INCONCLUSIVE" in verdicts.values() else "PASS")
    return verdicts

assert classify([1000,1000],[700,700],[1000,1000],[1100,1100],2000,2000,300)["overall"] == "PASS"
assert classify([1000,1000],[700,701],[1000,1000],[1000,1000],2000,2000,300)["heap"] == "FAIL"
assert classify([1000,1000],[600,600],[1000,1000],[800,1200],2000,2000,300)["overall"] == "INCONCLUSIVE"
assert classify([1000,1000],[600,600],[1000,1000],[1000,1000],2000,2001,300)["peak"] == "FAIL"
assert classify([1000,1000],[600,float('nan')],[1000,1000],[1000,1000],2000,2000,300)["overall"] == "INVALID"
assert classify([1000,1000],[900,900],[1000,1000],[800,1200],2000,2000,300) == {
    "heap":"FAIL", "load":"INCONCLUSIVE", "peak":"PASS", "overall":"FAIL"}
assert classify([1000,1000],[600,600],[3200,1800],[1000,1050],2000,2000,300)["load"] == "PASS"
assert classify([1000,1000],[600,600],[1000,1000],[800,1200],2000,2001,300)["overall"] == "FAIL"
assert classify([1000,1000],[600,600],[1000,1000],[1101,1101],2000,2000,300)["load"] == "FAIL"
assert classify([],[],[],[],2000,2000,300)["overall"] == "INVALID"
if __name__ == "__main__":
    print("acceptance classifier: 10 boundary assertions passed")
