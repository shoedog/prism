import pytest

from tier_a.model import FunctionDef, Location
from tier_a.strata import classify, inventory_diff, sample_strata


def fd(name, file, kind="function", container=None, start=1, end=5, sel=None):
    return FunctionDef(name, kind, container, Location(file, start, end),
                       sel if sel is not None else start)


def counts(fds):
    c = {}
    for f in fds:
        c[f.name] = c.get(f.name, 0) + 1
    return c


def test_precedence_collision_beats_scoped_and_unique():
    fds = [fd("run", "pkg/a/x.go", kind="method"),
           fd("run", "pkg/b/y.go"),
           fd("solo", "pkg/c/z.go"),
           fd("top", "main.go")]
    n = counts(fds)
    assert classify(fds[0], n, "go") == "C-method"
    assert classify(fds[1], n, "go") == "C-name"
    assert classify(fds[2], n, "go") == "Q-scoped"   # unique free fn, subdir package
    assert classify(fds[3], n, "go") == "U-free"


def test_python_q_scoped_requires_package_dir():
    # is_nested for python: any ancestor dir with __init__.py in the universe
    fds = [fd("f", "pkg/mod.py"), fd("g", "script.py")]
    n = counts(fds)
    assert classify(fds[0], n, "python", package_dirs={"pkg"}) == "Q-scoped"
    assert classify(fds[1], n, "python", package_dirs={"pkg"}) == "U-free"


def test_rust_unique_method_is_u_method_not_q():
    f = fd("only", "src/deep/mod.rs", kind="method")
    assert classify(f, {"only": 1}, "rust") == "U-method"


def test_inventory_diff_uses_selection_containment():
    oracle = [fd("a", "src/x.rs", start=5, end=20, sel=9)]   # doc-comment offset
    prism = [fd("a", "src/x.rs", start=9, end=20, sel=9)]
    d = inventory_diff(oracle, prism)
    assert d.matched and not d.prism_missing and not d.prism_extra


def test_sampling_is_deterministic_and_respects_shortfall():
    # all at src/lib.rs: crate root, so unique free fns land in U-free (not Q-scoped)
    fds = [fd(f"u{i}", "src/lib.rs", start=1 + 2 * i, end=2 + 2 * i) for i in range(20)]
    fds += [fd("dup", "src/a.rs"), fd("dup", "src/b.rs", start=9, end=12)]
    n = counts(fds)
    s1 = sample_strata(fds, n, "rust", seed=42, per_stratum=8)
    s2 = sample_strata(fds, n, "rust", seed=42, per_stratum=8)
    assert s1 == s2
    assert len(s1["C-name"]) == 2   # shortfall: takes all eligible
    assert len(s1["U-free"]) == 8


def test_filter_to_universe_drops_out_of_universe_prism_records():
    # §2.4: the SAME include/exclude filter applies to prism's inventory — without
    # this, prism's whole-repo walk floods prism_extra with tests/fixtures/ and
    # eval/fixtures/ records on the prism corpus (review M5).
    from tier_a.strata import filter_to_universe
    recs = [fd("a", "src/lib.rs"), fd("b", "tests/fixtures/x.py"),
            fd("c", "eval/fixtures/rust/free_fn_same_file/main.rs")]
    kept = filter_to_universe(recs, universe_files={"src/lib.rs"})
    assert [r.name for r in kept] == ["a"]


def test_filter_to_universe_canonicalizes_leading_dot():
    from tier_a.strata import filter_to_universe
    kept = filter_to_universe([fd("a", "./src/a.rs")], universe_files={"src/a.rs"})
    assert [r.name for r in kept] == ["a"]
    assert kept[0].location.file == "src/a.rs"


def test_filter_to_universe_empty_intersection_raises():
    from tier_a.strata import filter_to_universe
    with pytest.raises(ValueError, match="empty intersection"):
        filter_to_universe([fd("a", "outside.rs")], universe_files={"src/a.rs"})
