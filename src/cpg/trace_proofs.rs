//! Walk-proof switches: an env-driven measurement knob that makes the taint trace walk refuse the
//! crossing hops a candidate I-walk rule would refuse (lane 8, DESIGN 5.2 (i)-(v) plus the two
//! return rules), so the sink-level recall cost of each rule, alone and combined, can be read off
//! the real trace-review sink lines.
//!
//! OFF by default; with every switch off nothing is read, parsed or recorded and the walk's output
//! (stdout, stderr, cache) is exactly the walk's without this module.
//!
//! - `PRISM_TAINT_WALK_PROOFS_FILE=<path>`: JSON
//!   `{"version":1,"rules":{"<rule>":[{"relation":"CallDescent|ReturnFlow|ReturnInput",
//!   "from":"<key>","to":"<key>","root":"<key>"?}, ..]}}`. A key is the crossing ledger's
//!   `from=`/`to=`/`root=` value with its Debug quotes and escapes removed (the ledger prints each
//!   identity with `{:?}`) and its `#<digits>` node index dropped (`CodePropertyGraph::trace_key`);
//!   an absent `root` means the exclusion applies under every root.
//! - `PRISM_TAINT_WALK_PROOFS=<comma list>`: the rules switched ON, from `i,ii,iii,iv,v,
//!   return_flow,return_input` or `all`; unset or empty is every switch off.
//!
//! A switched-on rule refuses a filed hop exactly as a refused gate would (boundary record, ledger
//! line) with the reason of that rule: i NoMatchingSite, ii AmbiguousSite, iii NotExact,
//! iv UnprovenSlot, v AmbiguousSite, return_flow ReturnNotEnteredSite. A filed ReturnInput hop
//! under return_input is a cut (UnprovenReturnInput). A rule only matches hops of its own
//! relation (i..v: CallDescent, return_flow: ReturnFlow, return_input: ReturnInput), the first
//! matching rule in that order supplies the reason, and the descent gate cache never sees any of
//! it. The configuration is read, and validated, once per process when a walk starts (not only
//! when a hop reaches the hook); a value that is not UTF-8 is an error. When a switch is on, one
//! stderr line `taint-walk-proofs rule=<r> kept=<n> dropped_relation_mismatch=<m>` per
//! switched-on rule reports the filed entries kept and those dropped for a relation other than
//! the rule's. A misconfiguration (switches without a file, an unreadable or malformed file, an unknown
//! rule, a switched-on rule the file lacks) panics naming the variable: a measurement that is
//! misconfigured must not look like a clean baseline.
//!
//! Scope: the primary and bypass walks (`taint_trace_from_root`, used by trace-review and
//! navigation). The legacy flat taint walk of the default review is intentionally NOT affected.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::Write;
use std::sync::OnceLock;

use petgraph::graph::NodeIndex;
use serde::Deserialize;

use super::{CodePropertyGraph, LossReason, Relation};

const SWITCHES: &str = "PRISM_TAINT_WALK_PROOFS";
const FILE: &str = "PRISM_TAINT_WALK_PROOFS_FILE";
const SUMMARY: &str = "taint-walk-proofs ";

/// A variable's value; unset is `None`, a value that is not UTF-8 is an error naming it.
fn utf8(name: &str, value: Option<OsString>) -> Result<Option<String>, String> {
    value
        .map(|value| {
            value
                .into_string()
                .map_err(|_| format!("{name}: the value is not valid UTF-8"))
        })
        .transpose()
}

#[cfg(test)]
thread_local! {
    /// A test-installed configuration that stands in for the process-wide one on this thread.
    static OVERRIDE: std::cell::RefCell<Option<Option<std::rc::Rc<WalkProofs>>>> =
        const { std::cell::RefCell::new(None) };
}

/// Runs `f` with the test-installed configuration (`None` meaning every switch off) in place of
/// the process-wide one, then restores what was there.
#[cfg(test)]
pub(super) fn with_walk_proofs<R>(proofs: Option<WalkProofs>, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<Option<std::rc::Rc<WalkProofs>>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OVERRIDE.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let _restore =
        Restore(OVERRIDE.with(|slot| slot.borrow_mut().replace(proofs.map(std::rc::Rc::new))));
    f()
}

/// The configuration in force: the process-wide one (or, in tests, the thread's override).
fn with_proofs<R>(f: impl FnOnce(Option<&WalkProofs>) -> R) -> R {
    #[cfg(test)]
    if let Some(installed) = OVERRIDE.with(|slot| slot.borrow().clone()) {
        return f(installed.as_deref());
    }
    f(WalkProofs::global())
}

/// Reads the configuration now (once per process) so a misconfiguration panics at walk entry
/// even if no hop ever reaches the hook; one `Option` test when off.
pub(super) fn check_walk_proofs() {
    with_proofs(|_| ());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Rule {
    I,
    Ii,
    Iii,
    Iv,
    V,
    ReturnFlow,
    ReturnInput,
}

impl Rule {
    const ALL: [Rule; 7] = [
        Rule::I,
        Rule::Ii,
        Rule::Iii,
        Rule::Iv,
        Rule::V,
        Rule::ReturnFlow,
        Rule::ReturnInput,
    ];

    fn name(self) -> &'static str {
        match self {
            Rule::I => "i",
            Rule::Ii => "ii",
            Rule::Iii => "iii",
            Rule::Iv => "iv",
            Rule::V => "v",
            Rule::ReturnFlow => "return_flow",
            Rule::ReturnInput => "return_input",
        }
    }

    fn reason(self) -> LossReason {
        match self {
            Rule::I => LossReason::NoMatchingSite,
            Rule::Ii | Rule::V => LossReason::AmbiguousSite,
            Rule::Iii => LossReason::NotExact,
            Rule::Iv => LossReason::UnprovenSlot,
            Rule::ReturnFlow => LossReason::ReturnNotEnteredSite,
            Rule::ReturnInput => LossReason::UnprovenReturnInput,
        }
    }

    /// The only hop relation this rule can match.
    fn relation(self) -> Relation {
        match self {
            Rule::ReturnFlow => Relation::ReturnFlow,
            Rule::ReturnInput => Relation::ReturnInput,
            _ => Relation::CallDescent,
        }
    }

    fn parse(name: &str) -> Option<Rule> {
        Rule::ALL.into_iter().find(|rule| rule.name() == name)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileModel {
    version: u32,
    rules: BTreeMap<String, Vec<EntryModel>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryModel {
    relation: RelationModel,
    from: String,
    to: String,
    root: Option<String>,
}

#[derive(Deserialize, Clone, Copy, PartialEq)]
enum RelationModel {
    CallDescent,
    ReturnFlow,
    ReturnInput,
}

impl RelationModel {
    fn relation(self) -> Relation {
        match self {
            Self::CallDescent => Relation::CallDescent,
            Self::ReturnFlow => Relation::ReturnFlow,
            Self::ReturnInput => Relation::ReturnInput,
        }
    }
}

/// `(from, to, root)`; the relation is the rule's own.
type Exclusion = (String, String, Option<String>);

/// A switched-on rule, its filed exclusions, and how many filed entries of another relation
/// than the rule's were dropped at load.
#[derive(Debug)]
struct RuleSet {
    rule: Rule,
    entries: BTreeSet<Exclusion>,
    dropped: usize,
}

/// The switched-on rules and their filed exclusions.
#[derive(Debug)]
pub(super) struct WalkProofs {
    rules: Vec<RuleSet>,
}

/// The rules a switch list turns on; empty when it is unset, blank or only separators.
fn parse_switches(switches: Option<&str>) -> Result<BTreeSet<Rule>, String> {
    let mut on = BTreeSet::new();
    for item in switches
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        if item == "all" {
            on.extend(Rule::ALL);
        } else {
            on.insert(Rule::parse(item).ok_or_else(|| {
                format!("{SWITCHES}: unknown rule `{item}` (expected i, ii, iii, iv, v, return_flow, return_input or all)")
            })?);
        }
    }
    Ok(on)
}

impl WalkProofs {
    /// `Ok(None)` when no switch is on (the file is then not even read). Errors name the variable.
    pub(super) fn from_parts(
        file_text: Option<&str>,
        switches: Option<&str>,
    ) -> Result<Option<Self>, String> {
        let on = parse_switches(switches)?;
        if on.is_empty() {
            return Ok(None);
        }
        let text = file_text.ok_or_else(|| {
            format!("{SWITCHES} is set but {FILE} is not: the walk proofs need their file")
        })?;
        let model: FileModel =
            serde_json::from_str(text).map_err(|error| format!("{FILE}: malformed: {error}"))?;
        if model.version != 1 {
            return Err(format!(
                "{FILE}: unsupported version {} (expected 1)",
                model.version
            ));
        }
        if let Some(name) = model.rules.keys().find(|name| Rule::parse(name).is_none()) {
            return Err(format!("{FILE}: unknown rule `{name}`"));
        }
        let mut rules = Vec::new();
        for rule in on {
            let entries = model.rules.get(rule.name()).ok_or_else(|| {
                format!(
                    "{SWITCHES} switches rule `{}` on but {FILE} has no such rule",
                    rule.name()
                )
            })?;
            // An entry of another relation than the rule's can never match a hop of it.
            let kept: BTreeSet<Exclusion> = entries
                .iter()
                .filter(|entry| entry.relation.relation() == rule.relation())
                .map(|e| (e.from.clone(), e.to.clone(), e.root.clone()))
                .collect();
            let dropped = entries
                .iter()
                .filter(|entry| entry.relation.relation() != rule.relation())
                .count();
            rules.push(RuleSet {
                rule,
                entries: kept,
                dropped,
            });
        }
        Ok(Some(Self { rules }))
    }

    /// The proofs the two variables describe. `Ok(None)` when no rule is on (the file is then not
    /// read); a value that is not UTF-8, an unreadable file or any other misconfiguration is an
    /// `Err` naming the variable.
    pub(super) fn from_env_values(
        file: Option<OsString>,
        switches: Option<OsString>,
    ) -> Result<Option<Self>, String> {
        let switches = utf8(SWITCHES, switches)?;
        let file = utf8(FILE, file)?;
        let on = parse_switches(switches.as_deref())?;
        // The file is read only when a rule is on, exactly as `from_parts` decides.
        let text = match file.filter(|_| !on.is_empty()) {
            Some(path) => Some(
                std::fs::read_to_string(&path)
                    .map_err(|error| format!("{FILE}: cannot read {path}: {error}"))?,
            ),
            None => None,
        };
        Self::from_parts(text.as_deref(), switches.as_deref())
    }

    /// One line per switched-on rule: the filed entries kept, and those dropped because their
    /// relation is not the rule's.
    pub(super) fn summary(&self) -> String {
        self.rules
            .iter()
            .map(|set| {
                format!(
                    "{SUMMARY}rule={} kept={} dropped_relation_mismatch={}\n",
                    set.rule.name(),
                    set.entries.len(),
                    set.dropped
                )
            })
            .collect()
    }

    /// Loads the proofs from the given environment values and prints their summary to stderr;
    /// panics on a misconfiguration (loud on purpose).
    pub(super) fn load(file: Option<OsString>, switches: Option<OsString>) -> Option<Self> {
        let proofs =
            Self::from_env_values(file, switches).unwrap_or_else(|message| panic!("{message}"));
        if let Some(proofs) = &proofs {
            // A diagnostic only: it has no bearing on the walk.
            let _ = std::io::stderr()
                .lock()
                .write_all(proofs.summary().as_bytes());
        }
        proofs
    }

    /// The process-wide proofs, read once; panics on a misconfiguration.
    pub(super) fn global() -> Option<&'static WalkProofs> {
        static PROOFS: OnceLock<Option<WalkProofs>> = OnceLock::new();
        PROOFS
            .get_or_init(|| Self::load(std::env::var_os(FILE), std::env::var_os(SWITCHES)))
            .as_ref()
    }

    /// The reason the first switched-on rule filing this hop refuses it with.
    pub(super) fn refusal_for_keys(
        &self,
        relation: Relation,
        from: &str,
        to: &str,
        root: &str,
    ) -> Option<LossReason> {
        self.rules
            .iter()
            .filter(|set| set.rule.relation() == relation)
            .find(|set| {
                let hop = |root| (from.to_owned(), to.to_owned(), root);
                set.entries.contains(&hop(None))
                    || set.entries.contains(&hop(Some(root.to_owned())))
            })
            .map(|set| set.rule.reason())
    }
}

impl CodePropertyGraph {
    /// Whether a switched-on walk-proof rule refuses the hop `from -> to` under `root`; `None`
    /// (one `Option` test) when no switch is on.
    pub(super) fn walk_proof_refusal(
        &self,
        relation: Relation,
        from: NodeIndex,
        to: NodeIndex,
        root: NodeIndex,
    ) -> Option<LossReason> {
        with_proofs(|proofs| {
            proofs?.refusal_for_keys(
                relation,
                &self.trace_key(from),
                &self.trace_key(to),
                &self.trace_key(root),
            )
        })
    }
}
