from tier_a.accounting import CorpusAccounting, evaluate_floors


def test_probe_outcomes_and_rates():
    acc = CorpusAccounting()
    acc.record("s1", "ok")
    acc.record("s2", "oracle_error")
    acc.record("s3", "sut_error")
    acc.record("s4", "inventory_miss")
    assert acc.oracle_error_rate() == 0.25
    assert acc.sut_error_rate() == 0.25
    assert acc.successful() == 1
    assert acc.inventory_misses == ["s4"]


def test_inventory_miss_scores_all_oracle_edges_as_fn():
    # §2.5: an unmatched sampled symbol is not seeded; its oracle edges are ALL FN
    acc = CorpusAccounting()
    fn_sites = acc.score_inventory_miss(oracle_sites={("a.rs", 1), ("a.rs", 9)})
    assert fn_sites == {("a.rs", 1), ("a.rs", 9)}


def test_floors_failure_based_not_population_based():
    # stratum floor: successful >= min(6, eligible) — natural shortfall non-gating
    ok, reasons = evaluate_floors(
        strata={
            "U-free": {"eligible": 3, "successful": 3},
            "C-method": {"eligible": 8, "successful": 5},
        },
        oracle_error_rate=0.05,
        sut_error_rate=0.0,
        oracle_floor=0.10,
        sut_floor=0.05,
    )
    assert not ok and any("C-method" in r for r in reasons)


def test_floors_corpus_rates():
    ok, reasons = evaluate_floors(
        strata={"U-free": {"eligible": 8, "successful": 8}},
        oracle_error_rate=0.30,
        sut_error_rate=0.06,
        oracle_floor=0.25,
        sut_floor=0.05,
    )
    assert not ok and len(reasons) == 2


def test_floors_pass():
    ok, reasons = evaluate_floors(
        strata={"U-free": {"eligible": 8, "successful": 7}},
        oracle_error_rate=0.05,
        sut_error_rate=0.0,
        oracle_floor=0.10,
        sut_floor=0.05,
    )
    assert ok and reasons == []


def test_inventory_misses_fill_stratum_floor_when_scored():
    # Runner semantics: ok + inventory_miss both increment successful; the floor
    # guards unscored probes, not bad Prism recall.
    probes = {
        "p1": {"outcome": "ok", "stratum": "C-method"},
        "p2": {"outcome": "inventory_miss", "stratum": "C-method"},
        "p3": {"outcome": "inventory_miss", "stratum": "C-method"},
        "p4": {"outcome": "inventory_miss", "stratum": "C-method"},
        "p5": {"outcome": "inventory_miss", "stratum": "C-method"},
        "p6": {"outcome": "inventory_miss", "stratum": "C-method"},
    }
    successful = sum(p["outcome"] in {"ok", "inventory_miss"} for p in probes.values())
    ok, reasons = evaluate_floors(
        strata={"C-method": {"eligible": 8, "successful": successful}},
        oracle_error_rate=0.0,
        sut_error_rate=0.0,
        oracle_floor=0.10,
        sut_floor=0.05,
    )
    assert ok and reasons == []
