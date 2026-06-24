from tier_c.report import prism_delta, gate_decision, StageMetrics, assemble_cell, Cell, assemble_cell_2x2

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

def test_assemble_cell_computes_prism_deltas_and_gate():
    # one stage, one language; precision ON vs OFF per model
    per_id = {
        "opus-4.8+prism": StageMetrics(precision=0.9, recall=0.8, planted=0.7, used_prism=True, tokens=100),
        "opus-4.8":       StageMetrics(precision=0.6, recall=0.5, planted=0.4, used_prism=False, tokens=90),
    }
    cell = assemble_cell(stage="spec", language="python", per_id=per_id,
                         models=["opus-4.8"], analyze_failure_rate=0.0, detectable=False)
    assert isinstance(cell, Cell)
    assert abs(cell.prism_precision_delta["opus-4.8"] - 0.3) < 1e-9
    assert cell.gate.decision == "GO"          # material lift, low failure
    assert cell.itt_available_rate == 0.5       # 1 of 2 variants had prism (opus-4.8+prism)
    assert cell.per_protocol_used_rate == 0.5   # 1 of 2 actually used it


def _sm(p): return StageMetrics(precision=p, recall=p, planted=0.0, used_prism=False, tokens=10)

def test_2x2_five_contrasts():
    per_id = {
        "opus-4.8": _sm(0.4), "opus-4.8+lsp": _sm(0.6),
        "opus-4.8+prism": _sm(0.7), "opus-4.8+prism+lsp": _sm(0.9),
    }
    c = assemble_cell_2x2(stage="spec", language="python", per_id=per_id, models=["opus-4.8"],
                          analyze_failure_rate=0.0, detectable=False)
    assert abs(c.prism_at_lsp_off["opus-4.8"] - 0.3) < 1e-9    # 0.7-0.4
    assert abs(c.prism_at_lsp_on["opus-4.8"]  - 0.3) < 1e-9    # 0.9-0.6  (primary gate)
    assert abs(c.lsp_at_prism_off["opus-4.8"] - 0.2) < 1e-9    # 0.6-0.4
    assert abs(c.interaction["opus-4.8"]      - 0.0) < 1e-9    # (0.9-0.6)-(0.7-0.4)
