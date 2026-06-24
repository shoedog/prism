from tier_c.report import prism_delta, gate_decision

def test_prism_delta_is_within_model_on_minus_off():
    # precision: opus+prism 0.9 vs opus(off) 0.6 -> +0.3
    by_id = {"opus-4.8+prism": 0.9, "opus-4.8": 0.6}
    assert abs(prism_delta(by_id, "opus-4.8") - 0.3) < 1e-9

def test_gate_go_when_material_lift_and_low_failure():
    d = gate_decision(precision_delta=0.25, recall_delta=0.2, planted_delta=0.3,
                      analyze_failure_rate=0.0, cost_ok=True, detectable_judges=False)
    assert d.decision == "GO"

def test_gate_nogo_when_high_analyze_failure():
    d = gate_decision(precision_delta=0.25, recall_delta=0.2, planted_delta=0.3,
                      analyze_failure_rate=0.6, cost_ok=True, detectable_judges=False)
    assert d.decision == "NO-GO"
    assert "coverage" in d.reason.lower()

def test_gate_nogo_when_flat():
    d = gate_decision(precision_delta=0.0, recall_delta=0.0, planted_delta=0.0,
                      analyze_failure_rate=0.0, cost_ok=True, detectable_judges=False)
    assert d.decision == "NO-GO"
