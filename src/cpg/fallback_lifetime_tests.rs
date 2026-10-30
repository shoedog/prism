//! Test-only ownership adapters, following navigation's R0 witness pattern.
//! Tokens travel with the actual owned values (including the merged graphs),
//! never with a parallel simulated branch. Handles cross the build pool explicitly.
use super::{CodePropertyGraph, CpgContext};
use crate::ast::ParsedFile;
use crate::diff::{DiffInfo, DiffInput, ModifyType};
use crate::navigation::{queries, NavigationIndex, NavigationSession};
use crate::repo_loader::{load_repo, LoadedRepo};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, Weak};

#[derive(Default)]
struct State {
    owners: Vec<Weak<()>>,
    starts: Vec<(&'static str, Vec<bool>)>,
    clones: usize,
}

#[derive(Clone, Default)]
pub(crate) struct Probe(Option<Arc<Mutex<State>>>);
thread_local! {
    static ACTIVE: RefCell<Probe> = RefCell::new(Probe::default());
}
struct Measurement;
impl Drop for Measurement {
    fn drop(&mut self) {
        ACTIVE.with_borrow_mut(|p| *p = Probe::default());
    }
}
fn measure() -> (Measurement, Probe) {
    let probe = Probe(Some(Arc::new(Mutex::new(State::default()))));
    ACTIVE.with_borrow_mut(|p| *p = probe.clone());
    (Measurement, probe)
}
impl Probe {
    pub(crate) fn current() -> Self {
        ACTIVE.with_borrow(Clone::clone)
    }
    pub(crate) fn track<T>(&self, value: T, count: usize) -> Tracked<T> {
        let owners: Vec<_> = (0..count).map(|_| Arc::new(())).collect();
        if let Some(state) = &self.0 {
            state
                .lock()
                .unwrap()
                .owners
                .extend(owners.iter().map(Arc::downgrade));
        }
        Tracked {
            value,
            _owners: owners,
        }
    }
    pub(crate) fn record(&self, site: &'static str) {
        if let Some(state) = &self.0 {
            let mut state = state.lock().unwrap();
            let live = state.owners.iter().map(|w| w.upgrade().is_some()).collect();
            state.starts.push((site, live));
        }
    }
    pub(crate) fn clones(&self) {
        if let Some(state) = &self.0 {
            state.lock().unwrap().clones += 2;
        }
    }
    fn released(&self, site: &'static str, count: usize) {
        assert_eq!(
            self.0.as_ref().unwrap().lock().unwrap().starts,
            vec![(site, vec![false; count])],
            "abandoned graphs must be dead when {site} full build starts"
        );
    }
    fn no_fallback(&self) {
        assert!(
            self.0.as_ref().unwrap().lock().unwrap().starts.is_empty(),
            "ordinary incremental/cached/scoped path must not fall back"
        );
    }
}
pub(crate) struct Tracked<T> {
    value: T,
    _owners: Vec<Arc<()>>,
}
impl<T> Tracked<T> {
    pub(crate) fn into_inner(self) -> T {
        self.value
    }
}
impl<T> Deref for Tracked<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}
impl<T> DerefMut for Tracked<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

fn write(root: &std::path::Path, path: &str, source: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, source).unwrap();
}
fn wire(repo: &Arc<LoadedRepo>, index: NavigationIndex) -> Vec<u8> {
    let session = NavigationSession {
        repo: repo.clone(),
        index: Arc::new(index),
    };
    serde_json::to_vec(&[
        queries::callees(&session, Some("run"), None, None, 1).unwrap(),
        queries::callers(&session, Some("run"), None, None, 1).unwrap(),
        queries::ego_graph(&session, Some("run"), None, None, 1, &["Call"]).unwrap(),
    ])
    .unwrap()
}
fn parity(actual: &CodePropertyGraph, files: &BTreeMap<String, ParsedFile>) {
    let full = CodePropertyGraph::build(files);
    assert!(
        super::dfg_observation::fieldwise_payload(actual)
            == super::dfg_observation::fieldwise_payload(&full),
        "fresh full graph/cache payload bytes"
    );
    assert_eq!(
        super::dfg_observation::lookup_indexes(actual),
        super::dfg_observation::lookup_indexes(&full),
        "fresh full lookup bytes"
    );
}
const RUST: &str = "fn helper(x: i32) -> i32 { x }\nfn run() { println!(\"{}\", helper(1)); }\n";
const PYTHON: &str = "def helper(x):\n    return x\ndef run():\n    return helper(1)\n";

#[test]
fn incremental_owner_drops_graphs() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "app.py", PYTHON);
    let old = load_repo(root.path()).unwrap();
    let mut cpg = CodePropertyGraph::build(&old.files);
    // Exercise the authority marker without requiring an external compiler.
    cpg.call_graph.owner_ephemeral = true;
    write(
        root.path(),
        "app.py",
        &PYTHON.replace("helper(1)", "helper(2)"),
    );
    let repo = load_repo(root.path()).unwrap();
    let (_measurement, probe) = measure();
    let result = CodePropertyGraph::build_incremental(
        cpg.call_graph,
        cpg.dfg,
        &BTreeSet::from(["app.py".into()]),
        &repo.files,
        None,
    );
    probe.released("incremental_owner", 2);
    parity(&result, &repo.files);
}

#[test]
fn incremental_macro_drops_graphs() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "app.rs", RUST);
    write(root.path(), "macros.rs", "fn unrelated() {}\n");
    let old = load_repo(root.path()).unwrap();
    let cpg = CodePropertyGraph::build(&old.files);
    write(
        root.path(),
        "macros.rs",
        "macro_rules! println { ($($t:tt)*) => {} }\nfn unrelated() {}\n",
    );
    let repo = load_repo(root.path()).unwrap();
    let (_measurement, probe) = measure();
    let result = CodePropertyGraph::build_incremental(
        cpg.call_graph,
        cpg.dfg,
        &BTreeSet::from(["macros.rs".into()]),
        &repo.files,
        None,
    );
    probe.released("incremental_macro", 2);
    parity(&result, &repo.files);
}

#[test]
fn incremental_python_drops_merged_graphs() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "app.py",
        "from pkg.models import Client\ndef run(client: Client):\n    client.send()\n",
    );
    write(
        root.path(),
        "pkg/models.py",
        "class Placeholder:\n    pass\n",
    );
    let old = load_repo(root.path()).unwrap();
    let cpg = CodePropertyGraph::build(&old.files);
    write(
        root.path(),
        "pkg/models.py",
        "class Client:\n    def send(self):\n        pass\n",
    );
    let repo = load_repo(root.path()).unwrap();
    let (_measurement, probe) = measure();
    let result = CodePropertyGraph::build_incremental(
        cpg.call_graph,
        cpg.dfg,
        &BTreeSet::from(["pkg/models.py".into()]),
        &repo.files,
        None,
    );
    probe.released("incremental_python", 2);
    parity(&result, &repo.files);
}

#[test]
fn context_owner_drops_cpg() {
    for (cpg_owner, cg_owner) in [(true, false), (false, true)] {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "app.py", PYTHON);
        let old = load_repo(root.path()).unwrap();
        let mut cpg = CodePropertyGraph::build(&old.files);
        cpg.ephemeral_owner = cpg_owner;
        cpg.call_graph.owner_ephemeral = cg_owner;
        write(
            root.path(),
            "app.py",
            &PYTHON.replace("helper(1)", "helper(2)"),
        );
        let repo = load_repo(root.path()).unwrap();
        let (_measurement, probe) = measure();
        let result = CpgContext::build_with_cached_cpg(&repo.files, cpg, None);
        probe.released("context_owner", 2);
        parity(&result.cpg, &repo.files);
    }
}
fn diff(path: &str) -> DiffInput {
    DiffInput {
        files: vec![DiffInfo {
            file_path: path.into(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([1]),
        }],
    }
}
#[test]
fn context_scope_drops_skeleton() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "app.py",
        "from helper import helper\ndef run():\n    return helper(1)\n",
    );
    write(root.path(), "helper.py", "def helper(x):\n    return x\n");
    write(
        root.path(),
        "helper.py",
        "def helper(x):\n    return x + 1\n",
    );
    let repo = load_repo(root.path()).unwrap();
    let (_measurement, probe) = measure();
    let result = CpgContext::build_scoped(&repo.files, &diff("app.py"), None);
    assert!(
        result.scope.is_none(),
        "fixture must cover over half the files"
    );
    probe.released("context_scope", 1); // There is no skeleton DFG.
    parity(&result.cpg, &repo.files);
}

#[test]
fn navigation_owner_skips_clones() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "app.py", PYTHON);
    let old = load_repo(root.path()).unwrap();
    let mut previous = NavigationIndex::build(&old);
    previous.cpg.call_graph.owner_ephemeral = true;
    write(
        root.path(),
        "app.py",
        &PYTHON.replace("helper(1)", "helper(2)"),
    );
    let repo = Arc::new(load_repo(root.path()).unwrap());
    let (_measurement, probe) = measure();
    let result = NavigationIndex::build_incremental_from_previous(
        &previous,
        &repo,
        &BTreeSet::from(["app.py".into()]),
    );
    assert_eq!(
        probe.0.as_ref().unwrap().lock().unwrap().clones,
        0,
        "ephemeral fallback must be selected before cloning CG/DFG"
    );
    assert_eq!(
        wire(&repo, result),
        wire(&repo, NavigationIndex::build(&repo))
    );
    assert!(
        previous.cpg.call_graph.owner_ephemeral,
        "published index remains alive"
    );
}

#[test]
fn stable_proofs_and_ordinary_owner_do_not_fall_back() {
    for (path, source, edited) in [
        (
            "app.rs",
            RUST.to_string(),
            RUST.replace("helper(1)", "helper(2)"),
        ),
        (
            "app.py",
            PYTHON.to_string(),
            PYTHON.replace("helper(1)", "helper(2)"),
        ),
        (
            "pkg/models.py",
            "class Client:\n    def send(self):\n        return 1\n".into(),
            "class Client:\n    def send(self):\n        return 2\n".into(),
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), path, &source);
        if path.starts_with("pkg/") {
            write(
                root.path(),
                "app.py",
                "from pkg.models import Client\ndef run(client: Client):\n    client.send()\n",
            );
        }
        let old = load_repo(root.path()).unwrap();
        let previous = NavigationIndex::build(&old);
        write(root.path(), path, &edited);
        let repo = Arc::new(load_repo(root.path()).unwrap());
        let (_measurement, probe) = measure();
        let result = NavigationIndex::build_incremental_from_previous(
            &previous,
            &repo,
            &BTreeSet::from([path.into()]),
        );
        probe.no_fallback();
        assert_eq!(probe.0.as_ref().unwrap().lock().unwrap().clones, 2);
        assert_eq!(
            wire(&repo, result),
            wire(&repo, NavigationIndex::build(&repo))
        );
        let cpg = CodePropertyGraph::build(&repo.files);
        let cached = CpgContext::build_with_cached_cpg(&repo.files, cpg, None);
        probe.no_fallback();
        parity(&cached.cpg, &repo.files);
    }
}

#[test]
fn small_scope_and_early_full_build_have_no_discard_fallback() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        ("app.py", PYTHON),
        ("b.py", "def b():\n    pass\n"),
        ("c.py", "def c():\n    pass\n"),
    ] {
        write(root.path(), path, source);
    }
    let repo = load_repo(root.path()).unwrap();
    let (_measurement, probe) = measure();
    let scoped = CpgContext::build_scoped(&repo.files, &diff("app.py"), None);
    assert_eq!(scoped.scope.as_ref().unwrap().scoped_files.len(), 1);
    let single = BTreeMap::from([("app.py".into(), repo.files["app.py"].clone())]);
    let expected = CodePropertyGraph::build_enriched_without_scope_graph(&single, None);
    assert!(
        super::dfg_observation::fieldwise_payload(&scoped.cpg)
            == super::dfg_observation::fieldwise_payload(&expected),
        "scoped build bytes"
    );
    for (files, diff) in [
        (&repo.files, DiffInput { files: vec![] }),
        (&single, diff("app.py")),
    ] {
        let full = CpgContext::build_scoped(files, &diff, None);
        assert!(full.scope.is_none());
        parity(&full.cpg, files);
    }
    probe.no_fallback();
}
