//! #602 S4b2 round controls. REAL: a temporary repository loaded by `load_review_inputs`, the
//! union map built from its originals plus files read through S4a's `loader.read`, call sites
//! from the core `CallGraph::build`. CONTROLLED / SYNTHETIC cases are labelled where used.
//! Every real fixture asserts its own preconditions first ("inadmissible precondition:").

use super::*;
use crate::api::review_loading::{capture, observe_root};
use crate::api::{
    build_context, load_review_inputs, ContextReadOutcome, ContextUnavailable, ReviewBarrelHop,
    ReviewHintBasis, ReviewImportHint, ReviewInputs, ReviewOptions,
};
use crate::call_graph::{CallGraph, CallSite};
use crate::cpg_cache::{self, CacheResult};
use crate::languages::Language;
use crate::type_provider::{LanguageVersion, TypeResolutionMode};
use std::path::{Path, PathBuf};

const BUDGET: u64 = 1 << 20;

const CHAIN: [(&str, &str); 3] = [
    (
        "main.js",
        "import { fa } from './a';\nexport function main(x) { fa(x); }\n",
    ),
    (
        "a.js",
        "import { fb } from './b';\nexport function fa(x) { fb(x); }\n",
    ),
    ("b.js", "export function fb(x) {}\n"),
];
const LONG: [(&str, &str); 6] = [
    (
        "main.js",
        "import { f1 } from './a';\nexport function f0(x) { f1(x); }\n",
    ),
    (
        "a.js",
        "import { f2 } from './b';\nexport function f1(x) { f2(x); }\n",
    ),
    (
        "b.js",
        "import { f3 } from './c';\nexport function f2(x) { f3(x); }\n",
    ),
    (
        "c.js",
        "import { f4 } from './d';\nexport function f3(x) { f4(x); }\n",
    ),
    (
        "d.js",
        "import { f5 } from './e';\nexport function f4(x) { f5(x); }\n",
    ),
    ("e.js", "export function f5(x) {}\n"),
];
const A_V2: &str = "import { f2 } from './b';\nimport { f5 } from './e';\nexport function f1(x) { f2(x); f5(x); }\n";
const ALIAS: [(&str, &str); 4] = [
    (
        "tsconfig.json",
        r#"{"compilerOptions":{"moduleResolution":"node10","baseUrl":".","paths":{"@lib/*":["src/lib/*"]}}}"#,
    ),
    (
        "src/app.ts",
        "import { run } from './mid';\nexport function h(x: string) { run(x); }\n",
    ),
    (
        "src/mid.ts",
        "import { f } from '@lib/cmd';\nexport function run(x: string) { f(x); }\n",
    ),
    ("src/lib/cmd.ts", "export function f(x: string) {}\n"),
];
const GO: [(&str, &str); 6] = [
    ("go.mod", "module example.com/app\n\ngo 1.21\n"),
    (
        "main.go",
        "package main\n\nimport \"example.com/app/svc\"\n\nfunc h(x string) {\n\tsvc.Do(x)\n}\n",
    ),
    (
        "svc/svc.go",
        "package svc\n\nimport \"example.com/app/util\"\n\nfunc Do(x string) {\n\tutil.Pick(x)\n\tutil.Run(x)\n\tutil.Gone(x)\n}\n",
    ),
    ("util/b.go", "package util\n\nfunc Pick(x string) {}\n"),
    ("util/run_linux.go", "package util\n\nfunc Run(x string) {}\n"),
    ("util/run_windows.go", "package util\n\nfunc Run(x string) {}\n"),
];
const PY: [(&str, &str); 4] = [
    ("pkg/__init__.py", ""),
    (
        "pkg/main.py",
        "from .mid import go\n\n\ndef h(x):\n    go(x)\n",
    ),
    (
        "pkg/mid.py",
        "from .leaf import work\n\n\ndef go(x):\n    work(x)\n",
    ),
    ("pkg/leaf.py", "def work(x):\n    pass\n"),
];
const BARREL: [(&str, &str); 9] = [
    (
        "tsconfig.json",
        r#"{"compilerOptions":{"moduleResolution":"node10","baseUrl":".","paths":{"@two/*":["one/*","two/*"]}}}"#,
    ),
    ("main.js", "export const main = 1;\n"),
    (
        "pkg/index.js",
        "export { g as h } from './impl';\nexport * from './star';\nexport { dup } from './impl';\nexport { dup } from './star';\nexport { multi } from '@two/m';\n",
    ),
    ("pkg/other.js", "export { g as h } from './impl';\n"),
    ("pkg/impl.js", "export function g(x) {}\n"),
    ("pkg/star.js", "export function starred(x) {}\nexport function dup(x) {}\n"),
    ("one/m.js", "export function multi(x) {}\n"),
    ("two/m.js", "export function multi(x) {}\n"),
    (
        "probe.js",
        "import { multi } from '@two/m';\nexport function p(x) { multi(x); }\n",
    ),
];
const HOPS: [(&str, &str); 6] = [
    ("main.js", "export const main = 1;\n"),
    ("lib/b1.js", "export { q as f } from './b2';\n"),
    ("lib/b2.js", "export { g as q } from './impl';\n"),
    ("lib/impl.js", "export function g(x) {}\n"),
    ("lib/c1.js", "export { x } from './c2';\n"),
    ("lib/c2.js", "export { x } from './c1';\n"),
];

fn write(root: &Path, files: &[(&str, &str)]) {
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

/// REAL: files under `<tmp>/repo`, the named subset loaded as the review's diff (round zero).
fn fixture(files: &[(&str, &str)], loaded: &[&str]) -> (tempfile::TempDir, ReviewInputs) {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    write(&repo, files);
    let diff = serde_json::json!({"files": loaded.iter().map(|file| serde_json::json!({
        "file_path": file, "modify_type": "Modified", "diff_lines": []
    })).collect::<Vec<_>>()});
    (
        dir,
        load_review_inputs(&ReviewOptions::new(repo), &diff.to_string()).unwrap(),
    )
}

fn repo(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("repo")
}

fn keys(files: &BTreeMap<String, ParsedFile>) -> Vec<&str> {
    files.keys().map(String::as_str).collect()
}

/// REAL: the originals plus `extra`, each read through S4a's reader at its current bytes.
fn union(inputs: &ReviewInputs, extra: &[&str]) -> BTreeMap<String, ParsedFile> {
    let loader = ReviewContextLoader::new(inputs).unwrap();
    let mut map = inputs.files.clone();
    for path in extra {
        match loader.read(path, BUDGET).outcome {
            ContextReadOutcome::Loaded(file) => {
                map.insert(file.path, file.parsed);
            }
            _ => panic!("inadmissible precondition: the reader refused {path}"),
        }
    }
    map
}

/// The one call to `callee` made inside `file`, from a core-built call graph.
fn call_in(files: &BTreeMap<String, ParsedFile>, file: &str, callee: &str) -> CallSite {
    let graph = CallGraph::build(files);
    let mut found = graph
        .calls
        .values()
        .flatten()
        .filter(|c| c.caller.file == file && c.callee_name == callee);
    let call = found
        .next()
        .unwrap_or_else(|| panic!("inadmissible precondition: no call to {callee} in {file}"));
    assert!(
        found.next().is_none(),
        "inadmissible precondition: {callee} is called twice in {file}"
    );
    call.clone()
}

/// The `InRepo` files of a hint; `None` for no hint.
fn in_repo_files(hint: Option<ReviewImportHint>) -> Option<Vec<String>> {
    match hint {
        None => None,
        Some(ReviewImportHint::InRepo { files, .. }) => Some(files),
        Some(other) => panic!("expected an in-repo hint or none, got {other:?}"),
    }
}

fn hop(file: &str, member: &str) -> Option<ReviewBarrelHop> {
    Some(ReviewBarrelHop {
        file: file.to_string(),
        member: member.to_string(),
    })
}

/// What round zero publishes that a round must not move, as text.
fn round_zero(inputs: &ReviewInputs, loader: &ReviewContextLoader<'_>) -> String {
    let sources: Vec<_> = inputs
        .files
        .iter()
        .map(|(path, file)| (path, &file.source))
        .collect();
    let scope = &inputs.scope_graph_inputs;
    format!(
        "{sources:?}\n{:?}\n{:?}\n{:?}\n{:?}",
        inputs.sources,
        scope.all_file_paths,
        scope.js_paths_snapshot.topology(),
        loader.captured_scope().js_paths_snapshot.topology()
    )
}

// H01
#[test]
fn round_hints_follow_a_context_caller() {
    let (_dir, inputs) = fixture(&CHAIN, &["main.js"]);
    assert_eq!(
        keys(&inputs.files),
        ["main.js"],
        "inadmissible precondition: only main.js is original"
    );
    let loader = ReviewContextLoader::new(&inputs).unwrap();
    let all = union(&inputs, &["a.js", "b.js"]);
    let (into_a, into_b) = (call_in(&all, "main.js", "fa"), call_in(&all, "a.js", "fb"));
    // The present defect: the legacy handle has no caller entry for a.js.
    let mut legacy = ReviewBoundaryHints::new(&inputs).unwrap();
    assert_eq!(
        legacy.hint_for_call(&into_b),
        None,
        "inadmissible precondition: the legacy hints already see the context caller"
    );
    assert_eq!(
        in_repo_files(legacy.hint_for_call(&into_a)).unwrap(),
        ["a.js"],
        "inadmissible precondition: a.js is not yet loaded for the legacy hints"
    );
    // Round over {main, a}: the call in a.js hints b.js, the call into the loaded a.js is silent.
    let main_a = union(&inputs, &["a.js"]);
    let round = ReviewContextRound::new(&loader, &main_a);
    let mut hints = round.hints().unwrap();
    assert!(
        matches!(hints.hint_for_call(&into_b), Some(ReviewImportHint::InRepo { files, spec, member, basis: ReviewHintBasis::Bound, .. })
            if files == ["b.js"] && spec == "./b" && member.as_deref() == Some("fb"))
    );
    assert_eq!(hints.hint_for_call(&into_a), None);
    // Round over {main, a, b}: b.js is loaded as well.
    let round = ReviewContextRound::new(&loader, &all);
    assert_eq!(round.hints().unwrap().hint_for_call(&into_b), None);
}

// H02
#[test]
fn round_hints_leave_round_zero_unchanged() {
    let (_dir, inputs) = fixture(&ALIAS, &["src/app.ts"]);
    let loader = ReviewContextLoader::new(&inputs).unwrap();
    let map = union(&inputs, &["src/mid.ts"]);
    let (original, context) = (
        call_in(&map, "src/app.ts", "run"),
        call_in(&map, "src/mid.ts", "f"),
    );
    let legacy = || {
        in_repo_files(
            ReviewBoundaryHints::new(&inputs)
                .unwrap()
                .hint_for_call(&original),
        )
    };
    let (before, answer) = (round_zero(&inputs, &loader), legacy());
    assert_eq!(
        answer.as_deref(),
        Some(&["src/mid.ts".to_string()][..]),
        "inadmissible precondition: the legacy answer for the original caller"
    );
    let round = ReviewContextRound::new(&loader, &map);
    let found = in_repo_files(round.hints().unwrap().hint_for_call(&context));
    assert_eq!(found.unwrap(), ["src/lib/cmd.ts"], "the round's own hint");
    assert_eq!(round_zero(&inputs, &loader), before);
    assert_eq!(legacy(), answer);
}

// H03: the hint row is REAL. Hints read only captured config bytes, the root and the listing, so
// no hint can tell a rebased scope from the captured one; the second part is CONTROLLED (it reads
// the round's private scope) and is the only discriminator of the rebase (a state check).
#[test]
fn round_hints_use_rebased_js_alias_inputs() {
    let (_dir, inputs) = fixture(&ALIAS, &["src/app.ts"]);
    let loader = ReviewContextLoader::new(&inputs).unwrap();
    let map = union(&inputs, &["src/mid.ts"]);
    let call = call_in(&map, "src/mid.ts", "f");
    assert_eq!(
        ReviewBoundaryHints::new(&inputs)
            .unwrap()
            .hint_for_call(&call),
        None,
        "inadmissible precondition: the original map has no caller for the alias import"
    );
    let round = ReviewContextRound::new(&loader, &map);
    let hint = round.hints().unwrap().hint_for_call(&call);
    assert!(
        matches!(&hint, Some(ReviewImportHint::InRepo { files, spec, .. })
            if files == &["src/lib/cmd.ts"] && spec == "@lib/cmd"),
        "{hint:?}"
    );
    let union_paths: BTreeSet<String> = map.keys().cloned().collect();
    assert_ne!(
        union_paths, inputs.scope_graph_inputs.all_file_paths,
        "inadmissible precondition: the union differs from the original paths"
    );
    assert_eq!(round.scope.all_file_paths, union_paths);
}

// H04
#[test]
fn round_hints_go_declaring_files_only() {
    let (_dir, inputs) = fixture(&GO, &["main.go"]);
    let loader = ReviewContextLoader::new(&inputs).unwrap();
    let map = union(&inputs, &["svc/svc.go"]);
    let calls = ["Pick", "Run", "Gone"].map(|name| call_in(&map, "svc/svc.go", name));
    let mut legacy = ReviewBoundaryHints::new(&inputs).unwrap();
    for call in &calls {
        assert_eq!(
            legacy.hint_for_call(call),
            None,
            "inadmissible precondition: the legacy hints see the Go context caller"
        );
    }
    let round = ReviewContextRound::new(&loader, &map);
    let mut hints = round.hints().unwrap();
    let package = ["util/b.go", "util/run_linux.go", "util/run_windows.go"];
    let want = [
        (&["util/b.go"][..], "singleton"),
        (
            &["util/run_linux.go", "util/run_windows.go"][..],
            "multiple",
        ),
        (&[][..], "declared nowhere"),
    ];
    for (call, (declaring_want, label)) in calls.iter().zip(want) {
        match hints.hint_for_call(call) {
            Some(ReviewImportHint::InRepo {
                files, declaring, ..
            }) => {
                assert_eq!(declaring, declaring_want, "{label}");
                assert_eq!(files, package, "{label}: files stay the whole package");
            }
            other => panic!("{label}: expected an in-repo Go hint, got {other:?}"),
        }
    }
}

// H05
#[test]
fn round_hints_python_context_caller() {
    let (_dir, inputs) = fixture(&PY, &["pkg/main.py"]);
    let loader = ReviewContextLoader::new(&inputs).unwrap();
    let map = union(&inputs, &["pkg/mid.py"]);
    let call = call_in(&map, "pkg/mid.py", "work");
    assert_eq!(
        ReviewBoundaryHints::new(&inputs)
            .unwrap()
            .hint_for_call(&call),
        None,
        "inadmissible precondition: the original map has no Python caller"
    );
    let round = ReviewContextRound::new(&loader, &map);
    let hint = round.hints().unwrap().hint_for_call(&call);
    assert!(
        matches!(&hint, Some(ReviewImportHint::InRepo { files, member, .. })
            if files == &["pkg/leaf.py"] && member.as_deref() == Some("work")),
        "{hint:?}"
    );
    // Negative: once leaf.py is loaded it is no longer a boundary.
    let both = union(&inputs, &["pkg/mid.py", "pkg/leaf.py"]);
    let round = ReviewContextRound::new(&loader, &both);
    assert_eq!(round.hints().unwrap().hint_for_call(&call), None);
}

// H06
#[test]
fn barrel_hop_follows_one_named_reexport() {
    let (_dir, inputs) = fixture(&BARREL, &["main.js", "probe.js"]);
    let loader = ReviewContextLoader::new(&inputs).unwrap();
    let probe_call = call_in(&inputs.files, "probe.js", "multi");
    let probe = ReviewBoundaryHints::new(&inputs)
        .unwrap()
        .hint_for_call(&probe_call);
    assert!(
        matches!(&probe, Some(ReviewImportHint::InRepo { files, basis: ReviewHintBasis::Candidates, .. })
            if files == &["one/m.js", "two/m.js"]),
        "inadmissible precondition: the alias must name two files: {probe:?}"
    );
    let map = union(&inputs, &["pkg/index.js"]);
    let round = ReviewContextRound::new(&loader, &map);
    let mut hints = round.hints().unwrap();
    assert_eq!(
        hints.follow_barrel("pkg/index.js", "h"),
        hop("pkg/impl.js", "g")
    );
    for member in ["starred", "dup", "multi", "absent"] {
        assert_eq!(
            hints.follow_barrel("pkg/index.js", member),
            None,
            "{member}"
        );
    }
    // A barrel that exists on disk but is not in the round's map gives nothing.
    assert_eq!(hints.follow_barrel("pkg/other.js", "h"), None);
    let with_other = union(&inputs, &["pkg/other.js"]);
    let round = ReviewContextRound::new(&loader, &with_other);
    assert_eq!(
        round.hints().unwrap().follow_barrel("pkg/other.js", "h"),
        hop("pkg/impl.js", "g"),
        "control: the same barrel hops once it is in the map"
    );
}

// H07
#[test]
fn barrel_hop_is_one_hop_per_call() {
    let (_dir, inputs) = fixture(&HOPS, &["main.js"]);
    let loader = ReviewContextLoader::new(&inputs).unwrap();
    let first = union(&inputs, &["lib/b1.js"]);
    let round = ReviewContextRound::new(&loader, &first);
    let mut hints = round.hints().unwrap();
    assert_eq!(hints.follow_barrel("lib/b1.js", "f"), hop("lib/b2.js", "q"));
    assert_eq!(
        hints.follow_barrel("lib/b2.js", "q"),
        None,
        "b2 is not in the map yet"
    );
    // Both barrels in the map: the first call still stops at the second barrel.
    let both = union(&inputs, &["lib/b1.js", "lib/b2.js"]);
    let round = ReviewContextRound::new(&loader, &both);
    let mut hints = round.hints().unwrap();
    assert_eq!(hints.follow_barrel("lib/b1.js", "f"), hop("lib/b2.js", "q"));
    assert_eq!(
        hints.follow_barrel("lib/b2.js", "q"),
        hop("lib/impl.js", "g")
    );
    // A two-barrel cycle: every call is one step and returns the pair it already returned.
    let cycle = union(&inputs, &["lib/c1.js", "lib/c2.js"]);
    let round = ReviewContextRound::new(&loader, &cycle);
    let mut hints = round.hints().unwrap();
    let mut at = "lib/c1.js".to_string();
    let mut path = Vec::new();
    for _ in 0..4 {
        let next = hints.follow_barrel(&at, "x").unwrap();
        path.push(next.file.clone());
        at = next.file;
    }
    assert_eq!(path, ["lib/c2.js", "lib/c1.js", "lib/c2.js", "lib/c1.js"]);
}

fn snapshot_dir(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let (mut out, mut pending) = (BTreeMap::new(), vec![dir.to_path_buf()]);
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                out.insert(path, bytes);
            }
        }
    }
    out
}

/// REAL: `caller_file->callee_file` for every resolved call of a round's graph, and whether the
/// build was scoped.
fn round_graph(
    inputs: &ReviewInputs,
    baseline: &TypeRegistry,
    extra: &[&str],
) -> (BTreeSet<String>, bool) {
    let loader = ReviewContextLoader::new(inputs).unwrap();
    let map = union(inputs, extra);
    let round = ReviewContextRound::new(&loader, &map);
    let ctx = round.build_context(inputs.type_db.as_ref(), baseline);
    let graph = &ctx.cpg.call_graph;
    let edges = graph
        .calls
        .keys()
        .flat_map(|from| {
            graph
                .callees_of(&from.name, &from.file, 1)
                .into_iter()
                .map(move |(to, _)| format!("{}->{}", from.file, to.file))
        })
        .collect();
    (edges, ctx.scope.is_some())
}

// H08: the cache HIT is the real cache module (`load_cache_with_topology`), not a stand-in.
#[test]
fn round_context_is_full_and_uncached() {
    let (dir, inputs) = fixture(&LONG, &["main.js"]);
    let rest = ["a.js", "b.js", "c.js", "d.js", "e.js"];
    let wanted = ["main.js->a.js", "a.js->b.js", "d.js->e.js"];
    let full = |edges: &BTreeSet<String>, scoped: bool| {
        assert!(!scoped, "the round's build must be a full build");
        assert!(wanted.iter().all(|edge| edges.contains(*edge)), "{edges:?}");
    };
    // Round zero built with `scoped_cpg = true`.
    let mut scoped = ReviewOptions::new(repo(&dir));
    scoped.scoped_cpg = true;
    let built = build_context(&inputs, &scoped).unwrap();
    let (edges, was_scoped) = round_graph(&inputs, &built.ctx.types, &rest);
    full(&edges, was_scoped);
    // Round zero served by a cache HIT.
    let cache = dir.path().join("cache");
    let mut cached = ReviewOptions::new(repo(&dir));
    cached.cache_dir = Some(cache.clone());
    drop(build_context(&inputs, &cached).unwrap());
    let hashes = cpg_cache::compute_file_hashes(&inputs.sources);
    let key = cpg_cache::compute_topology_key(&hashes, &inputs.scope_graph_inputs.manifest_hashes);
    assert!(
        matches!(
            cpg_cache::load_cache_with_topology(&hashes, &key, false, &cache),
            CacheResult::Hit(_)
        ),
        "inadmissible precondition: the second round-zero build must be a cache HIT"
    );
    let hit = build_context(&inputs, &cached).unwrap();
    let before = snapshot_dir(&cache);
    assert!(
        !before.is_empty(),
        "inadmissible precondition: the cache holds files"
    );
    let (edges, was_scoped) = round_graph(&inputs, &hit.ctx.types, &rest);
    full(&edges, was_scoped);
    assert_eq!(snapshot_dir(&cache), before, "the round touched the cache");
    // Changed context bytes between two rounds reach the second round's graph.
    assert!(
        !edges.contains("a.js->e.js"),
        "inadmissible precondition: the edge is new"
    );
    std::fs::write(repo(&dir).join("a.js"), A_V2).unwrap();
    let (edges, _) = round_graph(&inputs, &hit.ctx.types, &rest);
    assert!(edges.contains("a.js->e.js"), "{edges:?}");
    assert_eq!(
        snapshot_dir(&cache),
        before,
        "the second round touched the cache"
    );
}

// H09
#[test]
fn round_context_keeps_baseline_registry_settings() {
    let (dir, inputs) = fixture(&CHAIN, &["main.js"]);
    let mut baseline = build_context(&inputs, &ReviewOptions::new(repo(&dir))).unwrap();
    assert!(
        baseline.ctx.types.mode == TypeResolutionMode::Eager
            && baseline.ctx.types.target_versions.is_empty(),
        "inadmissible precondition: a fresh registry holds the default settings"
    );
    baseline.ctx.types.mode = TypeResolutionMode::Scoped;
    let version = LanguageVersion::parse("18").unwrap();
    baseline
        .ctx
        .types
        .set_target_version(Language::JavaScript, version);
    let loader = ReviewContextLoader::new(&inputs).unwrap();
    let map = union(&inputs, &["a.js", "b.js"]);
    let round = ReviewContextRound::new(&loader, &map);
    let ctx = round.build_context(None, &baseline.ctx.types);
    assert!(
        ctx.types.mode == TypeResolutionMode::Scoped,
        "mode not kept"
    );
    assert_eq!(
        ctx.types.target_versions, baseline.ctx.types.target_versions,
        "target_versions not kept"
    );
}

// H10: SYNTHETIC capture failure plus a REAL poisoned public listing.
#[test]
fn listing_unavailable_reaches_round_hints() {
    let (dir, mut inputs) = fixture(&CHAIN, &["main.js"]);
    let failed = || crate::repo_loader::ListingError {
        dir: String::new(),
        message: "scripted listing failure".to_string(),
    };
    // Real capture, poisoned public listing: the legacy handle refuses, the round does not.
    inputs.source_listing = Err(failed());
    assert!(
        ReviewBoundaryHints::new(&inputs).is_err(),
        "inadmissible precondition: the legacy handle must refuse a failed public listing"
    );
    let loader = ReviewContextLoader::new(&inputs).unwrap();
    let all = union(&inputs, &["a.js", "b.js"]);
    let call = call_in(&all, "a.js", "fb");
    let main_a = union(&inputs, &["a.js"]);
    let round = ReviewContextRound::new(&loader, &main_a);
    let hint = round.hints().unwrap().hint_for_call(&call);
    assert_eq!(in_repo_files(hint).unwrap(), ["b.js"]);
    // SYNTHETIC: S4a records a failed listing as no loader, so no round can be constructed
    // (`ReviewContextRound::new` needs a loader). Proven by construction, not counted at runtime.
    let root = repo(&dir);
    let observed = observe_root(&root).unwrap();
    inputs.context_capture = capture(
        &root,
        Ok(observed),
        &inputs.scope_graph_inputs,
        &Err(failed()),
    );
    assert!(matches!(
        ReviewContextLoader::new(&inputs).err(),
        Some(ContextUnavailable::ListingUnavailable { .. })
    ));
}
