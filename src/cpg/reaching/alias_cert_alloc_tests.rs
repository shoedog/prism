//! #783 option A, slice 4a cost gate: heap allocations of the certificate stages, per population (`DESIGN-astra.md`
//! 5). A counting global allocator (this test binary only) counts the allocations of the two stage calls on
//! one thread. The pinned rule: an owner with no bare-identifier declarator allocates nothing in either stage.
use super::alias_cert::AliasCerts;
use super::census_defs_tests::{parse, recorded};
use crate::cpg::binding_facts::BindingFacts;
use crate::js_write_census::alias_cert::candidates;
use crate::js_write_census::refusals::classes;
use crate::js_write_census::tables::Tables;
use crate::js_write_census::walk::{census_mapped, Mapping};
use crate::languages::Language;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::BTreeSet;

struct Counting;

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}

fn bump(size: usize) {
    if ON.with(Cell::get) {
        COUNT.with(|c| {
            let (n, b) = c.get();
            c.set((n + 1, b + size));
        });
    }
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        bump(l.size());
        System.alloc(l)
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        bump(l.size());
        System.alloc_zeroed(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        bump(new);
        System.realloc(p, l, new)
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// `(allocations, bytes)` of `f`, on this thread.
fn measure<T>(f: impl FnOnce() -> T) -> (T, (usize, usize)) {
    COUNT.with(|c| c.set((0, 0)));
    ON.with(|o| o.set(true));
    let out = f();
    ON.with(|o| o.set(false));
    (out, COUNT.with(Cell::get))
}

/// `(stage 1, stage 2)` allocation counts and the number of candidates, for the first owner of `src`.
fn stages(src: &str) -> ((usize, usize), (usize, usize), usize) {
    let parsed = parse(Language::JavaScript, src);
    let owner = parsed.all_functions()[0];
    let (defs, edges) = recorded(&parsed, &owner);
    let bases: BTreeSet<&str> = defs.iter().map(|d| d.path.base.as_str()).collect();
    let census = census_mapped(&parsed, Tables::shared(), owner, Mapping::Bases(&bases));
    assert!(census.refusals.is_empty());
    let cl = classes(&parsed, owner, &census.sites, &bases);
    let (cands, one) = measure(|| candidates(&parsed, owner, &census.sites, &cl));
    let facts = BindingFacts::new(&parsed, &owner, &defs, edges.iter());
    let (_, two) = measure(|| AliasCerts::certify(&cands, &defs, &facts));
    (one, two, cands.len())
}

const NO_ALIAS: &str = "function g(o, p) {\n  o.x = 0;\n  o.y = 1;\n  p.x = o.x;\n  o.x = p.y;\n  o.z.w = 2;\n  const q = make(o);\n  q.x = 3;\n  return sink(o.x, q.x);\n}";
const ALL_REFUSED: &str =
    "function g(o, p) {\n  let u = o;\n  u = p;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}";
const ADMITTED_UNAFFECTED: &str =
    "function g(o) {\n  const u = o;\n  o.x = 0;\n  log(u.y);\n  return sink(o.x);\n}";
const ADMITTED_AFFECTED: &str =
    "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}";

#[test]
fn an_owner_with_no_alias_candidate_allocates_nothing_in_either_stage() {
    let (one, two, n) = stages(NO_ALIAS);
    assert_eq!((n, one, two), (0, (0, 0), (0, 0)));
}

#[test]
fn the_populations_with_candidates_allocate_per_candidate_only() {
    for (name, src) in [
        ("all-refused", ALL_REFUSED),
        ("admitted-no-affected-edge", ADMITTED_UNAFFECTED),
        ("admitted-affected-edges", ADMITTED_AFFECTED),
    ] {
        let (one, two, n) = stages(src);
        eprintln!("alloc {name}: candidates={n} stage1={one:?} stage2={two:?}");
        assert_eq!(n, 1, "{name}");
        // Stage 1: the candidate vector and its two names; stage 2: the entry vector and its clone.
        assert!(one.0 <= 5 && two.0 <= 6, "{name}: {one:?} {two:?}");
    }
}

/// Slice 4b: the refinement's own allocations, as the difference between the production pass and the same
/// pass without the refinement (`Pass::NoAliasKill`, which keeps the certificates), per population.
fn refinement_allocs(src: &str) -> (usize, usize) {
    use super::census_pass::Pass;
    let parsed = parse(Language::JavaScript, src);
    let owner = parsed.all_functions()[0];
    let (defs, edges) = recorded(&parsed, &owner);
    let cfg = crate::cfg::build_cfg_edges_with_arms(&parsed);
    let index = super::FileCfgEdgeIndex::new(&cfg);
    let mut run = |pass| {
        measure(|| {
            super::reaching_definitions_impl(
                &parsed,
                &owner,
                &defs,
                &edges,
                &[],
                &index,
                pass,
                &mut None,
            )
        })
        .1
    };
    let (with, without) = (run(Pass::Census), run(Pass::NoAliasKill));
    (
        with.0.wrapping_sub(without.0),
        with.1.wrapping_sub(without.1),
    )
}

#[test]
fn the_refinement_allocates_only_for_an_admitted_alias_with_a_write_through_it() {
    let none = refinement_allocs(NO_ALIAS);
    let refused = refinement_allocs(ALL_REFUSED);
    let unaffected = refinement_allocs(ADMITTED_UNAFFECTED);
    let affected = refinement_allocs(ADMITTED_AFFECTED);
    eprintln!("refine-alloc none={none:?} refused={refused:?} admitted-unaffected={unaffected:?} admitted-affected={affected:?}");
    assert_eq!(none, (0, 0));
    assert_eq!(refused, (0, 0));
    assert_eq!(unaffected, (0, 0));
    assert!(affected.0 > 0);
}
