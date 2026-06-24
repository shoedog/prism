from tier_c.planted import PlantedError, inject, score_catch, sanitation_ok

PLANTS = [
    PlantedError(kind="file", token="src/ghost_module.rs"),
    PlantedError(kind="function", token="frobnicate_nonexistent"),
]

def test_inject_appends_planted_refs():
    frame, plants = inject("Original spec text.", PLANTS)
    assert "src/ghost_module.rs" in frame and "frobnicate_nonexistent" in frame
    assert plants == PLANTS

def test_score_catch_counts_flagged_plants():
    out_text = "Note: src/ghost_module.rs does not exist; ignoring it."
    rep = score_catch(out_text, PLANTS)
    assert rep.caught == 1 and rep.total == 2
    assert abs(rep.recall - 0.5) < 1e-9

def test_sanitation_rejects_surviving_plant():
    # carried frame still references a planted token -> not clean
    assert not sanitation_ok("the plan uses frobnicate_nonexistent()", PLANTS)
    assert sanitation_ok("the plan uses real_function()", PLANTS)
