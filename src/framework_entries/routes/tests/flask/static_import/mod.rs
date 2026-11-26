//! Stage D rows for the Python static import binding rule (DESIGN 5.3, STAGE-D-ROWS.md
//! "Python router and view value boundary"), as Flask source-fact tests.
//!
//! One test per row id: `fk_NN` for a row, `fk_cNN` for its control. Rows are grouped by class
//! (`withheld`, `proven`, `known_limits`) and the owner-pending shapes live in `pending`.
//!
//! Scope of these tests (checkpoint 1): the `FlaskEvent` source facts (receiver, literal path,
//! written callback status, declared and effective methods, effective endpoint, view
//! metadata, mount child) plus the exact `import_refusals()` multiset. Exposure, dispatch
//! binding and serving clauses of a row belong to checkpoint 2 and are skipped here; each
//! module lists the skipped clauses per row. Native-runtime observations in the rows are
//! evidence for the pin, not assertions.
//!
//! Registrations written inside a class body or a method (fk-81..84 and controls fk-c82..84)
//! are not visited by `flask_facts` (only module scope or an invoked factory frame is
//! active), so those rows assert the refusal multiset only.
use super::*;
use crate::framework_entries::routes::python::flask::FlaskRule;
use crate::framework_entries::routes::receivers::refusals::{ImportRefusal, ImportRefusalReason};

mod fixtures;
mod known_limits;
mod pending;
mod proven;
mod proven_mounts;
mod withheld;
mod withheld_mounts;

use fixtures::*;
use ImportRefusalReason::*;

pub(super) type Files = Vec<(String, String)>;
/// `[reason, file, enclosing source text, span text]`, as in the row's `expect_diagnostics`.
pub(super) type Diag = (
    ImportRefusalReason,
    &'static str,
    &'static str,
    &'static str,
);

pub(super) fn files(list: &[(&str, &str)]) -> Files {
    list.iter()
        .map(|&(path, source)| (path.to_owned(), source.to_owned()))
        .collect()
}

/// Run the Flask facts of `entry` over a fresh app and module cache.
pub(super) fn run<R>(
    files: &Files,
    entry: &str,
    inspect: impl FnOnce(&[FlaskEvent<'_>], Vec<ImportRefusal>) -> R,
) -> R {
    let sources: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, source)| (path.as_str(), source.as_str()))
        .collect();
    let parsed = parsed(&sources);
    let bindings = index_python(&parsed);
    let events = flask_facts(&bindings, entry, &SetupContext::default());
    inspect(&events, bindings.import_refusals())
}

/// Leak a test-built string into a row table entry (tests only).
pub(super) fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

fn identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The unique `span` inside the unique `enclosing` text of `source`, as UTF-8 bytes. A span
/// equal to its enclosing text is the whole occurrence; otherwise the first match at identifier
/// boundaries (a bare name is one identifier token).
fn locate(source: &str, enclosing: &str, span: &str) -> (usize, usize) {
    let mut found = source.match_indices(enclosing);
    let (at, _) = found.next().expect("enclosing text present");
    assert!(
        found.next().is_none(),
        "enclosing text {enclosing:?} not unique"
    );
    let inner = if span == enclosing {
        0
    } else {
        enclosing
            .match_indices(span)
            .map(|(i, _)| i)
            .find(|&i| {
                let before = enclosing[..i].chars().next_back();
                let after = enclosing[i + span.len()..].chars().next();
                !before.is_some_and(identifier_char) && !after.is_some_and(identifier_char)
            })
            .unwrap_or_else(|| panic!("span {span:?} not a token of {enclosing:?}"))
    };
    (at + inner, at + inner + span.len())
}

pub(super) fn refusals(files: &Files, diags: &[Diag]) -> Vec<ImportRefusal> {
    let mut wanted: Vec<_> = diags
        .iter()
        .map(|&(reason, file, enclosing, span)| {
            let source = &files.iter().find(|(path, _)| path == file).expect("file").1;
            let (start_byte, end_byte) = locate(source, enclosing, span);
            ImportRefusal {
                reason,
                file: file.into(),
                start_byte,
                end_byte,
            }
        })
        .collect();
    wanted.sort();
    wanted
}

fn receiver_proved(receiver: &Receiver) -> bool {
    matches!(receiver, Receiver::Router { .. })
}

#[derive(Clone, Copy)]
enum Cb {
    Skip,
    Def(&'static str, &'static str),
    DefIn(&'static str),
    Unresolved(Reason, &'static str),
}

#[derive(Clone, Copy)]
enum Meth {
    Skip,
    Known(&'static [&'static str]),
    Unknown,
}

#[derive(Clone, Copy)]
enum Ep {
    Skip,
    Known(&'static str),
    Unknown,
}

#[derive(Clone, Copy)]
enum Decl {
    Skip,
    None,
    Known(&'static [&'static str]),
}

/// Expected source facts of one `FlaskEvent::Rule`.
#[derive(Clone, Copy)]
pub(super) struct RuleSpec {
    path: &'static str,
    receiver: Option<Reason>,
    callback: Cb,
    methods: Meth,
    endpoint: Ep,
    declared: Decl,
    metadata_unknown: bool,
}

const DEFAULT_METHODS: &[&str] = &["GET", "HEAD", "OPTIONS"];

/// A rule on a proved receiver at a literal path; every other field unconstrained.
pub(super) fn rule(path: &'static str) -> RuleSpec {
    RuleSpec {
        path,
        receiver: None,
        callback: Cb::Skip,
        methods: Meth::Skip,
        endpoint: Ep::Skip,
        declared: Decl::Skip,
        metadata_unknown: false,
    }
}

/// Grounded receiver, callback resolved to `file`:`name`, inferred endpoint `name` and the
/// default effective GET/HEAD/OPTIONS, declared methods null.
pub(super) fn proven_view(path: &'static str, file: &'static str, name: &'static str) -> RuleSpec {
    rule(path)
        .def(file, name)
        .methods(DEFAULT_METHODS)
        .endpoint(name)
        .declared_none()
}

/// A refused no-effect callee: the written callback identity stays, view-derived metadata is
/// withheld.
pub(super) fn withheld_callee(
    path: &'static str,
    file: &'static str,
    name: &'static str,
) -> RuleSpec {
    rule(path)
        .def(file, name)
        .methods_unknown()
        .endpoint_unknown()
        .metadata_unknown()
}

/// A refused imported view: unresolved `imported_unproven` callback, metadata withheld.
pub(super) fn withheld_view(path: &'static str, text: &'static str) -> RuleSpec {
    rule(path)
        .unresolved(Reason::ImportedUnproven, text)
        .methods_unknown()
        .endpoint_unknown()
}

impl RuleSpec {
    pub(super) fn def(mut self, file: &'static str, name: &'static str) -> Self {
        self.callback = Cb::Def(file, name);
        self
    }
    /// The callback resolves into `file` (its normalised name is not asserted).
    pub(super) fn def_in(mut self, file: &'static str) -> Self {
        self.callback = Cb::DefIn(file);
        self
    }
    pub(super) fn unresolved(mut self, reason: Reason, text: &'static str) -> Self {
        self.callback = Cb::Unresolved(reason, text);
        self
    }
    pub(super) fn methods(mut self, methods: &'static [&'static str]) -> Self {
        self.methods = Meth::Known(methods);
        self
    }
    pub(super) fn methods_unknown(mut self) -> Self {
        self.methods = Meth::Unknown;
        self
    }
    pub(super) fn endpoint(mut self, endpoint: &'static str) -> Self {
        self.endpoint = Ep::Known(endpoint);
        self
    }
    pub(super) fn endpoint_unknown(mut self) -> Self {
        self.endpoint = Ep::Unknown;
        self
    }
    pub(super) fn declared(mut self, methods: &'static [&'static str]) -> Self {
        self.declared = Decl::Known(methods);
        self
    }
    pub(super) fn declared_none(mut self) -> Self {
        self.declared = Decl::None;
        self
    }
    pub(super) fn metadata_unknown(mut self) -> Self {
        self.metadata_unknown = true;
        self
    }
    /// The receiver is retained unproven with this reason (candidate route).
    pub(super) fn unproven(mut self, reason: Reason) -> Self {
        self.receiver = Some(reason);
        self
    }
}

/// Expected mount event: the parent is the grounded app; the child is either proved as the
/// allocation in `child` or `mount_child_unproven`.
#[derive(Clone, Copy)]
pub(super) struct MountSpec(pub Option<&'static str>);

/// What `flask_facts(entry)` yields.
#[derive(Default)]
pub(super) struct Expect {
    rules: Vec<RuleSpec>,
    mounts: Vec<MountSpec>,
}

pub(super) fn rules(specs: &[RuleSpec]) -> Expect {
    Expect {
        rules: specs.to_vec(),
        mounts: vec![],
    }
}

/// No rule and no mount at all (a dropped site or a registration `flask_facts` does not visit).
pub(super) fn nothing() -> Expect {
    Expect::default()
}

pub(super) fn mount(child: &'static str) -> Expect {
    Expect {
        rules: vec![],
        mounts: vec![MountSpec(Some(child))],
    }
}

pub(super) fn mount_unproven() -> Expect {
    Expect {
        rules: vec![],
        mounts: vec![MountSpec(None)],
    }
}

impl Expect {
    /// Also expect one mount event whose child is proved as an allocation in `child`, or
    /// `mount_child_unproven` for `None`.
    pub(super) fn with_mount(mut self, child: Option<&'static str>) -> Self {
        self.mounts.push(MountSpec(child));
        self
    }
}

fn assert_rule(rule: &FlaskRule<'_>, spec: &RuleSpec, label: &str) {
    let route = &rule.route;
    match spec.receiver {
        None => assert!(
            receiver_proved(&route.receiver),
            "{label}: receiver {:?}",
            route.receiver
        ),
        Some(reason) => assert_eq!(
            route.receiver,
            Receiver::Unproven { reason },
            "{label}: receiver"
        ),
    }
    let declaration = route.declaration.as_ref().expect("declaration");
    assert_eq!(
        declaration.path.literal.as_deref(),
        Some(spec.path),
        "{label}: path"
    );
    match spec.callback {
        Cb::Skip => {}
        Cb::Def(file, name) => {
            let [Callback {
                status: CallbackStatus::Resolved { identity, .. },
                ..
            }] = route.callbacks.as_slice()
            else {
                panic!(
                    "{label}: expected a resolved callback, got {:?}",
                    route.callbacks
                )
            };
            assert_eq!(
                (identity.file.as_str(), identity.name.as_str()),
                (file, name),
                "{label}"
            );
        }
        Cb::DefIn(file) => {
            let [Callback {
                status: CallbackStatus::Resolved { identity, .. },
                ..
            }] = route.callbacks.as_slice()
            else {
                panic!(
                    "{label}: expected a resolved callback, got {:?}",
                    route.callbacks
                )
            };
            assert_eq!(identity.file, file, "{label}");
        }
        Cb::Unresolved(reason, text) => {
            let [Callback {
                status:
                    CallbackStatus::Unresolved {
                        text: actual,
                        reason: actual_reason,
                    },
                ..
            }] = route.callbacks.as_slice()
            else {
                panic!(
                    "{label}: expected an unresolved callback, got {:?}",
                    route.callbacks
                )
            };
            assert_eq!((*actual_reason, actual.as_str()), (reason, text), "{label}");
        }
    }
    match spec.methods {
        Meth::Skip => {}
        Meth::Known(methods) => assert_eq!(
            rule.effective_methods,
            MethodSet::known(methods.iter().copied()),
            "{label}: effective methods"
        ),
        Meth::Unknown => assert!(
            matches!(rule.effective_methods, MethodSet::Unknown { .. }),
            "{label}: effective methods {:?}",
            rule.effective_methods
        ),
    }
    match spec.endpoint {
        Ep::Skip => {}
        Ep::Known(name) => assert_eq!(
            rule.effective_endpoint,
            EffectiveEndpoint::Known(name.into()),
            "{label}: endpoint"
        ),
        Ep::Unknown => assert_eq!(
            rule.effective_endpoint,
            EffectiveEndpoint::Unknown,
            "{label}"
        ),
    }
    match spec.declared {
        Decl::Skip => {}
        Decl::None => assert_eq!(declaration.methods, None, "{label}: declared methods"),
        Decl::Known(methods) => assert_eq!(
            declaration.methods,
            Some(MethodSet::known(methods.iter().copied())),
            "{label}: declared methods"
        ),
    }
    if spec.metadata_unknown {
        assert!(
            matches!(rule.view_metadata.methods, Some(MethodSet::Unknown { .. })),
            "{label}: view metadata {:?}",
            rule.view_metadata.methods
        );
    }
}

/// Assert the source facts of `entry` and the exact refusal multiset.
#[track_caller]
pub(super) fn check(files: &Files, entry: &str, expect: Expect, diags: &[Diag]) {
    let wanted = refusals(files, diags);
    run(files, entry, |events, mut got| {
        let found: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                FlaskEvent::Rule(rule) => Some(&**rule),
                _ => None,
            })
            .collect();
        assert_eq!(found.len(), expect.rules.len(), "rule events");
        for (index, (rule, spec)) in found.into_iter().zip(&expect.rules).enumerate() {
            assert_rule(rule, spec, &format!("rule {index} ({})", spec.path));
        }
        let mounts: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                FlaskEvent::Mount(mount) => Some(mount),
                _ => None,
            })
            .collect();
        assert_eq!(mounts.len(), expect.mounts.len(), "mount events");
        for (mount, MountSpec(child)) in mounts.into_iter().zip(&expect.mounts) {
            assert!(
                receiver_proved(&mount.parent),
                "mount parent {:?}",
                mount.parent
            );
            match (child, &mount.child) {
                (Some(file), Receiver::Router { identity, .. }) => {
                    assert_eq!(identity.file, *file, "mount child allocation")
                }
                (None, Receiver::Unproven { reason }) => {
                    assert_eq!(*reason, Reason::MountChildUnproven)
                }
                (_, other) => panic!("mount child {other:?}, expected {child:?}"),
            }
        }
        got.sort();
        assert_eq!(got, wanted, "import refusals");
    });
}
