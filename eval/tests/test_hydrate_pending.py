from tools.hydrate_pending import hydrate_record


def test_hydrate_record_preserves_tier_when_present():
    # P6a: cli._pending_for_probe tags candidate-tier prism_only pendings with a
    # "tier" key (exact-tier pendings omit it entirely). hydrate_record rebuilds
    # the record from scratch, so it must copy "tier" through explicitly or a
    # hydrated candidate silently loses its confidence tier.
    rec = {
        "corpus": "ripgrep",
        "measurement": "callers",
        "direction": "prism_only",
        "seed_def": "src/main.rs:10",
        "site": "src/main.rs:20",
        "tier": "candidate",
    }
    out = hydrate_record(rec, 0, "/nonexistent/root", seed_ctx=2, site_ctx=3)
    assert out["tier"] == "candidate"


def test_hydrate_record_omits_tier_when_absent():
    rec = {
        "corpus": "ripgrep",
        "measurement": "callers",
        "direction": "prism_only",
        "seed_def": "src/main.rs:10",
        "site": "src/main.rs:20",
    }
    out = hydrate_record(rec, 0, "/nonexistent/root", seed_ctx=2, site_ctx=3)
    assert "tier" not in out
